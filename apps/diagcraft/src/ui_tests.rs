use super::*;

#[test]
fn automatic_text_fitting_is_one_undo_and_keeps_the_selected_connected_node() {
    let (_, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [250., 250.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [600., 250.]);
    app.doc.connect(a, b, ConnectorKind::Straight).unwrap();
    app.selected = vec![a];
    app.commit();
    let before = app.doc.clone();
    let node = app.doc.node_mut(a).unwrap();
    node.text = "A long label 中文 français Deutsch español italiano português ".repeat(6);
    node.text_style.wrap = true;
    node.text_style.auto_fit = true;
    app.commit();
    assert_eq!(app.selected, vec![a]);
    assert_eq!(
        app.doc.node(a).unwrap().position,
        before.node(a).unwrap().position
    );
    assert_eq!(
        app.doc.node(a).unwrap().size[0],
        before.node(a).unwrap().size[0]
    );
    assert!(app.doc.node(a).unwrap().size[1] > before.node(a).unwrap().size[1]);
    let fitted = app.doc.clone();
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc, fitted);
}

#[test]
fn fit_text_command_handles_a_mixed_selection_and_undoes_once() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [250., 250.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [600., 250.]);
    let e = app.doc.connect(a, b, ConnectorKind::Straight).unwrap();
    app.doc.node_mut(a).unwrap().text = "A significantly longer label to fit".into();
    app.doc.node_mut(b).unwrap().text = "中文\nA decision".into();
    app.commit();
    app.selected = vec![a, b, e];
    let before = app.doc.clone();
    app.run_command(commands_ui::Command::FitText, &ctx);
    assert_eq!(app.selected, vec![a, b, e]);
    assert_eq!(app.doc.connectors, before.connectors);
    assert!(!app.doc.node(a).unwrap().text_style.auto_fit);
    assert_ne!(app.doc, before);
    app.undo();
    assert_eq!(app.doc, before);
    app.selected = vec![e];
    assert!(
        app.command_unavailable(commands_ui::Command::FitText)
            .is_some()
    );
}

#[test]
fn manual_resize_turns_off_fitting_and_escape_restores_the_automatic_setting() {
    for cancel in [false, true] {
        let (ctx, mut app) = setup();
        app.snap = false;
        app.show_quick_toolbar = false;
        let id = app.doc.add_node(ShapeKind::Rectangle, [600., 400.]);
        let node = app.doc.node_mut(id).unwrap();
        node.text_style.auto_fit = true;
        node.text_style.wrap = true;
        app.selected = vec![id];
        app.commit();
        let before = app.doc.clone();
        let node = app.doc.node(id).unwrap();
        let rect = Rect::from_min_size(app.screen(node.position), vec2(node.size[0], node.size[1]));
        let (at, _) = resize_handles(rect)[4];
        pointer(&ctx, &mut app, at, true, egui::Modifiers::NONE);
        assert!(matches!(app.drag, Some(Drag::Resize { .. })));
        assert!(!app.doc.node(id).unwrap().text_style.auto_fit);
        if cancel {
            gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
            assert_eq!(app.doc, before);
        } else {
            pointer(
                &ctx,
                &mut app,
                at + vec2(40., 30.),
                false,
                egui::Modifiers::NONE,
            );
            assert!(!app.doc.node(id).unwrap().text_style.auto_fit);
            app.undo();
            assert_eq!(app.doc, before);
        }
    }
}

#[test]
fn native_document_text_uses_shared_lines_fonts_and_fits_the_reserved_area() {
    for monospace in [false, true] {
        for bold in [false, true] {
            let (ctx, mut app) = setup();
            let id = app.doc.add_node(ShapeKind::Diamond, [350., 350.]);
            let node = app.doc.node_mut(id).unwrap();
            node.text = "English français Grüße 中文绘图 español português italiano".into();
            node.text_style.wrap = true;
            node.text_style.auto_fit = true;
            node.text_style.monospace = monospace;
            node.text_style.bold = bold;
            app.commit();
            let node = app.doc.node(id).unwrap().clone();
            let expected = node.display_text();
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        app.canvas = ui.max_rect();
                        app.paint_node(ui.painter(), &node);
                    });
                },
            );
            let (origin, size) = diagcraft::text_layout::area(&node);
            let area = Rect::from_min_size(app.screen(origin), vec2(size[0], size[1]) * app.zoom)
                .expand(2.);
            let text = output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let Shape::Text(text) = &shape.shape {
                        (text.galley.job.text == expected).then_some(text)
                    } else {
                        None
                    }
                })
                .unwrap();
            let bounds = text.galley.rect.translate(text.pos.to_vec2());
            assert!(
                area.contains_rect(bounds),
                "mono={monospace} bold={bold}: text {bounds:?}, area {area:?}"
            );
            assert_eq!(
                text.galley.job.sections[0].format.line_height,
                Some(node.text_layout().line_height * app.zoom)
            );
        }
    }
}

#[test]
fn language_changes_preserve_user_text_selection_and_undo_and_persist_separately() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.doc.node_mut(id).unwrap().text = "Enregistrer {0} Grüße 中文".into();
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    for language in Language::ALL {
        app.set_language(Some(language));
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![id]);
        assert!(app.settings.job.is_none());
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("interface-v1.json");
    app.interface_path = Some(path.clone());
    app.set_language(Some(Language::German));
    app.settings.check_at_startup = true;
    app.settings.experimental = false;
    app.save_interface().unwrap();
    let (_, mut reopened) = setup();
    reopened.interface_path = Some(path.clone());
    reopened.restore_interface();
    assert_eq!(reopened.language, Some(Language::German));
    assert!(reopened.settings.check_at_startup);
    assert!(!reopened.settings.experimental);
    app.reset_interface(&ctx);
    assert_eq!(app.language, Some(Language::German));
    assert!(app.settings.check_at_startup && !app.settings.experimental);
    assert_eq!(app.doc, before);
    app.undo();
    assert_ne!(app.doc, before);
    std::fs::write(&path, br#"{"version":1,"library":false}"#).unwrap();
    reopened.restore_interface();
    assert_eq!(reopened.language, None);
    assert!(!reopened.settings.check_at_startup && !reopened.show_library);
    i18n::activate(Language::French);
}

#[test]
fn document_tab_titles_matching_translation_keys_are_displayed_verbatim() {
    let (ctx, mut app) = setup();
    app.doc.title = "Enregistrer".into();
    app.stash_tab();
    app.set_language(Some(Language::Chinese));
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
            ..Default::default()
        },
        |ctx| app.header(ctx),
    );
    assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
        egui::Shape::Text(text) if text.galley.job.text == "Enregistrer")));
    assert_eq!(app.doc.title, "Enregistrer");
    i18n::activate(Language::French);
}

#[test]
fn command_search_uses_the_selected_language_and_settings_block_drawing_shortcuts() {
    use commands_ui::{Command, results};
    for (language, query, command) in [
        (Language::English, "save as", Command::SaveAs),
        (Language::German, "seitengroße", Command::PageSettings),
        (Language::Spanish, "actualizaciones", Command::Updates),
        (Language::Italian, "aggiornamenti", Command::Updates),
        (Language::Portuguese, "atualizações", Command::Updates),
        (Language::Chinese, "更新", Command::Updates),
    ] {
        i18n::activate(language);
        assert!(
            results(query).iter().any(|entry| entry.command == command),
            "{query}"
        );
    }
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    app.settings.open = true;
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Delete, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    assert!(app.command_dialog_blocked());
}

#[test]
fn chinese_catalogue_and_all_seven_language_settings_render_with_bundled_fonts() {
    let (ctx, mut app) = setup();
    let font = FontId::proportional(14.);
    ctx.fonts_mut(|fonts| {
        for row in i18n::catalogue().values() {
            for c in row["zh-Hans"].chars().filter(|c| c.is_alphabetic()) {
                assert!(fonts.has_glyph(&font, c), "Missing glyph: {c}");
            }
        }
        assert!(fonts.has_glyphs(&font, "Grüße Español Português 中文"));
    });
    let before = app.doc.clone();
    for language in Language::ALL {
        app.set_language(Some(language));
        app.settings.open = true;
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                ..Default::default()
            },
            |ctx| {
                app.header(ctx);
                app.sidebar(ctx);
                app.canvas(ctx, false);
                app.settings_dialog(ctx);
            },
        );
        assert!(!output.shapes.is_empty());
        assert_eq!(app.doc, before);
        assert!(app.settings.job.is_none());
    }
    i18n::activate(Language::French);
}

fn command_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280., 760.))),
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            app.canvas(ctx, !app.command_palette.open && !app.show_help);
            app.command_dialog(ctx);
            app.help_dialog(ctx);
        },
    )
}

fn command_key(key: Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

#[test]
fn palette_search_understands_accents_aliases_and_multiple_words() {
    use commands_ui::{Command, results};
    assert_eq!(results("export pdf")[0].command, Command::ExportPdf);
    assert_eq!(results("retablir")[0].command, Command::Redo);
    assert_eq!(results("bezier")[0].command, Command::Bezier);
    assert_eq!(results("chatgpt")[0].command, Command::Agents);
    assert!(
        results("page taille")
            .iter()
            .any(|s| s.command == Command::PageSettings)
    );
    assert!(results("commande inexistante").is_empty());
    assert!(!results("").iter().any(|s| s.command == Command::Palette));
}

#[test]
fn command_palette_consumes_typing_navigation_and_escape_without_editing_the_drawing() {
    use commands_ui::Command;
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::K, egui::Modifiers::COMMAND)],
        egui::Modifiers::COMMAND,
    );
    assert!(app.command_palette.open);
    command_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Text("page".into())],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.command_palette.query, "page");
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::ArrowDown, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Escape, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(!app.command_palette.open);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    assert!(!ctx.wants_keyboard_input());
    app.run_command(Command::Duplicate, &ctx);
    assert_eq!(app.doc.nodes.len(), 2);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn palette_executes_enter_once_and_blocks_commands_behind_other_dialogs() {
    use commands_ui::Command;
    let (ctx, mut app) = setup();
    app.open_command_palette();
    app.command_palette.query = "ajouter page".into();
    command_frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    let before = app.doc.clone();
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Enter, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(!app.command_palette.open);
    assert_eq!(app.doc.page_count(), 2);
    app.undo();
    assert_eq!(app.doc, before);
    app.show_pdf_export = true;
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::K, egui::Modifiers::COMMAND)],
        egui::Modifiers::COMMAND,
    );
    assert!(!app.command_palette.open);
    app.show_pdf_export = false;
    assert!(app.command_unavailable(Command::Duplicate).is_some());
    app.open_command_palette();
    app.command_palette.query = "dupliquer".into();
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Enter, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(app.command_palette.open);
    assert_eq!(app.doc, before);
}

#[test]
fn help_is_modal_and_preserves_selection_until_escape() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::F1, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(app.show_help);
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Delete, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Escape, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(!app.show_help);
    assert_eq!(app.selected, vec![id]);
}

#[test]
fn keyboard_paste_is_not_lost_or_duplicated_when_native_text_arrives_with_the_shortcut() {
    let (ctx, mut app) = setup();
    frame(
        &ctx,
        &mut app,
        vec![
            command_key(Key::V, egui::Modifiers::COMMAND),
            egui::Event::Paste("Une idée".into()),
        ],
        egui::Modifiers::COMMAND,
    );
    assert_eq!(app.doc.nodes.len(), 1);
    assert_eq!(app.doc.nodes[0].text, "Une idée");
    app.undo();
    assert!(app.doc.nodes.is_empty());
}

#[test]
fn command_zoom_keeps_the_document_point_under_the_center_fixed() {
    let (ctx, mut app) = setup();
    app.zoom = 0.5;
    let center = app.canvas.center();
    let before = app.world(center);
    app.run_command(commands_ui::Command::ActualSize, &ctx);
    assert_eq!(app.zoom, 1.);
    assert_eq!(app.world(center), before);
    assert!(!app.fit_next);
    assert!(!app.history.can_undo());
}

#[test]
fn command_palette_dismisses_when_a_higher_priority_dialog_arrives() {
    let (ctx, mut app) = setup();
    app.open_command_palette();
    app.command_palette.query = "ajouter page".into();
    app.pending = Some(Pending::Close);
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Enter, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(!app.command_palette.open);
    assert_eq!(app.doc.page_count(), 1);
    assert!(matches!(app.pending, Some(Pending::Close)));
}

#[test]
fn palette_enter_activates_the_tab_focused_close_button_instead_of_the_selected_command() {
    let (ctx, mut app) = setup();
    app.open_command_palette();
    app.command_palette.query = "ajouter page".into();
    command_frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    command_frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Tab, egui::Modifiers::SHIFT)],
        egui::Modifiers::SHIFT,
    );
    command_frame(
        &ctx,
        &mut app,
        vec![command_key(Key::Enter, egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(!app.command_palette.open);
    assert_eq!(app.doc.page_count(), 1);
}

#[test]
fn clicking_a_palette_result_executes_one_action_without_clicking_through() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    app.open_command_palette();
    app.command_palette.query = "ajouter page".into();
    command_frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    let out = command_frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    let point = ux_text(&out, "Ajouter une page\nDocument");
    for pressed in [true, false] {
        command_frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(point),
                egui::Event::PointerButton {
                    pos: point,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            egui::Modifiers::NONE,
        );
    }
    assert!(!app.command_palette.open);
    assert_eq!(app.doc.page_count(), 2);
    assert!(app.doc.nodes.is_empty());
    app.undo();
    assert_eq!(app.doc.page_count(), 1);
    assert!(!app.history.can_undo());
}

fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::default(), None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    (ctx, app)
}

fn frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    events: Vec<egui::Event>,
    modifiers: egui::Modifiers,
) {
    let input = egui::RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
        events,
        modifiers,
        ..Default::default()
    };
    let _ = ctx.run(input, |ctx| {
        app.shortcuts(ctx);
        let before = app.doc.clone();
        app.canvas(ctx, true);
        if app.drag.is_none() && app.doc != before {
            app.commit();
        }
    });
}

fn pointer(
    ctx: &egui::Context,
    app: &mut DrawApp,
    p: Pos2,
    pressed: bool,
    modifiers: egui::Modifiers,
) {
    frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers,
            },
        ],
        modifiers,
    );
}

fn click(ctx: &egui::Context, app: &mut DrawApp, p: Pos2) {
    pointer(ctx, app, p, true, egui::Modifiers::NONE);
    pointer(ctx, app, p, false, egui::Modifiers::NONE);
}

#[test]
fn canvas_creates_connects_moves_and_undoes_one_drag() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    click(&ctx, &mut app, pos2(260., 180.));
    app.set_tool(Tool::Shape(ShapeKind::Diamond));
    click(&ctx, &mut app, pos2(650., 330.));
    assert_eq!(app.doc.nodes.len(), 2);
    assert!(
        app.rich_editor.is_none(),
        "Rapid creation must not start text editing"
    );
    let (first, second) = (app.doc.nodes[0].id, app.doc.nodes[1].id);
    app.set_tool(Tool::Connect);
    // Let the selection toolbar disappear, as it does in the tool button frame.
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    click(&ctx, &mut app, pos2(260., 180.));
    assert_eq!(app.connect_from, Some(first));
    click(&ctx, &mut app, pos2(650., 330.));
    assert_eq!(app.doc.connectors.len(), 1);
    assert_eq!(app.doc.connectors[0].to, second);
    let before = app.doc.clone();
    let route = before.route(&before.connectors[0]);
    app.set_tool(Tool::Select);
    app.smart_guides = false; // This test exercises grid translation; guides have dedicated tests.
    pointer(
        &ctx,
        &mut app,
        pos2(260., 180.),
        true,
        egui::Modifiers::NONE,
    );
    for p in [pos2(280., 190.), pos2(300., 210.), pos2(320., 230.)] {
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(p)],
            egui::Modifiers::NONE,
        );
    }
    pointer(
        &ctx,
        &mut app,
        pos2(320., 230.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(
        app.doc.node(first).unwrap().position,
        [
            before.nodes[0].position[0] + 60.,
            before.nodes[0].position[1] + 50.
        ]
    );
    assert_ne!(app.doc.route(&app.doc.connectors[0]), route);
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_ne!(app.doc, before);
    app.doc.validate().unwrap();
}

#[test]
fn canvas_resize_updates_attached_route_and_preserves_undo() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Ellipse, [260., 180.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [650., 330.]);
    app.doc.connect(a, b, ConnectorKind::Straight);
    app.commit();
    app.selected = vec![a];
    let before = app.doc.clone();
    let n = app.doc.node(a).unwrap();
    let handle = pos2(n.position[0] + n.size[0], n.position[1] + n.size[1]);
    pointer(&ctx, &mut app, handle, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        handle + vec2(80., 50.),
        false,
        egui::Modifiers::NONE,
    );
    // The dragged bottom edge snaps to y=270; the fixed top remains at y=138.
    assert_eq!(app.doc.node(a).unwrap().size, [300., 132.]);
    assert_ne!(
        app.doc.route(&app.doc.connectors[0]),
        before.route(&before.connectors[0])
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn zoom_and_pan_change_view_without_changing_document() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    let anchor = pos2(500., 300.);
    let world = app.world(anchor);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 80.),
                modifiers: egui::Modifiers::COMMAND,
            },
        ],
        egui::Modifiers::COMMAND,
    );
    assert!(app.zoom > 1.);
    let after = app.world(anchor);
    assert!((world[0] - after[0]).abs() < 0.01 && (world[1] - after[1]).abs() < 0.01);
    app.set_tool(Tool::Pan);
    pointer(&ctx, &mut app, anchor, true, egui::Modifiers::NONE);
    let pan = app.pan;
    pointer(
        &ctx,
        &mut app,
        anchor + vec2(60., 40.),
        false,
        egui::Modifiers::NONE,
    );
    assert!((app.pan - pan - vec2(60., 40.)).length() < 0.1);
    assert_eq!(app.doc, before);
    assert!(!app.dirty());
    assert!(!app.history.can_undo());
}

#[test]
fn new_tabs_preserve_unsaved_documents_and_saved_file_can_be_reopened() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    click(&ctx, &mut app, pos2(260., 180.));
    let edited = app.doc.clone();
    app.request(Pending::New);
    assert!(app.pending.is_none());
    assert!(app.doc.nodes.is_empty());
    assert_eq!(app.tabs[0].doc, edited);
    app.switch_tab(0);
    assert_eq!(app.doc, edited);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.draw");
    app.path = Some(path.clone());
    assert!(app.save(false));
    assert!(!app.dirty());
    app.request(Pending::New);
    assert!(app.doc.nodes.is_empty());
    app.request(Pending::Open(path));
    assert_eq!(app.doc, edited);
    assert!(!app.dirty());
}

#[test]
fn pointer_draws_selects_moves_and_undoes_curves() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Bezier);
    pointer(
        &ctx,
        &mut app,
        pos2(100., 120.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(500., 300.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.drawings.len(), 1);
    let curve = app.doc.drawings[0].clone();
    assert_eq!(curve.start, [100., 120.]);
    assert_eq!(curve.end, [500., 300.]);
    let mid = curve.points()[20];
    app.set_tool(Tool::Select);
    pointer(
        &ctx,
        &mut app,
        pos2(mid[0], mid[1]),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(mid[0] + 60., mid[1] + 40.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.drawings[0].start, [160., 160.]);
    app.undo();
    assert_eq!(app.doc.drawings[0], curve);
    app.undo();
    assert!(app.doc.drawings.is_empty());
}

#[test]
fn each_document_keeps_its_history_view_and_unsaved_changes() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    click(&ctx, &mut app, pos2(260., 180.));
    let first = app.doc.clone();
    app.pan = vec2(12., 40.);
    app.request(Pending::New);
    app.doc.title = "Deuxième".into();
    app.doc.add_node(ShapeKind::Ellipse, [400., 300.]);
    app.commit();
    let second = app.doc.clone();
    app.switch_tab(0);
    assert_eq!(app.doc, first);
    assert_eq!(app.pan, vec2(12., 40.));
    assert!(app.dirty());
    app.undo();
    assert!(app.doc.nodes.is_empty());
    app.switch_tab(1);
    assert_eq!(app.doc, second);
    app.undo();
    assert!(app.doc.nodes.is_empty());
    app.switch_tab(0);
    app.redo();
    assert_eq!(app.doc, first);
}

#[test]
fn closing_a_dirty_tab_requests_choice_and_preserves_other_tabs() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    click(&ctx, &mut app, pos2(260., 180.));
    let first = app.doc.clone();
    app.request(Pending::New);
    app.request(Pending::CloseTab(0));
    assert!(matches!(app.pending, Some(Pending::CloseTab(0))));
    assert_eq!(app.doc, first);
    assert_eq!(app.tabs.len(), 2);
    app.pending = None;
    assert_eq!(app.doc, first);
    app.execute(Pending::CloseTab(0));
    assert_eq!(app.tabs.len(), 1);
    assert!(app.doc.nodes.is_empty());
    assert!(!app.allow_close);
}

#[test]
fn closing_application_checks_dirty_inactive_documents() {
    let (_, mut app) = setup();
    app.doc.title = "À conserver".into();
    app.commit();
    app.request(Pending::New);
    assert!(!app.dirty());
    app.request(Pending::Close);
    assert!(matches!(app.pending, Some(Pending::Close)));
    assert_eq!(app.active_tab, 0);
    assert_eq!(app.doc.title, "À conserver");
    assert!(!app.allow_close);
    app.pending = None;
    app.execute(Pending::Close);
    assert!(app.allow_close);
}

#[test]
fn clipboard_table_timeline_and_styles_are_editable_and_undoable() {
    let (ctx, mut app) = setup();
    app.insert_table();
    assert_eq!(app.doc.nodes.len(), 9);
    assert_eq!(app.selected.len(), 9);
    assert!(app.doc.nodes[0].text_style.bold);
    app.doc.validate().unwrap();
    app.copy(&ctx, false);
    app.paste_fragment(app.clipboard.clone().unwrap());
    assert_eq!(app.doc.nodes.len(), 18);
    app.undo();
    assert_eq!(app.doc.nodes.len(), 9);
    app.change_text(|s| {
        s.bold = true;
        s.italic = true;
        s.align = TextAlign::Right;
    });
    app.request(Pending::New);
    app.insert_timeline();
    assert_eq!(app.doc.nodes.len(), 4);
    assert_eq!(app.doc.connectors.len(), 3);
    app.doc.validate().unwrap();
    app.undo();
    assert!(app.doc.nodes.is_empty());
}

#[test]
fn save_shortcut_works_while_a_text_field_is_focused() {
    let (ctx, mut app) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("texte.draw");
    app.path = Some(path.clone());
    app.doc.title = "Modification en cours".into();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            ..Default::default()
        },
        |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut app.doc.title).id_salt("focused_title"))
                    .request_focus();
            });
        },
    );
    assert!(ctx.wants_keyboard_input());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
        egui::Modifiers::COMMAND,
    );
    assert_eq!(
        Document::load(&path).unwrap().title,
        "Modification en cours"
    );
    assert!(!app.dirty());
}

