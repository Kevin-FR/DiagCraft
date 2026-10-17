//! Atomic diagram edits used by the contextual controls.
use crate::document::{ConnectorKind, Document, Node, Point, Port, ShapeKind};

impl Document {
    pub fn add_connector_waypoint(&mut self, id: u64, point: Point) -> Result<(), String> {
        self.validate()?;
        if !point.iter().all(|v| v.is_finite() && v.abs() <= 100_000.) {
            return Err("Position invalide.".into());
        }
        let mut next = self.clone();
        next.set_automatic_route(id, false)?;
        let edge = next.connectors.iter().find(|e| e.id == id).unwrap();
        if edge.waypoints.contains(&point) {
            *self = next;
            return Ok(());
        }
        if edge.waypoints.len() >= 16 {
            return Err("Maximum de 16 points de passage atteint.".into());
        }
        let route = next.route(edge);
        let start = *route.first().ok_or("Tracé indisponible.")?;
        let end = *route.last().unwrap();
        let mut previous = start;
        let mut closest = (f32::INFINITY, 0);
        for (i, p) in edge
            .waypoints
            .iter()
            .chain(std::iter::once(&end))
            .enumerate()
        {
            let corner = [p[0], previous[1]];
            let distance = distance(project(point, previous, corner), point)
                .min(distance(project(point, corner, *p), point));
            if distance < closest.0 {
                closest = (distance, i);
            }
            previous = *p;
        }
        next.connectors
            .iter_mut()
            .find(|e| e.id == id)
            .unwrap()
            .waypoints
            .insert(closest.1, point);
        next.validate()?;
        *self = next;
        Ok(())
    }
    /// Preserve the displayed elbow route when switching to editable waypoints.
    pub fn set_automatic_route(&mut self, id: u64, automatic: bool) -> Result<(), String> {
        let index = self
            .connectors
            .iter()
            .position(|e| e.id == id)
            .ok_or("Connecteur introuvable.")?;
        if automatic {
            self.connectors[index].waypoints.clear();
            return Ok(());
        }
        if self.connectors[index].kind != ConnectorKind::Elbow {
            return Err("Les points de passage nécessitent un connecteur à angle droit.".into());
        }
        if !self.connectors[index].waypoints.is_empty() {
            return Ok(());
        }
        let route = self.route(&self.connectors[index]);
        if route.len() < 2 {
            return Err("Tracé indisponible.".into());
        }
        let from_port = self
            .node(self.connectors[index].from)
            .map(|n| cardinal_port(n, route[0]))
            .unwrap_or(Port::Auto);
        let to_port = self
            .node(self.connectors[index].to)
            .map(|n| cardinal_port(n, *route.last().unwrap()))
            .unwrap_or(Port::Auto);
        let mut points = route[1..route.len() - 1].to_vec();
        if points.is_empty() {
            points.push([
                (route[0][0] + route[1][0]) / 2.,
                (route[0][1] + route[1][1]) / 2.,
            ]);
        }
        let edge = &mut self.connectors[index];
        edge.from_port = from_port;
        edge.to_port = to_port;
        edge.waypoints = points;
        Ok(())
    }

