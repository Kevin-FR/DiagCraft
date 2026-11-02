use super::*;

fn fixture(kind: usize, label: &str) -> (egui::Context, DrawApp, u64, Vec<Point>) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let id;
    let route;
    if kind < 2 {
        let a = doc.add_node(ShapeKind::Rectangle, [250., 250.]);
        let b = doc.add_node(ShapeKind::Rectangle, [900., 500.]);
        id = doc
            .connect(
                a,
                b,
                if kind == 0 {
                    ConnectorKind::Straight
                } else {
                    ConnectorKind::Elbow
                },
            )
            .unwrap();
        doc.connectors[0].label = label.into();
        route = doc.route(&doc.connectors[0]);
    } else {
        id = doc.add_drawing(
            if kind == 2 {
                DrawingKind::Line
            } else {
                DrawingKind::Bezier
            },
            [250., 300.],
            [900., 500.],
        );
        doc.drawings[0].bend = 60.;
        doc.drawings[0].label = label.into();
        route = doc.drawings[0].points();
    }
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.show_quick_toolbar = false;
    inline_frame(&ctx, &mut app, vec![]);
    (ctx, app, id, route)
}

fn label(doc: &Document, id: u64) -> (&str, f32, Point) {
    if let Some(edge) = doc.connectors.iter().find(|e| e.id == id) {
        (&edge.label, edge.label_t, edge.label_offset)
    } else {
        let d = doc.drawings.iter().find(|d| d.id == id).unwrap();
        (&d.label, d.label_t, d.label_offset)
    }
}

#[test]
fn clicking_then_typing_opens_line_text_without_losing_the_first_shortcut_letter() {
    for kind in 0..4 {
        let (ctx, mut app, id, route) = fixture(kind, "");
        let before = app.doc.clone();
        let point = [
            route[0][0] * 0.7 + route[1][0] * 0.3,
            route[0][1] * 0.7 + route[1][1] * 0.3,
        ];
        let at = app.screen(point);
        inline_pointer(&ctx, &mut app, at, true);
        inline_pointer(&ctx, &mut app, at, false);
        assert_eq!(app.selected, vec![id]);
        assert!(app.rich_editor.is_none());
        let view = (app.zoom, app.pan);
        inline_frame(
            &ctx,
            &mut app,
            vec![
                inline_key(Key::F, egui::Modifiers::NONE),
                egui::Event::Text("Flux 中文".into()),
            ],
        );
        assert!(app.editing_line_label(id));
        assert_eq!(app.rich_editor.as_ref().unwrap().draft.text, "Flux 中文");
        assert_eq!((app.zoom, app.pan), view);
        assert_eq!(app.doc, before);
        inline_frame(
            &ctx,
            &mut app,
            vec![inline_key(Key::Enter, egui::Modifiers::COMMAND)],
        );
        assert!(app.rich_editor.is_none());
        assert_eq!(label(&app.doc, id).0, "Flux 中文");
        assert!(
            (label(&app.doc, id).1 - diagcraft::routing::project_label(&route, point)).abs()
                < 0.001
        );
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}

#[test]
fn label_drag_is_free_in_both_axes_and_never_moves_the_path() {
    for kind in 0..4 {
        for cancel in [false, true] {
            let (ctx, mut app, id, _) = fixture(kind, "Déplacer librement");
            let before = app.doc.clone();
            let at = app.line_label_rect(&ctx, id).unwrap().center();
            inline_pointer(&ctx, &mut app, at, true);
            assert!(matches!(app.drag, Some(Drag::LineLabel { id: found, .. }) if found == id));
            let end = at + vec2(93., 77.);
            inline_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
            assert!(!app.history.can_undo());
            let offset = label(&app.doc, id).2;
            assert!((offset[0] - 93.).abs() < 0.001 && (offset[1] - 77.).abs() < 0.001);
            let mut normalized = app.doc.clone();
            if let Some(e) = normalized.connectors.iter_mut().find(|e| e.id == id) {
                e.label_offset = [0., 0.];
            }
            if let Some(d) = normalized.drawings.iter_mut().find(|d| d.id == id) {
                d.label_offset = [0., 0.];
            }
            assert_eq!(normalized, before);
            if cancel {
                inline_frame(
                    &ctx,
                    &mut app,
                    vec![inline_key(Key::Escape, egui::Modifiers::NONE)],
                );
                assert_eq!(app.doc, before);
                inline_pointer(&ctx, &mut app, end, false);
                assert!(!app.history.can_undo());
            } else {
                inline_pointer(&ctx, &mut app, end, false);
                let after = app.doc.clone();
                assert!(app.line_label_rect(&ctx, id).unwrap().contains(end));
                assert_eq!(app.hit_line_label(&ctx, end), Some(id));
                app.undo();
                assert_eq!(app.doc, before);
                assert!(!app.history.can_undo());
                app.redo();
                assert_eq!(app.doc, after);
            }
        }
    }
}

#[test]
fn labels_respect_visibility_occlusion_and_mixed_locked_selections() {
    let (ctx, mut app, id, _) = fixture(2, "Label");
    let at = app.line_label_rect(&ctx, id).unwrap().center();
    let blocker = app.doc.add_node(ShapeKind::Rectangle, app.world(at));
    app.commit();
    assert_eq!(
        app.hit_line_label(&ctx, at),
        None,
        "a block painted above the label owns the click"
    );
    app.doc.node_mut(blocker).unwrap().position = [1100., 700.];
    app.doc.set_objects_locked(&[blocker], true).unwrap();
    app.selected = vec![id, blocker];
    app.commit();
    let before = app.doc.clone();
    inline_pointer(&ctx, &mut app, at, true);
    assert!(app.drag.is_none());
    inline_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(at + vec2(90., 80.))],
    );
    inline_pointer(&ctx, &mut app, at + vec2(90., 80.), false);
    assert_eq!(app.doc, before);
    let layer = app.doc.create_layer("Hidden labels".into()).unwrap();
    app.doc.move_objects_to_layer(&[id], layer).unwrap();
    app.doc
        .update_layer(layer, None, Some(false), None)
        .unwrap();
    assert_eq!(app.hit_line_label(&ctx, at), None);
}