#[test]
fn all_ribbon_tabs_keep_the_canvas_aligned_at_supported_window_sizes() {
    for size in [vec2(1200., 760.), vec2(1440., 960.)] {
        let ctx = egui::Context::default();
        let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
        let mut top = None;
        for tab in [
            RibbonTab::Home,
            RibbonTab::Insert,
            RibbonTab::Design,
            RibbonTab::View,
            RibbonTab::Symbols,
            RibbonTab::Advanced,
        ] {
            app.ribbon_tab = tab;
            for _ in 0..2 {
                let _ = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
                        ..Default::default()
                    },
                    |ctx| {
                        app.header(ctx);
                        app.footer(ctx);
                        app.sidebar(ctx);
                        app.inspector(ctx);
                        app.canvas(ctx, false);
                    },
                );
            }
            let expected = *top.get_or_insert(app.canvas.top());
            assert!(
                (app.canvas.top() - expected).abs() < 1.,
                "Ribbon content expanded the header: {} vs {expected}",
                app.canvas.top()
            );
            assert!(app.canvas.right() <= size.x + 1. && app.canvas.bottom() <= size.y + 1.);
            assert!(app.canvas.width() > 500. && app.canvas.height() > 400.);
        }
    }
}

#[test]
fn chart_pointer_move_resize_clipboard_and_undo() {
    let (ctx, mut app) = setup();
    app.insert_chart(ChartKind::Columns);
    let original = app.doc.clone();
    let c = app.doc.charts[0].clone();
    let center = app.screen([
        c.position[0] + c.size[0] / 2.,
        c.position[1] + c.size[1] / 2.,
    ]);
    pointer(&ctx, &mut app, center, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        center + vec2(40., 30.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(
        app.doc.charts[0].position,
        [c.position[0] + 40., c.position[1] + 30.]
    );
    app.undo();
    assert_eq!(app.doc, original);
    app.selected = vec![c.id];
    let handle = app.screen([c.position[0] + c.size[0], c.position[1] + c.size[1]]);
    pointer(&ctx, &mut app, handle, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        handle + vec2(80., 40.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.charts[0].size, [560., 360.]);
    app.undo();
    assert_eq!(app.doc, original);
    app.selected = vec![c.id];
    app.copy(&ctx, false);
    app.paste_fragment(app.clipboard.clone().unwrap());
    assert_eq!(app.doc.charts.len(), 2);
    app.undo();
    assert_eq!(app.doc, original);
}

#[test]
fn personal_symbol_places_saved_style_at_pointer_with_new_id() {
    let (ctx, mut app) = setup();
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Database, [200., 200.]);
    let n = d.node_mut(id).unwrap();
    n.text = "Données client".into();
    n.fill = [200, 210, 240];
    n.size = [180., 140.];
    let custom = n.clone();
    app.doc.add_node(ShapeKind::Rectangle, [100., 100.]);
    app.choose_symbol(custom.clone());
    click(&ctx, &mut app, pos2(600., 400.));
    let placed = app.doc.nodes.last().unwrap();
    assert_ne!(placed.id, custom.id);
    assert_eq!(placed.text, custom.text);
    assert_eq!(placed.fill, custom.fill);
    assert_eq!(placed.size, custom.size);
    assert_eq!(placed.center(), [600., 400.]);
    assert!(app.pending_symbol.is_none());
    app.doc.validate().unwrap();
}

#[test]
fn templates_open_in_new_tabs_and_recents_persist_without_losing_edits() {
    let (_, mut app) = setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("library.json");
    app.library_path = Some(path.clone());
    app.doc.title = "Travail à conserver".into();
    app.commit();
    let original = app.doc.clone();
    let t = templates::catalog().remove(0);
    app.show_templates = true;
    app.open_template(&t);
    assert_eq!(app.doc, t.document);
    assert!(!app.show_templates);
    assert_eq!(Library::load(&path).unwrap().recent[0], t.id);
    app.switch_tab(0);
    assert_eq!(app.doc, original);
    assert!(app.dirty());
}

#[test]
fn all_library_panels_and_chart_properties_fit_the_smallest_window() {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::default(), None, None);
    app.insert_chart(ChartKind::StepArea);
    for panel in [
        LibraryPanel::Models,
        LibraryPanel::Symbols,
        LibraryPanel::Text,
        LibraryPanel::Charts,
    ] {
        app.library_panel = panel;
        for _ in 0..2 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                    ..Default::default()
                },
                |ctx| {
                    app.header(ctx);
                    app.footer(ctx);
                    app.sidebar(ctx);
                    app.inspector(ctx);
                    app.canvas(ctx, false);
                },
            );
        }
        assert!(app.canvas.width() > 500.);
        assert!(app.canvas.height() > 400.);
    }
    app.show_templates = true;
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
            ..Default::default()
        },
        |ctx| {
            app.canvas(ctx, false);
            app.library_dialogs(ctx);
        },
    );
    assert!(app.show_templates);
}

fn secondary_click(ctx: &egui::Context, app: &mut DrawApp, p: Pos2) {
    for pressed in [true, false] {
        frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Secondary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            egui::Modifiers::NONE,
        );
    }
}
fn text_position(ctx: &egui::Context, app: &mut DrawApp, text: &str) -> Pos2 {
    let mut result = None;
    for _ in 0..3 {
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
                ..Default::default()
            },
            |ctx| {
                app.canvas(ctx, true);
            },
        );
        for clipped in output.shapes {
            if let Shape::Text(t) = clipped.shape
                && t.galley.job.text == text
            {
                result = Some(t.pos + t.galley.rect.center().to_vec2());
            }
        }
    }
    result.unwrap_or_else(|| panic!("Texte introuvable : {text}"))
}
#[test]
fn right_click_targets_objects_without_panning_or_changing_history() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    let b = app.doc.add_node(ShapeKind::Ellipse, [800., 500.]);
    app.commit();
    let before = app.doc.clone();
    let pan = app.pan;
    secondary_click(&ctx, &mut app, pos2(300., 300.));
    assert!(app.context_open);
    assert_eq!(app.selected, vec![a]);
    assert_eq!(app.pan, pan);
    assert_eq!(app.doc, before);
    app.context_open = false;
    app.selected = vec![a, b];
    secondary_click(&ctx, &mut app, pos2(800., 500.));
    assert_eq!(app.selected, vec![a, b]);
    app.context_open = false;
    secondary_click(&ctx, &mut app, pos2(1100., 700.));
    assert!(app.selected.is_empty());
    assert_eq!(app.doc, before);
    app.undo();
    assert!(app.doc.nodes.is_empty());
}
#[test]
fn context_delete_button_cascades_and_is_one_undo_action() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [800., 500.]);
    app.doc.connect(a, b, ConnectorKind::Elbow);
    app.commit();
    let before = app.doc.clone();
    secondary_click(&ctx, &mut app, pos2(300., 300.));
    let p = text_position(&ctx, &mut app, "Supprimer");
    click(&ctx, &mut app, p);
    assert_eq!(app.doc.nodes.len(), 1);
    assert!(app.doc.connectors.is_empty());
    assert!(!app.context_open);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn dismissing_context_menu_does_not_click_through_to_the_canvas() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    secondary_click(&ctx, &mut app, pos2(300., 300.));
    text_position(&ctx, &mut app, "Supprimer");
    click(&ctx, &mut app, pos2(1100., 730.));
    assert!(!app.context_open);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
}
#[test]
fn contextual_paste_uses_clicked_location_and_supports_plain_text() {
    let (ctx, mut app) = setup();
    let mut fragment = Document::default();
    fragment.add_node(ShapeKind::Document, [200., 100.]);
    fragment.add_chart(ChartKind::Columns, [600., 400.]);
    app.context_position = [400., 300.];
    app.perform_action(context_menu::Action::Paste, &ctx);
    app.paste_text(&serde_json::to_string(&fragment).unwrap());
    assert_eq!(app.doc.nodes[0].position, [400., 300.]);
    assert_eq!(app.doc.charts.len(), 1);
    app.undo();
    assert!(app.doc.nodes.is_empty());
    app.context_position = [700., 500.];
    app.perform_action(context_menu::Action::Paste, &ctx);
    app.paste_text("Note collée");
    assert_eq!(app.doc.nodes[0].text, "Note collée");
    assert_eq!(app.doc.nodes[0].center(), [700., 500.]);
}
#[test]
fn floating_toolbar_does_not_create_shapes_behind_its_buttons() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.doc.add_node(ShapeKind::Ellipse, [300., 300.]);
    app.selected = vec![a];
    app.commit();
    app.set_tool(Tool::Shape(ShapeKind::Diamond));
    let p = text_position(&ctx, &mut app, "Premier plan");
    click(&ctx, &mut app, p);
    assert_eq!(app.doc.nodes.len(), 2);
    assert_eq!(app.doc.nodes.last().unwrap().id, a);
    app.doc.validate().unwrap();
}
#[test]
fn manual_connector_waypoint_drags_and_undoes_as_one_action() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [200., 250.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [900., 550.]);
    let id = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.doc.set_automatic_route(id, false).unwrap();
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    let pt = app.screen(app.doc.connectors[0].waypoints[0]);
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    pointer(&ctx, &mut app, pt, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        pt + vec2(40., 30.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(
        app.doc.connectors[0].waypoints[0],
        [
            before.connectors[0].waypoints[0][0] + 40.,
            before.connectors[0].waypoints[0][1] + 30.
        ]
    );
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn keyboard_context_menu_and_escape_preserve_selection_and_document() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::F10,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::SHIFT,
        }],
        egui::Modifiers::SHIFT,
    );
    assert!(app.context_open);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    assert!(!app.context_open);
    assert_eq!(app.selected, vec![id]);
    assert_eq!(app.doc, before);
}
#[test]
fn contextual_replacement_preserves_style_text_and_connected_ids() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [800., 500.]);
    app.doc.connect(id, b, ConnectorKind::Elbow);
    app.doc.node_mut(id).unwrap().text = "À conserver".into();
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    app.perform_action(context_menu::Action::Replace(ShapeKind::Database), &ctx);
    assert_eq!(app.doc.node(id).unwrap().kind, ShapeKind::Database);
    assert_eq!(app.doc.node(id).unwrap().text, "À conserver");
    assert_eq!(
        app.doc.node(id).unwrap().fill,
        before.node(id).unwrap().fill
    );
    assert_eq!(app.doc.connectors[0].from, id);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn rendering_context_and_properties_does_not_clamp_valid_imported_styles() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [200., 300.]);
    let b = app.doc.add_node(ShapeKind::Ellipse, [800., 500.]);
    let edge = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    let drawing = app
        .doc
        .add_drawing(DrawingKind::Bezier, [100., 600.], [600., 600.]);
    let n = app.doc.node_mut(a).unwrap();
    n.font_size = 96.;
    n.stroke_width = 12.;
    n.position[0] = -20.;
    app.doc.connectors[0].width = 12.;
    app.doc.drawings[0].width = 12.;
    app.doc.drawings[0].bend = 1200.;
    app.doc.validate().unwrap();
    let chart = app.doc.add_chart(ChartKind::Line, [400., 400.]);
    app.doc.charts[0].position[0] = -20.;
    for id in [a, edge, drawing, chart] {
        app.selected = vec![id];
        let before = app.doc.clone();
        app.open_context(pos2(500., 400.), false);
        for _ in 0..3 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 960.))),
                    ..Default::default()
                },
                |ctx| {
                    app.header(ctx);
                    app.inspector(ctx);
                    app.canvas(ctx, true);
                },
            );
        }
        assert_eq!(app.doc, before);
        app.context_open = false;
    }
}

