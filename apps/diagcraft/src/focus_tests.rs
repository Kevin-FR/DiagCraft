use super::*;

fn focus_key(ctx: &egui::Context, app: &mut DrawApp, key: Key, shift: bool) -> egui::FullOutput {
    panel_frame(
        ctx,
        app,
        vec2(1200., 760.),
        vec![command_key(
            key,
            if shift {
                egui::Modifiers::SHIFT
            } else {
                egui::Modifiers::NONE
            },
        )],
    )
}

fn focused_label(output: &egui::FullOutput) -> String {
    let tree = output
        .platform_output
        .accesskit_update
        .as_ref()
        .expect("accessibility tree");
    tree.nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .and_then(|(_, node)| node.label())
        .expect("named focus target")
        .to_owned()
}

#[test]
fn f6_cycles_real_workspace_controls_in_both_directions_without_opening_panes_or_editing() {
    for language in Language::ALL {
        for (library, inspector) in [(true, true), (false, false)] {
            let (ctx, mut app, id) = panel_setup();
            ctx.enable_accesskit();
            i18n::activate(language);
            app.selected = vec![id];
            app.show_library = library;
            app.show_inspector = inspector;
            let before = app.doc.clone();
            stable_panel(&ctx, &mut app, vec2(1200., 760.));
            let expected = [
                t(if inspector {
                    "Masquer les propriétés"
                } else {
                    "Afficher les propriétés"
                }),
                i18n::render("Pages ({0})", &[("0", "1".into())]),
                t("Fichier"),
                t(if library {
                    "Réduire la bibliothèque"
                } else {
                    "Afficher la bibliothèque"
                }),
                t("Feuille de dessin"),
            ];
            for label in &expected {
                let out = focus_key(&ctx, &mut app, Key::F6, false);
                assert_eq!(focused_label(&out), *label, "{}", language.code());
                let r = ctx
                    .read_response(ctx.memory(|m| m.focused()).unwrap())
                    .unwrap();
                assert!(ctx.content_rect().contains_rect(r.rect));
            }
            for label in expected[..4]
                .iter()
                .rev()
                .chain(std::iter::once(&expected[4]))
            {
                let out = focus_key(&ctx, &mut app, Key::F6, true);
                assert_eq!(focused_label(&out), *label);
            }
            assert_eq!((app.show_library, app.show_inspector), (library, inspector));
            assert_eq!(app.selected, vec![id]);
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn f6_anchors_keep_enter_for_the_control_and_canvas_keeps_drawing_shortcuts() {
    let (ctx, mut app, id) = panel_setup();
    ctx.enable_accesskit();
    app.selected = vec![id];
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    let before = app.doc.clone();
    focus_key(&ctx, &mut app, Key::F6, false);
    let opener = ctx.memory(|m| m.focused());
    focus_key(&ctx, &mut app, Key::Enter, false);
    assert!(app.show_inspector, "Enter activates the opener");
    assert!(
        app.rich_editor.is_none(),
        "Enter must not edit the selected block behind a button"
    );
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    assert_eq!(ctx.memory(|m| m.focused()), opener);
    focus_key(&ctx, &mut app, Key::Enter, false);
    assert!(!app.show_inspector);
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    assert_eq!(ctx.memory(|m| m.focused()), opener);
    focus_key(&ctx, &mut app, Key::Enter, false);
    assert!(app.show_inspector);
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    focus_key(&ctx, &mut app, Key::F6, true);
    assert!(app.focus_ui.canvas_focused(&ctx));
    // A focus ring must remain visible on the canvas, and arrow focus traversal
    // must not steal precise positioning from the drawing.
    let out = panel_frame(&ctx, &mut app, vec2(1200., 760.), vec![]);
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, Shape::Rect(r)
        if r.stroke.width == 2. && r.stroke.color == Color32::from_rgb(0, 83, 180)
        && r.rect.width() > 400.)));
    focus_key(&ctx, &mut app, Key::ArrowRight, false);
    panel_frame(
        &ctx,
        &mut app,
        vec2(1200., 760.),
        vec![nudge_key(
            Key::ArrowRight,
            false,
            false,
            egui::Modifiers::NONE,
        )],
    );
    assert!(app.focus_ui.canvas_focused(&ctx));
    assert_eq!(
        app.doc.node(id).unwrap().position[0],
        before.node(id).unwrap().position[0] + 1.
    );
    app.undo();
    assert_eq!(app.doc, before);
    app.selected = vec![id];
    focus_key(&ctx, &mut app, Key::F2, false);
    assert!(app.rich_editor.is_some());
    focus_key(&ctx, &mut app, Key::F6, false);
    assert!(
        app.rich_editor.is_some(),
        "F6 must not validate or discard inline text"
    );
}