#[test]
fn line_text_keyboard_command_and_double_click_edit_the_existing_label() {
    for kind in 0..4 {
        let (ctx, mut app, id, _) = fixture(kind, "Existing");
        app.selected = vec![id];
        assert!(
            app.command_unavailable(commands_ui::Command::EditText)
                .is_none()
        );
        inline_frame(
            &ctx,
            &mut app,
            vec![inline_key(Key::F2, egui::Modifiers::NONE)],
        );
        assert!(app.editing_line_label(id));
        app.finish_inline_text(&ctx, true);
        inline_frame(&ctx, &mut app, vec![]);
        inline_frame(&ctx, &mut app, vec![]);
        let at = app.line_label_rect(&ctx, id).unwrap().center();
        for _ in 0..2 {
            inline_pointer(&ctx, &mut app, at, true);
            inline_pointer(&ctx, &mut app, at, false);
        }
        assert!(app.editing_line_label(id), "double click kind {kind}");
        assert_eq!(app.rich_editor.as_ref().unwrap().draft.text, "Existing");
    }
}

#[test]
fn labels_above_blocks_remain_draggable_in_their_layer_order() {
    for kind in [0, 2] {
        let (ctx, mut app, id, _) = fixture(kind, "Upper label");
        let at = app.line_label_rect(&ctx, id).unwrap().center();
        let blocker = app.doc.add_node(ShapeKind::Rectangle, app.world(at));
        let layer = app.doc.create_layer("Labels above".into()).unwrap();
        app.doc.move_objects_to_layer(&[id], layer).unwrap();
        app.commit();
        assert_eq!(app.doc.hit_node(app.world(at)), Some(blocker));
        assert_eq!(app.hit_line_label(&ctx, at), Some(id));
        inline_pointer(&ctx, &mut app, at, true);
        assert!(matches!(app.drag, Some(Drag::LineLabel { id: target, .. }) if target == id));
        inline_frame(
            &ctx,
            &mut app,
            vec![inline_key(Key::Escape, egui::Modifiers::NONE)],
        );
    }
}

#[test]
fn plain_paste_edits_a_line_but_diagram_paste_keeps_pasting_objects() {
    for kind in [0, 2] {
        let (ctx, mut app, id, route) = fixture(kind, "");
        let at = app.screen([
            (route[0][0] + route[1][0]) / 2.,
            (route[0][1] + route[1][1]) / 2.,
        ]);
        inline_pointer(&ctx, &mut app, at, true);
        inline_pointer(&ctx, &mut app, at, false);
        inline_frame(
            &ctx,
            &mut app,
            vec![
                inline_key(Key::V, egui::Modifiers::COMMAND),
                egui::Event::Paste("Collé 中文".into()),
            ],
        );
        assert!(app.editing_line_label(id));
        assert_eq!(app.rich_editor.as_ref().unwrap().draft.text, "Collé 中文");
        app.finish_inline_text(&ctx, true);
        inline_frame(&ctx, &mut app, vec![]);
        inline_frame(&ctx, &mut app, vec![]);
        let mut fragment = Document::default();
        fragment.add_node(ShapeKind::Ellipse, [300., 300.]);
        let count = app.doc.nodes.len();
        inline_frame(
            &ctx,
            &mut app,
            vec![
                inline_key(Key::V, egui::Modifiers::COMMAND),
                egui::Event::Paste(serde_json::to_string(&fragment).unwrap()),
            ],
        );
        assert!(app.rich_editor.is_none());
        assert_eq!(app.doc.nodes.len(), count + 1);
    }
}
