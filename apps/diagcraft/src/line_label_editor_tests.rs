use super::*;

fn line_label_fixture(connector: bool, text: &str) -> (egui::Context, DrawApp, u64, Point) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let id;
    let anchor;
    if connector {
        let a = doc.add_node(ShapeKind::Rectangle, [250., 200.]);
        let b = doc.add_node(ShapeKind::Rectangle, [850., 500.]);
        id = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
        doc.connectors[0].label = text.into();
        doc.connectors[0].label_t = 0.3;
        doc.connectors[0].label_offset = [18., -12.];
        anchor = doc.connectors[0].label_position(&doc.route(&doc.connectors[0]));
    } else {
        id = doc.add_drawing(DrawingKind::Bezier, [250., 300.], [850., 500.]);
        doc.drawings[0].bend = 80.;
        doc.drawings[0].label = text.into();
        doc.drawings[0].label_t = 0.3;
        doc.drawings[0].label_offset = [18., -12.];
        anchor = doc.drawings[0].label_position();
    }
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.show_library = false;
    app.show_quick_toolbar = false;
    app.selected = vec![id];
    (ctx, app, id, anchor)
}

fn line_label(doc: &Document, connector: bool) -> &str {
    if connector {
        &doc.connectors[0].label
    } else {
        &doc.drawings[0].label
    }
}

#[test]
fn line_label_draft_keeps_anchor_and_has_only_plain_text_controls() {
    for connector in [false, true] {
        let (ctx, mut app, id, anchor) = line_label_fixture(connector, "Before");
        let before = app.doc.clone();
        app.open_line_label_at(id, Some([900., 700.]));
        assert!(app.editing_line_label(id));
        inline_frame(&ctx, &mut app, vec![]);
        inline_frame(&ctx, &mut app, vec![]);
        inline_frame(
            &ctx,
            &mut app,
            vec![inline_key(Key::B, egui::Modifiers::COMMAND)],
        );
        assert!(app.rich_editor.as_ref().unwrap().draft.text_runs.is_empty());
        assert!(!app.rich_editor.as_ref().unwrap().draft.text_style.bold);
        inline_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Text(
                "Longer replacement 中文\nsecond line".into(),
            )],
        );
        let output = inline_frame(&ctx, &mut app, vec![]);
        let editor = app.rich_editor.as_ref().unwrap();
        assert!((editor.node_rect.center() - app.screen(anchor)).length() < 0.01);
        assert_eq!(editor.draft.font_size, 14.);
        assert!(editor.draft.text_style.auto_fit);
        assert!(!editor.draft.text_style.wrap);
        for label in ["Terminer", "Annuler"] {
            inline_text_shape(&output, &t(label));
        }
        assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
            Shape::Text(text) if ["B", "I", "U", t("Plus").as_str()].contains(&text.galley.job.text.as_str()))));
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        app.apply_node_text().unwrap();
        assert_eq!(
            line_label(&app.doc, connector),
            "Longer replacement 中文\nsecond line"
        );
        let mut normalized = app.doc.clone();
        if connector {
            normalized.connectors[0].label = "Before".into();
        } else {
            normalized.drawings[0].label = "Before".into();
        }
        assert_eq!(normalized, before);
        let after = app.doc.clone();
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        app.redo();
        assert_eq!(app.doc, after);
    }
}

#[test]
fn empty_line_label_projects_the_click_and_cancel_preserves_the_source() {
    for connector in [false, true] {
        let (ctx, mut app, id, _) = line_label_fixture(connector, "");
        let before = app.doc.clone();
        let route = if connector {
            app.doc.route(&app.doc.connectors[0])
        } else {
            app.doc.drawings[0].points()
        };
        let click = diagcraft::routing::label_position(&route, 0.75);
        let expected_t = diagcraft::routing::project_label(&route, click);
        app.open_line_label_at(id, Some(click));
        app.rich_editor.as_mut().unwrap().draft.text = "Projected".into();
        assert_eq!(app.doc, before);
        assert!(app.finish_inline_text(&ctx, true));
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        app.open_line_label_at(id, Some(click));
        app.rich_editor.as_mut().unwrap().draft.text = "Projected".into();
        app.apply_node_text().unwrap();
        if connector {
            assert!((app.doc.connectors[0].label_t - expected_t).abs() < 0.001);
            assert_eq!(app.doc.connectors[0].label_offset, [18., -12.]);
        } else {
            assert!((app.doc.drawings[0].label_t - expected_t).abs() < 0.001);
            assert_eq!(app.doc.drawings[0].label_offset, [18., -12.]);
        }
        app.undo();
        assert_eq!(app.doc, before);
    }
}

#[test]
fn line_label_validation_rejects_stale_and_oversized_drafts_atomically() {
    for connector in [false, true] {
        let (ctx, mut app, id, _) = line_label_fixture(connector, "Before");
        let before = app.doc.clone();
        app.open_line_label_at(id, None);
        inline_frame(&ctx, &mut app, vec![]);
        let oversized = "中".repeat(1500);
        inline_frame(&ctx, &mut app, vec![egui::Event::Text(oversized.clone())]);
        assert_eq!(app.rich_editor.as_ref().unwrap().draft.text, oversized);
        assert!(!app.finish_inline_text(&ctx, false));
        assert!(app.rich_editor.is_some());
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        app.rich_editor.as_mut().unwrap().draft.text = "Valid".into();
        app.doc.title = "Changed externally".into();
        let current = app.doc.clone();
        assert!(app.apply_node_text().is_err());
        assert_eq!(app.doc, current);
        assert!(app.finish_inline_text(&ctx, true));
        app.doc.set_objects_locked(&[id], true).unwrap();
        app.open_line_label_at(id, None);
        assert!(app.rich_editor.is_none());
    }
}

#[test]
fn line_label_ime_and_save_keep_the_last_input_in_one_undo_step() {
    for connector in [false, true] {
        let (ctx, mut app, id, _) = line_label_fixture(connector, "Before");
        let before = app.doc.clone();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("label.draw");
        before.save(&path).unwrap();
        app.path = Some(path.clone());
        app.open_line_label_at(id, None);
        inline_frame(&ctx, &mut app, vec![]);
        inline_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Ime(egui::ImeEvent::Enabled),
                egui::Event::Ime(egui::ImeEvent::Preedit("中文".into())),
            ],
        );
        inline_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::Ime(egui::ImeEvent::Commit("中文".into())),
                inline_key(Key::S, egui::Modifiers::COMMAND),
            ],
        );
        assert!(app.rich_editor.is_none());
        assert_eq!(line_label(&app.doc, connector), "中文");
        assert_eq!(Document::load(&path).unwrap(), app.doc);
        assert!(!app.dirty());
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