#[test]
fn library_manager_stages_preferences_applies_atomically_and_preserves_drawing() {
    let (ctx, mut app) = setup();
    app.doc = Document::demo();
    app.commit();
    let before = app.doc.clone();
    let original = app.library.visible_libraries.clone();
    app.open_symbol_manager();
    app.visible_draft = vec!["uml".into()];
    app.show_symbol_manager = false;
    assert_eq!(app.library.visible_libraries, original);
    app.open_symbol_manager();
    app.visible_draft.clear();
    app.apply_libraries();
    assert!(app.library.visible_libraries.is_empty());
    assert_eq!(app.doc, before);
    let temp = tempfile::tempdir().unwrap();
    let invalid = temp.path().join("directory.json");
    std::fs::create_dir(&invalid).unwrap();
    app.library_path = Some(invalid);
    app.open_symbol_manager();
    app.visible_draft = vec!["basic".into()];
    app.apply_libraries();
    assert!(app.error.is_some());
    assert!(app.show_symbol_manager);
    assert!(app.library.visible_libraries.is_empty());
    drop(ctx);
}
#[test]
fn save_archives_previous_work_and_failed_saves_do_not_add_new_version() {
    let (_, mut app) = setup();
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("work.draw");
    app.doc = Document::demo();
    assert!(app.save_to(file.clone()));
    let original = app.doc.clone();
    app.doc.title = "Second save".into();
    assert!(app.save_to(file.clone()));
    let key = app.gallery.projects()[0].key.clone();
    assert_eq!(app.gallery.revision(&key, 1).unwrap(), original);
    assert!(!app.dirty());
    let count = app.gallery.projects().len();
    app.doc.title = "Must remain unsaved".into();
    assert!(!app.save_to(temp.path().join("missing/work.draw")));
    assert!(app.dirty());
    assert_eq!(app.gallery.projects().len(), count);
    app.restore_revision(&key, 1);
    assert!(app.path.is_none());
    assert_eq!(app.doc.nodes, original.nodes);
    app.switch_tab(0);
    assert_eq!(app.doc.title, "Must remain unsaved");
    assert!(app.dirty());
    assert_eq!(Document::load(&file).unwrap().title, "Second save");
}
#[test]
fn gallery_failure_does_not_report_successfully_saved_work_as_unsaved() {
    let (_, mut app) = setup();
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("cache");
    std::fs::write(&root, b"not a folder").unwrap();
    app.gallery = gallery::Gallery::open(root).unwrap();
    app.doc.title = "Saved despite cache error".into();
    let path = temp.path().join("primary.draw");
    assert!(app.save_to(path.clone()));
    assert!(!app.dirty());
    assert_eq!(Document::load(&path).unwrap().title, app.doc.title);
    assert!(
        app.error
            .as_deref()
            .unwrap()
            .contains("Document enregistré")
    );
}
#[test]
fn workspace_dialogs_fit_supported_window_and_do_not_mutate_documents() {
    for mode in ["bibliotheques", "travaux", "versions"] {
        let ctx = egui::Context::default();
        let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
        app.capture_tab(mode);
        let doc = app.doc.clone();
        for iteration in 0..3 {
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                    ..Default::default()
                },
                |ctx| {
                    app.header(ctx);
                    app.footer(ctx);
                    app.sidebar(ctx);
                    app.canvas(ctx, false);
                    app.workspace_dialogs(ctx);
                },
            );
            if iteration == 2 {
                let id = if mode == "bibliotheques" {
                    "symbol_manager"
                } else {
                    "saved_projects"
                };
                let rect = ctx.memory(|m| m.area_rect(egui::Id::new(id))).unwrap();
                assert!(
                    rect.left() >= 0.
                        && rect.top() >= 0.
                        && rect.right() <= 1200.
                        && rect.bottom() <= 760.,
                    "{mode}: {rect:?}"
                );
            }
            for shape in &output.shapes {
                if let Shape::Rect(r) = &shape.shape {
                    if r.fill == Color32::from_black_alpha(100) {
                        continue;
                    }
                    assert!(r.rect.width() < 2000.);
                }
            }
        }
        assert_eq!(app.doc, doc);
    }
}
fn workspace_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            events,
            ..Default::default()
        },
        |ctx| {
            app.header(ctx);
            app.footer(ctx);
            app.canvas(ctx, false);
            app.workspace_dialogs(ctx);
        },
    )
}
fn workspace_click_text(ctx: &egui::Context, app: &mut DrawApp, text: &str) {
    let mut points = vec![];
    let mut labels = vec![];
    for _ in 0..3 {
        points.clear();
        labels.clear();
        let output = workspace_frame(ctx, app, vec![]);
        for s in output.shapes {
            if let Shape::Text(t) = s.shape {
                labels.push(t.galley.job.text.clone());
                if t.galley.job.text == text {
                    points.push(t.pos + t.galley.rect.center().to_vec2());
                }
            }
        }
    }
    let pos = *points.last().unwrap_or_else(|| {
        panic!(
            "Bouton introuvable : {text}. Gestionnaire={}, zone={:?}. Labels: {labels:?}",
            app.show_symbol_manager,
            ctx.memory(|m| m.area_rect(egui::Id::new("symbol_manager")))
        )
    });
    for pressed in [true, false] {
        workspace_frame(
            ctx,
            app,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}
#[test]
fn manager_checkbox_cancel_and_apply_buttons_update_persisted_preferences() {
    let (ctx, mut app) = setup();
    let temp = tempfile::tempdir().unwrap();
    app.library_path = Some(temp.path().join("library.json"));
    let original = app.library.visible_libraries.clone();
    app.open_symbol_manager();
    workspace_click_text(&ctx, &mut app, "Formes simples");
    assert!(app.visible_draft.iter().any(|s| s == "basic"));
    assert!(
        app.show_symbol_manager,
        "Le clic sur la case ferme le gestionnaire"
    );
    workspace_click_text(&ctx, &mut app, "Annuler");
    assert!(!app.show_symbol_manager);
    assert_eq!(app.library.visible_libraries, original);
    app.open_symbol_manager();
    workspace_click_text(&ctx, &mut app, "Formes simples");
    workspace_click_text(&ctx, &mut app, "Appliquer");
    assert!(!app.show_symbol_manager);
    assert!(
        Library::load(app.library_path.as_ref().unwrap())
            .unwrap()
            .visible("basic")
    );
}
#[test]
fn gallery_buttons_recover_older_version_and_keep_current_work() {
    let (ctx, mut app) = setup();
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("work.draw");
    app.doc.title = "Première version".into();
    assert!(app.save_to(file.clone()));
    app.doc.title = "Deuxième version".into();
    assert!(app.save_to(file));
    app.doc.title = "Modifications non enregistrées".into();
    app.open_gallery();
    workspace_click_text(&ctx, &mut app, "Versions (2)");
    assert!(app.gallery_versions.is_some());
    workspace_click_text(&ctx, &mut app, "Récupérer une copie");
    assert!(!app.show_gallery);
    assert_eq!(app.doc.title, "Copie — Première version");
    assert!(app.path.is_none());
    app.switch_tab(0);
    assert_eq!(app.doc.title, "Modifications non enregistrées");
    assert!(app.dirty());
}

#[test]
fn eight_resize_handles_keep_the_opposite_edge_fixed_and_undo_as_one_action() {
    for index in 0..8 {
        let (ctx, mut app) = setup();
        app.snap = false;
        app.show_quick_toolbar = false;
        let id = app.doc.add_node(ShapeKind::Rectangle, [600., 400.]);
        app.selected = vec![id];
        app.commit();
        let before = app.doc.clone();
        let n = app.doc.node(id).unwrap();
        let origin = n.position;
        let size = n.size;
        let rect = Rect::from_min_size(app.screen(origin), vec2(size[0], size[1]));
        let (p, direction) = resize_handles(rect)[index];
        pointer(&ctx, &mut app, p, true, egui::Modifiers::NONE);
        assert!(matches!(app.drag, Some(Drag::Resize { .. })));
        pointer(
            &ctx,
            &mut app,
            p + vec2(40., 20.),
            false,
            egui::Modifiers::NONE,
        );
        let n = app.doc.node(id).unwrap();
        for a in 0..2 {
            if direction[a] < 0 {
                assert_eq!(n.position[a] + n.size[a], origin[a] + size[a]);
                assert!(n.position[a] > origin[a]);
            } else {
                assert_eq!(n.position[a], origin[a]);
                assert_eq!(
                    n.size[a],
                    size[a] + if direction[a] > 0 { [40., 20.][a] } else { 0. }
                );
            }
        }
        app.undo();
        assert_eq!(app.doc, before);
    }
}

#[test]
fn linked_shape_picker_remains_available_in_the_context_menu() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    let id = app.doc.add_node(ShapeKind::Diamond, [500., 400.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    let r = app.selected_node_rect().unwrap();
    let p = connection_buttons(r)
        .iter()
        .find(|(_, port)| *port == Port::Right)
        .unwrap()
        .0;
    app.open_context(p, false);
    app.quick_link_port = Some(Port::Right);
    assert!(app.context_open);
    // The picker is rendered and its shape buttons have pointer senses and tooltips.
    text_position(&ctx, &mut app, "Ajouter une forme liée");
    app.perform_action(
        context_menu::Action::Linked(ShapeKind::Document, Port::Right),
        &ctx,
    );
    assert_eq!(app.doc.nodes.len(), 2);
    assert_eq!(app.doc.connectors[0].from_port, Port::Right);
    assert!(!app.context_open);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn endpoint_drag_reconnects_and_empty_drop_releases_with_undo() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    let a = app.doc.add_node(ShapeKind::Rectangle, [200., 300.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [800., 300.]);
    let c = app.doc.add_node(ShapeKind::Rectangle, [800., 600.]);
    let id = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    let p = app.screen(*app.doc.route(&app.doc.connectors[0]).last().unwrap());
    pointer(&ctx, &mut app, p, true, egui::Modifiers::NONE);
    assert!(matches!(
        app.drag,
        Some(Drag::Endpoint { start: false, .. })
    ));
    pointer(
        &ctx,
        &mut app,
        pos2(800., 560.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.connectors[0].to, c);
    assert_eq!(app.doc.connectors[0].to_port, Port::Top);
    app.undo();
    assert_eq!(app.doc, before);
    app.selected = vec![id];
    pointer(&ctx, &mut app, p, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        pos2(600., 650.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.connectors[0].to, 0);
    assert_eq!(app.doc.connectors[0].free_end, Some([600., 650.]));
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc.connectors[0].to, 0);
}

#[test]
fn automatic_segment_handle_becomes_an_editable_elbow_in_one_drag() {
    let (ctx, mut app) = setup();
    app.snap = false;
    app.show_quick_toolbar = false;
    let a = app.doc.add_node(ShapeKind::Rectangle, [200., 300.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [900., 500.]);
    let id = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    let handle = app.segment_handles(id)[0];
    let p = app.screen(handle);
    pointer(&ctx, &mut app, p, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        p + vec2(0., 40.),
        false,
        egui::Modifiers::NONE,
    );
    assert!(
        app.doc.connectors[0]
            .waypoints
            .contains(&[handle[0], handle[1] + 40.])
    );
    assert_ne!(
        app.doc.route(&app.doc.connectors[0]),
        before.route(&before.connectors[0])
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn pinned_toolbar_keeps_position_and_never_edits_document_on_render() {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [500., 400.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [800., 600.]);
    app.selected = vec![a];
    app.commit();
    let before = app.doc.clone();
    let p = text_position(&ctx, &mut app, "Épingler");
    click(&ctx, &mut app, p);
    assert!(app.toolbar_pinned);
    let pos = app.toolbar_position;
    app.selected = vec![b];
    text_position(&ctx, &mut app, "Détacher");
    assert_eq!(app.toolbar_position, pos);
    assert_eq!(app.doc, before);
    let bounds = ctx
        .memory(|m| m.area_rect(egui::Id::new("selection_toolbar")))
        .unwrap();
    assert!(bounds.height() < 145., "{bounds:?}");
    assert!(app.canvas.contains_rect(bounds));
}

#[test]
fn connector_brush_copies_styles_without_changing_geometry_and_can_undo() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    let a = app.doc.add_node(ShapeKind::Rectangle, [150., 250.]);
    let b = app.doc.add_node(ShapeKind::Rectangle, [850., 250.]);
    let c = app.doc.add_node(ShapeKind::Rectangle, [150., 600.]);
    let d = app.doc.add_node(ShapeKind::Rectangle, [850., 600.]);
    let source = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.doc.connect(c, d, ConnectorKind::Straight).unwrap();
    app.doc.connectors[0].pattern = LinePattern::Dotted;
    app.doc.connectors[0].end_head = ArrowHead::Diamond;
    app.doc.connectors[0].color = [120, 40, 80];
    app.commit();
    app.selected = vec![source];
    let before = app.doc.clone();
    app.perform_action(context_menu::Action::Brush, &ctx);
    click(&ctx, &mut app, pos2(500., 600.));
    assert_eq!(app.doc.connectors[1].pattern, LinePattern::Dotted);
    assert_eq!(app.doc.connectors[1].end_head, ArrowHead::Diamond);
    assert_eq!(app.doc.connectors[1].kind, ConnectorKind::Straight);
    assert_eq!(
        (app.doc.connectors[1].from, app.doc.connectors[1].to),
        (c, d)
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn connect_tool_creates_both_free_endpoints_and_body_drag_is_undoable() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    app.snap = false;
    app.set_tool(Tool::Connect);
    click(&ctx, &mut app, pos2(250., 250.));
    click(&ctx, &mut app, pos2(900., 550.));
    assert!(app.doc.nodes.is_empty());
    assert_eq!(app.doc.connectors.len(), 1);
    assert_eq!(app.doc.connectors[0].free_start, Some([250., 250.]));
    assert_eq!(app.doc.connectors[0].free_end, Some([900., 550.]));
    let id = app.doc.connectors[0].id;
    app.set_tool(Tool::Select);
    app.selected.clear();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(350., 250.),
        true,
        egui::Modifiers::NONE,
    );
    assert!(matches!(app.drag, Some(Drag::FreeConnector { .. })));
    pointer(
        &ctx,
        &mut app,
        pos2(390., 280.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.connectors[0].free_start, Some([290., 280.]));
    assert_eq!(app.selected, vec![id]);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn navigating_pages_is_clean_and_edits_on_inactive_pages_are_saved() {
    let (ctx, mut app) = setup();
    app.doc = templates::multipage();
    app.saved = app.doc.clone();
    app.history = History::new(&app.doc);
    app.selected.clear();
    app.switch_page(1);
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    assert!(!app.dirty());
    assert!(!app.history.can_undo());
    app.doc.node_mut(2).unwrap().text = "Texte conservé".into();
    app.commit();
    app.switch_page(2);
    assert!(app.dirty());
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("dossier.draw");
    app.save_to(path.clone());
    assert!(!app.dirty());
    let mut loaded = Document::load(&path).unwrap();
    loaded.switch_page(1).unwrap();
    assert_eq!(loaded.node(2).unwrap().text, "Texte conservé");
    app.switch_page(0);
    assert!(!app.dirty());
}
#[test]
fn page_dimensions_cancel_apply_and_navigation_preserve_other_pages() {
    let (ctx, mut app) = setup();
    app.doc = templates::multipage();
    app.history = History::new(&app.doc);
    let before = app.doc.clone();
    app.open_page_settings();
    app.page_draft = None;
    assert_eq!(app.doc, before);
    app.doc
        .resize_page(
            "Format personnalisé".into(),
            [1000., 1000.],
            diagcraft::pages::PageLayout {
                unit: diagcraft::pages::Unit::Millimeters,
                auto_size: false,
            },
        )
        .unwrap();
    app.commit();
    assert_eq!(app.doc.page(1), before.page(1));
    app.undo();
    assert_eq!(app.doc, before);
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
            ..Default::default()
        },
        |ctx| {
            app.header(ctx);
            app.footer(ctx);
            app.pages_bar(ctx);
            app.canvas(ctx, true);
        },
    );
    assert!(!output.shapes.is_empty());
    assert!(app.canvas.bottom() < 720.);
    assert_eq!(app.doc, before);
}
#[test]
fn multipage_dialogs_fit_the_smallest_window_and_render_without_edits() {
    for dialog in ["pages", "size"] {
        let (ctx, mut app) = setup();
        app.doc = templates::multipage();
        let before = app.doc.clone();
        if dialog == "pages" {
            app.show_pages = true;
        } else {
            app.open_page_settings();
        }
        for _ in 0..3 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                    ..Default::default()
                },
                |ctx| {
                    app.pages_dialogs(ctx);
                },
            );
        }
        let name = if dialog == "pages" {
            "page_manager"
        } else {
            "page_dimensions"
        };
        let rect = ctx.memory(|m| m.area_rect(egui::Id::new(name))).unwrap();
        assert!(
            rect.width() <= 1160. && rect.height() <= 720.,
            "{name}: {rect:?}"
        );
        assert_eq!(app.doc, before);
    }
}
#[test]
fn auto_page_size_grows_with_drag_and_undo_restores_size() {
    let (ctx, mut app) = setup();
    app.snap = false;
    app.show_quick_toolbar = false;
    app.doc.page_size = [600., 500.];
    app.doc.page_layout.auto_size = true;
    let n = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
    app.commit();
    app.selected = vec![n];
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(300., 300.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(900., 650.),
        false,
        egui::Modifiers::NONE,
    );
    assert!(app.doc.page_size[0] > 1000.);
    assert!(app.doc.page_size[1] > 700.);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn group_selection_moves_free_points_and_elbows_with_their_nodes() {
    let (ctx, mut app) = setup();
    app.snap = false;
    app.show_quick_toolbar = false;
    let a = app.doc.add_node(ShapeKind::Rectangle, [200., 300.]);
    let id = app
        .doc
        .connect_endpoints(a, 0, None, Some([800., 600.]), ConnectorKind::Elbow)
        .unwrap();
    app.doc.set_automatic_route(id, false).unwrap();
    app.commit();
    let before = app.doc.clone();
    app.perform_action(context_menu::Action::SelectAll, &ctx);
    assert!(app.selected.contains(&id));
    pointer(
        &ctx,
        &mut app,
        pos2(200., 300.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(240., 330.),
        false,
        egui::Modifiers::NONE,
    );
    let expected = before
        .route(&before.connectors[0])
        .iter()
        .map(|p| [p[0] + 40., p[1] + 30.])
        .collect::<Vec<_>>();
    assert_eq!(app.doc.route(&app.doc.connectors[0]), expected);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn agent_batch_preview_preserves_unsaved_work_and_applies_one_undo_step() {
    let (_, mut app) = setup();
    app.doc.title = "Mon travail non enregistré".into();
    app.commit();
    let before = app.doc.clone();
    let saved = app.saved.clone();
    app.open_agent_panel();
    assert!(app.agent_panel.is_some());
    let patch=serde_json::json!({"api_version":1,"expected_revision":diagcraft::agent::revision(&app.doc),"operations":[{"op":"add_node","page":0,"alias":"a","kind":"rectangle","position":[100,100],"size":[220,84],"text":"A"},{"op":"add_node","page":0,"alias":"b","kind":"text","position":[400,100],"size":[220,48],"text":"B"}]}).to_string();
    assert_eq!(app.doc, before);
    app.apply_agent_patch(&patch).unwrap();
    assert_eq!(app.doc.nodes.len(), 2);
    assert_eq!(app.saved, saved);
    app.agent_panel = None;
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc.nodes.len(), 2);
}
#[test]
fn agent_panel_blocks_document_shortcuts_and_rejects_stale_patch() {
    let (ctx, mut app) = setup();
    let patch=serde_json::json!({"api_version":1,"expected_revision":diagcraft::agent::revision(&app.doc),"operations":[{"op":"set_title","title":"Agent"}]}).to_string();
    app.doc.title = "Changement utilisateur".into();
    app.commit();
    app.open_agent_panel();
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::Z,
            physical_key: Some(Key::Z),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::COMMAND,
        }],
        egui::Modifiers::COMMAND,
    );
    assert_eq!(app.doc.title, "Changement utilisateur");
    assert!(app.apply_agent_patch(&patch).is_err());
    assert_eq!(app.doc.title, "Changement utilisateur");
}
#[test]
fn copied_agent_context_contains_schema_guide_and_unsaved_revision() {
    let doc = Document {
        title: "Pas encore enregistré".into(),
        ..Document::default()
    };
    let context: serde_json::Value =
        serde_json::from_str(&agent_ui::context_for_agent(&doc).unwrap()).unwrap();
    assert_eq!(context["revision"], diagcraft::agent::revision(&doc));
    assert_eq!(
        context["agent_contract"]["patch_schema"]["properties"]["api_version"]["const"],
        1
    );
    assert!(
        context["agent_contract"]["guide"]
            .as_str()
            .unwrap()
            .contains("expected_revision")
    );
}

fn connection_fixture() -> (egui::Context, DrawApp, u64, u64) {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    app.snap = false;
    let a = app.doc.add_node(ShapeKind::Rounded, [250., 300.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [800., 480.]);
    app.commit();
    (ctx, app, a, b)
}
#[test]
fn blue_handle_drags_directly_to_an_existing_block_in_one_undo_step() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    let before = app.doc.clone();
    let start = connection_buttons(app.selected_node_rect().unwrap())[1].0;
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    assert_eq!(app.connect_from, Some(a));
    assert!(matches!(app.drag, Some(Drag::Connect { .. })));
    assert!(!app.context_open);
    let end = app.screen(app.doc.node(b).unwrap().center());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(end)],
        egui::Modifiers::NONE,
    );
    assert!(app.doc.connectors.is_empty());
    assert_eq!(app.doc.nodes, before.nodes);
    pointer(&ctx, &mut app, end, false, egui::Modifiers::NONE);
    assert_eq!(app.doc.connectors.len(), 1);
    let edge = &app.doc.connectors[0];
    assert_eq!((edge.from, edge.to), (a, b));
    assert_eq!(edge.from_port, Port::Right);
    assert!(app.tool == Tool::Select);
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc.connectors.len(), 1);
}
#[test]
fn hover_reveals_handles_on_unselected_block_and_bridge_to_handle_does_not_flicker() {
    let (ctx, mut app, a, b) = connection_fixture();
    let center = app.screen(app.doc.node(a).unwrap().center());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(center)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.hover_link_node, Some(a));
    assert!(app.selected.is_empty());
    let start = connection_buttons(app.node_screen_rect(app.doc.node(a).unwrap()))[1].0;
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(start)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.connection_handle_at(start), Some((a, Port::Right)));
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    let end = app.screen(app.doc.node(b).unwrap().center());
    pointer(&ctx, &mut app, end, false, egui::Modifiers::NONE);
    assert_eq!(app.doc.connectors[0].to, b);
}
#[test]
fn link_tool_supports_press_drag_release_as_well_as_two_clicks() {
    let (ctx, mut app, a, b) = connection_fixture();
    let before = app.doc.clone();
    app.set_tool(Tool::Connect);
    let start = app.screen(app.doc.node(a).unwrap().center());
    let end = app.screen(app.doc.node(b).unwrap().center());
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    pointer(&ctx, &mut app, end, false, egui::Modifiers::NONE);
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (a, b)
    );
    app.undo();
    assert_eq!(app.doc, before);
    app.set_tool(Tool::Connect);
    click(&ctx, &mut app, start);
    assert_eq!(app.connect_from, Some(a));
    click(&ctx, &mut app, end);
    assert_eq!(app.doc.connectors.len(), 1);
}
#[test]
fn a_click_on_blue_handle_arms_linking_without_opening_a_picker() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    let start = connection_buttons(app.selected_node_rect().unwrap())[2].0;
    click(&ctx, &mut app, start);
    assert_eq!(app.connect_from, Some(a));
    assert!(app.drag.is_none());
    assert!(!app.context_open);
    assert!(app.doc.connectors.is_empty());
    let end = app.screen(app.doc.node(b).unwrap().center());
    click(&ctx, &mut app, end);
    assert_eq!(app.doc.connectors[0].from_port, Port::Bottom);
}
#[test]
fn ribbon_link_action_reuses_one_selected_block_and_links_two_selected_blocks_immediately() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    app.activate_connect_tool();
    assert_eq!(app.connect_from, Some(a));
    let end = app.screen(app.doc.node(b).unwrap().center());
    click(&ctx, &mut app, end);
    assert_eq!(app.doc.connectors.len(), 1);
    app.undo();
    app.selected = vec![b, a];
    app.activate_connect_tool();
    assert_eq!(app.doc.connectors.len(), 1);
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (b, a)
    );
    assert!(app.tool == Tool::Select);
}
#[test]
fn magnetic_connection_targets_use_screen_distance_at_different_zooms() {
    let (_, mut app, a, b) = connection_fixture();
    for zoom in [0.25, 1., 2.] {
        app.zoom = zoom;
        app.pan = vec2(10., 25.);
        let n = app.doc.node(b).unwrap();
        let top = app.screen(n.port_position(Port::Top, n.center()));
        let near = top - vec2(0., 9.);
        let target = app.connection_target(near).unwrap();
        assert_eq!(target.id, b);
        assert_eq!(target.port, Port::Top);
        app.selected = vec![a];
        app.activate_connect_tool();
        assert!(app.finish_connection(near));
        let e = app.doc.connectors.last().unwrap();
        assert_eq!(e.to, b);
        assert!(e.free_end.is_none());
        app.undo();
    }
}
#[test]
fn escaping_a_link_drag_or_switching_page_discards_only_the_draft() {
    let (ctx, mut app, a, _) = connection_fixture();
    app.selected = vec![a];
    let before = app.doc.clone();
    let start = connection_buttons(app.selected_node_rect().unwrap())[1].0;
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    assert!(app.connect_from.is_none());
    assert!(app.drag.is_none());
    pointer(
        &ctx,
        &mut app,
        pos2(800., 480.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    app.doc.add_page(false).unwrap();
    app.switch_page(0);
    app.selected = vec![a];
    app.activate_connect_tool();
    app.switch_page(1);
    assert!(app.connect_from.is_none());
    assert!(app.drag.is_none());
    assert_eq!(app.doc.object_count(), before.object_count());
}
#[test]
fn dropping_a_link_in_empty_canvas_keeps_the_source_attached() {
    let (ctx, mut app, a, _) = connection_fixture();
    app.selected = vec![a];
    let start = connection_buttons(app.selected_node_rect().unwrap())[2].0;
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        pos2(500., 650.),
        false,
        egui::Modifiers::NONE,
    );
    let e = &app.doc.connectors[0];
    assert_eq!(e.from, a);
    assert_eq!(e.to, 0);
    assert_eq!(e.free_end, Some([500., 650.]));
    assert_eq!(e.from_port, Port::Bottom);
}
#[test]
fn self_link_and_duplicate_leave_document_intact_and_allow_another_target() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.set_tool(Tool::Connect);
    let start = app.screen(app.doc.node(a).unwrap().center());
    click(&ctx, &mut app, start);
    click(&ctx, &mut app, start);
    assert!(app.doc.connectors.is_empty());
    assert_eq!(app.connect_from, Some(a));
    click(&ctx, &mut app, pos2(800., 480.));
    let before = app.doc.clone();
    app.selected = vec![a];
    app.activate_connect_tool();
    assert!(!app.finish_connection(pos2(800., 480.)));
    assert_eq!(app.doc, before);
    assert_eq!(app.connect_from, Some(a));
    let c = app.doc.add_node(ShapeKind::Rectangle, [900., 650.]);
    assert!(app.finish_connection(pos2(900., 650.)));
    assert_eq!(app.doc.connectors.len(), 2);
    assert_eq!(app.doc.connectors.last().unwrap().to, c);
    assert_eq!(app.doc.connectors[0].to, b);
}
#[test]
fn drop_on_ui_outside_canvas_cancels_link_instead_of_creating_a_free_end() {
    let (ctx, mut app, a, _) = connection_fixture();
    app.selected = vec![a];
    let before = app.doc.clone();
    let start = connection_buttons(app.selected_node_rect().unwrap())[1].0;
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        pos2(-20., 200.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    assert!(app.connect_from.is_none());
    assert!(app.drag.is_none());
    assert!(app.tool == Tool::Select);
}
#[test]
fn reconnecting_endpoint_near_block_snaps_instead_of_accidentally_leaving_it_free() {
    let (ctx, mut app, a, b) = connection_fixture();
    let c = app.doc.add_node(ShapeKind::Rectangle, [800., 650.]);
    let id = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    let start = app.screen(*app.doc.route(&app.doc.connectors[0]).last().unwrap());
    let n = app.doc.node(c).unwrap();
    let near = app.screen(n.port_position(Port::Top, n.center())) - vec2(0., 8.);
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    pointer(&ctx, &mut app, near, false, egui::Modifiers::NONE);
    assert_eq!(app.doc.connectors[0].to, c);
    assert_eq!(app.doc.connectors[0].to_port, Port::Top);
    assert!(app.doc.connectors[0].free_end.is_none());
    app.undo();
    assert_eq!(app.doc, before);
}

fn full_connection_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1440., 960.))),
            events,
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            let before = app.doc.clone();
            app.header(ctx);
            app.footer(ctx);
            app.pages_bar(ctx);
            app.sidebar(ctx);
            app.inspector(ctx);
            app.canvas(ctx, true);
            if app.drag.is_none() && app.doc != before {
                app.commit();
            }
        },
    )
}
fn full_pointer(ctx: &egui::Context, app: &mut DrawApp, pos: Pos2, pressed: bool) {
    full_connection_frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}
#[test]
fn full_window_with_sidebar_inspector_and_toolbar_allows_a_handle_to_block_drag() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.show_quick_toolbar = true;
    app.selected = vec![a];
    app.fit_next = true;
    for _ in 0..3 {
        full_connection_frame(&ctx, &mut app, vec![]);
    }
    let before = app.doc.clone();
    let start = connection_buttons(app.selected_node_rect().unwrap())[1].0;
    let end = app.screen(app.doc.node(b).unwrap().center());
    full_connection_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(start)]);
    full_pointer(&ctx, &mut app, start, true);
    assert!(matches!(app.drag, Some(Drag::Connect { .. })));
    full_connection_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(end)]);
    full_pointer(&ctx, &mut app, end, false);
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (a, b)
    );
    assert_eq!(app.doc.nodes, before.nodes);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn actual_ribbon_button_starts_linking_without_a_dropdown() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    app.fit_next = true;
    let mut button = None;
    for _ in 0..3 {
        let out = full_connection_frame(&ctx, &mut app, vec![]);
        for clipped in out.shapes {
            if let Shape::Text(t) = clipped.shape
                && t.galley.job.text == "Relier · C"
            {
                button = Some(t.pos + t.galley.size() / 2.);
            }
        }
    }
    let button = button.expect("direct link button");
    full_pointer(&ctx, &mut app, button, true);
    full_pointer(&ctx, &mut app, button, false);
    assert_eq!(app.connect_from, Some(a));
    assert!(!app.context_open);
    let end = app.screen(app.doc.node(b).unwrap().center());
    full_pointer(&ctx, &mut app, end, true);
    full_pointer(&ctx, &mut app, end, false);
    assert_eq!(app.doc.connectors[0].to, b);
}
#[test]
fn link_two_selected_blocks_honors_ids_when_an_unselected_block_covers_the_target() {
    let (_, mut app, a, b) = connection_fixture();
    let c = app
        .doc
        .add_node(ShapeKind::Rectangle, app.doc.node(b).unwrap().center());
    app.commit();
    assert_eq!(app.doc.hit_node(app.doc.node(b).unwrap().center()), Some(c));
    app.selected = vec![a, b];
    app.activate_connect_tool();
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (a, b)
    );
}

