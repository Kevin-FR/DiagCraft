use super::*;

fn numeric_resize_setup(kind: ShapeKind) -> (egui::Context, DrawApp, u64) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let id = doc.add_node(kind, [400., 300.]);
    let node = doc.node_mut(id).unwrap();
    node.size = [280., 120.];
    node.text =
        "Une phrase assez longue pour suivre la largeur du bloc sans modifier son contenu.".into();
    node.text_style.wrap = false;
    node.text_style.auto_fit = false;
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.set_language(Some(Language::French));
    app.selected = vec![id];
    app.show_inspector = true;
    app.show_library = false;
    app.show_quick_toolbar = false;
    app.inspector_width = 400.;
    app.ribbon_expanded = Some(false);
    (ctx, app, id)
}

fn numeric_width_control(output: &egui::FullOutput) -> Pos2 {
    let label = ux_text(output, "L");
    output
        .shapes
        .iter()
        .find_map(|shape| match &shape.shape {
            Shape::Text(text)
                if text.galley.job.text == "280"
                    && text.pos.x > label.x
                    && (text.pos.y + text.galley.size().y / 2. - label.y).abs() < 12. =>
            {
                Some(text.pos + text.galley.size() / 2.)
            }
            _ => None,
        })
        .expect("Width DragValue must be visible beside its label")
}

#[test]
fn numeric_width_drag_wraps_shapes_preserves_text_and_undoes_atomically() {
    let size = vec2(1440., 1200.);
    for kind in [ShapeKind::Rectangle, ShapeKind::Diamond, ShapeKind::Text] {
        let (ctx, mut app, id) = numeric_resize_setup(kind);
        let output = stable_panel(&ctx, &mut app, size);
        panel_click(
            &ctx,
            &mut app,
            size,
            ux_text(&output, "Position et dimensions"),
        );
        let output = stable_panel(&ctx, &mut app, size);
        let start = numeric_width_control(&output);
        let before = app.doc.clone();
        assert!(!app.history.can_undo());
        panel_pointer(&ctx, &mut app, size, start, true);
        let end = start - vec2(120., 0.);
        panel_frame(&ctx, &mut app, size, vec![egui::Event::PointerMoved(end)]);
        panel_pointer(&ctx, &mut app, size, end, false);
        let resized = app.doc.node(id).unwrap();
        let source = before.node(id).unwrap();
        assert!(
            resized.size[0] < source.size[0],
            "width did not shrink for {kind:?}"
        );
        assert_eq!(resized.size[1], source.size[1]);
        assert_eq!(resized.text_style.wrap, kind != ShapeKind::Text);
        assert!(!resized.text_style.auto_fit);
        let mut normalized = resized.clone();
        normalized.size = source.size;
        normalized.text_style = source.text_style.clone();
        assert_eq!(
            &normalized, source,
            "numeric resizing must preserve text and styling"
        );
        let after = app.doc.clone();
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        app.redo();
        assert_eq!(app.doc, after);
    }
}

#[test]
fn protected_shape_ignores_numeric_width_drag_without_enabling_wrap() {
    let size = vec2(1440., 1200.);
    let (ctx, mut app, id) = numeric_resize_setup(ShapeKind::Rectangle);
    let output = stable_panel(&ctx, &mut app, size);
    panel_click(
        &ctx,
        &mut app,
        size,
        ux_text(&output, "Position et dimensions"),
    );
    stable_panel(&ctx, &mut app, size);
    app.doc.set_objects_locked(&[id], true).unwrap();
    app.commit();
    let before = app.doc.clone();
    let output = stable_panel(&ctx, &mut app, size);
    let start = numeric_width_control(&output);
    panel_pointer(&ctx, &mut app, size, start, true);
    let end = start - vec2(120., 0.);
    panel_frame(&ctx, &mut app, size, vec![egui::Event::PointerMoved(end)]);
    panel_pointer(&ctx, &mut app, size, end, false);
    assert_eq!(app.doc, before);
}
