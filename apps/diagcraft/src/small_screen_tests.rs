use super::*;

fn short_dialog_frame(
    ctx: &egui::Context,
    app: &mut DrawApp,
    size: Vec2,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
            modifiers: events
                .iter()
                .find_map(|event| match event {
                    egui::Event::Key { modifiers, .. } => Some(*modifiers),
                    _ => None,
                })
                .unwrap_or_default(),
            events,
            ..Default::default()
        },
        |ctx| {
            app.header(ctx);
            app.footer(ctx);
            app.pages_bar(ctx);
            app.side_panels(ctx);
            app.canvas(ctx, false);
            app.settings_dialog(ctx);
            app.command_dialog(ctx);
            app.help_dialog(ctx);
            app.library_dialogs(ctx);
            app.workspace_dialogs(ctx);
            app.pages_dialogs(ctx);
            app.layers_dialog(ctx);
            app.png_dialog(ctx);
            app.mermaid_dialog(ctx);
            app.agent_dialog(ctx);
            app.recovery_dialog(ctx);
        },
    )
}

fn short_dialog_click(ctx: &egui::Context, app: &mut DrawApp, size: Vec2, position: Pos2) {
    for pressed in [true, false] {
        short_dialog_frame(
            ctx,
            app,
            size,
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
}

fn visible_dialog_text(output: &egui::FullOutput, label: &str, screen: Rect) -> bool {
    output.shapes.iter().any(|clipped| {
        if let Shape::Text(text) = &clipped.shape {
            let rect = Rect::from_min_size(text.pos, text.galley.size());
            text.galley.job.text == label
                && screen.contains_rect(rect)
                && clipped.clip_rect.contains(rect.center())
        } else {
            false
        }
    })
}

#[test]
fn shorter_windows_keep_both_panes_usable_and_restore_preferred_widths() {
    for language in Language::ALL {
        for ribbon in [None, Some(false), Some(true)] {
            let (ctx, mut app, id) = panel_setup();
            i18n::activate(language);
            app.show_library = true;
            app.show_inspector = true;
            app.show_quick_toolbar = false;
            app.selected = vec![id];
            app.library_width = 400.;
            app.inspector_width = 400.;
            app.ribbon_expanded = ribbon;
            let before = app.doc.clone();
            stable_panel(&ctx, &mut app, vec2(1600., 960.));
            let original = app.canvas;
            for size in [vec2(1200., 680.), vec2(1280., 720.)] {
                let output = stable_panel(&ctx, &mut app, size);
                let screen = Rect::from_min_size(Pos2::ZERO, size);
                let controls = workspace_panel_icons(output);
                assert!(controls.iter().all(|point| screen.contains(*point)));
                assert!(screen.contains_rect(app.canvas));
                assert!(
                    app.canvas.width() >= 490. && app.canvas.height() > 330.,
                    "{} {size:?}: {:?}",
                    language.code(),
                    app.canvas
                );
                assert_eq!((app.library_width, app.inspector_width), (400., 400.));
            }
            stable_panel(&ctx, &mut app, vec2(1600., 960.));
            assert!((app.canvas.left() - original.left()).abs() < 1.);
            assert!((app.canvas.right() - original.right()).abs() < 1.);
            assert_eq!(app.doc, before);
            assert_eq!(app.selected, vec![id]);
            assert!(!app.history.can_undo());
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn central_dialogs_fit_short_windows_in_seven_languages_without_edits() {
    let mut failures = vec![];
    for language in Language::ALL {
        for size in [vec2(1200., 680.), vec2(1280., 720.)] {
            for (mode, modal, close_label) in [
                ("agents-context", "agents_panel", "Fermer"),
                ("agents-preview", "agents_panel", "Fermer"),
                ("modeles", "template_gallery", "Fermer"),
                ("bibliotheques", "symbol_manager", "Annuler"),
                ("travaux", "saved_projects", "Fermer"),
                ("versions", "saved_projects", "Fermer"),
                ("pages", "page_manager", "Fermer"),
                ("tailles-pages", "page_dimensions", "Annuler"),
                ("settings", "application_settings", "Fermer"),
                ("pdf", "pdf_export", "Annuler"),
                ("png", "png-export", "Annuler"),
                ("mermaid-import", "mermaid-exchange", "Annuler"),
                ("mermaid-export", "mermaid-exchange", "Annuler"),
                ("layers", "organization_layers", "Fermer"),
                ("recovery", "recovery", "Plus tard"),
                ("help", "keyboard_help", "Fermer"),
                ("palette", "command_palette", "Fermer"),
            ] {
                let ctx = egui::Context::default();
                let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
                i18n::activate(language);
                match mode {
                    "agents-context" => app.open_agent_panel(),
                    "agents-preview" => app.capture_agents(),
                    "settings" => app.settings.open = true,
                    "pdf" => app.show_pdf_export = true,
                    "png" => app.open_png_export(),
                    "layers" => app.open_layers(),
                    "recovery" => app.prepare_recovery_capture(),
                    "help" => app.show_help = true,
                    "palette" => app.open_command_palette(),
                    _ => app.capture_tab(mode),
                }
                let before = app.doc.clone();
                let mut output = None;
                for _ in 0..3 {
                    output = Some(short_dialog_frame(&ctx, &mut app, size, vec![]));
                }
                let output = output.unwrap();
                let screen = Rect::from_min_size(Pos2::ZERO, size);
                let rect = ctx
                    .memory(|memory| memory.area_rect(egui::Id::new(modal)))
                    .unwrap_or_else(|| panic!("missing modal: {mode}"));
                if !screen.contains_rect(rect) {
                    failures.push(format!("{} {size:?} {mode}: {rect:?}", language.code()));
                }
                if !visible_dialog_text(&output, &t(close_label), screen) {
                    failures.push(format!(
                        "{} {size:?} {mode}: inaccessible {close_label}",
                        language.code()
                    ));
                }
                assert_eq!(app.doc, before, "{} {mode}", language.code());
            }
        }
    }
    i18n::activate(Language::French);
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn agent_guide_and_apply_action_are_reachable_without_displacing_the_fixed_close_button() {
    for language in Language::ALL {
        let size = vec2(1200., 680.);
        let screen = Rect::from_min_size(Pos2::ZERO, size);
        let ctx = egui::Context::default();
        let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
        i18n::activate(language);
        app.capture_agents();
        let before = app.doc.clone();
        let mut out = short_dialog_frame(&ctx, &mut app, size, vec![]);
        for _ in 0..2 {
            out = short_dialog_frame(&ctx, &mut app, size, vec![]);
        }
        let close = ux_text(&out, &t("Fermer"));
        let guide = ux_text(&out, &t("CLI et connexion MCP"));
        short_dialog_click(&ctx, &mut app, size, guide);
        for _ in 0..3 {
            out = short_dialog_frame(&ctx, &mut app, size, vec![]);
        }
        let rect = ctx
            .memory(|memory| memory.area_rect(egui::Id::new("agents_panel")))
            .unwrap();
        assert!(screen.contains_rect(rect), "{}: {rect:?}", language.code());
        assert!(visible_dialog_text(
            &out,
            &t("Vos outils, votre modèle"),
            screen
        ));
        assert!(visible_dialog_text(&out, &t("Fermer"), screen));
        assert!((ux_text(&out, &t("Fermer")) - close).length() < 1.);
        let preview = ux_text(&out, &t("2  Examiner le résultat"));
        short_dialog_click(&ctx, &mut app, size, preview);
        for _ in 0..3 {
            out = short_dialog_frame(&ctx, &mut app, size, vec![]);
        }
        let rect = ctx
            .memory(|memory| memory.area_rect(egui::Id::new("agents_panel")))
            .unwrap();
        let hover = rect.center() + vec2(160., 0.);
        short_dialog_frame(
            &ctx,
            &mut app,
            size,
            vec![
                egui::Event::PointerMoved(hover),
                wheel(vec2(0., -800.), egui::Modifiers::NONE),
            ],
        );
        for _ in 0..12 {
            out = short_dialog_frame(&ctx, &mut app, size, vec![]);
        }
        assert!(
            visible_dialog_text(&out, &t("Appliquer les modifications"), screen),
            "{}",
            language.code()
        );
        assert!(visible_dialog_text(&out, &t("Fermer"), screen));
        assert!((ux_text(&out, &t("Fermer")) - close).length() < 1.);
        assert_eq!(app.doc, before);
        let apply = ux_text(&out, &t("Appliquer les modifications"));
        short_dialog_click(&ctx, &mut app, size, apply);
        assert!(app.agent_panel.is_none());
        assert_ne!(app.doc, before);
        app.undo();
        assert_eq!(app.doc, before);
    }
    i18n::activate(Language::French);
}