fn panel_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            focused: !events
                .iter()
                .any(|e| matches!(e, egui::Event::WindowFocused(false))),
            modifiers: events
                .iter()
                .find_map(|e| {
                    if let egui::Event::Key { modifiers, .. } = e {
                        Some(*modifiers)
                    } else {
                        None
                    }
                })
                .unwrap_or_default(),
            events,
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            let before = app.doc.clone();
            app.header(ctx);
            app.footer(ctx);
            app.pages_bar(ctx);
            app.side_panels(ctx);
            app.canvas(ctx, app.rich_editor.is_none());
            app.finish_frame_edit(ctx, &before);
            app.inline_text_toolbar(ctx);
        },
    )
}
fn panel_pointer(ctx: &egui::Context, app: &mut DrawApp, size: Vec2, p: Pos2, pressed: bool) {
    panel_frame(
        ctx,
        app,
        size,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
}
fn panel_click(ctx: &egui::Context, app: &mut DrawApp, size: Vec2, p: Pos2) {
    panel_pointer(ctx, app, size, p, true);
    panel_pointer(ctx, app, size, p, false);
}
fn header_panel_icons(out: egui::FullOutput) -> Vec<Pos2> {
    let mut result = out
        .shapes
        .into_iter()
        .filter_map(|s| {
            if let Shape::Rect(r) = s.shape {
                ((r.rect.width() - 20.).abs() < 0.1
                    && (r.rect.height() - 16.).abs() < 0.1
                    && r.rect.center().y < 38.)
                    .then_some(r.rect.center())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    result.sort_by(|a, b| a.x.total_cmp(&b.x));
    result
}
fn panel_setup() -> (egui::Context, DrawApp, u64) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rounded, [500., 400.]);
    let app = DrawApp::from_context(&ctx, doc, None, None);
    (ctx, app, id)
}
#[test]
fn default_window_hides_document_properties_and_simple_selection_keeps_them_hidden() {
    let (ctx, mut app, id) = panel_setup();
    let size = vec2(1280., 800.);
    assert!(!app.show_inspector);
    for _ in 0..3 {
        let out = panel_frame(&ctx, &mut app, size, vec![]);
        assert!(
            !out.shapes
                .iter()
                .any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text=="Votre document"))
        );
    }
    assert!(app.canvas.width() > 900.);
    let p = app.screen(app.doc.node(id).unwrap().center());
    panel_click(&ctx, &mut app, size, p);
    assert_eq!(app.selected, vec![id]);
    assert!(!app.show_inspector);
    panel_click(&ctx, &mut app, size, p);
    assert!(
        app.rich_editor.is_some(),
        "double-click opens the content editor"
    );
    assert!(
        !app.show_inspector,
        "editing leaves the canvas width unchanged"
    );
    assert_eq!(app.rich_editor.as_ref().unwrap().draft.id, id);
}
#[test]
fn header_icons_collapse_and_reopen_panels_without_modifying_the_drawing() {
    let (ctx, mut app, _) = panel_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let mut icons = vec![];
    for _ in 0..3 {
        icons = header_panel_icons(panel_frame(&ctx, &mut app, size, vec![]));
    }
    assert_eq!(icons.len(), 2);
    let width = app.canvas.width();
    assert!(icons[1].x > size.x - 60.);
    panel_click(&ctx, &mut app, size, icons[0]);
    assert!(!app.show_library);
    for _ in 0..3 {
        panel_frame(&ctx, &mut app, size, vec![]);
    }
    assert!(app.canvas.width() > width + 250.);
    let collapsed = app.canvas.width();
    panel_click(&ctx, &mut app, size, icons[1]);
    assert!(app.show_inspector);
    for _ in 0..3 {
        panel_frame(&ctx, &mut app, size, vec![]);
    }
    assert!(app.canvas.width() < collapsed - 240.);
    panel_click(&ctx, &mut app, size, icons[1]);
    assert!(!app.show_inspector);
    panel_click(&ctx, &mut app, size, icons[0]);
    assert!(app.show_library);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn compact_rail_keeps_all_five_icons_and_symbols_reopens_in_one_click() {
    let (ctx, mut app, _) = panel_setup();
    app.show_library = false;
    let size = vec2(1280., 800.);
    let mut buttons = vec![];
    for _ in 0..3 {
        let out = panel_frame(&ctx, &mut app, size, vec![]);
        buttons = out
            .shapes
            .into_iter()
            .filter_map(|s| {
                if let Shape::Rect(r) = s.shape {
                    ((r.rect.width() - 36.).abs() < 0.1
                        && (r.rect.height() - 40.).abs() < 0.1
                        && r.rect.center().x < 48.)
                        .then_some(r.rect.center())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
    }
    buttons.sort_by(|a, b| a.y.total_cmp(&b.y));
    assert_eq!(buttons.len(), 5);
    assert!(app.canvas.left() < 75.);
    panel_click(&ctx, &mut app, size, buttons[1]);
    assert!(app.show_library);
    assert!(app.library_panel == LibraryPanel::Symbols);
    assert!(!app.show_inspector);
}
#[test]
fn inspector_close_button_returns_space_and_text_edit_opens_without_reopening_it() {
    let (ctx, mut app, id) = panel_setup();
    let size = vec2(1280., 800.);
    app.show_inspector = true;
    let mut close = None;
    for _ in 0..3 {
        let out = panel_frame(&ctx, &mut app, size, vec![]);
        for s in out.shapes {
            if let Shape::Rect(r) = s.shape
                && (r.rect.width() - 20.).abs() < 0.1
                && (r.rect.height() - 16.).abs() < 0.1
                && r.rect.center().x > size.x - 70.
                && r.rect.center().y > app.canvas.top()
            {
                close = Some(r.rect.center());
            }
        }
    }
    panel_click(&ctx, &mut app, size, close.expect("close properties"));
    assert!(!app.show_inspector);
    app.selected = vec![id];
    app.perform_action(context_menu::Action::Text, &ctx);
    assert!(!app.show_inspector);
    assert_eq!(app.rich_editor.as_ref().unwrap().draft.id, id);
}
#[test]
fn compact_panels_fit_small_windows_and_restore_at_larger_sizes() {
    let (ctx, mut app, _) = panel_setup();
    app.show_library = false;
    let before = app.doc.clone();
    for size in [vec2(1000., 660.), vec2(1280., 800.), vec2(1440., 960.)] {
        let mut icons = vec![];
        for _ in 0..3 {
            icons = header_panel_icons(panel_frame(&ctx, &mut app, size, vec![]));
        }
        assert_eq!(icons.len(), 2);
        assert!(icons.iter().all(|p| p.x > 0. && p.x < size.x && p.y < 38.));
        assert!(app.canvas.width() > 900.);
        assert!(app.canvas.height() > 330.);
        assert!(app.canvas.bottom() < size.y);
    }
    assert!(!app.show_library);
    assert!(!app.show_inspector);
    assert_eq!(app.doc, before);
}

fn wheel(delta: Vec2, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::MouseWheel {
        unit: egui::MouseWheelUnit::Point,
        delta,
        modifiers,
    }
}

#[test]
fn trackpad_precise_diagonal_motion_is_immediate_at_any_zoom_and_has_no_replay() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    for zoom in [0.2, 1., 2.5] {
        app.zoom = zoom;
        let initial = app.pan;
        let deltas = [
            vec2(125.5, -83.25),
            vec2(20., -12.),
            vec2(3., -1.5),
            vec2(0.5, -0.25),
        ];
        for delta in deltas {
            let pan = app.pan;
            frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::PointerMoved(pos2(550., 450.)),
                    wheel(delta, egui::Modifiers::NONE),
                ],
                egui::Modifiers::NONE,
            );
            assert!((app.pan - pan - delta).length() < 0.001);
            assert_eq!(app.zoom, zoom);
        }
        let expected = deltas.into_iter().fold(initial, |pan, delta| pan + delta);
        for _ in 0..15 {
            frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
        }
        assert!((app.pan - expected).length() < 0.001);
    }
    assert_eq!(app.doc, before);
    assert!(!app.dirty());
    assert!(!app.history.can_undo());
}

#[test]
fn trackpad_pinch_keeps_pointer_anchor_and_applies_each_factor_once() {
    let (ctx, mut app) = setup();
    let anchor = pos2(580., 370.);
    let world = app.world(anchor);
    let before = app.doc.clone();
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            egui::Event::Zoom(1.2),
            egui::Event::Zoom(1.1),
        ],
        egui::Modifiers::NONE,
    );
    assert!((app.zoom - 1.32).abs() < 0.0001);
    assert!(
        (vec2(app.world(anchor)[0], app.world(anchor)[1]) - vec2(world[0], world[1])).length()
            < 0.001
    );
    for factor in [0.0001, 100_000.] {
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::Zoom(factor)],
            egui::Modifiers::NONE,
        );
        assert!((0.2..=2.5).contains(&app.zoom));
        assert!((app.world(anchor)[0] - world[0]).abs() < 0.001);
        assert!((app.world(anchor)[1] - world[1]).abs() < 0.001);
    }
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

#[test]
fn trackpad_modifier_zoom_uses_event_modifiers_without_smoothed_tail_or_double_zoom() {
    let (ctx, mut app) = setup();
    let anchor = pos2(580., 370.);
    let world = app.world(anchor);
    // A modifier may have been released by the end of the same native frame.
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            wheel(vec2(0., 80.), egui::Modifiers::COMMAND),
        ],
        egui::Modifiers::NONE,
    );
    let expected = (80_f32 * 0.005).exp();
    assert!((app.zoom - expected).abs() < 0.0001);
    let pan = app.pan;
    for _ in 0..15 {
        frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    }
    assert_eq!(app.zoom, expected);
    assert_eq!(app.pan, pan);
    assert!((app.world(anchor)[0] - world[0]).abs() < 0.001);
    assert!((app.world(anchor)[1] - world[1]).abs() < 0.001);
    frame(
        &ctx,
        &mut app,
        vec![wheel(vec2(0., -80.), egui::Modifiers::CTRL)],
        egui::Modifiers::CTRL,
    );
    assert!((app.zoom - 1.).abs() < 0.0001);
}

#[test]
fn trackpad_shift_and_mouse_wheel_units_preserve_navigation() {
    let (ctx, mut app) = setup();
    let anchor = pos2(580., 370.);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            wheel(vec2(0., 50.), egui::Modifiers::SHIFT),
        ],
        egui::Modifiers::SHIFT,
    );
    assert_eq!(app.pan, vec2(50., 0.));
    frame(
        &ctx,
        &mut app,
        vec![wheel(vec2(30., 0.), egui::Modifiers::SHIFT)],
        egui::Modifiers::SHIFT,
    );
    assert_eq!(app.pan, vec2(80., 0.));
    let line_speed = ctx.options(|o| o.input_options.line_scroll_speed);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Line,
            delta: vec2(0., -2.),
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.pan, vec2(80., -2. * line_speed));
}

#[test]
fn trackpad_canvas_navigation_ignores_library_inspector_and_floating_controls() {
    let (ctx, mut app, _) = panel_setup();
    app.show_inspector = true;
    let size = vec2(1280., 800.);
    for _ in 0..3 {
        panel_frame(&ctx, &mut app, size, vec![]);
    }
    let before = app.doc.clone();
    let pan = app.pan;
    let zoom = app.zoom;
    for anchor in [
        pos2(180., 450.),
        pos2(1220., 450.),
        app.canvas.max - vec2(100., 40.),
    ] {
        // Settle hover / layers before the gesture, as in the native loop.
        panel_frame(
            &ctx,
            &mut app,
            size,
            vec![egui::Event::PointerMoved(anchor)],
        );
        panel_frame(
            &ctx,
            &mut app,
            size,
            vec![
                wheel(vec2(25., -80.), egui::Modifiers::NONE),
                egui::Event::Zoom(1.2),
            ],
        );
        assert_eq!(app.pan, pan);
        assert_eq!(app.zoom, zoom);
    }
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

#[test]
fn trackpad_navigates_pending_connection_and_targeting_uses_the_new_view() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    app.activate_connect_tool();
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    let before = app.doc.clone();
    let anchor = pos2(580., 370.);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            wheel(vec2(-130., -60.), egui::Modifiers::NONE),
            egui::Event::Zoom(1.25),
        ],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.connect_from, Some(a));
    assert_eq!(app.doc, before);
    let target = app.screen(app.doc.node(b).unwrap().center());
    click(&ctx, &mut app, target);
    assert_eq!(app.doc.connectors.len(), 1);
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (a, b)
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn trackpad_during_link_drag_preserves_source_and_can_finish_after_panning() {
    let (ctx, mut app, a, b) = connection_fixture();
    app.selected = vec![a];
    let before = app.doc.clone();
    let start = connection_buttons(app.selected_node_rect().unwrap())[1].0;
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![wheel(vec2(-100., 50.), egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert!(matches!(app.drag, Some(Drag::Connect { .. })));
    assert_eq!(app.doc, before);
    assert_eq!(app.pan, vec2(-100., 50.));
    let target = app.screen(app.doc.node(b).unwrap().center());
    pointer(&ctx, &mut app, target, false, egui::Modifiers::NONE);
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (a, b)
    );
}

#[test]
fn trackpad_gesture_does_not_accidentally_move_an_object_during_its_drag() {
    let (ctx, mut app, a, _) = connection_fixture();
    let before = app.doc.clone();
    let start = app.screen(app.doc.node(a).unwrap().center());
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    assert!(matches!(app.drag, Some(Drag::Move { .. })));
    let pan = app.pan;
    let zoom = app.zoom;
    frame(
        &ctx,
        &mut app,
        vec![
            wheel(vec2(100., 50.), egui::Modifiers::NONE),
            egui::Event::Zoom(1.2),
        ],
        egui::Modifiers::NONE,
    );
    pointer(&ctx, &mut app, start, false, egui::Modifiers::NONE);
    assert_eq!(app.doc, before);
    assert_eq!(app.pan, pan);
    assert_eq!(app.zoom, zoom);
}

#[test]
fn trackpad_and_mouse_pan_do_not_overwrite_each_others_translation() {
    let (ctx, mut app) = setup();
    app.set_tool(Tool::Pan);
    let start = pos2(500., 350.);
    pointer(&ctx, &mut app, start, true, egui::Modifiers::NONE);
    frame(
        &ctx,
        &mut app,
        vec![wheel(vec2(100., 50.), egui::Modifiers::NONE)],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.pan, vec2(100., 50.));
    pointer(
        &ctx,
        &mut app,
        start + vec2(60., 20.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.pan, vec2(160., 70.));
    assert!(!app.dirty());
}

#[test]
fn trackpad_ignores_invalid_pinch_and_view_stays_finite() {
    let (ctx, mut app) = setup();
    let anchor = pos2(500., 350.);
    for factor in [f32::NAN, f32::INFINITY, 0., -1.] {
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(anchor), egui::Event::Zoom(factor)],
            egui::Modifiers::NONE,
        );
        assert_eq!(app.zoom, 1.);
        assert_eq!(app.pan, Vec2::ZERO);
    }
}

#[test]
fn trackpad_does_not_navigate_behind_an_open_context_menu() {
    let (ctx, mut app) = setup();
    let anchor = pos2(500., 350.);
    app.open_context(anchor, false);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(anchor)],
        egui::Modifiers::NONE,
    );
    let before = app.doc.clone();
    let pan = app.pan;
    let zoom = app.zoom;
    frame(
        &ctx,
        &mut app,
        vec![
            wheel(vec2(100., 50.), egui::Modifiers::NONE),
            egui::Event::Zoom(1.2),
        ],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.pan, pan);
    assert_eq!(app.zoom, zoom);
    assert_eq!(app.doc, before);
}

#[test]
fn trackpad_view_is_restored_per_document_and_f_recenters_the_page() {
    let (ctx, mut app) = setup();
    let before = app.doc.clone();
    let anchor = pos2(500., 350.);
    frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(anchor),
            wheel(vec2(-180., 95.), egui::Modifiers::NONE),
            egui::Event::Zoom(1.3),
        ],
        egui::Modifiers::NONE,
    );
    let pan = app.pan;
    let zoom = app.zoom;
    app.open_tab(Document::default(), None);
    app.switch_tab(0);
    assert_eq!(app.pan, pan);
    assert_eq!(app.zoom, zoom);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::F,
            physical_key: Some(Key::F),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    let page_center = app.screen([app.doc.page_size[0] / 2., app.doc.page_size[1] / 2.]);
    assert!(page_center.distance(app.canvas.center()) < 0.01);
    assert_eq!(app.doc, before);
    assert!(!app.dirty());
    assert!(!app.history.can_undo());
}

