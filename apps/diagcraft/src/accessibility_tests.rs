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
