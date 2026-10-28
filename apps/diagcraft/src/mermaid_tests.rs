use super::*;
fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
    app.fit_next = false;
    (ctx, app)
}
#[test]
fn import_preview_preserves_pages_and_applying_is_a_single_undo() {
    let (_, mut app) = setup();
    let before = app.doc.clone();
    app.open_mermaid(false);
    let mut panel = app.mermaid_ui.take().unwrap();
    panel.source = SAMPLE.into();
    preview_import(&mut panel, &app.doc);
    assert!(panel.preview.is_some());
    assert_eq!(app.doc, before);
    app.apply_mermaid_preview(&panel).unwrap();
    assert_eq!(app.doc.page_count(), before.page_count() + 1);
    assert_eq!(app.doc.page(0), before.page(0));
    assert_eq!(app.doc.title, before.title);
    assert!(!app.doc.nodes.is_empty());
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn stale_import_preview_cannot_overwrite_later_edits() {
    let (_, mut app) = setup();
    app.open_mermaid(false);
    let mut panel = app.mermaid_ui.take().unwrap();
    panel.source = SAMPLE.into();
    preview_import(&mut panel, &app.doc);
    app.doc.title = "Changed later".into();
    let before = app.doc.clone();
    assert!(app.apply_mermaid_preview(&panel).is_err());
    assert_eq!(app.doc, before);
}
#[test]
fn malformed_or_oversized_mermaid_cannot_create_a_preview_or_modify_the_document() {
    let (_, mut app) = setup();
    app.open_mermaid(false);
    let before = app.doc.clone();
    let mut panel = app.mermaid_ui.take().unwrap();
    for source in [
        "sequenceDiagram\n A->>B: text".into(),
        "a".repeat(MAX_SOURCE + 1),
    ] {
        panel.source = source;
        preview_import(&mut panel, &app.doc);
        assert!(panel.preview.is_none());
        assert!(!panel.diagnostics.is_empty());
        assert_eq!(app.doc, before);
    }
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.mmd");
    std::fs::write(&path, vec![b'a'; MAX_SOURCE + 1]).unwrap();
    assert!(read_source(&path).is_err());
    std::fs::write(&path, [0xff, 0xfe]).unwrap();
    assert!(read_source(&path).is_err());
}
#[test]
fn export_requires_loss_acknowledgment_and_detects_stale_documents() {
    let (_, mut app) = setup();
    let before = app.doc.clone();
    app.open_mermaid(true);
    let mut panel = app.mermaid_ui.take().unwrap();
    assert!(panel.error.is_none());
    assert!(!panel.diagnostics.is_empty());
    assert!(!panel.can_export());
    panel.acknowledged = true;
    assert!(panel.can_export());
    panel.check_current(&app.doc).unwrap();
    assert_eq!(app.doc, before);
    app.doc.title = "Different revision".into();
    assert!(panel.check_current(&app.doc).is_err());
}
#[test]
fn mermaid_modal_blocks_canvas_commands_is_read_only_and_fits_small_screens() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    app.open_mermaid(false);
    app.mermaid_ui.as_mut().unwrap().source = SAMPLE.into();
    let mut panel = app.mermaid_ui.take().unwrap();
    preview_import(&mut panel, &app.doc);
    app.mermaid_ui = Some(panel);
    assert!(app.command_dialog_blocked());
    app.open_command_palette();
    assert!(!app.command_palette.open);
    for _ in 0..3 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                ..Default::default()
            },
            |ctx| app.mermaid_dialog(ctx),
        );
        for clipped in output.shapes {
            if let egui::Shape::Text(text) = clipped.shape
                && (text.galley.text() == t("Importer du Mermaid")
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

#[test]
fn mermaid_export_never_replaces_existing_files_or_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let original = temp.path().join("source.draw");
    std::fs::write(&original, b"original drawing").unwrap();
    assert!(save_source(&original, SAMPLE).is_err());
    assert_eq!(std::fs::read(&original).unwrap(), b"original drawing");
    let fresh = temp.path().join("new.mmd");
    save_source(&fresh, SAMPLE).unwrap();
    assert_eq!(std::fs::read_to_string(&fresh).unwrap(), SAMPLE);
    assert!(save_source(&fresh, "other data").is_err());
    assert_eq!(std::fs::read_to_string(&fresh).unwrap(), SAMPLE);
    #[cfg(unix)]
    {
        let link = temp.path().join("link.mmd");
        std::os::unix::fs::symlink(&original, &link).unwrap();
        assert!(save_source(&link, SAMPLE).is_err());
        assert_eq!(std::fs::read(&original).unwrap(), b"original drawing");
        let dangling = temp.path().join("dangling.mmd");
        let absent = temp.path().join("absent.draw");
        std::os::unix::fs::symlink(&absent, &dangling).unwrap();
        assert!(save_source(&dangling, SAMPLE).is_err());
        assert!(!absent.exists());
    }
}

#[test]
fn layout_notice_uses_interface_translation_without_mutating_machine_diagnostics() {
    let import = mermaid::import(SAMPLE).unwrap();
    let export = mermaid::export(&import.document, 0).unwrap();
    let previous = i18n::active();
    for language in Language::ALL {
        i18n::activate(language);
        for (notice, exporting, source) in [
            (
                &import.diagnostics[0],
                false,
                "DiagCraft crée un placement modifiable à partir de la structure du flowchart.",
            ),
            (
                &export.diagnostics[0],
                true,
                "L’export Mermaid ne conserve pas les positions, tailles, parcours, ports, styles ou l’apparence des pages.",
            ),
        ] {
            let before = serde_json::to_value(notice).unwrap();
            let (message, hint) = notice_text(notice, exporting);
            assert_eq!(message, i18n::translate(language, source));
            assert!(!hint.is_empty());
            assert_eq!(serde_json::to_value(notice).unwrap(), before);
            assert_eq!(notice.code, "layout_changed");
            if language != Language::English {
                assert_ne!(message, notice.message);
            }
        }
    }
    i18n::activate(previous);
}