    pub fn add_linked_shape(
        &mut self,
        from: u64,
        kind: ShapeKind,
        direction: Port,
    ) -> Result<u64, String> {
        self.validate()?;
        if self.object_count() + 2 > 10_000 {
            return Err("Limite de 10 000 objets atteinte.".into());
        }
        if direction == Port::Auto {
            return Err("Choisissez une direction.".into());
        }
        let source = self.node(from).ok_or("Forme introuvable.")?.clone();
        let mut next = self.clone();
        let id = next.add_node(kind, source.center());
        let n = next.node_mut(id).unwrap();
        n.position = source.position;
        match direction {
            Port::Right => n.position[0] += source.size[0] + 80.,
            Port::Bottom => n.position[1] += source.size[1] + 80.,
            Port::Left => n.position[0] -= n.size[0] + 80.,
            Port::Top => n.position[1] -= n.size[1] + 80.,
            Port::Auto => unreachable!(),
        }
        if n.position.iter().any(|v| *v < 0.) {
            return Err("Pas assez de place dans cette direction. Déplacez la forme ou choisissez un autre côté.".into());
        }
        n.fill = source.fill;
        n.stroke = source.stroke;
        n.stroke_width = source.stroke_width;
        n.text_color = source.text_color;
        n.font_size = source.font_size;
        n.text_style = source.text_style.clone();
        n.appearance = source.appearance.clone();
        n.text_style.wrap = true;
        let extent = [
            n.position[0] + n.size[0] + 40.,
            n.position[1] + n.size[1] + 40.,
        ];
        for (axis, v) in extent.into_iter().enumerate() {
            next.page_size[axis] = next.page_size[axis].max(v);
        }
        let edge_id = next
            .connect(from, id, ConnectorKind::Elbow)
            .ok_or("Connexion impossible.")?;
        let edge = next
            .connectors
            .iter_mut()
            .find(|e| e.id == edge_id)
            .unwrap();
        edge.from_port = direction;
        edge.to_port = opposite(direction);
        next.validate()?;
        *self = next;
        Ok(id)
    }

    /// Split an existing link at its nearest segment, retaining its label and style.
    pub fn insert_shape_on_connector(
        &mut self,
        edge_id: u64,
        kind: ShapeKind,
        point: Point,
    ) -> Result<u64, String> {
        self.validate()?;
        if !point.iter().all(|v| v.is_finite() && v.abs() <= 100_000.) {
            return Err("Position invalide.".into());
        }
        if self.object_count() + 2 > 10_000 {
            return Err("Limite de 10 000 objets atteinte.".into());
        }
        let original = self
            .connectors
            .iter()
            .find(|e| e.id == edge_id)
            .ok_or("Connecteur introuvable.")?
            .clone();
        let route = self.route(&original);
        let (segment, center) = route
            .windows(2)
            .enumerate()
            .map(|(i, p)| (i, project(point, p[0], p[1])))
            .min_by(|(_, a), (_, b)| distance(*a, point).total_cmp(&distance(*b, point)))
            .ok_or("Tracé indisponible.")?;
        let mut next = self.clone();
        let id = next.add_node(kind, center);
        let n = next.node(id).unwrap().clone();
        if n.position.iter().any(|v| *v < 0.) || self.nodes.iter().any(|old| overlaps(old, &n)) {
            return Err("Ce segment manque de place. Éloignez les formes avant d’insérer une forme sur le connecteur.".into());
        }
        let new_edge_id = next.next_id();
        let mut first = original.clone();
        first.to = id;
        first.free_end = None;
        first.to_port = Port::Auto;
        let mut second = original.clone();
        second.id = new_edge_id;
        second.from = id;
        second.free_start = None;
        second.from_port = Port::Auto;
        second.label.clear();
        second.arrow_start = false;
        if !original.waypoints.is_empty() {
            first.waypoints = route[1..=segment]
                .iter()
                .copied()
                .filter(|p| !n.contains(*p))
                .collect();
            second.waypoints = route[segment + 1..route.len() - 1]
                .iter()
                .copied()
                .filter(|p| !n.contains(*p))
                .collect();
        }
        *next
            .connectors
            .iter_mut()
            .find(|e| e.id == edge_id)
            .unwrap() = first;
        next.connectors.push(second);
        for (axis, v) in [
            n.position[0] + n.size[0] + 40.,
            n.position[1] + n.size[1] + 40.,
        ]
        .into_iter()
        .enumerate()
        {
            next.page_size[axis] = next.page_size[axis].max(v);
        }
        next.validate()?;
        *self = next;
        Ok(id)
    }
}
fn overlaps(a: &Node, b: &Node) -> bool {
    a.position[0] < b.position[0] + b.size[0]
        && b.position[0] < a.position[0] + a.size[0]
        && a.position[1] < b.position[1] + b.size[1]
        && b.position[1] < a.position[1] + a.size[1]
}
fn distance(a: Point, b: Point) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}
fn project(p: Point, a: Point, b: Point) -> Point {
    let v = [b[0] - a[0], b[1] - a[1]];
    let len = v[0] * v[0] + v[1] * v[1];
    let t = if len > 0.0001 {
        ((p[0] - a[0]) * v[0] + (p[1] - a[1]) * v[1]) / len
    } else {
        0.
    }
    .clamp(0., 1.);
    [a[0] + t * v[0], a[1] + t * v[1]]
}
fn cardinal_port(n: &Node, p: Point) -> Port {
    let c = n.center();
    if (p[0] - c[0]).abs() < 0.01 {
        if p[1] < c[1] { Port::Top } else { Port::Bottom }
    } else if (p[1] - c[1]).abs() < 0.01 {
        if p[0] < c[0] { Port::Left } else { Port::Right }
    } else {
        Port::Auto
    }
}
fn opposite(p: Port) -> Port {
    match p {
        Port::Left => Port::Right,
        Port::Right => Port::Left,
        Port::Top => Port::Bottom,
        Port::Bottom => Port::Top,
        Port::Auto => Port::Auto,
    }
}

