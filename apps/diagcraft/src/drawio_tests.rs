use super::*;
fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
    app.fit_next = false;
    (ctx, app)
}
fn load_panel(panel: &mut Panel) {
    // Parses the bundled sample: two vertices, one edge, one page named "Page 1".
    panel.source = Some(SAMPLE.into());
    panel.pages = drawio::pages(SAMPLE).unwrap();
    panel.error = None;
}
#[test]
fn drawio_import_preview_preserves_pages_and_applies_as_a_single_undo() {
    let (_, mut app) = setup();
    let before = app.doc.clone();
    app.open_drawio_import();
    let mut panel = app.drawio_ui.take().unwrap();
    load_panel(&mut panel);
    preview_import(&mut panel, &app.doc);
    assert!(panel.preview.is_some());
    assert!(panel.diagnostics.is_empty(), "{:?}", panel.diagnostics);
    assert_eq!(app.doc, before);
    app.apply_drawio_preview(&panel).unwrap();
    assert_eq!(app.doc.page_count(), before.page_count() + 1);
    assert_eq!(app.doc.page(0), before.page(0));
    assert_eq!(app.doc.title, before.title);
    // Two vertices + one connector imported onto the new page.
    assert_eq!(app.doc.nodes.len(), 2);
    assert_eq!(app.doc.connectors.len(), 1);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn drawio_page_selector_imports_only_the_chosen_page() {
    let (_, mut app) = setup();
    app.open_drawio_import();
    let mut panel = app.drawio_ui.take().unwrap();
    let multi = r#"<mxfile host="app" type="device">
      <diagram id="a" name="Première"><mxGraphModel><root>
        <mxCell id="0"/><mxCell id="1" parent="0"/>
        <mxCell id="x" value="Un" vertex="1" parent="1"><mxGeometry x="0" y="0" width="80" height="40" as="geometry"/></mxCell>
      </root></mxGraphModel></diagram>
      <diagram id="b" name="Deuxième"><mxGraphModel><root>
        <mxCell id="0"/><mxCell id="1" parent="0"/>
        <mxCell id="y" value="Deux" vertex="1" parent="1"><mxGeometry x="0" y="0" width="80" height="40" as="geometry"/></mxCell>
      </root></mxGraphModel></diagram>
    </mxfile>"#;
    panel.source = Some(multi.into());
    panel.pages = drawio::pages(multi).unwrap();
    assert_eq!(panel.pages, vec!["Première", "Deuxième"]);
    panel.page = 1;
    preview_import(&mut panel, &app.doc);
    let imported = panel.preview.as_ref().unwrap().document.clone();
    assert_eq!(imported.page_name, "Deuxième");
    assert_eq!(imported.nodes.len(), 1);
    // Diagnostics signal that the other page was not taken up.
    assert!(panel.diagnostics.iter().any(|n| n.code == "page_skipped"));
}
#[test]
fn stale_drawio_import_preview_cannot_overwrite_later_edits() {
    let (_, mut app) = setup();
    app.open_drawio_import();
    let mut panel = app.drawio_ui.take().unwrap();
    load_panel(&mut panel);
    preview_import(&mut panel, &app.doc);
    app.doc.title = "Changed later".into();
    let before = app.doc.clone();
    assert!(app.apply_drawio_preview(&panel).is_err());
    assert_eq!(app.doc, before);
}
#[test]
fn malformed_drawio_cannot_create_a_preview_or_modify_the_document() {
    let (_, mut app) = setup();
    app.open_drawio_import();
    let before = app.doc.clone();
    let mut panel = app.drawio_ui.take().unwrap();
    panel.source = Some("<mxfile><diagram>".into());
    panel.pages = drawio::pages("<mxfile><diagram>").unwrap_or_default();
    preview_import(&mut panel, &app.doc);
    assert!(panel.preview.is_none());
    assert!(!panel.diagnostics.is_empty() || panel.error.is_some());
    assert_eq!(app.doc, before);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.drawio");
    std::fs::write(&path, vec![b'a'; MAX_SOURCE + 1]).unwrap();
    assert!(read_source(&path).is_err());
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(read_source(&path).is_err());
}
#[test]
fn drawio_modal_blocks_canvas_and_fits_small_screens() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    app.open_drawio_import();
    let mut panel = app.drawio_ui.take().unwrap();
    load_panel(&mut panel);
    preview_import(&mut panel, &app.doc);
    app.drawio_ui = Some(panel);
    assert!(app.command_dialog_blocked());
    app.open_command_palette();
    assert!(!app.command_palette.open);
    for _ in 0..2 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                ..Default::default()
            },
            |ctx| app.drawio_dialog(ctx),
        );
        for clipped in output.shapes {
            if let egui::Shape::Text(text) = clipped.shape
                && (text.galley.text() == t("Importer un draw.io local")
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