#[test]
fn f6_leaves_a_title_field_committing_its_edit_and_tab_still_visits_controls() {
    let (ctx, mut app, _) = panel_setup();
    ctx.enable_accesskit();
    app.show_inspector = true;
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("inspector_document_title")));
    panel_frame(
        &ctx,
        &mut app,
        vec2(1200., 760.),
        vec![egui::Event::Text(" nouveau".into())],
    );
    let title = app.doc.title.clone();
    assert!(title.contains("nouveau"));
    let out = focus_key(&ctx, &mut app, Key::F6, false);
    assert_eq!(focused_label(&out), "Pages (1)");
    let tab = focus_key(&ctx, &mut app, Key::Tab, false);
    assert_eq!(focused_label(&tab), t("Ajouter une page"));
    focus_key(&ctx, &mut app, Key::F6, true);
    assert_eq!(app.doc.title, title);
    app.undo();
    assert!(!app.doc.title.contains("nouveau"));
}

#[test]
fn workspace_focus_commands_are_searchable_and_palette_returns_focus_to_the_real_control() {
    use commands_ui::{Command, results};
    let (ctx, mut app, _) = panel_setup();
    ctx.enable_accesskit();
    object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
    assert_eq!(results("zone suivante")[0].command, Command::NextRegion);
    object_key(&ctx, &mut app, Key::K, egui::Modifiers::COMMAND);
    object_search_frame(
        &ctx,
        &mut app,
        vec![egui::Event::Text("zone suivante".into())],
        vec2(1200., 760.),
    );
    object_key(&ctx, &mut app, Key::F6, egui::Modifiers::NONE);
    assert!(app.command_palette.open, "F6 must not escape a modal");
    object_key(&ctx, &mut app, Key::Enter, egui::Modifiers::NONE);
    assert!(!app.command_palette.open);
    for _ in 0..2 {
        object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
    }
    let out = object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
    assert_eq!(focused_label(&out), t("Afficher les propriétés"));
}

#[test]
fn workspace_icon_names_are_localized_in_the_accessibility_tree() {
    for language in Language::ALL {
        let (ctx, mut app, _) = panel_setup();
        ctx.enable_accesskit();
        i18n::activate(language);
        let out = stable_panel(&ctx, &mut app, vec2(1200., 760.));
        let tree = out.platform_output.accesskit_update.unwrap();
        for key in [
            "Ajouter une page",
            "Nouveau document · Ctrl/⌘ N",
            "Augmenter le zoom",
            "Réduire le zoom",
            "Feuille de dessin",
            "Développer le ruban",
            "Symboles",
        ] {
            assert!(
                tree.nodes
                    .iter()
                    .any(|(_, n)| n.label() == Some(t(key).as_str())),
                "{}: {key}",
                language.code()
            );
        }
        let close = i18n::render("Fermer le document : {0}", &[("0", app.doc.title.clone())]);
        assert!(
            tree.nodes
                .iter()
                .any(|(_, n)| n.label() == Some(close.as_str()))
        );
    }
    i18n::activate(Language::French);
}

#[test]
fn tab_can_choose_a_named_symbol_with_visible_focus_then_f6_places_it_on_the_canvas() {
    let (ctx, mut app, _) = panel_setup();
    ctx.enable_accesskit();
    app.library_panel = LibraryPanel::Symbols;
    app.library_width = 300.;
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    // From the canvas, Shift+F6 reaches the library's stable toggle.
    focus_key(&ctx, &mut app, Key::F6, true);
    let mut found = None;
    for _ in 0..40 {
        let out = focus_key(&ctx, &mut app, Key::Tab, false);
        let tree = out.platform_output.accesskit_update.as_ref().unwrap();
        if tree
            .nodes
            .iter()
            .any(|(id, n)| *id == tree.focus && n.label() == Some("Rectangle"))
        {
            found = Some(out);
            break;
        }
    }
    let out = found.expect("keyboard must reach the first rectangle symbol");
    assert!(out.shapes.iter().any(|s| matches!(&s.shape, Shape::Rect(r)
        if r.stroke.width == 2. && r.stroke.color == Color32::from_rgb(0, 83, 180)
        && (r.rect.width() - 44.).abs() < 0.1)));
    focus_key(&ctx, &mut app, Key::Enter, false);
    assert!(matches!(app.tool, Tool::Shape(ShapeKind::Rectangle)));
    focus_key(&ctx, &mut app, Key::F6, false);
    assert!(app.focus_ui.canvas_focused(&ctx));
    let count = app.doc.nodes.len();
    focus_key(&ctx, &mut app, Key::Enter, false);
    assert_eq!(app.doc.nodes.len(), count + 1);
    assert!(app.rich_editor.is_some());
}

#[test]
fn canvas_focus_keeps_keyboard_context_menu_copy_available() {
    let (ctx, mut app, id) = panel_setup();
    app.selected = vec![id];
    stable_panel(&ctx, &mut app, vec2(1200., 760.));
    focus_key(&ctx, &mut app, Key::F6, false);
    focus_key(&ctx, &mut app, Key::F6, true);
    assert!(app.focus_ui.canvas_focused(&ctx));
    let before = app.doc.clone();
    focus_key(&ctx, &mut app, Key::F10, true);
    assert!(app.context_open);
    let out = panel_frame(
        &ctx,
        &mut app,
        vec2(1200., 760.),
        vec![command_key(Key::C, egui::Modifiers::COMMAND)],
    );
    assert!(out.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text.contains(&app.doc.node(id).unwrap().text))));
    assert_eq!(app.doc, before);
}