fn style_label(out: &egui::FullOutput, label: &str) -> Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.job.text == label && t.pos.x > 900. => {
                Some(t.pos + t.galley.size() / 2.)
            }
            _ => None,
        })
        .unwrap_or_else(|| {
            panic!(
                "Missing style control: {label}. Texts: {:?}",
                out.shapes
                    .iter()
                    .filter_map(|s| if let Shape::Text(t) = &s.shape {
                        (t.pos.x > 900.).then_some((&t.galley.job.text, t.pos))
                    } else {
                        None
                    })
                    .collect::<Vec<_>>()
            )
        })
}
#[test]
fn appearance_tabs_do_not_modify_document_and_text_edit_still_focuses() {
    let (ctx, mut app, id) = panel_setup();
    app.show_inspector = true;
    app.selected = vec![id];
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let mut out = panel_frame(&ctx, &mut app, size, vec![]);
    for _ in 0..2 {
        out = panel_frame(&ctx, &mut app, size, vec![]);
    }
    for label in ["Ligne", "Ombre", "Remplir"] {
        let p = style_label(&out, label);
        panel_click(&ctx, &mut app, size, p);
        out = panel_frame(&ctx, &mut app, size, vec![]);
        assert_eq!(app.doc, before);
    }
    assert!(!app.history.can_undo());
    app.perform_action(context_menu::Action::Text, &ctx);
    panel_frame(&ctx, &mut app, size, vec![]);
    assert!(ctx.wants_keyboard_input());
    assert_eq!(app.doc, before);
    panel_frame(
        &ctx,
        &mut app,
        size,
        vec![egui::Event::Text("Texte saisi".into())],
    );
    assert!(
        app.rich_editor
            .as_ref()
            .unwrap()
            .draft
            .text
            .contains("Texte saisi")
    );
    assert_eq!(app.doc, before);
    app.apply_node_text().unwrap();
    assert!(app.doc.node(id).unwrap().text.contains("Texte saisi"));
}
#[test]
fn fill_dropdown_edits_the_selected_node_and_remains_undoable() {
    let (ctx, mut app, id) = panel_setup();
    app.show_inspector = true;
    app.selected = vec![id];
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let mut out = panel_frame(&ctx, &mut app, size, vec![]);
    for _ in 0..2 {
        out = panel_frame(&ctx, &mut app, size, vec![]);
    }
    let p = style_label(&out, "Uni");
    panel_click(&ctx, &mut app, size, p);
    let out = panel_frame(&ctx, &mut app, size, vec![]);
    let p = style_label(&out, "Aucun");
    panel_click(&ctx, &mut app, size, p);
    assert_eq!(
        app.doc.node(id).unwrap().appearance.fill.mode,
        diagcraft::appearance::PaintMode::None
    );
    app.doc.validate().unwrap();
    app.commit();
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn shadow_preset_gallery_applies_a_real_effect_and_undo_restores_the_node() {
    let (ctx, mut app, id) = panel_setup();
    app.show_inspector = true;
    app.selected = vec![id];
    app.style_tab = appearance_ui::StyleTab::Shadow;
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let mut out = panel_frame(&ctx, &mut app, size, vec![]);
    for _ in 0..2 {
        out = panel_frame(&ctx, &mut app, size, vec![]);
    }
    let p = style_label(&out, "Préréglages d’ombre");
    panel_click(&ctx, &mut app, size, p);
    let mut out = panel_frame(&ctx, &mut app, size, vec![]);
    for _ in 0..2 {
        out = panel_frame(&ctx, &mut app, size, vec![]);
    }
    let mut images = out
        .shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Mesh(m) = &s.shape {
                let r = m.calc_bounds();
                (r.center().x > 850.
                    && r.width() > 30.
                    && r.width() < 90.
                    && s.clip_rect.contains(r.center()))
                .then_some(r.center())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    images.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    assert!(images.len() >= 3);
    panel_click(&ctx, &mut app, size, images[0]);
    assert_ne!(
        app.doc.node(id).unwrap().appearance.shadow.kind,
        diagcraft::appearance::ShadowKind::None
    );
    app.doc.validate().unwrap();
    app.commit();
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn shadow_offsets_can_be_reached_by_scrolling_the_small_screen_inspector() {
    let (ctx, mut app, id) = panel_setup();
    app.show_inspector = true;
    app.selected = vec![id];
    app.style_tab = appearance_ui::StyleTab::Shadow;
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    for _ in 0..3 {
        panel_frame(&ctx, &mut app, size, vec![]);
    }
    let pan = app.pan;
    panel_frame(
        &ctx,
        &mut app,
        size,
        vec![
            egui::Event::PointerMoved(pos2(1170., 600.)),
            wheel(vec2(0., -650.), egui::Modifiers::NONE),
        ],
    );
    let mut visible = false;
    for _ in 0..15 {
        let out = panel_frame(&ctx, &mut app, size, vec![]);
        visible|=out.shapes.iter().any(|s|matches!(&s.shape,Shape::Text(t) if t.galley.job.text=="Décalage Y" && s.clip_rect.contains(t.pos+t.galley.size()/2.)));
    }
    assert!(visible);
    assert_eq!(app.doc, before);
    assert_eq!(app.pan, pan);
    assert!(!app.history.can_undo());
}

fn multi_style_setup() -> (egui::Context, DrawApp, [u64; 2]) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rounded, [400., 350.]);
    let b = doc.add_node(ShapeKind::Diamond, [800., 450.]);
    for (i, id) in [a, b].into_iter().enumerate() {
        let n = doc.node_mut(id).unwrap();
        n.text = format!("Texte indépendant {i}");
        n.fill = if i == 0 {
            [180, 230, 210]
        } else {
            [230, 190, 210]
        };
        n.stroke_width = 1. + i as f32 * 3.;
        n.font_size = 14. + i as f32 * 10.;
        n.text_color = if i == 0 { [10, 30, 50] } else { [80, 10, 50] };
        n.appearance.shadow =
            diagcraft::appearance::Shadow::preset(diagcraft::appearance::ShadowKind::Outer, i * 8);
    }
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.show_library = false;
    app.show_inspector = true;
    app.show_quick_toolbar = false;
    app.selected = vec![a, b];
    (ctx, app, [a, b])
}
fn stable_panel(ctx: &egui::Context, app: &mut DrawApp, size: Vec2) -> egui::FullOutput {
    panel_frame(ctx, app, size, vec![]);
    panel_frame(ctx, app, size, vec![]);
    panel_frame(ctx, app, size, vec![])
}
fn style_slider(out: &egui::FullOutput, label: &str) -> (Pos2, Rect) {
    let text = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t) if t.galley.job.text == label && t.pos.x > 900. => Some(t),
            _ => None,
        })
        .unwrap();
    let bottom = text.pos.y + text.galley.size().y;
    let rail = out
        .shapes
        .iter()
        .filter_map(|s| match &s.shape {
            Shape::Rect(r)
                if r.rect.left() >= text.pos.x - 1.
                    && r.rect.width() > 70.
                    && r.rect.height() < 12.
                    && r.rect.center().y > bottom
                    && r.rect.center().y < bottom + 40. =>
            {
                Some(r.rect)
            }
            _ => None,
        })
        .max_by(|a, b| a.width().total_cmp(&b.width()))
        .expect("Visible slider rail");
    let thumb = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r)
                if r.rect.left() >= text.pos.x - 1.
                    && r.rect.width() < 40.
                    && r.rect.height() >= 15.
                    && (r.rect.center().y - rail.center().y).abs() < 2. =>
            {
                Some(r.rect.center())
            }
            Shape::Circle(c)
                if c.center.x >= text.pos.x && (c.center.y - rail.center().y).abs() < 2. =>
            {
                Some(c.center)
            }
            _ => None,
        })
        .expect("Visible slider thumb");
    assert!(out.shapes.iter().any(|s| s.clip_rect.contains(thumb)));
    (thumb, rail)
}
fn style_drag(
    ctx: &egui::Context,
    app: &mut DrawApp,
    size: Vec2,
    label: &str,
    release: bool,
) -> Pos2 {
    let out = reveal_style(ctx, app, size, label);
    let (thumb, rail) = style_slider(&out, label);
    panel_pointer(ctx, app, size, thumb, true);
    let mut end = thumb;
    for fraction in [0.25, 0.5, 0.75] {
        end = pos2(rail.left() + rail.width() * fraction, thumb.y);
        panel_frame(ctx, app, size, vec![egui::Event::PointerMoved(end)]);
    }
    if release {
        panel_pointer(ctx, app, size, end, false);
    }
    end
}
fn reveal_style(
    ctx: &egui::Context,
    app: &mut DrawApp,
    size: Vec2,
    label: &str,
) -> egui::FullOutput {
    let visible = |out: &egui::FullOutput| {
        out.shapes.iter().any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text==label && t.pos.x>900. && s.clip_rect.contains(t.pos+t.galley.size()/2.+vec2(0.,if label == "Typographie des formes" {0.} else {45.}))))
    };
    let mut out = stable_panel(ctx, app, size);
    if visible(&out) {
        return out;
    }
    out = scroll_properties(ctx, app, size, 2500.);
    for _ in 0..15 {
        if visible(&out) {
            return out;
        }
        out = scroll_properties(ctx, app, size, -110.);
    }
    panic!("Cannot reveal control: {label}");
}
fn scroll_properties(
    ctx: &egui::Context,
    app: &mut DrawApp,
    size: Vec2,
    delta: f32,
) -> egui::FullOutput {
    panel_frame(
        ctx,
        app,
        size,
        vec![
            egui::Event::PointerMoved(pos2(size.x - 120., size.y - 200.)),
            wheel(vec2(0., delta), egui::Modifiers::NONE),
        ],
    );
    for _ in 0..15 {
        panel_frame(ctx, app, size, vec![]);
    }
    panel_frame(ctx, app, size, vec![])
}
#[test]
fn multi_fill_drag_preserves_colors_and_is_one_undo_step_with_redo() {
    let (ctx, mut app, ids) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let end = style_drag(&ctx, &mut app, size, "Transparence", false);
    assert!(app.property_edit);
    assert!(
        !app.history.can_undo(),
        "a held slider must not fill the history"
    );
    assert!(app.doc.node(ids[0]).unwrap().appearance.fill.transparency > 50.);
    let mut expected = before.clone();
    for n in &mut expected.nodes {
        n.appearance.fill.transparency = app.doc.node(ids[0]).unwrap().appearance.fill.transparency;
    }
    assert_eq!(app.doc, expected);
    panel_pointer(&ctx, &mut app, size, end, false);
    assert!(!app.property_edit);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.redo();
    assert_eq!(app.doc, expected);
    assert!(!app.history.can_redo());
}
#[test]
fn two_separate_slider_gestures_produce_two_undo_steps() {
    let (ctx, mut app, ids) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    style_drag(&ctx, &mut app, size, "Transparence", true);
    let first = app.doc.clone();
    style_drag(&ctx, &mut app, size, "Ombre / teinte", true);
    assert_ne!(app.doc, first);
    app.undo();
    assert_eq!(app.doc, first);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.selected = ids.to_vec();
}
#[test]
fn mixed_selection_line_width_preserves_routes_patterns_text_and_charts() {
    let (ctx, mut app, ids) = multi_style_setup();
    let e = app
        .doc
        .connect(ids[0], ids[1], ConnectorKind::Elbow)
        .unwrap();
    let d = app
        .doc
        .add_drawing(DrawingKind::Bezier, [100., 600.], [850., 650.]);
    let t = app.doc.add_node(ShapeKind::Text, [250., 100.]);
    let c = app.doc.add_chart(ChartKind::Bars, [600., 700.]);
    app.doc.connectors[0].pattern = LinePattern::Dashed;
    app.doc.drawings[0].appearance.pattern = LinePattern::Dotted;
    app.replace(app.doc.clone(), None);
    app.selected = vec![e, ids[1], d, ids[0], t, c];
    app.style_tab = appearance_ui::StyleTab::Line;
    let before = app.doc.clone();
    style_drag(&ctx, &mut app, vec2(1440., 960.), "Épaisseur", true);
    let width = app.doc.connectors[0].width;
    assert!(width > 10.);
    let mut expected = before.clone();
    for n in &mut expected.nodes {
        if n.kind != ShapeKind::Text {
            n.stroke_width = width;
        }
    }
    expected.connectors[0].width = width;
    expected.drawings[0].width = width;
    assert_eq!(app.doc, expected);
    app.doc.validate().unwrap();
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn uniform_fill_button_changes_only_the_current_tab_and_selected_shapes() {
    let (ctx, mut app, ids) = multi_style_setup();
    let unselected = app.doc.add_node(ShapeKind::Rectangle, [100., 650.]);
    app.replace(app.doc.clone(), None);
    app.selected = ids.to_vec();
    let before = app.doc.clone();
    let size = vec2(1280., 800.);
    let out = reveal_style(&ctx, &mut app, size, "Uniformiser cet onglet");
    let p = style_label(&out, "Uniformiser cet onglet");
    panel_click(&ctx, &mut app, size, p);
    let mut expected = before.clone();
    let sample = before.node(ids[0]).unwrap();
    let n = expected.node_mut(ids[1]).unwrap();
    n.fill = sample.fill;
    n.appearance.fill = sample.appearance.fill.clone();
    assert_eq!(app.doc, expected);
    assert_eq!(app.doc.node(unselected), before.node(unselected));
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn reset_multi_shadow_works_even_when_the_first_shape_already_has_defaults() {
    let (ctx, mut app, ids) = multi_style_setup();
    app.doc.node_mut(ids[0]).unwrap().appearance.shadow = Default::default();
    app.replace(app.doc.clone(), None);
    app.selected = ids.to_vec();
    app.style_tab = appearance_ui::StyleTab::Shadow;
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    stable_panel(&ctx, &mut app, size);
    let out = scroll_properties(&ctx, &mut app, size, -700.);
    panel_click(
        &ctx,
        &mut app,
        size,
        style_label(&out, "Réinitialiser cet onglet"),
    );
    let mut expected = before.clone();
    for n in &mut expected.nodes {
        n.appearance.shadow = Default::default();
    }
    assert_eq!(app.doc, expected);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn a_multi_shadow_preset_replaces_all_shadow_fields_even_if_sample_is_unchanged() {
    let (ctx, mut app, ids) = multi_style_setup();
    app.style_tab = appearance_ui::StyleTab::Shadow;
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let out = stable_panel(&ctx, &mut app, size);
    panel_click(
        &ctx,
        &mut app,
        size,
        style_label(&out, "Préréglages d’ombre"),
    );
    let out = stable_panel(&ctx, &mut app, size);
    let mut images = out
        .shapes
        .iter()
        .filter_map(|s| {
            if let Shape::Mesh(m) = &s.shape {
                let r = m.calc_bounds();
                (r.center().x > 900.
                    && r.width() > 30.
                    && r.width() < 90.
                    && s.clip_rect.contains(r.center()))
                .then_some(r.center())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    images.sort_by(|a, b| a.y.total_cmp(&b.y).then(a.x.total_cmp(&b.x)));
    panel_click(&ctx, &mut app, size, images[0]);
    let mut expected = before.clone();
    expected.node_mut(ids[1]).unwrap().appearance.shadow =
        before.node(ids[0]).unwrap().appearance.shadow.clone();
    assert_eq!(app.doc, expected);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn multi_shadow_x_drag_preserves_each_objects_y_offset() {
    let (ctx, mut app, _) = multi_style_setup();
    app.style_tab = appearance_ui::StyleTab::Shadow;
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    stable_panel(&ctx, &mut app, size);
    scroll_properties(&ctx, &mut app, size, -550.);
    style_drag(&ctx, &mut app, size, "Décalage X", true);
    let mut expected = before.clone();
    for n in &mut expected.nodes {
        n.appearance.shadow.offset[0] = app.doc.nodes[0].appearance.shadow.offset[0];
    }
    assert_ne!(app.doc, before);
    assert_eq!(app.doc, expected);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn selecting_only_links_shows_line_controls_and_updates_canonical_dash_patterns() {
    let (ctx, mut app, ids) = multi_style_setup();
    let e = app
        .doc
        .connect(ids[0], ids[1], ConnectorKind::Elbow)
        .unwrap();
    let d = app
        .doc
        .add_drawing(DrawingKind::Line, [100., 600.], [850., 650.]);
    app.replace(app.doc.clone(), None);
    app.selected = vec![e, d];
    let size = vec2(1440., 960.);
    let before = app.doc.clone();
    let out = reveal_style(&ctx, &mut app, size, "Continu");
    assert!(app.style_tab == appearance_ui::StyleTab::Line);
    assert_eq!(app.doc, before);
    panel_click(&ctx, &mut app, size, style_label(&out, "Continu"));
    let out = stable_panel(&ctx, &mut app, size);
    panel_click(&ctx, &mut app, size, style_label(&out, "Tirets"));
    assert_eq!(app.doc.connectors[0].pattern, LinePattern::Dashed);
    assert_eq!(app.doc.drawings[0].appearance.pattern, LinePattern::Dashed);
    let mut expected = before.clone();
    expected.connectors[0].pattern = LinePattern::Dashed;
    expected.drawings[0].appearance.pattern = LinePattern::Dashed;
    assert_eq!(app.doc, expected);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn multi_typography_preserves_independent_texts_sizes_colors_and_geometry() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let out = reveal_style(&ctx, &mut app, size, "Typographie des formes");
    panel_click(
        &ctx,
        &mut app,
        size,
        style_label(&out, "Typographie des formes"),
    );
    let out = scroll_properties(&ctx, &mut app, size, -450.);
    panel_click(&ctx, &mut app, size, style_label(&out, "Gras"));
    let mut expected = before.clone();
    for n in &mut expected.nodes {
        n.text_style.bold = true;
    }
    assert_eq!(app.doc, expected);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn a_pending_property_drag_is_committed_before_opening_another_document() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let end = style_drag(&ctx, &mut app, size, "Transparence", false);
    let edited = app.doc.clone();
    app.open_tab(Document::default(), None);
    assert!(!app.property_edit);
    panel_pointer(&ctx, &mut app, size, end, false);
    app.switch_tab(0);
    assert_eq!(app.doc, edited);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.switch_tab(1);
    assert_eq!(app.doc, Document::default());
    assert!(!app.history.can_undo());
}
#[test]
fn saving_during_a_property_drag_keeps_the_undo_step_and_saved_styles() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let end = style_drag(&ctx, &mut app, size, "Transparence", false);
    let edited = app.doc.clone();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("multi.draw");
    assert!(app.save_to(path.clone()));
    assert!(!app.dirty());
    assert!(!app.property_edit);
    assert_eq!(Document::load(&path).unwrap(), edited);
    panel_pointer(&ctx, &mut app, size, end, false);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(app.dirty());
    assert!(!app.history.can_undo());
    app.redo();
    assert!(!app.dirty());
}
#[test]
fn losing_window_focus_finishes_a_property_gesture_without_a_release_event() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    style_drag(&ctx, &mut app, size, "Transparence", false);
    panel_frame(
        &ctx,
        &mut app,
        size,
        vec![egui::Event::WindowFocused(false)],
    );
    assert!(!app.property_edit);
    assert!(app.history.can_undo());
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

#[test]
fn a_single_shape_slider_drag_is_also_one_undo_action() {
    let (ctx, mut app, ids) = multi_style_setup();
    app.selected = vec![ids[0]];
    let before = app.doc.clone();
    style_drag(&ctx, &mut app, vec2(1280., 800.), "Transparence", true);
    assert_ne!(app.doc, before);
    assert_eq!(app.doc.node(ids[1]), before.node(ids[1]));
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn keyboard_undo_flushes_a_pending_property_drag_before_reverting_it() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let end = style_drag(&ctx, &mut app, size, "Transparence", false);
    assert!(app.property_edit);
    let command = egui::Modifiers {
        command: true,
        mac_cmd: true,
        ..egui::Modifiers::NONE
    };
    panel_frame(
        &ctx,
        &mut app,
        size,
        vec![egui::Event::Key {
            key: Key::Z,
            physical_key: Some(Key::Z),
            pressed: true,
            repeat: false,
            modifiers: command,
        }],
    );
    assert_eq!(app.doc, before);
    assert!(!app.property_edit);
    panel_pointer(&ctx, &mut app, size, end, false);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn browsing_multi_style_tabs_never_changes_styles_or_history() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let mut out = stable_panel(&ctx, &mut app, size);
    for label in ["Ligne", "Ombre", "Remplir"] {
        panel_click(&ctx, &mut app, size, style_label(&out, label));
        out = stable_panel(&ctx, &mut app, size);
        assert_eq!(app.doc, before);
        assert!(!app.property_edit);
        assert!(!app.history.can_undo());
        assert!(!app.dirty());
    }
}

fn ux_text(out: &egui::FullOutput, text: &str) -> Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Text(t)
                if t.galley.job.text == text
                    && s.clip_rect.contains(t.pos + t.galley.size() / 2.) =>
            {
                Some(t.pos + t.galley.size() / 2.)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing visible command: {text}"))
}
fn ribbon_chevron(out: &egui::FullOutput) -> Pos2 {
    out.shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::LineSegment { points, stroke }
                if (stroke.width - 1.8).abs() < 0.01
                    && (points[1].x - points[0].x - 6.).abs() < 0.01
                    && points[0].y > 40.
                    && points[0].y < 85. =>
            {
                Some(pos2(points[1].x, (points[0].y + points[1].y) / 2.))
            }
            _ => None,
        })
        .expect("Persistent ribbon toggle")
}
#[test]
fn compact_ribbon_gains_height_and_its_toggle_preserves_the_manual_choice() {
    let (ctx, mut app, _) = panel_setup();
    let before = app.doc.clone();
    let size = vec2(1280., 800.);
    let out = stable_panel(&ctx, &mut app, size);
    let compact_height = app.canvas.height();
    assert!(!app.ribbon_is_expanded(&ctx));
    panel_click(&ctx, &mut app, size, ribbon_chevron(&out));
    let out = stable_panel(&ctx, &mut app, size);
    assert!(app.ribbon_is_expanded(&ctx));
    assert!((compact_height - app.canvas.height() - 60.).abs() < 1.);
    panel_click(&ctx, &mut app, size, ribbon_chevron(&out));
    stable_panel(&ctx, &mut app, vec2(1440., 960.));
    assert!(!app.ribbon_is_expanded(&ctx));
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn automatic_ribbon_adapts_without_changing_the_document_or_selection() {
    let (ctx, mut app, id) = panel_setup();
    app.selected = vec![id];
    let before = app.doc.clone();
    for (size, expanded) in [
        (vec2(1280., 800.), false),
        (vec2(1440., 960.), true),
        (vec2(1440., 800.), false),
        (vec2(1280., 960.), false),
    ] {
        stable_panel(&ctx, &mut app, size);
        assert_eq!(app.ribbon_is_expanded(&ctx), expanded);
        assert_eq!(app.selected, vec![id]);
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
#[test]
fn clicking_a_ribbon_tab_keeps_compact_mode_and_displays_its_own_commands() {
    let (ctx, mut app, id) = panel_setup();
    let size = vec2(1280., 800.);
    let before = app.doc.clone();
    let out = stable_panel(&ctx, &mut app, size);
    panel_click(&ctx, &mut app, size, ux_text(&out, "Insérer"));
    let out = stable_panel(&ctx, &mut app, size);
    assert!(!app.ribbon_is_expanded(&ctx));
    assert!(app.ribbon_tab == RibbonTab::Insert);
    ux_text(&out, "Tableau");
    ux_text(&out, "Ajouter une page");
    panel_click(&ctx, &mut app, size, ux_text(&out, "Accueil"));
    let out = stable_panel(&ctx, &mut app, size);
    panel_click(&ctx, &mut app, size, ux_text(&out, "Propriétés"));
    stable_panel(&ctx, &mut app, size);
    assert!(app.show_inspector);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.selected = vec![id];
    let out = stable_panel(&ctx, &mut app, size);
    panel_click(&ctx, &mut app, size, ux_text(&out, "Main · H"));
    assert!(app.tool == Tool::Pan);
    assert_eq!(app.doc, before);
}
#[test]
fn compact_link_button_connects_two_selected_shapes_in_one_undo_step() {
    let (ctx, mut app, ids) = multi_style_setup();
    let before = app.doc.clone();
    let size = vec2(1280., 800.);
    let out = stable_panel(&ctx, &mut app, size);
    assert!(!app.ribbon_is_expanded(&ctx));
    panel_click(&ctx, &mut app, size, ux_text(&out, "Relier · C"));
    assert_eq!(
        (app.doc.connectors[0].from, app.doc.connectors[0].to),
        (ids[0], ids[1])
    );
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

fn marquee_setup() -> (egui::Context, DrawApp, Vec<u64>) {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [300., 250.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [650., 400.]);
    let outside = app.doc.add_node(ShapeKind::Ellipse, [1000., 680.]);
    let line = app
        .doc
        .add_drawing(DrawingKind::Line, [150., 550.], [800., 550.]);
    app.doc
        .add_drawing(DrawingKind::Bezier, [200., 690.], [750., 690.]);
    let chart = app.doc.add_chart(ChartKind::Columns, [970., 170.]);
    let c = app.doc.charts.iter_mut().find(|c| c.id == chart).unwrap();
    c.position = [850., 100.];
    c.size = [240., 140.];
    let edge = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.saved = app.doc.clone();
    app.history = History::new(&app.doc);
    app.show_quick_toolbar = false;
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    assert_ne!(outside, a);
    (ctx, app, vec![a, b, line, edge])
}
fn marquee_drag(
    ctx: &egui::Context,
    app: &mut DrawApp,
    start: Pos2,
    end: Pos2,
    shift: bool,
    release: bool,
) {
    let modifiers = egui::Modifiers {
        shift,
        ..egui::Modifiers::NONE
    };
    pointer(ctx, app, start, true, modifiers);
    frame(ctx, app, vec![egui::Event::PointerMoved(end)], modifiers);
    if release {
        pointer(ctx, app, end, false, modifiers);
    }
}
#[test]
fn marquee_previews_and_selects_shapes_links_and_drawings_without_an_undo_step() {
    let (ctx, mut app, ids) = marquee_setup();
    let before = app.doc.clone();
    marquee_drag(
        &ctx,
        &mut app,
        pos2(180., 170.),
        pos2(780., 600.),
        false,
        false,
    );
    assert!(matches!(app.drag, Some(Drag::Marquee { .. })));
    assert_eq!(app.selected, ids);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    pointer(
        &ctx,
        &mut app,
        pos2(780., 600.),
        false,
        egui::Modifiers::NONE,
    );
    assert!(app.drag.is_none());
    assert_eq!(app.selected, ids);
    assert_eq!(app.doc, before);
    assert!(!app.dirty());
    app.doc.page_layout.auto_size = true;
    app.doc.page_size = [500., 400.];
    let before = app.doc.clone();
    app.saved = before.clone();
    app.history = History::new(&before);
    marquee_drag(
        &ctx,
        &mut app,
        pos2(80., 80.),
        pos2(150., 150.),
        false,
        false,
    );
    app.open_context(pos2(80., 80.), false);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn reversed_marquee_and_zoomed_marquee_select_the_same_objects() {
    let (ctx, mut app, ids) = marquee_setup();
    let before = app.doc.clone();
    marquee_drag(
        &ctx,
        &mut app,
        pos2(780., 600.),
        pos2(180., 170.),
        false,
        true,
    );
    assert_eq!(app.selected, ids);
    app.selected.clear();
    app.zoom = 0.6;
    app.pan = vec2(110., 60.);
    let start = app.screen([180., 170.]);
    let end = app.screen([780., 600.]);
    marquee_drag(&ctx, &mut app, start, end, false, true);
    assert_eq!(app.selected, ids);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn shift_marquee_adds_to_the_selection_and_escape_restores_the_previous_ids() {
    let (ctx, mut app, ids) = marquee_setup();
    let initial = app.doc.nodes[2].id;
    app.selected = vec![initial];
    let before = app.doc.clone();
    marquee_drag(
        &ctx,
        &mut app,
        pos2(180., 170.),
        pos2(780., 600.),
        true,
        false,
    );
    let mut expected = vec![initial];
    expected.extend(ids);
    assert_eq!(app.selected, expected);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::Escape,
            physical_key: Some(Key::Escape),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(780., 600.),
        false,
        egui::Modifiers::NONE,
    );
    assert!(app.drag.is_none());
    assert_eq!(app.selected, vec![initial]);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn an_empty_click_clears_selection_while_shift_click_and_small_jitter_preserve_it() {
    let (ctx, mut app, ids) = marquee_setup();
    let before = app.doc.clone();
    app.selected = ids.clone();
    marquee_drag(
        &ctx,
        &mut app,
        pos2(100., 100.),
        pos2(102., 102.),
        true,
        true,
    );
    assert_eq!(app.selected, ids);
    marquee_drag(
        &ctx,
        &mut app,
        pos2(100., 100.),
        pos2(102., 102.),
        false,
        true,
    );
    assert!(app.selected.is_empty());
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn marquee_uses_shape_contours_and_curve_segments_instead_of_empty_bounding_boxes() {
    for kind in [ShapeKind::Diamond, ShapeKind::Ellipse] {
        let (ctx, mut app) = setup();
        let id = app.doc.add_node(kind, [600., 400.]);
        app.history = History::new(&app.doc);
        let n = app.doc.node(id).unwrap();
        let start = pos2(n.position[0] + 1., n.position[1] + 1.);
        let end = start + vec2(12., 8.);
        marquee_drag(&ctx, &mut app, start, end, false, true);
        assert!(app.selected.is_empty(), "empty corner of {kind:?}");
    }
    let (ctx, mut app) = setup();
    let actor = app.doc.add_node(ShapeKind::Actor, [600., 400.]);
    let n = app.doc.node(actor).unwrap();
    let start = pos2(n.position[0] + n.size[0] * 0.34, n.position[1]);
    let end = pos2(
        n.position[0] + n.size[0] * 0.66,
        n.position[1] + n.size[1] * 0.28,
    );
    marquee_drag(&ctx, &mut app, start, end, false, true);
    assert_eq!(app.selected, vec![actor]);
    let (ctx, mut app) = setup();
    let id = app
        .doc
        .add_drawing(DrawingKind::Bezier, [300., 300.], [700., 300.]);
    app.doc.drawings[0].bend = 200.;
    app.history = History::new(&app.doc);
    marquee_drag(
        &ctx,
        &mut app,
        pos2(450., 320.),
        pos2(470., 335.),
        false,
        true,
    );
    assert!(app.selected.is_empty());
    marquee_drag(
        &ctx,
        &mut app,
        pos2(450., 430.),
        pos2(480., 455.),
        false,
        true,
    );
    assert_eq!(app.selected, vec![id]);
}
#[test]
fn marquee_does_not_pick_hidden_objects_outside_the_page_or_canvas() {
    let (ctx, mut app, _) = marquee_setup();
    let hidden = app.doc.add_node(ShapeKind::Rectangle, [2000., 700.]);
    let before = app.doc.clone();
    app.history = History::new(&before);
    marquee_drag(
        &ctx,
        &mut app,
        pos2(100., 80.),
        pos2(2400., 1800.),
        false,
        true,
    );
    assert!(!app.selected.contains(&hidden));
    assert!(app.selected.contains(&app.doc.charts[0].id));
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn trackpad_cannot_change_coordinates_during_a_marquee_and_focus_loss_cancels_it() {
    let (ctx, mut app, ids) = marquee_setup();
    app.selected = vec![ids[0]];
    let before = app.doc.clone();
    let pan = app.pan;
    let zoom = app.zoom;
    marquee_drag(
        &ctx,
        &mut app,
        pos2(180., 170.),
        pos2(780., 600.),
        false,
        false,
    );
    frame(
        &ctx,
        &mut app,
        vec![
            wheel(vec2(150., -85.), egui::Modifiers::NONE),
            egui::Event::Zoom(1.4),
        ],
        egui::Modifiers::NONE,
    );
    assert_eq!((app.pan, app.zoom), (pan, zoom));
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            focused: false,
            ..Default::default()
        },
        |ctx| app.canvas(ctx, true),
    );
    assert!(app.drag.is_none());
    assert_eq!(app.selected, vec![ids[0]]);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn shift_click_adds_and_removes_connectors_and_curves_without_replacing_other_ids() {
    let (ctx, mut app, ids) = marquee_setup();
    app.selected = vec![ids[0]];
    let before = app.doc.clone();
    let edge = app.doc.connectors[0].clone();
    let route = app.doc.route(&edge);
    let a = route[1];
    let b = route[2];
    let point = pos2((a[0] + b[0]) / 2., (a[1] + b[1]) / 2.);
    let shift = egui::Modifiers {
        shift: true,
        ..egui::Modifiers::NONE
    };
    pointer(&ctx, &mut app, point, true, shift);
    pointer(&ctx, &mut app, point, false, shift);
    assert_eq!(app.selected, vec![ids[0], edge.id]);
    let p = pos2(400., 550.);
    pointer(&ctx, &mut app, p, true, shift);
    pointer(&ctx, &mut app, p, false, shift);
    assert_eq!(app.selected, vec![ids[0], edge.id, ids[2]]);
    pointer(&ctx, &mut app, p, true, shift);
    pointer(&ctx, &mut app, p, false, shift);
    assert_eq!(app.selected, vec![ids[0], edge.id]);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn dragging_a_curve_in_a_multi_selection_moves_the_whole_selection_in_one_undo() {
    let (ctx, mut app, ids) = marquee_setup();
    app.selected = ids.clone();
    app.snap = false;
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(400., 550.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(440., 575.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.selected, ids);
    assert_eq!(
        app.doc.nodes[0].position,
        [
            before.nodes[0].position[0] + 40.,
            before.nodes[0].position[1] + 25.
        ]
    );
    assert_eq!(app.doc.drawings[0].start, [190., 575.]);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn shift_f_centers_the_selection_and_f_restores_page_framing_without_edits() {
    let (ctx, mut app, ids) = marquee_setup();
    app.selected = vec![ids[0], ids[1]];
    let before = app.doc.clone();
    let old = app.zoom;
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::F,
            physical_key: Some(Key::F),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        }],
        egui::Modifiers {
            shift: true,
            ..egui::Modifiers::NONE
        },
    );
    let r = app.selection_rect().unwrap();
    assert!(app.canvas.shrink(30.).contains_rect(r));
    assert!(r.center().distance(app.canvas.center()) < 0.01);
    assert!(app.zoom > old);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::Key {
            key: Key::F,
            physical_key: Some(Key::F),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    let center = app.screen(app.doc.page_size.map(|v| v / 2.));
    assert!(center.distance(app.canvas.center()) < 0.01);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn selection_zoom_handles_horizontal_lines_and_shadows_and_ignores_an_empty_selection() {
    let (ctx, mut app) = setup();
    let id = app
        .doc
        .add_drawing(DrawingKind::Line, [400., 350.], [700., 350.]);
    app.selected = vec![id];
    let before = app.doc.clone();
    app.fit_selection(&ctx);
    assert!(app.zoom.is_finite());
    assert!(
        app.selection_rect()
            .unwrap()
            .center()
            .distance(app.canvas.center())
            < 0.01
    );
    app.selected.clear();
    let view = (app.zoom, app.pan);
    app.fit_selection(&ctx);
    assert_eq!((app.zoom, app.pan), view);
    assert_eq!(app.doc, before);
    let id = app.doc.add_node(ShapeKind::Rounded, [600., 400.]);
    let n = app.doc.node_mut(id).unwrap();
    n.appearance.shadow =
        diagcraft::appearance::Shadow::preset(diagcraft::appearance::ShadowKind::Perspective, 8);
    n.appearance.shadow.offset = [150., 170.];
    n.appearance.shadow.blur = 30.;
    let b = diagcraft::appearance_svg::node_bounds(n);
    let pos = n.position;
    app.selected = vec![id];
    let before = app.doc.clone();
    app.fit_selection(&ctx);
    let r = Rect::from_min_max(
        app.screen([pos[0] + b[0], pos[1] + b[1]]),
        app.screen([pos[0] + b[2], pos[1] + b[3]]),
    );
    assert!(app.canvas.shrink(30.).contains_rect(r));
    assert_eq!(app.doc, before);
}
#[test]
fn shift_f_does_not_steal_a_shortcut_from_a_focused_text_editor() {
    let (ctx, mut app, id) = panel_setup();
    app.selected = vec![id];
    app.show_inspector = true;
    app.focus_text = Some(id);
    let size = vec2(1280., 800.);
    stable_panel(&ctx, &mut app, size);
    assert!(ctx.wants_keyboard_input());
    let before = app.doc.clone();
    let view = (app.zoom, app.pan);
    panel_frame(
        &ctx,
        &mut app,
        size,
        vec![egui::Event::Key {
            key: Key::F,
            physical_key: Some(Key::F),
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers {
                shift: true,
                ..egui::Modifiers::NONE
            },
        }],
    );
    assert_eq!((app.zoom, app.pan), view);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

#[test]
fn the_selection_zoom_button_fits_with_both_panels_and_frames_objects_without_edits() {
    let (ctx, mut app, ids) = multi_style_setup();
    let size = vec2(1280., 800.);
    app.show_library = true;
    let before = app.doc.clone();
    let out = stable_panel(&ctx, &mut app, size);
    let selection = ux_text(&out, "Sélection");
    assert!(app.canvas.shrink(10.).contains(selection));
    let label = out
        .shapes
        .iter()
        .find_map(|s| {
            if let Shape::Text(t) = &s.shape {
                (t.galley.job.text == "Sélection").then_some(t)
            } else {
                None
            }
        })
        .unwrap();
    assert!(
        app.canvas
            .shrink(10.)
            .contains_rect(Rect::from_min_size(label.pos, label.galley.size()).expand(10.))
    );
    let zoom = app.zoom;
    panel_click(&ctx, &mut app, size, selection);
    stable_panel(&ctx, &mut app, size);
    assert!(app.zoom > zoom);
    assert_eq!(app.selected, ids);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

fn placement_setup() -> (egui::Context, DrawApp, [u64; 2]) {
    let (ctx, mut app) = setup();
    let a = app.doc.add_node(ShapeKind::Rectangle, [150., 240.]);
    let b = app.doc.add_node(ShapeKind::Diamond, [450., 440.]);
    for (id, p) in [(a, [100., 200.]), (b, [400., 400.])] {
        let n = app.doc.node_mut(id).unwrap();
        n.position = p;
        n.size = [100., 80.];
    }
    app.selected = vec![a];
    app.show_quick_toolbar = false;
    app.saved = app.doc.clone();
    app.history = History::new(&app.doc);
    (ctx, app, [a, b])
}
#[test]
fn guides_use_a_six_screen_point_threshold_at_every_zoom() {
    let (_, mut app, [a, _]) = placement_setup();
    for zoom in [0.2, 1., 2.5] {
        app.zoom = zoom;
        let (delta, guides) =
            app.move_placement(&[(a, [100., 200.])], &[], [300. + 4. / zoom, 0.], false);
        assert!((delta[0] - 300.).abs() < 0.001);
        assert_eq!(guides.len(), 1);
        let (_, guides) =
            app.move_placement(&[(a, [100., 200.])], &[], [300. + 8. / zoom, 0.], false);
        assert!(guides.is_empty());
    }
}
#[test]
fn guides_respect_grid_toggle_option_and_stationary_clicks() {
    let (_, mut app, [a, _]) = placement_setup();
    let original = [(a, [100., 200.])];
    let (delta, guides) = app.move_placement(&original, &[], [297., 13.], false);
    assert_eq!(delta, [300., 10.]);
    assert!(!guides.is_empty());
    let (delta, guides) = app.move_placement(&original, &[], [297., 13.], true);
    assert_eq!(delta, [297., 13.]);
    assert!(guides.is_empty());
    app.smart_guides = false;
    let (delta, guides) = app.move_placement(&original, &[], [294., 13.], false);
    assert_eq!(delta, [290., 10.]);
    assert!(guides.is_empty());
    app.smart_guides = true;
    app.snap = false;
    let (delta, guides) = app.move_placement(&original, &[], [297., 13.], false);
    assert_eq!(delta, [297., 13.]);
    assert!(guides.is_empty());
    app.snap = true;
    let (delta, guides) = app.move_placement(&original, &[], [0., 0.], false);
    assert_eq!(delta, [0., 0.]);
    assert!(guides.is_empty());
}
#[test]
fn guides_can_align_centers_or_opposite_edges_and_use_chart_targets() {
    let (_, mut app, [a, b]) = placement_setup();
    app.doc.node_mut(b).unwrap().size = [200., 80.];
    let (delta, _) = app.move_placement(&[(a, [100., 200.])], &[], [347., 0.], false);
    assert_eq!(delta[0], 350.); // both centres x=500
    let (delta, _) = app.move_placement(&[(a, [100., 200.])], &[], [197., 0.], false);
    assert_eq!(delta[0], 200.); // right edge touches left edge x=400
    app.doc.nodes.retain(|n| n.id != b);
    let c = app.doc.add_chart(ChartKind::Columns, [500., 450.]);
    app.doc.charts[0].position = [400., 400.];
    app.doc.charts[0].size = [200., 120.];
    let (delta, guides) = app.move_placement(&[(a, [100., 200.])], &[], [347., 0.], false);
    assert_eq!(delta[0], 350.);
    assert!(!guides.is_empty());
    app.selected = vec![c];
    let (delta, _) = app.move_placement(&[], &[(c, [400., 400.])], [-197., 0.], false);
    assert_eq!(delta[0], -200.); // chart left touches node right
}
#[test]
fn selected_and_off_page_objects_do_not_attract_a_drag() {
    let (_, mut app, [a, b]) = placement_setup();
    app.doc.page_size = [600., 600.];
    app.doc.node_mut(b).unwrap().position = [800., 400.];
    let (_, guides) = app.move_placement(&[(a, [100., 200.])], &[], [297., 0.], false);
    assert!(guides.is_empty());
    app.doc.node_mut(b).unwrap().position = [400., 400.];
    app.selected.push(b);
    let (_, guides) = app.move_placement(
        &[(a, [100., 200.]), (b, [400., 400.])],
        &[],
        [27., 0.],
        false,
    );
    assert!(guides.is_empty());
}
#[test]
fn group_page_clamping_preserves_spacing_and_rejects_impossible_guides() {
    let (_, mut app, [a, b]) = placement_setup();
    app.doc.page_size = [600., 600.];
    app.selected = vec![a, b];
    let original = [(a, [100., 200.]), (b, [400., 400.])];
    let (delta, _) = app.move_placement(&original, &[], [207., 0.], false);
    assert_eq!(delta[0], 100.);
    let (delta, _) = app.move_placement(&original, &[], [-300., 0.], true);
    assert_eq!(delta[0], -100.);
    let c = app.doc.add_node(ShapeKind::Rectangle, [500., 100.]);
    app.doc.node_mut(c).unwrap().position = [500., 100.];
    app.doc.node_mut(c).unwrap().size = [100., 80.];
    // Group's left is near x=500, but its right would cross the page.
    let (delta, guides) = app.move_placement(&original, &[], [397., 0.], false);
    assert_eq!(delta[0], 100.);
    assert!(guides.is_empty());
}
#[test]
fn pointer_guides_snap_the_block_update_its_link_and_undo_once() {
    let (ctx, mut app, [a, b]) = placement_setup();
    app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.history = History::new(&app.doc);
    app.saved = app.doc.clone();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(447., 240.))],
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.node(a).unwrap().position, [400., 200.]);
    assert!(!app.move_guides.is_empty());
    assert_ne!(
        app.doc.route(&app.doc.connectors[0]),
        before.route(&before.connectors[0])
    );
    pointer(
        &ctx,
        &mut app,
        pos2(447., 240.),
        false,
        egui::Modifiers::NONE,
    );
    assert!(app.drag.is_none());
    assert!(app.move_guides.is_empty());
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.redo();
    assert_eq!(app.doc.node(a).unwrap().position, [400., 200.]);
}
#[test]
fn option_during_a_pointer_drag_places_the_block_without_snapping() {
    let (ctx, mut app, [a, _]) = placement_setup();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    let alt = egui::Modifiers {
        alt: true,
        ..egui::Modifiers::NONE
    };
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(447., 253.))],
        alt,
    );
    assert_eq!(app.doc.node(a).unwrap().position, [397., 213.]);
    assert!(app.move_guides.is_empty());
    pointer(&ctx, &mut app, pos2(447., 253.), false, alt);
    app.undo();
    assert_eq!(app.doc.node(a).unwrap().position, [100., 200.]);
}
#[test]
fn dragging_a_group_at_the_page_edge_moves_paths_and_charts_together() {
    let (ctx, mut app, [a, b]) = placement_setup();
    app.doc.page_size = [600., 600.];
    let c = app.doc.add_chart(ChartKind::Columns, [300., 400.]);
    app.doc.charts[0].position = [200., 300.];
    app.doc.charts[0].size = [120., 120.];
    let d = app
        .doc
        .add_drawing(DrawingKind::Line, [150., 350.], [250., 350.]);
    app.selected = vec![a, b, c, d];
    app.snap = false;
    let before = app.doc.clone();
    app.history = History::new(&before);
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(357., 240.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc.node(a).unwrap().position, [200., 200.]);
    assert_eq!(app.doc.node(b).unwrap().position, [500., 400.]);
    assert_eq!(app.doc.charts[0].position, [300., 300.]);
    assert_eq!(app.doc.drawings[0].start, [250., 350.]);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn all_six_alignments_support_nodes_and_charts_without_changing_styles_or_paths() {
    for axis in 0..2 {
        for anchor in [0., 0.5, 1.] {
            let (_, mut app, [a, b]) = placement_setup();
            let c = app.doc.add_chart(ChartKind::Columns, [350., 300.]);
            app.doc.charts[0].position = [200., 150.];
            app.doc.charts[0].size = [120., 120.];
            let d = app
                .doc
                .add_drawing(DrawingKind::Line, [80., 90.], [120., 90.]);
            let e = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
            app.selected = vec![a, b, c, d, e];
            let before = app.doc.clone();
            app.history = History::new(&before);
            app.align_selected(axis, anchor);
            let objects = app.placement_selection();
            let target = objects[0].1.min[axis] + objects[0].1.size()[axis] * anchor;
            assert!(
                objects
                    .iter()
                    .all(|(_, r)| (r.min[axis] + r.size()[axis] * anchor - target).abs() < 0.001)
            );
            for n in &app.doc.nodes {
                let old = before.node(n.id).unwrap();
                assert_eq!(n.text, old.text);
                assert_eq!(n.appearance, old.appearance);
                assert_eq!(n.position[1 - axis], old.position[1 - axis]);
            }
            assert_eq!(app.doc.drawings, before.drawings);
            assert_eq!(app.doc.connectors, before.connectors);
            app.doc.validate().unwrap();
            app.undo();
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
    }
}
#[test]
fn distribution_spaces_different_sizes_keeps_extremes_and_undoes_once() {
    for vertical in [false, true] {
        let (_, mut app, [a, b]) = placement_setup();
        let c = app.doc.add_chart(ChartKind::Columns, [300., 300.]);
        app.doc.charts[0].position = [250., 300.];
        app.doc.charts[0].size = [120., 120.];
        app.selected = vec![b, c, a];
        let before = app.doc.clone();
        app.history = History::new(&before);
        app.distribute(vertical);
        let axis = usize::from(vertical);
        let mut objects = app.placement_selection();
        objects.sort_by(|a, b| a.1.min[axis].total_cmp(&b.1.min[axis]));
        let left = objects[1].1.min[axis] - objects[0].1.max[axis];
        let right = objects[2].1.min[axis] - objects[1].1.max[axis];
        assert!((left - right).abs() < 0.001);
        assert_eq!(
            app.doc.node(a).unwrap().position,
            before.node(a).unwrap().position
        );
        assert_eq!(
            app.doc.node(b).unwrap().position,
            before.node(b).unwrap().position
        );
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
#[test]
fn alignment_without_enough_compatible_objects_is_a_noop() {
    let (_, mut app, [a, b]) = placement_setup();
    let e = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![a, e];
    let before = app.doc.clone();
    app.history = History::new(&before);
    app.align_selected(0, 0.);
    app.distribute(false);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn alignment_controls_are_visible_and_actionable_in_the_compact_inspector() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    app.ribbon_expanded = Some(false);
    let before = app.doc.clone();
    let out = stable_panel(&ctx, &mut app, size);
    let left = ux_text(&out, "Gauche");
    assert!(app.canvas.right() < left.x && left.y < app.canvas.top() + 200.);
    panel_click(&ctx, &mut app, size, left);
    assert_eq!(app.doc.nodes[0].position[0], app.doc.nodes[1].position[0]);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn compact_alignment_menu_works_with_the_inspector_closed() {
    let (ctx, mut app, _) = multi_style_setup();
    let size = vec2(1280., 800.);
    app.show_inspector = false;
    app.ribbon_expanded = Some(false);
    let before = app.doc.clone();
    let out = stable_panel(&ctx, &mut app, size);
    let menu = ux_text(&out, "Aligner");
    panel_click(&ctx, &mut app, size, menu);
    let out = stable_panel(&ctx, &mut app, size);
    let top = ux_text(&out, "Haut");
    panel_click(&ctx, &mut app, size, top);
    assert_eq!(app.doc.nodes[0].position[1], app.doc.nodes[1].position[1]);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn guide_preferences_are_view_only_and_restored_per_document_tab() {
    let (_, mut app, _) = placement_setup();
    let before = app.doc.clone();
    app.smart_guides = false;
    app.open_tab(Document::default(), None);
    app.smart_guides = true;
    app.switch_tab(0);
    assert!(!app.smart_guides);
    assert_eq!(app.doc, before);
    assert!(!app.dirty());
    assert!(!app.history.can_undo());
    app.switch_tab(1);
    assert!(app.smart_guides);
}
#[test]
fn option_resize_uses_exact_dimensions_and_preserves_the_opposite_corner() {
    let (ctx, mut app, [a, _]) = placement_setup();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(200., 280.),
        true,
        egui::Modifiers::NONE,
    );
    assert!(matches!(app.drag, Some(Drag::Resize { .. })));
    pointer(
        &ctx,
        &mut app,
        pos2(213., 297.),
        false,
        egui::Modifiers {
            alt: true,
            ..egui::Modifiers::NONE
        },
    );
    assert_eq!(app.doc.node(a).unwrap().position, [100., 200.]);
    assert_eq!(app.doc.node(a).unwrap().size, [113., 97.]);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn option_free_connector_move_preserves_exact_offsets_and_undo() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    let id = app
        .doc
        .connect_endpoints(
            0,
            0,
            Some([200., 200.]),
            Some([600., 400.]),
            ConnectorKind::Elbow,
        )
        .unwrap();
    let before = app.doc.clone();
    app.history = History::new(&before);
    pointer(
        &ctx,
        &mut app,
        pos2(300., 200.),
        true,
        egui::Modifiers::NONE,
    );
    assert!(matches!(app.drag, Some(Drag::FreeConnector { .. })));
    pointer(
        &ctx,
        &mut app,
        pos2(313., 217.),
        false,
        egui::Modifiers {
            alt: true,
            ..egui::Modifiers::NONE
        },
    );
    let e = app.doc.connectors.iter().find(|e| e.id == id).unwrap();
    assert_eq!(e.free_start, Some([213., 217.]));
    assert_eq!(e.free_end, Some([613., 417.]));
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn every_alignment_control_is_fully_visible_in_the_narrow_properties_panel() {
    for size in [vec2(1200., 760.), vec2(1280., 800.)] {
        let (ctx, mut app, _) = multi_style_setup();
        app.show_library = true;
        app.ribbon_expanded = Some(false);
        let out = stable_panel(&ctx, &mut app, size);
        for label in [
            "Gauche",
            "Centre H",
            "Droite",
            "Haut",
            "Centre V",
            "Bas",
            "Espacer H",
            "Espacer V",
        ] {
            assert!(out.shapes.iter().any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text==label && t.pos.x>app.canvas.right() && s.clip_rect.contains_rect(Rect::from_min_size(t.pos,t.galley.size())))),"Clipped control: {label}");
        }
    }
}
#[test]
fn guide_hint_remains_visible_when_the_page_is_smaller_than_the_canvas() {
    let (ctx, mut app, _) = placement_setup();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(447., 240.))],
        egui::Modifiers::NONE,
    );
    app.zoom = 0.5;
    app.pan = vec2(100., 40.);
    let out = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            ..Default::default()
        },
        |ctx| app.canvas(ctx, false),
    );
    assert!(out.shapes.iter().any(|s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text.starts_with("Aligné · X ") && s.clip_rect.contains_rect(Rect::from_min_size(t.pos,t.galley.size())))));
}

fn gesture_key(ctx: &egui::Context, app: &mut DrawApp, key: Key, modifiers: egui::Modifiers) {
    frame(
        ctx,
        app,
        vec![egui::Event::Key {
            key,
            physical_key: Some(key),
            pressed: true,
            repeat: false,
            modifiers,
        }],
        modifiers,
    );
}
#[test]
fn escape_restores_a_moving_group_and_its_routes_without_an_undo_step() {
    let (ctx, mut app, [a, b]) = placement_setup();
    let e = app
        .doc
        .connect_endpoints(a, 0, None, Some([700., 600.]), ConnectorKind::Elbow)
        .unwrap();
    app.doc.set_automatic_route(e, false).unwrap();
    let d = app
        .doc
        .add_drawing(DrawingKind::Bezier, [200., 550.], [600., 550.]);
    let c = app.doc.add_chart(ChartKind::Columns, [800., 200.]);
    app.selected = vec![a, b, e, d, c];
    app.snap = false;
    let before = app.doc.clone();
    app.history = History::new(&before);
    app.saved = before.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 290.))],
        egui::Modifiers::NONE,
    );
    assert_ne!(app.doc, before);
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![a, b, e, d, c]);
    assert!(app.drag.is_none() && app.drag_snapshot.is_none());
    assert!(!app.dirty() && !app.history.can_undo());
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(310., 350.))],
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(310., 350.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn escape_resize_restores_the_original_size_style_and_attached_route() {
    let (ctx, mut app, [a, b]) = placement_setup();
    app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.doc.node_mut(a).unwrap().appearance.shadow =
        diagcraft::appearance::Shadow::preset(diagcraft::appearance::ShadowKind::Outer, 2);
    let before = app.doc.clone();
    app.history = History::new(&before);
    pointer(
        &ctx,
        &mut app,
        pos2(200., 280.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(250., 330.))],
        egui::Modifiers::SHIFT,
    );
    assert_ne!(app.doc, before);
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![a]);
    assert!(!app.history.can_undo());
}
#[test]
fn escape_on_a_new_waypoint_restores_automatic_routing() {
    let (ctx, mut app, [a, b]) = placement_setup();
    let e = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![e];
    let before = app.doc.clone();
    app.history = History::new(&before);
    let at = app.screen(app.segment_handles(e)[0]);
    pointer(&ctx, &mut app, at, true, egui::Modifiers::NONE);
    assert!(matches!(app.drag, Some(Drag::Waypoint { .. })));
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(at + vec2(20., 30.))],
        egui::Modifiers::NONE,
    );
    assert_ne!(app.doc, before);
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![e]);
    assert!(!app.history.can_undo());
}
#[test]
fn escape_on_an_existing_waypoint_or_free_connector_restores_the_full_geometry() {
    for waypoint in [false, true] {
        let (ctx, mut app) = setup();
        app.show_quick_toolbar = false;
        let e = app
            .doc
            .connect_endpoints(
                0,
                0,
                Some([200., 200.]),
                Some([600., 400.]),
                ConnectorKind::Elbow,
            )
            .unwrap();
        app.doc.add_connector_waypoint(e, [400., 200.]).unwrap();
        app.selected = vec![e];
        let before = app.doc.clone();
        app.history = History::new(&before);
        let at = if waypoint {
            pos2(400., 200.)
        } else {
            pos2(250., 200.)
        };
        pointer(&ctx, &mut app, at, true, egui::Modifiers::NONE);
        if waypoint {
            assert!(matches!(app.drag, Some(Drag::Waypoint { .. })));
        } else {
            assert!(matches!(app.drag, Some(Drag::FreeConnector { .. })));
        }
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(at + vec2(23., 37.))],
            egui::Modifiers::NONE,
        );
        assert_ne!(app.doc, before);
        gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![e]);
        assert!(!app.history.can_undo());
    }
}
#[test]
fn escape_on_a_connector_endpoint_discards_the_preview_and_keeps_the_link() {
    let (ctx, mut app, [a, b]) = placement_setup();
    let e = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![e];
    let before = app.doc.clone();
    app.history = History::new(&before);
    let at = app.screen(*app.doc.route(&app.doc.connectors[0]).last().unwrap());
    pointer(&ctx, &mut app, at, true, egui::Modifiers::NONE);
    assert!(matches!(app.drag, Some(Drag::Endpoint { .. })));
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(800., 600.))],
        egui::Modifiers::NONE,
    );
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    pointer(
        &ctx,
        &mut app,
        pos2(800., 600.),
        false,
        egui::Modifiers::NONE,
    );
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![e]);
    assert!(!app.history.can_undo());
}
#[test]
fn escape_drawing_a_line_or_curve_restores_the_previous_selection() {
    for tool in [Tool::Line, Tool::Bezier] {
        let (ctx, mut app, [a, _]) = placement_setup();
        app.set_tool(tool);
        let before = app.doc.clone();
        pointer(
            &ctx,
            &mut app,
            pos2(750., 100.),
            true,
            egui::Modifiers::NONE,
        );
        frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(pos2(850., 150.))],
            egui::Modifiers::NONE,
        );
        assert_eq!(app.doc.drawings.len(), 1);
        gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![a]);
        assert!(!app.history.can_undo());
        pointer(
            &ctx,
            &mut app,
            pos2(850., 150.),
            false,
            egui::Modifiers::NONE,
        );
        assert_eq!(app.doc, before);
    }
}
#[test]
fn too_short_a_drawing_does_not_dirty_the_document_or_replace_selection() {
    let (ctx, mut app, [a, _]) = placement_setup();
    app.set_tool(Tool::Line);
    let before = app.doc.clone();
    click(&ctx, &mut app, pos2(750., 100.));
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![a]);
    assert!(!app.dirty() && !app.history.can_undo());
}
#[test]
fn cancelling_a_drag_preserves_prior_unsaved_edits_and_the_redo_branch() {
    let (ctx, mut app, [a, _]) = placement_setup();
    let original = app.doc.clone();
    app.doc.title = "Avant le geste".into();
    app.property_edit = true;
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(250., 280.))],
        egui::Modifiers::NONE,
    );
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc.title, "Avant le geste");
    assert!(app.dirty());
    assert_eq!(
        app.doc.node(a).unwrap().position,
        original.node(a).unwrap().position
    );
    app.undo();
    assert_eq!(app.doc, original);
    assert!(!app.history.can_undo());
    assert!(app.history.can_redo());
    app.selected = vec![a];
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(250., 280.))],
        egui::Modifiers::NONE,
    );
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, original);
    assert!(app.history.can_redo());
    app.redo();
    assert_eq!(app.doc.title, "Avant le geste");
    assert_eq!(
        app.doc.node(a).unwrap().position,
        original.node(a).unwrap().position
    );
}
#[test]
fn command_z_during_a_drag_cancels_only_the_current_gesture() {
    let (ctx, mut app, _) = placement_setup();
    let original = app.doc.clone();
    app.doc.title = "Modification précédente".into();
    app.commit();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    gesture_key(&ctx, &mut app, Key::Z, egui::Modifiers::COMMAND);
    assert_eq!(app.doc, before);
    app.undo();
    assert_eq!(app.doc, original);
    assert!(!app.history.can_undo());
}
#[test]
fn keyboard_edits_cannot_interleave_with_an_active_drag() {
    let (ctx, mut app, _) = placement_setup();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    let during = app.doc.clone();
    gesture_key(&ctx, &mut app, Key::Delete, egui::Modifiers::NONE);
    gesture_key(&ctx, &mut app, Key::D, egui::Modifiers::COMMAND);
    assert_eq!(app.doc, during);
    assert!(app.drag.is_some());
    pointer(
        &ctx,
        &mut app,
        pos2(230., 280.),
        false,
        egui::Modifiers::NONE,
    );
    app.undo();
    assert!(!app.history.can_undo());
}
#[test]
fn focus_loss_cancels_a_geometry_drag_without_leaving_a_stuck_operation() {
    let (ctx, mut app, _) = placement_setup();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            focused: false,
            events: vec![egui::Event::WindowFocused(false)],
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            let before = app.doc.clone();
            app.canvas(ctx, true);
            app.finish_frame_edit(ctx, &before);
        },
    );
    assert_eq!(app.doc, before);
    assert!(app.drag.is_none() && app.drag_snapshot.is_none());
    assert!(!app.history.can_undo());
}
#[test]
fn saving_during_a_geometry_drag_finishes_it_without_later_replay() {
    let (ctx, mut app, [a, _]) = placement_setup();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("gesture.draw");
    assert!(app.save_to(path.clone()));
    let saved = Document::load(&path).unwrap();
    assert_ne!(
        saved.node(a).unwrap().position,
        before.node(a).unwrap().position
    );
    assert!(app.drag.is_none() && app.drag_snapshot.is_none());
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, saved);
    assert!(!app.dirty());
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}
#[test]
fn switching_documents_during_a_drag_cannot_restore_a_snapshot_in_the_wrong_tab() {
    let (ctx, mut app, [a, _]) = placement_setup();
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    let moved = app.doc.clone();
    app.open_tab(Document::default(), None);
    assert!(app.drag.is_none() && app.drag_snapshot.is_none());
    let second = app.doc.clone();
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, second);
    app.switch_tab(0);
    assert_eq!(app.doc, moved);
    assert_ne!(
        app.doc.node(a).unwrap().position,
        before.node(a).unwrap().position
    );
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn all_eight_handles_preserve_proportions_and_their_opposite_anchor_with_shift() {
    for index in 0..8 {
        let (ctx, mut app, [a, _]) = placement_setup();
        app.snap = false;
        let before = app.doc.clone();
        let n = app.doc.node(a).unwrap();
        let size = n.size;
        let origin = n.position;
        let (at, handle) = resize_handles(app.selected_node_rect().unwrap())[index];
        pointer(&ctx, &mut app, at, true, egui::Modifiers::NONE);
        pointer(
            &ctx,
            &mut app,
            at + vec2(f32::from(handle[0]) * 40., f32::from(handle[1]) * 30.),
            false,
            egui::Modifiers::SHIFT,
        );
        let n = app.doc.node(a).unwrap();
        assert!((n.size[0] / n.size[1] - size[0] / size[1]).abs() < 0.0001);
        for axis in 0..2 {
            let anchor = match handle[axis] {
                -1 => 1.,
                1 => 0.,
                _ => 0.5,
            };
            assert!(
                (n.position[axis] + n.size[axis] * anchor - origin[axis] - size[axis] * anchor)
                    .abs()
                    < 0.001
            );
        }
        app.doc.validate().unwrap();
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
#[test]
fn shift_resize_snaps_only_the_driving_axis_and_option_preserves_precision() {
    for alt in [false, true] {
        let (ctx, mut app, [a, _]) = placement_setup();
        app.doc.node_mut(a).unwrap().position = [103., 207.];
        app.history = History::new(&app.doc);
        let before = app.doc.clone();
        pointer(
            &ctx,
            &mut app,
            pos2(203., 287.),
            true,
            egui::Modifiers::NONE,
        );
        pointer(
            &ctx,
            &mut app,
            pos2(216., 303.),
            false,
            egui::Modifiers {
                shift: true,
                alt,
                ..egui::Modifiers::NONE
            },
        );
        let n = app.doc.node(a).unwrap();
        assert_eq!(n.position, [103., 207.]);
        assert!((n.size[0] / n.size[1] - 1.25).abs() < 0.0001);
        assert_eq!(n.size[1], if alt { 96. } else { 93. });
        app.undo();
        assert_eq!(app.doc, before);
    }
}
#[test]
fn proportional_resize_respects_page_edges_minimums_and_maximums() {
    use gestures_ui::{ResizeSettings, resize_geometry};
    let settings = || ResizeSettings {
        bounds: [600., 600.],
        minimum: 24.,
        grid: None,
        proportional: true,
    };
    let (p, s) = resize_geometry([400., 400.], [100., 80.], [1, 1], [500., 500.], settings());
    assert_eq!(p, [400., 400.]);
    assert_eq!(s, [200., 160.]);
    let (p, s) = resize_geometry(
        [100., 200.],
        [100., 80.],
        [-1, -1],
        [500., 500.],
        settings(),
    );
    assert!((s[0] - 30.).abs() < 0.001 && (s[1] - 24.).abs() < 0.001);
    assert_eq!([p[0] + s[0], p[1] + s[1]], [200., 280.]);
    let (_, s) = resize_geometry(
        [100., 200.],
        [100., 80.],
        [1, 1],
        [100000., 100000.],
        ResizeSettings {
            bounds: [100000., 100000.],
            minimum: 24.,
            grid: None,
            proportional: true,
        },
    );
    assert_eq!(s, [20000., 16000.]);
}
#[test]
fn charts_preserve_their_ratio_and_valid_minimum_dimensions() {
    let (ctx, mut app) = setup();
    app.show_quick_toolbar = false;
    app.snap = false;
    let c = app.doc.add_chart(ChartKind::Columns, [500., 400.]);
    app.doc.charts[0].position = [300., 300.];
    app.doc.charts[0].size = [240., 120.];
    app.selected = vec![c];
    app.history = History::new(&app.doc);
    let before = app.doc.clone();
    pointer(
        &ctx,
        &mut app,
        pos2(540., 420.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(600., 450.),
        false,
        egui::Modifiers::SHIFT,
    );
    assert_eq!(app.doc.charts[0].size, [300., 150.]);
    app.doc.validate().unwrap();
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.selected = vec![c];
    pointer(
        &ctx,
        &mut app,
        pos2(540., 420.),
        true,
        egui::Modifiers::NONE,
    );
    pointer(
        &ctx,
        &mut app,
        pos2(400., 350.),
        false,
        egui::Modifiers::SHIFT,
    );
    assert_eq!(app.doc.charts[0].size, [240., 120.]);
    app.doc.validate().unwrap();
}
#[test]
fn changing_shift_during_resize_recalculates_from_the_original_geometry() {
    let (ctx, mut app, [a, _]) = placement_setup();
    app.snap = false;
    pointer(
        &ctx,
        &mut app,
        pos2(200., 280.),
        true,
        egui::Modifiers::NONE,
    );
    let end = pos2(240., 310.);
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(end)],
        egui::Modifiers::SHIFT,
    );
    assert_eq!(app.doc.node(a).unwrap().size, [140., 112.]);
    frame(&ctx, &mut app, vec![], egui::Modifiers::NONE);
    assert_eq!(app.doc.node(a).unwrap().size, [140., 110.]);
    frame(&ctx, &mut app, vec![], egui::Modifiers::SHIFT);
    assert_eq!(app.doc.node(a).unwrap().size, [140., 112.]);
}
#[test]
fn cursors_describe_all_resize_handles_and_object_movement() {
    let (_, mut app, [a, _]) = placement_setup();
    let expected = [
        egui::CursorIcon::ResizeNwSe,
        egui::CursorIcon::ResizeVertical,
        egui::CursorIcon::ResizeNeSw,
        egui::CursorIcon::ResizeHorizontal,
        egui::CursorIcon::ResizeNwSe,
        egui::CursorIcon::ResizeVertical,
        egui::CursorIcon::ResizeNeSw,
        egui::CursorIcon::ResizeHorizontal,
    ];
    for (i, (at, _)) in resize_handles(app.selected_node_rect().unwrap())
        .into_iter()
        .enumerate()
    {
        assert_eq!(app.manipulation_cursor(Some(at)), expected[i]);
    }
    assert_eq!(
        app.manipulation_cursor(Some(pos2(150., 240.))),
        egui::CursorIcon::Grab
    );
    assert_eq!(
        app.manipulation_cursor(Some(pos2(800., 700.))),
        egui::CursorIcon::Default
    );
    app.select_for_drag(a, [150., 240.], false);
    assert_eq!(
        app.manipulation_cursor(Some(pos2(800., 700.))),
        egui::CursorIcon::Grabbing
    );
}
#[test]
fn geometry_feedback_wraps_and_stays_visible_above_zoom_on_a_small_canvas() {
    for resize in [false, true] {
        let (ctx, mut app, [a, _]) = placement_setup();
        if resize {
            pointer(
                &ctx,
                &mut app,
                pos2(200., 280.),
                true,
                egui::Modifiers::NONE,
            );
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(pos2(240., 310.))],
                egui::Modifiers::SHIFT,
            );
        } else {
            pointer(
                &ctx,
                &mut app,
                pos2(150., 240.),
                true,
                egui::Modifiers::NONE,
            );
            frame(
                &ctx,
                &mut app,
                vec![egui::Event::PointerMoved(pos2(447., 240.))],
                egui::Modifiers::NONE,
            );
        }
        let doc = app.doc.clone();
        let selected = app.selected.clone();
        app.show_library = true;
        app.show_inspector = true;
        app.ribbon_expanded = Some(false);
        let out = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
                ..Default::default()
            },
            |ctx| {
                app.header(ctx);
                app.footer(ctx);
                app.pages_bar(ctx);
                app.side_panels(ctx);
                app.canvas(ctx, false);
            },
        );
        let text = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                Shape::Text(t)
                    if t.galley.job.text.contains("Échap : annuler")
                        && (t.galley.job.text.contains("Maj : proportions")
                            || t.galley.job.text.starts_with("Aligné · X")) =>
                {
                    Some((s.clip_rect, t))
                }
                _ => None,
            })
            .unwrap();
        let rect = Rect::from_min_size(text.1.pos, text.1.galley.size());
        assert!(text.0.contains_rect(rect));
        assert!(app.canvas.contains_rect(rect));
        assert!(rect.bottom() < app.canvas.bottom() - 65.);
        assert_eq!(app.doc, doc);
        assert_eq!(app.selected, selected);
        assert_eq!(app.selected, vec![a]);
    }
}
#[test]
fn proportional_resize_keeps_imported_off_page_positions_and_dimensions_valid() {
    use gestures_ui::{ResizeSettings, resize_geometry};
    for origin in [[100000., 100000.], [-100000., -100000.], [20000., 100.]] {
        for (_, handle) in resize_handles(Rect::from_min_size(Pos2::ZERO, vec2(100., 80.))) {
            for delta in [[100000., 100000.], [-100000., -100000.], [13., 17.]] {
                let (position, size) = resize_geometry(
                    origin,
                    [100., 80.],
                    handle,
                    delta,
                    ResizeSettings {
                        bounds: [600., 600.],
                        minimum: 24.,
                        grid: Some(10.),
                        proportional: true,
                    },
                );
                assert!(position.iter().all(|v| (-100000.0..=100000.0).contains(v)));
                assert!(size.iter().all(|v| (24.0..=20000.0).contains(v)));
                assert!((size[0] / size[1] - 1.25).abs() < 0.0001);
            }
        }
    }
}
#[test]
fn connector_endpoint_and_bend_handles_expose_a_crosshair_cursor() {
    let (_, mut app, [a, b]) = placement_setup();
    let e = app.doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    app.selected = vec![e];
    let route = app.doc.route(&app.doc.connectors[0]);
    for pt in [route[0], *route.last().unwrap(), app.segment_handles(e)[0]] {
        assert_eq!(
            app.manipulation_cursor(Some(app.screen(pt))),
            egui::CursorIcon::Crosshair
        );
    }
}
#[test]
fn closing_an_inactive_tab_finishes_the_gesture_and_keeps_its_undo_step() {
    let (ctx, mut app, [a, _]) = placement_setup();
    let before = app.doc.clone();
    app.open_tab(Document::default(), None);
    app.switch_tab(0);
    app.selected = vec![a];
    pointer(
        &ctx,
        &mut app,
        pos2(150., 240.),
        true,
        egui::Modifiers::NONE,
    );
    frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(pos2(230., 280.))],
        egui::Modifiers::NONE,
    );
    let moved = app.doc.clone();
    assert_ne!(moved, before);
    app.close_tab(1);
    assert_eq!(app.doc, moved);
    assert!(app.drag.is_none() && app.drag_snapshot.is_none());
    gesture_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
    assert_eq!(app.doc, moved);
    app.undo();
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
}

