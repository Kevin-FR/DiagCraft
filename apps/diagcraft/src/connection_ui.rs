//! Direct linking gestures, shared magnetic targets and truthful route previews.
use super::*;
const LINK_RADIUS: f32 = 13.; // screen points, independent of document zoom
#[derive(Clone, Copy)]
pub(super) struct LinkTarget {
    pub id: u64,
    pub port: Port,
}
impl DrawApp {
    pub(super) fn clear_connection(&mut self) {
        self.connect_from = None;
        self.connect_origin = None;
        self.connect_port = Port::Auto;
        self.hover_link_node = None;
        if matches!(self.drag, Some(Drag::Connect { .. })) {
            self.drag = None;
        }
    }
    pub(super) fn activate_connect_tool(&mut self) {
        let nodes = self
            .selected
            .iter()
            .copied()
            .filter(|id| self.doc.node(*id).is_some())
            .collect::<Vec<_>>();
        self.set_tool(Tool::Connect);
        if nodes.len() == 1 && self.selected.len() == 1 {
            self.begin_connection(nodes[0], Port::Auto, None, None);
        } else if nodes.len() == 2 && self.selected.len() == 2 {
            self.begin_connection(nodes[0], Port::Auto, None, None);
            let center = self.screen(self.doc.node(nodes[1]).unwrap().center());
            let mut draft = self.draft_connector(center).unwrap();
            // Explicit selection must win over hit testing on overlapping blocks.
            draft.to = nodes[1];
            draft.to_port = Port::Auto;
            draft.free_end = None;
            self.finish_draft(draft);
        }
    }
    pub(super) fn begin_connection(
        &mut self,
        id: u64,
        port: Port,
        free: Option<Point>,
        press: Option<Pos2>,
    ) {
        self.clear_connection();
        self.context_open = false;
        self.quick_link_port = None;
        self.tool = Tool::Connect;
        self.connect_from = Some(id);
        self.connect_origin = free;
        self.connect_port = port;
        self.selected = if id == 0 { vec![] } else { vec![id] };
        self.drag = press.map(|press| Drag::Connect { press });
        self.status =
            "Glissez vers un autre bloc, ou cliquez sur le bloc d’arrivée · Échap pour annuler."
                .into();
    }
    pub(super) fn connection_target(&self, pos: Pos2) -> Option<LinkTarget> {
        let world = self.world(pos);
        // Actual shape hits take precedence over another shape's nearby handles.
        let top = self.organized_hit(world);
        if top.is_some_and(|id| self.doc.node(id).is_none()) {
            return None;
        }
        let direct = top;
        let view = self.doc.organization_view();
        let mut nodes = self
            .doc
            .nodes
            .iter()
            .filter(|n| view.is_object_visible(n.id) && direct.is_none_or(|id| id == n.id))
            .collect::<Vec<_>>();
        nodes.sort_by_key(|n| view.layer_order(view.object_layer(n.id)));
        for n in nodes.into_iter().rev() {
            let rect = self.node_screen_rect(n);
            let explicit = connection_buttons(rect)
                .into_iter()
                .find(|(p, _)| p.distance(pos) <= LINK_RADIUS)
                .map(|(_, p)| p)
                .or_else(|| {
                    [Port::Top, Port::Right, Port::Bottom, Port::Left]
                        .into_iter()
                        .find(|p| {
                            self.screen(n.port_position(*p, world)).distance(pos) <= LINK_RADIUS
                        })
                });
            if direct == Some(n.id)
                || explicit.is_some()
                || self.screen(n.boundary_towards(world)).distance(pos) <= LINK_RADIUS
            {
                return Some(LinkTarget {
                    id: n.id,
                    port: explicit.unwrap_or(Port::Auto),
                });
            }
        }
        None
    }
    pub(super) fn node_screen_rect(&self, n: &Node) -> Rect {
        Rect::from_min_size(
            self.screen(n.position),
            vec2(n.size[0], n.size[1]) * self.zoom,
        )
    }
    pub(super) fn update_link_hover(&mut self, pos: Option<Pos2>, over: bool) {
        if !over || self.context_open {
            self.hover_link_node = None;
            return;
        }
        let Some(pos) = pos else {
            self.hover_link_node = None;
            return;
        };
        if let Some(id) = self
            .organized_hit(self.world(pos))
            .filter(|id| self.doc.node(*id).is_some())
        {
            self.hover_link_node = Some(id);
        } else if !self
            .hover_link_node
            .and_then(|id| self.doc.node(id))
            .is_some_and(|n| self.node_screen_rect(n).expand(42.).contains(pos))
        {
            self.hover_link_node = None;
        }
    }
    pub(super) fn connection_handle_at(&self, pos: Pos2) -> Option<(u64, Port)> {
        if self.tool != Tool::Select || self.format_brush.is_some() || self.edge_brush.is_some() {
            return None;
        }
        self.connection_handle_node().and_then(|n| {
            connection_buttons(self.node_screen_rect(n))
                .into_iter()
                .find(|(p, _)| p.distance(pos) <= LINK_RADIUS)
                .map(|(_, p)| (n.id, p))
        })
    }
    pub(super) fn connection_handle_node(&self) -> Option<&Node> {
        self.hover_link_node
            .filter(|id| self.doc.is_object_visible(*id))
            .and_then(|id| self.doc.node(id))
            .or_else(|| {
                (self.selected.len() == 1 && self.doc.is_object_visible(self.selected[0]))
                    .then(|| self.doc.node(self.selected[0]))
                    .flatten()
            })
    }
    fn draft_connector(&self, pos: Pos2) -> Option<document::Connector> {
        let from = self.connect_from?;
        let target = self.connection_target(pos);
        Some(document::Connector {
            id: 0,
            from,
            to: target.map_or(0, |t| t.id),
            free_start: self.connect_origin,
            free_end: target.is_none().then_some(self.world(pos)),
            kind: self.connector_kind,
            label: String::new(),
            color: [0, 125, 250],
            width: 1.8,
            arrow: true,
            arrow_start: false,
            from_port: self.connect_port,
            to_port: target.map_or(Port::Auto, |t| t.port),
            waypoints: vec![],
            label_t: 0.5,
            pattern: LinePattern::Solid,
            start_head: ArrowHead::Triangle,
            end_head: ArrowHead::Triangle,
            appearance: Default::default(),
        })
    }
    pub(super) fn finish_connection(&mut self, pos: Pos2) -> bool {
        let Some(draft) = self.draft_connector(pos) else {
            return false;
        };
        self.finish_draft(draft)
    }
    fn finish_draft(&mut self, draft: document::Connector) -> bool {
        if draft.from != 0 && draft.to == draft.from {
            self.status = "Choisissez un autre bloc pour terminer le lien.".into();
            return false;
        }
        match self.doc.connect_endpoints(
            draft.from,
            draft.to,
            draft.free_start,
            draft.free_end,
            draft.kind,
        ) {
            Ok(id) => {
                let edge = self.doc.connectors.iter_mut().find(|e| e.id == id).unwrap();
                edge.from_port = draft.from_port;
                edge.to_port = draft.to_port;
                let attached = edge.from != 0 && edge.to != 0;
                self.set_tool(Tool::Select);
                self.selected = vec![id];
                self.hover_link_node = None;
                self.commit();
                self.status = if attached {
                    "Blocs reliés · le lien suit leurs déplacements · ⌘ Z pour annuler."
                } else {
                    "Lien créé avec une extrémité libre · glissez-la sur un bloc pour l’accrocher."
                }
                .into();
                true
            }
            Err(e) => {
                self.status = e;
                false
            }
        }
    }
    pub(super) fn paint_connection_preview(&self, p: &egui::Painter) {
        let pointer = if self.capture_link_preview {
            Some(self.screen(self.doc.nodes[1].center()))
        } else {
            p.ctx().input(|i| i.pointer.interact_pos())
        };
        let Some(pos) = pointer else {
            return;
        };
        if !self.canvas.contains(pos) {
            return;
        }
        if let Some(Drag::Connect { press }) = self.drag
            && press.distance(pos) < 6.
        {
            if let Some(n) = self.connect_from.and_then(|id| self.doc.node(id)) {
                p.rect_stroke(
                    self.node_screen_rect(n).expand(5.),
                    6,
                    Stroke::new(2., ACCENT),
                    StrokeKind::Outside,
                );
            }
            paint_link_hint(
                p,
                self.canvas,
                pos,
                "Glissez vers un autre bloc, ou cliquez sur l’arrivée",
                ACCENT,
            );
            return;
        }
        let target = self.connection_target(pos);
        let (edge, source, invalid, dragging) =
            if let Some(Drag::Endpoint { id, start, point }) = &self.drag {
                let Some(original) = self.doc.connectors.iter().find(|e| e.id == *id) else {
                    return;
                };
                let mut edge = original.clone();
                let other = if *start { edge.to } else { edge.from };
                let node = target.map_or(0, |t| t.id);
                let port = target.map_or(Port::Auto, |t| t.port);
                if *start {
                    edge.from = node;
                    edge.from_port = port;
                    edge.free_start = (node == 0).then_some(*point);
                } else {
                    edge.to = node;
                    edge.to_port = port;
                    edge.free_end = (node == 0).then_some(*point);
                }
                (
                    edge,
                    None,
                    target.is_some_and(|t| t.id == other && other != 0),
                    true,
                )
            } else if let Some(edge) = self.draft_connector(pos) {
                let invalid = edge.from != 0
                    && edge.to != 0
                    && (edge.from == edge.to
                        || self
                            .doc
                            .connectors
                            .iter()
                            .any(|e| e.from == edge.from && e.to == edge.to));
                let source = Some(edge.from);
                (
                    edge,
                    source,
                    invalid,
                    matches!(self.drag, Some(Drag::Connect { .. })),
                )
            } else {
                return;
            };
        if let Some(source) = source.and_then(|id| self.doc.node(id)) {
            p.rect_stroke(
                self.node_screen_rect(source).expand(5.),
                6,
                Stroke::new(2., ACCENT),
                StrokeKind::Outside,
            );
        }
        let color = if invalid {
            Color32::from_rgb(202, 66, 66)
        } else {
            ACCENT
        };
        if let Some(n) = target.and_then(|t| self.doc.node(t.id)) {
            p.rect_stroke(
                self.node_screen_rect(n).expand(5.),
                6,
                Stroke::new(
                    2.5,
                    if invalid {
                        color
                    } else {
                        Color32::from_rgb(21, 151, 111)
                    },
                ),
                StrokeKind::Outside,
            );
            for (_, port) in connection_buttons(self.node_screen_rect(n)) {
                let at = self.screen(n.port_position(port, self.world(pos)));
                p.circle(
                    at,
                    4.,
                    Color32::WHITE,
                    Stroke::new(1.5, if invalid { color } else { ACCENT }),
                );
            }
        }
        let points = self
            .doc
            .route(&edge)
            .into_iter()
            .map(|p| self.screen(p))
            .collect::<Vec<_>>();
        if !invalid {
            paint_pattern(p, &points, Stroke::new(2., color), LinePattern::Dashed);
            if edge.arrow {
                paint_head(
                    p,
                    &points,
                    color,
                    10.,
                    2.,
                    edge.end_head,
                    rgb(self.doc.page_style.background),
                );
            }
        }
        let endpoint = points.last().copied().unwrap_or(pos);
        p.circle(endpoint, 6., Color32::WHITE, Stroke::new(2., color));
        let hint = if invalid {
            if source == target.map(|t| t.id) || matches!(self.drag, Some(Drag::Endpoint { .. })) {
                "Choisissez un autre bloc"
            } else {
                "Ces blocs sont déjà reliés"
            }
        } else if target.is_some() {
            if dragging {
                "Relâchez pour relier"
            } else {
                "Cliquez pour relier"
            }
        } else if dragging {
            "Relâchez pour laisser une extrémité libre"
        } else {
            "Cliquez pour une extrémité libre"
        };
        paint_link_hint(p, self.canvas, pos, hint, color);
    }
}
pub(super) fn paint_link_hint(
    p: &egui::Painter,
    canvas: Rect,
    at: Pos2,
    text: &str,
    color: Color32,
) {
    let galley = p.layout_no_wrap(t(text), FontId::proportional(12.), color);
    let size = galley.size() + vec2(16., 10.);
    let origin = pos2(
        (at.x + 18.).clamp(
            canvas.left() + 6.,
            (canvas.right() - size.x - 6.).max(canvas.left() + 6.),
        ),
        (at.y + 24.).clamp(
            canvas.top() + 6.,
            (canvas.bottom() - size.y - 6.).max(canvas.top() + 6.),
        ),
    );
    let r = Rect::from_min_size(origin, size);
    p.rect_filled(r, 5, Color32::WHITE);
    p.rect_stroke(r, 5, Stroke::new(1., BORDER), StrokeKind::Inside);
    p.galley(origin + vec2(8., 5.), galley, color);
}
