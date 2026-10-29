//! Direct insertion reuses the engine's atomic split operation.
use super::*;

impl DrawApp {
    pub(super) fn insertion_target(&self, kind: ShapeKind, point: Point) -> Option<u64> {
        if kind == ShapeKind::Text
            || self.pending_symbol.is_some()
            || self.doc.hit_node(point).is_some()
        {
            return None;
        }
        self.hit_connector_cached(point, 8. / self.zoom.max(0.01))
            .filter(|id| self.doc.can_edit_object(*id))
    }

    pub(super) fn try_insert_on_connector(&mut self, kind: ShapeKind, point: Point) -> bool {
        let Some(edge) = self.insertion_target(kind, point) else {
            return false;
        };
        match self.doc.insert_shape_on_connector(edge, kind, point) {
            Ok(id) => {
                let node = self.doc.node_mut(id).unwrap();
                node.text = t(&node.text);
                node.text_style.wrap = true;
                self.selected = vec![id];
                self.set_tool(Tool::Select);
                self.commit();
            }
            Err(error) => self.error = Some(error),
        }
        // A failed split must not silently create a disconnected block.
        true
    }

    pub(super) fn paint_connector_insertion(&self, painter: &egui::Painter, pointer: Option<Pos2>) {
        let Tool::Shape(kind) = self.tool else {
            return;
        };
        if self.rich_editor.is_some() || self.drag.is_some() || self.context_open {
            return;
        }
        let Some(pointer) = pointer else {
            return;
        };
        let Some(id) = self.insertion_target(kind, self.world(pointer)) else {
            return;
        };
        let points: Vec<_> = self
            .cached_route(id)
            .into_iter()
            .map(|p| self.screen(p))
            .collect();
        painter.add(egui::Shape::line(points, Stroke::new(3., ACCENT)));
        painter.circle_filled(pointer, 9., Color32::WHITE);
        painter.circle_stroke(pointer, 9., Stroke::new(1.5, ACCENT));
        for delta in [vec2(4., 0.), vec2(0., 4.)] {
            painter.line_segment([pointer - delta, pointer + delta], Stroke::new(1.5, ACCENT));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn setup() -> (DrawApp, u64) {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let a = doc.add_node(ShapeKind::Rectangle, [200., 300.]);
        let b = doc.add_node(ShapeKind::Rectangle, [1000., 300.]);
        let edge = doc.connect(a, b, ConnectorKind::Straight).unwrap();
        doc.connectors[0].label = "Keep 中文".into();
        doc.connectors[0].color = [130, 20, 50];
        (DrawApp::from_context(&ctx, doc, None, None), edge)
    }
    #[test]
    fn direct_insertion_preserves_link_and_is_one_undo() {
        let (mut app, edge) = setup();
        app.tool = Tool::Shape(ShapeKind::Diamond);
        let before = app.doc.clone();
        assert!(app.try_insert_on_connector(ShapeKind::Diamond, [600., 300.]));
        let after = app.doc.clone();
        after.validate().unwrap();
        assert_eq!(after.nodes.len(), 3);
        assert_eq!(after.connectors.len(), 2);
        let first = after.connectors.iter().find(|c| c.id == edge).unwrap();
        assert_eq!(first.label, "Keep 中文");
        assert_eq!(first.from, before.connectors[0].from);
        assert_eq!(first.to, app.selected[0]);
        let second = after.connectors.iter().find(|c| c.id != edge).unwrap();
        assert_eq!(second.from, first.to);
        assert_eq!(second.to, before.connectors[0].to);
        assert_eq!(second.color, first.color);
        assert!(second.label.is_empty());
        app.undo();
        assert_eq!(app.doc, before);
        app.redo();
        assert_eq!(app.doc, after);
    }
    #[test]
    fn failed_insert_is_atomic_and_keeps_shape_tool() {
        let (mut app, _) = setup();
        app.tool = Tool::Shape(ShapeKind::Diamond);
        let before = app.doc.clone();
        assert!(app.try_insert_on_connector(ShapeKind::Diamond, [340., 300.]));
        assert!(app.error.is_some());
        assert_eq!(app.doc, before);
        assert!(app.tool == Tool::Shape(ShapeKind::Diamond));
        assert!(!app.history.can_undo());
        assert!(!app.try_insert_on_connector(ShapeKind::Text, [600., 300.]));
        assert!(!app.try_insert_on_connector(ShapeKind::Diamond, [600., 500.]));
        assert!(!app.try_insert_on_connector(ShapeKind::Diamond, [200., 300.]));
    }
    #[test]
    fn protected_links_and_personal_symbols_are_not_split() {
        let (mut app, edge) = setup();
        app.doc.set_objects_locked(&[edge], true).unwrap();
        let before = app.doc.clone();
        assert!(!app.try_insert_on_connector(ShapeKind::Diamond, [600., 300.]));
        assert_eq!(app.doc, before);
        app.doc.set_objects_locked(&[edge], false).unwrap();
        app.pending_symbol = Some(app.doc.nodes[0].clone());
        assert!(!app.try_insert_on_connector(ShapeKind::Diamond, [600., 300.]));
        app.pending_symbol = None;
        let layer = app.doc.create_layer("Hidden".into()).unwrap();
        app.doc.move_objects_to_layer(&[edge], layer).unwrap();
        app.doc
            .update_layer(layer, None, Some(false), None)
            .unwrap();
        let before = app.doc.clone();
        assert!(!app.try_insert_on_connector(ShapeKind::Diamond, [600., 300.]));
        assert_eq!(app.doc, before);
    }
    #[test]
    fn manual_waypoints_survive_the_split_in_order() {
        let (mut app, edge) = setup();
        let link = &mut app.doc.connectors[0];
        link.kind = ConnectorKind::Elbow;
        link.waypoints = vec![[400., 300.], [800., 300.]];
        app.history = History::new(&app.doc);
        let before = app.doc.clone();
        assert!(app.try_insert_on_connector(ShapeKind::Rectangle, [600., 300.]));
        let first = app.doc.connectors.iter().find(|c| c.id == edge).unwrap();
        let second = app.doc.connectors.iter().find(|c| c.id != edge).unwrap();
        assert!(first.waypoints.contains(&[400., 300.]));
        assert!(second.waypoints.contains(&[800., 300.]));
        app.undo();
        assert_eq!(app.doc, before);
    }
}