#[test]
fn compact_tabs_have_distinct_commands_and_never_change_the_drawing() {
    let (ctx, mut app, id) = panel_setup();
    app.selected = vec![id];
    app.ribbon_expanded = Some(false);
    let before = app.doc.clone();
    let size = vec2(1200., 760.);
    let initial = stable_panel(&ctx, &mut app, size);
    let height = app.canvas.height();
    let mut out = initial;
    for (label, tab, command) in [
        ("Insérer", RibbonTab::Insert, "Tableau"),
        ("Conception", RibbonTab::Design, "Thèmes"),
        ("Affichage", RibbonTab::View, "Volets et ruban"),
        ("Symboles", RibbonTab::Symbols, "Afficher les formes"),
        ("Avancé", RibbonTab::Advanced, "Vérifier le document"),
        ("Accueil", RibbonTab::Home, "Relier · C"),
    ] {
        panel_click(&ctx, &mut app, size, ux_text(&out, label));
        out = stable_panel(&ctx, &mut app, size);
        assert!(app.ribbon_tab == tab);
        assert!(!app.ribbon_is_expanded(&ctx));
        ux_text(&out, command);
        assert!((app.canvas.height() - height).abs() < 1.);
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![id]);
        assert!(!app.history.can_undo());
    }
}

#[test]
fn wide_panel_preferences_survive_a_temporary_small_window() {
    let (ctx, mut app, id) = panel_setup();
    app.show_inspector = true;
    app.show_quick_toolbar = false;
    app.selected = vec![id];
    app.library_width = 400.;
    app.inspector_width = 400.;
    let before = app.doc.clone();
    stable_panel(&ctx, &mut app, vec2(1600., 960.));
    let left = app.canvas.left();
    let right_width = 1600. - app.canvas.right();
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    assert!(app.canvas.width() >= 490., "{}", app.canvas.width());
    assert!(app.canvas.height() > 400.);
    assert_eq!((app.library_width, app.inspector_width), (400., 400.));
    stable_panel(&ctx, &mut app, vec2(1600., 960.));
    assert!((app.canvas.left() - left).abs() < 1.);
    assert!((1600. - app.canvas.right() - right_width).abs() < 1.);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    assert!(!app.history.can_undo());
}

