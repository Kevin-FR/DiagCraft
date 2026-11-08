use super::*;
fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::default(), None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.show_library = false;
    app.show_quick_toolbar = false;
    (ctx, app)
}
fn frame(ctx: &egui::Context, app: &mut DrawApp, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
            events,
            ..Default::default()
        },
        |ctx| {
            let before = app.doc.clone();
            app.inspector(ctx);
            app.canvas(ctx, !app.image_ui.png_open);
            app.finish_frame_edit(ctx, &before);
            app.png_dialog(ctx);
        },
    )
}
#[test]
fn document_description_edit_is_a_single_undo_step() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    assert_eq!(before.description, String::new());
    app.doc.description = "Schéma d'architecture des flux".into();
    frame(&ctx, &mut app, vec![]);
    assert_eq!(app.doc.description, "Schéma d'architecture des flux");
    app.undo();
    assert_eq!(app.doc, before);
    // A second edit then undoing again still returns to the original state.
    app.doc.description = "Reprise".into();
    frame(&ctx, &mut app, vec![]);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn document_description_reaches_svg_desc_and_pdf_subject() {
    let (ctx, mut app) = setup();
    app.doc.title = "Mon diagramme".into();
    app.doc.description = "Une architecture claire".into();
    frame(&ctx, &mut app, vec![]);
    let svg_source = svg::export(&app.doc).unwrap();
    assert!(svg_source.contains("<desc>Une architecture claire</desc>"));
    let pdf_bytes = pdf::export(&app.doc, pdf::PageFormat::Drawing).unwrap();
    assert!(
        pdf_bytes
            .windows(b"/Subject".len())
            .any(|w| w == b"/Subject")
    );
}

#[test]
fn named_style_picker_apply_is_single_undo() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [400., 300.]);
    app.doc
        .create_named_style("Style A", [10, 10, 10], [2, 2, 2], [3, 3, 3], 2.0, 14.0)
        .unwrap();
    // Commit node creation + style definition as the base history point.
    app.commit();
    let before_apply = app.doc.clone();
    // Same action the node-style picker performs when the user picks a style.
    app.doc
        .apply_named_style_to_nodes(&[id], "Style A")
        .unwrap();
    app.commit();
    let node = app.doc.node(id).unwrap();
    assert_eq!(node.style_ref.as_deref(), Some("Style A"));
    assert_eq!(node.fill, [10, 10, 10]);
    app.undo();
    assert_eq!(app.doc, before_apply);
    let _ = ctx;
}

#[test]
fn named_style_picker_detach_is_single_undo() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [400., 300.]);
    app.doc
        .create_named_style("Style A", [10, 10, 10], [2, 2, 2], [3, 3, 3], 2.0, 14.0)
        .unwrap();
    app.commit();
    // Base state to detach from: the node carries the style reference.
    app.doc
        .apply_named_style_to_nodes(&[id], "Style A")
        .unwrap();
    app.commit();
    let before_detach = app.doc.clone();
    assert_eq!(
        app.doc.node(id).unwrap().style_ref.as_deref(),
        Some("Style A")
    );
    // The "Aucun" picker branch detaches.
    app.doc.detach_named_style(&[id]);
    app.commit();
    assert_eq!(app.doc.node(id).unwrap().style_ref, None);
    app.undo();
    assert_eq!(app.doc, before_detach);
    let _ = ctx;
}

#[test]
fn named_style_edit_propagates_to_linked_node_in_one_undo() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 200.]);
    app.doc
        .create_named_style(
            "S",
            [233, 246, 241],
            [39, 128, 107],
            [32, 53, 64],
            1.5,
            12.0,
        )
        .unwrap();
    app.commit();
    app.doc.apply_named_style_to_nodes(&[id], "S").unwrap();
    app.commit();
    let before = app.doc.clone();
    assert_eq!(app.doc.node(id).unwrap().fill, [233, 246, 241]);
    // Changing a value through the panel (edit_named_style_fill) propagates.
    app.doc.edit_named_style_fill("S", [255, 0, 0]).unwrap();
    app.commit();
    assert_eq!(app.doc.node(id).unwrap().fill, [255, 0, 0]);
    app.undo();
    assert_eq!(app.doc, before);
    let _ = ctx;
}

#[test]
fn named_styles_panel_renders_document_branch_with_existing_style() {
    let (ctx, mut app) = setup();
    app.doc
        .create_named_style(
            "En-tête",
            [240, 240, 250],
            [10, 10, 80],
            [20, 20, 20],
            2.0,
            16.0,
        )
        .unwrap();
    // Empty selection: the document branch draws named_styles_inspector.
    app.selected.clear();
    frame(&ctx, &mut app, vec![]);
    assert_eq!(app.doc.named_style_names(), vec!["En-tête"]);
}