impl Document {
    /// Reconnect one end without ever committing an invalid or duplicate relation.
    pub fn reconnect_connector(
        &mut self,
        id: u64,
        start: bool,
        target: u64,
        port: Port,
    ) -> Result<(), String> {
        self.validate()?;
        if self.node(target).is_none() {
            return Err("Forme introuvable.".into());
        }
        let mut next = self.clone();
        let edge = next
            .connectors
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("Connecteur introuvable.")?;
        if start {
            edge.from = target;
            edge.free_start = None;
            edge.from_port = port;
        } else {
            edge.to = target;
            edge.free_end = None;
            edge.to_port = port;
        }
        if edge.from == edge.to {
            return Err("Le connecteur doit relier deux formes différentes.".into());
        }
        let (from, to) = (edge.from, edge.to);
        if next
            .connectors
            .iter()
            .any(|e| e.id != id && e.from == from && e.to == to)
        {
            return Err("Ce lien existe déjà.".into());
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
}

impl Document {
    pub fn release_connector_end(
        &mut self,
        id: u64,
        start: bool,
        point: Point,
    ) -> Result<(), String> {
        self.validate()?;
        let mut next = self.clone();
        let e = next
            .connectors
            .iter_mut()
            .find(|e| e.id == id)
            .ok_or("Connecteur introuvable.")?;
        if start {
            e.from = 0;
            e.free_start = Some(point);
            e.from_port = Port::Auto;
        } else {
            e.to = 0;
            e.free_end = Some(point);
            e.to_port = Port::Auto;
        }
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn connect_endpoints(
        &mut self,
        from: u64,
        to: u64,
        free_start: Option<Point>,
        free_end: Option<Point>,
        kind: ConnectorKind,
    ) -> Result<u64, String> {
        self.validate()?;
        if self.object_count() >= 10_000 {
            return Err("Limite de 10 000 objets atteinte.".into());
        }
        if from != 0
            && self
                .connectors
                .iter()
                .any(|e| e.from == from && e.to == to && to != 0)
        {
            return Err("Ce lien existe déjà.".into());
        }
        let mut next = self.clone();
        let id = next.next_id();
        next.connectors.push(crate::document::Connector {
            id,
            from,
            to,
            free_start,
            free_end,
            kind,
            label: String::new(),
            color: [111, 128, 139],
            width: 1.8,
            arrow: true,
            arrow_start: false,
            from_port: Port::Auto,
            to_port: Port::Auto,
            waypoints: vec![],
            label_t: 0.5,
            pattern: Default::default(),
            start_head: Default::default(),
            end_head: Default::default(),
            appearance: Default::default(),
        });
        next.validate()?;
        let r = next.route(next.connectors.last().unwrap());
        if r.len() < 2
            || r.windows(2)
                .map(|p| (p[0][0] - p[1][0]).hypot(p[0][1] - p[1][1]))
                .sum::<f32>()
                < 2.
        {
            return Err("Le connecteur est trop court.".into());
        }
        *self = next;
        Ok(id)
    }
}