#[test]
fn panel_edge_drag_changes_the_preference_without_moving_objects() {
    let (ctx, mut app, id) = panel_setup();
    let size = vec2(1440., 960.);
    app.show_inspector = true;
    app.selected = vec![id];
    app.show_quick_toolbar = false;
    let before = app.doc.clone();
    stable_panel(&ctx, &mut app, size);
    for (name, right) in [("symbol_library", false), ("inspector", true)] {
        let rect = egui::containers::panel::PanelState::load(&ctx, egui::Id::new(name))
            .unwrap()
            .rect;
        let start = pos2(
            if right { rect.left() } else { rect.right() },
            rect.center().y,
        );
        let end = start + vec2(if right { -35. } else { 35. }, 0.);
        let width = if right {
            app.inspector_width
        } else {
            app.library_width
        };
        panel_frame(&ctx, &mut app, size, vec![egui::Event::PointerMoved(start)]);
        panel_pointer(&ctx, &mut app, size, start, true);
        panel_frame(&ctx, &mut app, size, vec![egui::Event::PointerMoved(end)]);
        panel_frame(&ctx, &mut app, size, vec![]);
        panel_pointer(&ctx, &mut app, size, end, false);
        stable_panel(&ctx, &mut app, size);
        let changed = if right {
            app.inspector_width
        } else {
            app.library_width
        };
        assert!(changed > width + 25., "{name}: {width} -> {changed}");
    }
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    assert!(!app.history.can_undo());
    assert!(app.drag.is_none());
}

#[test]
fn interface_preferences_roundtrip_and_reset_preserve_documents_and_undo() {
    let (ctx, mut app, id) = panel_setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("interface-v1.json");
    app.interface_path = Some(path.clone());
    app.library_width = 333.;
    app.inspector_width = 345.;
    app.show_library = false;
    app.show_inspector = true;
    app.ribbon_expanded = Some(false);
    app.ribbon_tab = RibbonTab::Insert;
    app.library_panel = LibraryPanel::Charts;
    app.show_quick_toolbar = false;
    app.save_interface().unwrap();
    let (_, mut reopened, _) = panel_setup();
    reopened.interface_path = Some(path.clone());
    reopened.restore_interface();
    assert_eq!(
        (reopened.library_width, reopened.inspector_width),
        (333., 345.)
    );
    assert!(!reopened.show_library && reopened.show_inspector && !reopened.show_quick_toolbar);
    assert!(
        reopened.ribbon_tab == RibbonTab::Insert && reopened.library_panel == LibraryPanel::Charts
    );
    assert_eq!(reopened.ribbon_expanded, Some(false));
    app.doc.node_mut(id).unwrap().text = "Une modification annulable".into();
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    app.run_command(commands_ui::Command::ResetInterface, &ctx);
    assert_eq!((app.library_width, app.inspector_width), (256., 280.));
    assert!(app.show_library && !app.show_inspector);
    assert_eq!(app.ribbon_expanded, None);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    app.undo();
    assert_ne!(app.doc, before);
    app.save_interface().unwrap();
    reopened.restore_interface();
    assert!(reopened.show_library && !reopened.show_inspector);
    assert_eq!(reopened.library_width, 256.);
}

