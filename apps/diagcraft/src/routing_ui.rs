//! Geometry-only route cache: camera, labels and selection never change a path.
use super::*;
use diagcraft::routing::{RouteStatus, RoutedConnector, RoutingContext};
use std::{collections::BTreeMap, sync::Arc};
#[derive(Default)]
pub(super) struct Cache {
    key: Option<Key>,
    routes: Arc<BTreeMap<u64, RoutedConnector>>,
}
#[derive(PartialEq)]
struct Key {
    nodes: Vec<(u64, ShapeKind, Point, Point, bool)>,
    charts: Vec<(u64, Point, Point, bool)>,
    connectors: Vec<EdgeGeometry>,
}
#[derive(PartialEq)]
struct EdgeGeometry {
    id: u64,
    from: u64,
    to: u64,
    free_start: Option<Point>,
    free_end: Option<Point>,
    kind: ConnectorKind,
    from_port: Port,
    to_port: Port,
    waypoints: Vec<Point>,
    width: f32,
}
impl From<&document::Connector> for EdgeGeometry {
    fn from(e: &document::Connector) -> Self {
        Self {
            id: e.id,
            from: e.from,
            to: e.to,
            free_start: e.free_start,
            free_end: e.free_end,
            kind: e.kind,
            from_port: e.from_port,
            to_port: e.to_port,
            waypoints: e.waypoints.clone(),
            width: e.width,
        }
    }
}
impl Cache {
    pub(super) fn scene(&mut self, doc: &Document) -> Arc<BTreeMap<u64, RoutedConnector>> {
        let view = doc.organization_view();
        let key = Key {
            nodes: doc
                .nodes
                .iter()
                .map(|n| {
                    (
                        n.id,
                        n.kind,
                        n.position,
                        n.size,
                        view.is_object_visible(n.id),
                    )
                })
                .collect(),
            charts: doc
                .charts
                .iter()
                .map(|c| (c.id, c.position, c.size, view.is_object_visible(c.id)))
                .collect(),
            connectors: doc.connectors.iter().map(EdgeGeometry::from).collect(),
        };
        if self.key.as_ref() != Some(&key) {
            let context = RoutingContext::new(doc);
            self.routes = Arc::new(
                doc.connectors
                    .iter()
                    .map(|e| (e.id, context.route(e)))
                    .collect(),
            );
            self.key = Some(key);
        }
        self.routes.clone()
    }
}
impl DrawApp {
    pub(super) fn cached_route(&self, id: u64) -> Vec<Point> {
        self.route_cache
            .borrow_mut()
            .scene(&self.doc)
            .get(&id)
            .map(|r| r.points.clone())
            .unwrap_or_default()
    }
    pub(super) fn hit_connector_cached(&self, point: Point, tolerance: f32) -> Option<u64> {
        let routes = self.route_cache.borrow_mut().scene(&self.doc);
        let view = self.doc.organization_view();
        self.doc
            .connectors
            .iter()
            .enumerate()
            .filter(|(_, e)| {
                view.is_object_visible(e.id)
                    && routes[&e.id]
                        .points
                        .windows(2)
                        .any(|p| document::segment_distance(point, p[0], p[1]) <= tolerance)
            })
            .max_by_key(|(index, e)| (view.layer_order(view.object_layer(e.id)), *index))
            .map(|(_, e)| e.id)
    }
    pub(super) fn routing_notice(&mut self, ui: &mut egui::Ui) {
        if self.selected.len() != 1 {
            return;
        }
        let Some(edge) = self
            .doc
            .connectors
            .iter()
            .find(|e| e.id == self.selected[0])
        else {
            return;
        };
        if edge.kind != ConnectorKind::Elbow {
            return;
        }
        let routes = self.route_cache.borrow_mut().scene(&self.doc);
        match routes[&edge.id].status {
            RouteStatus::Clear | RouteStatus::Direct => {
                ui.small(t(
                    "Le tracé contourne les blocs ; les points manuels restent fixes.",
                ));
            }
            RouteStatus::Constrained => {
                ui.colored_label(
                    Color32::from_rgb(170, 100, 20),
                    t("Passage bloqué : déplacez le bloc, l’extrémité ou le point manuel."),
                );
            }
            RouteStatus::Limited => {
                ui.colored_label(
                    Color32::from_rgb(170, 100, 20),
                    t("Tracé simplifié : espacez les blocs ou ajoutez un point de passage."),
                );
            }
        }
    }
    pub(super) fn prepare_routing_capture(&mut self) {
        let mut d = Document {
            title: "Des liens qui trouvent leur chemin".into(),
            page_size: [1280., 800.],
            ..Document::default()
        };
        let mut add = |pos: Point, size: Point, text: &str| {
            let id = d.add_node(ShapeKind::Rounded, pos);
            let n = d.node_mut(id).unwrap();
            n.position = pos;
            n.size = size;
            n.text = text.into();
            id
        };
        let a = add([80., 280.], [180., 80.], "Départ");
        let b = add([1020., 280.], [180., 80.], "Arrivée");
        add([420., 180.], [160., 300.], "Un bloc à contourner");
        add([720., 300.], [150., 250.], "Un autre obstacle");
        d.connect(a, b, ConnectorKind::Elbow).unwrap();
        d.connectors[0].from_port = Port::Right;
        d.connectors[0].to_port = Port::Left;
        let id = d
            .connect_endpoints(a, 0, None, Some([1050., 650.]), ConnectorKind::Elbow)
            .unwrap();
        let e = d.connectors.iter_mut().find(|e| e.id == id).unwrap();
        e.from_port = Port::Bottom;
        e.waypoints = vec![[340., 600.], [940., 600.]];
        e.label = "Points de passage conservés".into();
        e.color = [21, 151, 111];
        self.doc = d;
        self.history = History::new(&self.doc);
        self.selected = vec![id];
        self.show_library = false;
        self.show_inspector = true;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        self.fit_next = true;
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scene_reuses_routes_and_invalidates_for_geometry_and_visibility() {
        let mut d = Document::demo();
        let mut cache = Cache::default();
        let first = cache.scene(&d);
        d.title = "Only a title".into();
        d.nodes[0].text = "Only text".into();
        d.connectors[0].label = "Only a label".into();
        d.connectors[0].label_t = 0.2;
        assert!(Arc::ptr_eq(&first, &cache.scene(&d)));
        d.nodes[0].position[0] += 20.;
        assert!(!Arc::ptr_eq(&first, &cache.scene(&d)));
        let last = cache.scene(&d);
        d.connectors[0].waypoints = vec![[300., 600.]];
        assert!(!Arc::ptr_eq(&last, &cache.scene(&d)));
        let layer = d.create_layer("Hidden".into()).unwrap();
        let id = d.nodes[1].id;
        d.move_objects_to_layer(&[id], layer).unwrap();
        let last = cache.scene(&d);
        d.update_layer(layer, None, Some(false), None).unwrap();
        assert!(!Arc::ptr_eq(&last, &cache.scene(&d)));
        for e in &d.connectors {
            assert_eq!(cache.scene(&d)[&e.id], d.route_details(e));
        }
    }
}
