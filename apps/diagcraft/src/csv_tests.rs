use super::*;
fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
    app.fit_next = false;
    (ctx, app)
}
fn label_panel(panel: &mut Panel, source: &str, label: &str, connect: &[&str]) {
    panel.source = Some(source.into());
    panel.headers = diagcraft::csv::headers(source).unwrap();
    panel.label = panel.headers.iter().position(|h| h == label).unwrap_or(0);
    panel.connect_multi = panel
        .headers
        .iter()
        .map(|h| connect.iter().any(|c| c == h))
        .collect();
}
#[test]
fn csv_import_preview_preserves_pages_and_applies_as_a_single_undo() {
    let (_, mut app) = setup();
    let before = app.doc.clone();
    app.open_csv_import();
    let mut panel = app.csv_ui.take().unwrap();
    label_panel(&mut panel, SAMPLE, "id", &["responsable"]);
    panel.identity = Some(0);
    preview_import(&mut panel, &app.doc);
    assert!(panel.preview.is_some());
    assert_eq!(app.doc, before);
    app.apply_csv_preview(&panel).unwrap();
    assert_eq!(app.doc.page_count(), before.page_count() + 1);
    assert_eq!(app.doc.page(0), before.page(0));
    assert_eq!(app.doc.title, before.title);
    assert!(!app.doc.nodes.is_empty());
    // Root row has no relation; the three members each point to the root.
    assert_eq!(app.doc.connectors.len(), 3);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn stale_csv_import_preview_cannot_overwrite_later_edits() {
    let (_, mut app) = setup();
    app.open_csv_import();
    let mut panel = app.csv_ui.take().unwrap();
    label_panel(&mut panel, SAMPLE, "id", &[]);
    preview_import(&mut panel, &app.doc);
    app.doc.title = "Changed later".into();
    let before = app.doc.clone();
    assert!(app.apply_csv_preview(&panel).is_err());
    assert_eq!(app.doc, before);
}
#[test]
fn malformed_or_oversized_csv_cannot_create_a_preview_or_modify_the_document() {
    let (_, mut app) = setup();
    app.open_csv_import();
    let before = app.doc.clone();
    let mut panel = app.csv_ui.take().unwrap();
    for source in ["id\n".to_string(), "a,a\n1,2\n".to_string()] {
        let headers = diagcraft::csv::headers(&source).unwrap_or_default();
        panel.source = Some(source);
        panel.headers = headers;
        preview_import(&mut panel, &app.doc);
        assert!(panel.preview.is_none());
        assert!(!panel.diagnostics.is_empty() || panel.error.is_some());
        assert_eq!(app.doc, before);
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.csv");
    std::fs::write(&path, vec![b'a'; MAX_SOURCE + 1]).unwrap();
    assert!(read_source(&path).is_err());
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(read_source(&path).is_err());
}
#[test]
fn csv_modal_blocks_canvas_and_fits_small_screens() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    app.open_csv_import();
    let mut panel = app.csv_ui.take().unwrap();
    label_panel(&mut panel, SAMPLE, "id", &[]);
    preview_import(&mut panel, &app.doc);
    app.csv_ui = Some(panel);
    assert!(app.command_dialog_blocked());
    app.open_command_palette();
    assert!(!app.command_palette.open);
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                ..Default::default()
            },
            |ctx| app.csv_dialog(ctx),
        );
        for clipped in output.shapes {
            if let egui::Shape::Text(text) = clipped.shape
                && (text.galley.text() == t("Importer un CSV local")
                    || text.galley.text() == t("Importer sur une nouvelle page"))
            {
                assert!(
                    text.pos.x >= 0.
                        && text.pos.y >= 0.
                        && text.pos.y + text.galley.size().y <= 760.
                );
            }
        }
    }
    assert_eq!(app.doc, before);
}