#[test]
fn corrupt_or_future_interface_preferences_are_preserved_and_dimensions_are_bounded() {
    let (_, mut app, _) = panel_setup();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("interface-v1.json");
    for bytes in [
        b"broken JSON".to_vec(),
        br#"{"version":999}"#.to_vec(),
        vec![b' '; 8193],
    ] {
        std::fs::write(&path, &bytes).unwrap();
        app.interface_path = Some(path.clone());
        app.restore_interface();
        assert!(app.interface_path.is_none());
        app.save_interface().unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    std::fs::write(&path, br#"{"library_width":-10,"inspector_width":99999}"#).unwrap();
    app.interface_path = Some(path);
    app.restore_interface();
    assert_eq!((app.library_width, app.inspector_width), (240., 400.));
    assert_eq!(app.doc, app.saved);
}

#[test]
fn searching_hidden_symbol_libraries_does_not_change_enabled_libraries_or_the_document() {
    let (ctx, mut app, _) = panel_setup();
    app.library.visible_libraries = vec!["basic".into()];
    app.symbol_search = "classe uml".into();
    let before = app.doc.clone();
    let size = vec2(1280., 800.);
    let out = stable_panel(&ctx, &mut app, size);
    ux_text(&out, "Recherche dans toutes les bibliothèques");
    let tile = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            Shape::Rect(r)
                if (r.rect.width() - 48.).abs() < 0.1
                    && (r.rect.height() - 56.).abs() < 0.1
                    && r.rect.left() < app.canvas.left()
                    && s.clip_rect.contains_rect(r.rect) =>
            {
                Some(r.rect.center())
            }
            _ => None,
        })
        .expect("Class symbol found in a hidden library");
    panel_click(&ctx, &mut app, size, tile);
    assert!(app.pending_symbol.is_some());
    assert_eq!(app.library.visible_libraries, vec!["basic"]);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.symbol_search = "introuvable 123".into();
    let out = stable_panel(&ctx, &mut app, size);
    ux_text(&out, "Aucun symbole trouvé. Essayez un autre mot.");
    ux_text(&out, "+ Bibliothèques de symboles…");
}

fn rich_editor_frame(ctx: &egui::Context, app: &mut DrawApp, key: Option<(Key, egui::Modifiers)>) {
    let (events, modifiers) = key
        .map(|(key, modifiers)| {
            (
                vec![egui::Event::Key {
                    key,
                    physical_key: Some(key),
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
                modifiers,
            )
        })
        .unwrap_or_default();
    let _ = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            app.canvas(ctx, false);
            app.inline_text_toolbar(ctx);
        },
    );
}

#[test]
fn rich_editor_shortcuts_format_selection_cancel_and_keep_document_history_untouched() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [350., 250.]);
    app.doc.node_mut(id).unwrap().text = "Superbe 中文".into();
    app.commit();
    app.saved = app.doc.clone();
    app.history = History::new(&app.doc);
    let original = app.doc.clone();
    app.open_node_text(id);
    rich_editor_frame(&ctx, &mut app, None);
    rich_editor_frame(&ctx, &mut app, Some((Key::B, egui::Modifiers::COMMAND)));
    assert!(
        app.rich_editor
            .as_ref()
            .unwrap()
            .draft
            .resolved_style_at(0)
            .text_style
            .bold
    );
    assert_eq!(app.doc, original);
    rich_editor_frame(&ctx, &mut app, Some((Key::Z, egui::Modifiers::COMMAND)));
    assert!(app.rich_editor.as_ref().unwrap().draft.text_runs.is_empty());
    rich_editor_frame(
        &ctx,
        &mut app,
        Some((Key::Z, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT)),
    );
    assert!(
        app.rich_editor
            .as_ref()
            .unwrap()
            .draft
            .resolved_style_at(0)
            .text_style
            .bold
    );
    let id = egui::Id::new("rich_text_input");
    let mut state = egui::TextEdit::load_state(&ctx, id).unwrap();
    state
        .cursor
        .set_char_range(Some(egui::text::CCursorRange::two(
            egui::text::CCursor::new(8),
            egui::text::CCursor::new(10),
        )));
    state.store(&ctx, id);
    rich_editor_frame(&ctx, &mut app, None);
    rich_editor_frame(&ctx, &mut app, Some((Key::I, egui::Modifiers::COMMAND)));
    let n = &app.rich_editor.as_ref().unwrap().draft;
    assert!(n.resolved_style_at(8).text_style.italic && n.resolved_style_at(9).text_style.italic);
    assert!(!n.resolved_style_at(0).text_style.italic);
    rich_editor_frame(&ctx, &mut app, Some((Key::Escape, egui::Modifiers::NONE)));
    assert!(app.rich_editor.is_none());
    assert_eq!(app.doc, original);
    assert!(!app.dirty() && !app.history.can_undo());
}

#[test]
fn rich_editor_applies_as_one_undo_keeps_links_and_rejects_stale_drafts() {
    let (_, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [350., 250.]);
    let target = app.doc.add_node(ShapeKind::Rectangle, [700., 250.]);
    app.doc.connect(id, target, ConnectorKind::Elbow).unwrap();
    app.commit();
    let original = app.doc.clone();
    app.open_node_text(id);
    let n = &mut app.rich_editor.as_mut().unwrap().draft;
    n.set_text("test Le BLOC\n\nSuperbe 中文".into()).unwrap();
    n.format_text(
        5..7,
        Some(&diagcraft::rich_text::InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    n.text_style.auto_fit = true;
    n.text_style.wrap = true;
    app.apply_node_text().unwrap();
    assert!(app.rich_editor.is_none());
    assert_eq!(app.doc.connectors, original.connectors);
    assert_eq!(
        app.doc.node(id).unwrap().position,
        original.node(id).unwrap().position
    );
    let edited = app.doc.clone();
    app.undo();
    assert_eq!(app.doc, original);
    app.redo();
    assert_eq!(app.doc, edited);
    app.open_node_text(id);
    app.doc.title = "external edit".into();
    let current = app.doc.clone();
    assert!(app.apply_node_text().is_err());
    assert_eq!(app.doc, current);
    assert!(app.rich_editor.is_some());
}

#[test]
fn native_rich_text_paints_distinct_fonts_colors_and_decorations_inside_one_block() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [350., 250.]);
    let n = app.doc.node_mut(id).unwrap();
    n.text = "test Le BLOC\n\nSuperbe 中文".into();
    n.format_text(
        5..7,
        Some(&diagcraft::rich_text::InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    n.format_text(
        14..21,
        Some(&diagcraft::rich_text::InlineStyle {
            italic: Some(true),
            underline: Some(true),
            font_size: Some(28.),
            color: Some([170, 30, 60]),
            ..Default::default()
        }),
    )
    .unwrap();
    n.text_style.auto_fit = true;
    n.text_style.wrap = true;
    app.commit();
    let n = app.doc.node(id).unwrap().clone();
    let job = rich_text_ui::document_job(&n, 1., false);
    assert_eq!(job.text, n.display_text());
    assert!(
        job.sections
            .iter()
            .any(|s| s.format.font_id.family == egui::FontFamily::Name("SourceSans3-Bold".into()))
    );
    assert!(job.sections.iter().any(|s| s.format.font_id.size == 28.
        && s.format.color == rgb([170, 30, 60])
        && s.format.underline != Stroke::NONE));
    let _ = ctx.run(egui::RawInput::default(), |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| app.paint_node(ui.painter(), &n));
    });
}

fn inline_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut replay = std::mem::take(&mut app.replay_input);
    replay.extend(events);
    let events = replay;
    let modifiers = events
        .iter()
        .find_map(|e| {
            if let egui::Event::Key { modifiers, .. } = e {
                Some(*modifiers)
            } else {
                None
            }
        })
        .unwrap_or_default();
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 800.))),
            events,
            modifiers,
            ..Default::default()
        },
        |ctx| {
            app.inline_outside_press(ctx);
            app.shortcuts(ctx);
            app.canvas(ctx, true);
            app.inline_text_toolbar(ctx);
        },
    )
}
fn inline_key(key: Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: Some(key),
        pressed: true,
        repeat: false,
        modifiers,
    }
}
fn inline_text_shape(out: &egui::FullOutput, text: &str) -> egui::epaint::TextShape {
    out.shapes
        .iter()
        .find_map(|s| {
            if let Shape::Text(t) = &s.shape {
                (t.galley.job.text == text).then(|| t.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("Missing inline text: {text}"))
}
fn inline_pointer(
    ctx: &egui::Context,
    app: &mut DrawApp,
    p: Pos2,
    pressed: bool,
) -> egui::FullOutput {
    inline_frame(
        ctx,
        app,
        vec![
            egui::Event::PointerMoved(p),
            egui::Event::PointerButton {
                pos: p,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    )
}

#[test]
fn inline_double_click_requires_two_clicks_on_the_same_existing_block() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [350., 300.]);
    let other = app.doc.add_node(ShapeKind::Rectangle, [850., 550.]);
    app.commit();
    inline_frame(&ctx, &mut app, vec![]);
    let at = app.screen(app.doc.node(id).unwrap().center());
    let target = app.screen(app.doc.node(other).unwrap().center());
    inline_pointer(&ctx, &mut app, at, true);
    inline_pointer(&ctx, &mut app, at, false);
    inline_pointer(&ctx, &mut app, target, true);
    inline_pointer(&ctx, &mut app, target, false);
    assert!(
        app.rich_editor.is_none(),
        "Rapid clicks on different blocks are selection only"
    );
    for _ in 0..25 {
        inline_frame(&ctx, &mut app, vec![]);
    }
    for _ in 0..2 {
        inline_pointer(&ctx, &mut app, at, true);
        inline_pointer(&ctx, &mut app, at, false);
    }
    assert_eq!(app.rich_editor.as_ref().unwrap().draft.id, id);
}

#[test]
fn inline_editor_stays_in_the_block_at_each_zoom_and_alignment_without_a_modal() {
    for zoom in [0.5, 1., 2.] {
        for align in [TextAlign::Left, TextAlign::Center, TextAlign::Right] {
            let (ctx, mut app) = setup();
            app.zoom = zoom;
            let id = app.doc.add_node(ShapeKind::Rounded, [300., 200.]);
            let n = app.doc.node_mut(id).unwrap();
            n.text = "One line\n中文 Café".into();
            n.text_style.align = align;
            n.size = [320., 140.];
            n.format_text(
                9..11,
                Some(&diagcraft::rich_text::InlineStyle {
                    bold: Some(true),
                    ..Default::default()
                }),
            )
            .unwrap();
            app.commit();
            let before = app.doc.clone();
            app.open_node_text(id);
            let mut out = inline_frame(&ctx, &mut app, vec![]);
            for _ in 0..2 {
                out = inline_frame(&ctx, &mut app, vec![]);
            }
            let shape = inline_text_shape(&out, &app.doc.node(id).unwrap().text);
            let editor = app.rich_editor.as_ref().unwrap();
            let bounds = shape.galley.rect.translate(shape.pos.to_vec2());
            assert!(
                editor.content_rect.expand(2.).contains_rect(bounds),
                "{zoom:?} {align:?}: {bounds:?} vs {:?}",
                editor.content_rect
            );
            let expected = match align {
                TextAlign::Left => editor.content_rect.left(),
                TextAlign::Center => editor.content_rect.center().x,
                TextAlign::Right => editor.content_rect.right(),
            };
            let actual = match align {
                TextAlign::Left => bounds.left(),
                TextAlign::Center => bounds.center().x,
                TextAlign::Right => bounds.right(),
            };
            assert!((expected - actual).abs() < 2.);
            assert!((bounds.center().y - editor.content_rect.center().y).abs() < 2.);
            assert_eq!(shape.galley.rect.min.x, 0.);
            assert!(
                shape
                    .galley
                    .job
                    .sections
                    .iter()
                    .any(|s| (s.format.font_id.size - 18. * zoom).abs() < 0.01)
            );
            assert!(app.canvas.shrink(7.).contains_rect(editor.toolbar_rect));
            assert!(!out.shapes.iter().any(
                |s| matches!(&s.shape,Shape::Text(t) if t.galley.job.text=="Contenu du bloc")
            ));
            assert_eq!(app.doc, before);
        }
    }
}

#[test]
fn inline_click_places_a_unicode_caret_and_outside_click_commits_then_selects_the_next_block() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [300., 250.]);
    let other = app.doc.add_node(ShapeKind::Rectangle, [800., 450.]);
    let n = app.doc.node_mut(id).unwrap();
    n.text = "alpha 中文 café beta".into();
    n.size = [400., 100.];
    n.text_style.align = TextAlign::Left;
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    app.history = History::new(&before);
    app.open_node_text(id);
    inline_frame(&ctx, &mut app, vec![]);
    let out = inline_frame(&ctx, &mut app, vec![]);
    let shape = inline_text_shape(&out, &before.node(id).unwrap().text);
    let at = shape.pos
        + shape
            .galley
            .pos_from_cursor(egui::text::CCursor::new(6))
            .center()
            .to_vec2();
    app.finish_inline_text(&ctx, true);
    app.open_node_text_at(id, Some(at));
    inline_frame(&ctx, &mut app, vec![]);
    inline_frame(&ctx, &mut app, vec![egui::Event::Text("绘".into())]);
    assert_eq!(
        app.rich_editor.as_ref().unwrap().draft.text,
        "alpha 绘中文 café beta"
    );
    assert_eq!(app.doc, before);
    let target = app.screen(app.doc.node(other).unwrap().center());
    inline_pointer(&ctx, &mut app, target, true);
    inline_pointer(&ctx, &mut app, target, false);
    assert!(app.rich_editor.is_none());
    assert_eq!(app.selected, vec![other]);
    assert_eq!(app.doc.node(id).unwrap().text, "alpha 绘中文 café beta");
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn inline_drag_selects_words_and_toolbar_clicks_preserve_the_selection_and_position() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [350., 320.]);
    let n = app.doc.node_mut(id).unwrap();
    n.text = "alpha 中文 café beta".into();
    n.size = [420., 120.];
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    app.open_node_text(id);
    inline_frame(&ctx, &mut app, vec![]);
    let out = inline_frame(&ctx, &mut app, vec![]);
    let shape = inline_text_shape(&out, &before.node(id).unwrap().text);
    let at = |i| {
        shape.pos
            + shape
                .galley
                .pos_from_cursor(egui::text::CCursor::new(i))
                .center()
                .to_vec2()
    };
    inline_pointer(&ctx, &mut app, at(6), true);
    inline_frame(&ctx, &mut app, vec![egui::Event::PointerMoved(at(8))]);
    let out = inline_pointer(&ctx, &mut app, at(8), false);
    let range = egui::TextEdit::load_state(&ctx, egui::Id::new("rich_text_input"))
        .unwrap()
        .cursor
        .char_range()
        .unwrap()
        .as_sorted_char_range();
    assert_eq!(
        range,
        6..8,
        "The pointer drag must select exactly the Chinese word"
    );
    let bold = out
        .shapes
        .iter()
        .find_map(|s| {
            if let Shape::Text(t) = &s.shape {
                (t.galley.job.text == "B").then_some(t.pos + t.galley.rect.center().to_vec2())
            } else {
                None
            }
        })
        .unwrap();

    inline_pointer(&ctx, &mut app, bold, true);
    inline_pointer(&ctx, &mut app, bold, false);
    inline_frame(&ctx, &mut app, vec![]);
    let n = &app.rich_editor.as_ref().unwrap().draft;
    assert!(
        n.resolved_style_at(6).text_style.bold && n.resolved_style_at(7).text_style.bold,
        "runs: {:?}",
        n.text_runs
    );
    assert!(!n.resolved_style_at(0).text_style.bold && !n.resolved_style_at(9).text_style.bold);
    assert_eq!(n.position, before.node(id).unwrap().position);
    assert_eq!(app.doc, before);
}

#[test]
fn inline_keyboard_entry_newline_finish_cancel_and_save_obey_text_editing_rules() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [350., 300.]);
    app.selected = vec![id];
    app.commit();
    let before = app.doc.clone();
    inline_frame(
        &ctx,
        &mut app,
        vec![inline_key(Key::F2, egui::Modifiers::NONE)],
    );
    assert!(app.rich_editor.is_some());
    inline_frame(&ctx, &mut app, vec![egui::Event::Text("New label".into())]);
    inline_frame(
        &ctx,
        &mut app,
        vec![inline_key(Key::Enter, egui::Modifiers::NONE)],
    );
    assert!(app.rich_editor.as_ref().unwrap().draft.text.contains('\n'));
    inline_frame(
        &ctx,
        &mut app,
        vec![inline_key(Key::Escape, egui::Modifiers::NONE)],
    );
    assert_eq!(app.doc, before);
    inline_frame(
        &ctx,
        &mut app,
        vec![inline_key(Key::Enter, egui::Modifiers::NONE)],
    );
    inline_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Text("Saved directly".into())],
    );
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("inline.draw");
    before.save(&path).unwrap();
    app.path = Some(path.clone());
    inline_frame(
        &ctx,
        &mut app,
        vec![inline_key(Key::S, egui::Modifiers::COMMAND)],
    );
    assert!(app.rich_editor.is_none());
    assert!(!app.dirty());
    assert_eq!(
        Document::load(&path).unwrap().node(id).unwrap().text,
        "Saved directly"
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn inline_finishing_in_the_same_frame_as_typing_keeps_the_last_characters() {
    for action in [
        "outside", "done", "keyboard", "save", "ime", "close", "drop",
    ] {
        let (ctx, mut app) = setup();
        let id = app.doc.add_node(ShapeKind::Rectangle, [350., 300.]);
        let other = app.doc.add_node(ShapeKind::Rectangle, [850., 550.]);
        app.selected = vec![id];
        app.commit();
        let before = app.doc.clone();
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inline.draw");
        before.save(&path).unwrap();
        app.path = Some(path.clone());
        app.open_node_text(id);
        inline_frame(&ctx, &mut app, vec![]);
        let out = inline_frame(&ctx, &mut app, vec![]);
        let mut events = vec![egui::Event::Text("Last characters 中文".into())];
        if action == "ime" {
            inline_frame(
                &ctx,
                &mut app,
                vec![
                    egui::Event::Text(String::new()),
                    egui::Event::Ime(egui::ImeEvent::Enabled),
                    egui::Event::Ime(egui::ImeEvent::Preedit("中文".into())),
                ],
            );
            events = vec![egui::Event::Ime(egui::ImeEvent::Commit("中文".into()))];
        }
        let mut pointer = None;
        match action {
            "keyboard" => events.push(inline_key(Key::Enter, egui::Modifiers::COMMAND)),
            "save" => events.push(inline_key(Key::S, egui::Modifiers::COMMAND)),
            "close" => app.defer_inline_close(),
            "drop" => app.inline_open(path.clone()),
            _ => {
                let p = if action == "done" {
                    let shape = inline_text_shape(&out, "Terminer");
                    shape.pos + shape.galley.rect.center().to_vec2()
                } else {
                    app.screen(app.doc.node(other).unwrap().center())
                };
                events.extend([
                    egui::Event::PointerMoved(p),
                    egui::Event::PointerButton {
                        pos: p,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]);
                pointer = Some(p);
            }
        }
        inline_frame(&ctx, &mut app, events);
        if let Some(p) = pointer {
            inline_pointer(&ctx, &mut app, p, false);
            inline_frame(&ctx, &mut app, vec![]);
        }
        assert!(app.rich_editor.is_none(), "{action}");
        if action == "drop" {
            assert_eq!(app.tabs.len(), 2);
            app.switch_tab(0);
        }
        let expected = if action == "ime" {
            "中文"
        } else {
            "Last characters 中文"
        };
        assert_eq!(app.doc.node(id).unwrap().text, expected, "{action}");
        if action == "save" {
            assert_eq!(
                Document::load(&path).unwrap().node(id).unwrap().text,
                expected
            );
            assert!(!app.dirty());
        } else if action == "outside" || action == "ime" {
            assert_eq!(app.selected, vec![other]);
        } else if action == "close" {
            assert!(app.pending.is_some());
        }
        app.undo();
        assert_eq!(app.doc, before, "{action}");
    }
}

#[test]
fn inline_trackpad_navigation_keeps_the_draft_and_preview_connectors_follow_auto_fit() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rounded, [350., 300.]);
    let other = app.doc.add_node(ShapeKind::Rectangle, [750., 550.]);
    let edge = app.doc.connect(id, other, ConnectorKind::Elbow).unwrap();
    app.doc.node_mut(id).unwrap().text_style.auto_fit = true;
    app.doc.node_mut(id).unwrap().text_style.wrap = true;
    app.commit();
    let before = app.doc.clone();
    app.open_node_text(id);
    inline_frame(&ctx, &mut app, vec![]);
    inline_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Text("A long label 中文 français ".repeat(8))],
    );
    let preview = app.inline_preview_document().unwrap();
    assert!(preview.node(id).unwrap().size[1] > before.node(id).unwrap().size[1]);
    assert_ne!(
        preview.route(preview.connectors.iter().find(|e| e.id == edge).unwrap()),
        before.route(&before.connectors[0])
    );
    let old_pan = app.pan;
    let at = app.canvas.left_bottom() + vec2(30., -30.);
    inline_frame(
        &ctx,
        &mut app,
        vec![
            egui::Event::PointerMoved(at),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(25., 35.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert_eq!(app.pan, old_pan + vec2(25., 35.));
    let zoom = app.zoom;
    let at = app.rich_editor.as_ref().unwrap().content_rect.center();
    inline_frame(
        &ctx,
        &mut app,
        vec![egui::Event::PointerMoved(at), egui::Event::Zoom(1.25)],
    );
    assert!((app.zoom - zoom * 1.25).abs() < 0.001);
    assert!(app.rich_editor.is_some());
    assert_eq!(app.doc, before);
    app.finish_inline_text(&ctx, false);
    assert_eq!(app.doc.nodes, preview.nodes);
    assert_eq!(app.doc.connectors, before.connectors);
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn inline_invalid_drafts_do_not_execute_an_outside_action_and_tab_changes_commit_valid_text() {
    let (ctx, mut app) = setup();
    let id = app.doc.add_node(ShapeKind::Rectangle, [350., 300.]);
    app.commit();
    app.selected = vec![id];
    let before = app.doc.clone();
    app.open_node_text(id);
    inline_frame(&ctx, &mut app, vec![]);
    app.rich_editor.as_mut().unwrap().draft.font_size = 201.;
    let at = app.canvas.right_bottom() - vec2(30., 30.);
    inline_pointer(&ctx, &mut app, at, true);
    inline_pointer(&ctx, &mut app, at, false);
    assert!(app.rich_editor.is_some());
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    app.finish_inline_text(&ctx, true);
    app.open_tab(Document::default(), None);
    app.switch_tab(0);
    app.open_node_text(id);
    app.rich_editor
        .as_mut()
        .unwrap()
        .draft
        .set_text("Kept when switching tabs".into())
        .unwrap();
    app.switch_tab(1);
    assert!(app.rich_editor.is_none());
    app.switch_tab(0);
    assert_eq!(app.doc.node(id).unwrap().text, "Kept when switching tabs");
    app.undo();
    assert_eq!(app.doc, before);
}
