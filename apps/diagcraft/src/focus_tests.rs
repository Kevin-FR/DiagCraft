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

fn palette_region_command(
    ctx: &egui::Context,
    app: &mut DrawApp,
    backwards: bool,
) -> egui::FullOutput {
    object_key(ctx, app, Key::K, egui::Modifiers::COMMAND);
    assert!(app.command_palette.open);
    object_search_frame(
        ctx,
        app,
        vec![egui::Event::Text(if backwards {
            "zone précédente".into()
        } else {
            "zone suivante".into()
        })],
        vec2(1200., 760.),
    );
    object_key(ctx, app, Key::Enter, egui::Modifiers::NONE);
    assert!(!app.command_palette.open);
    object_search_frame(ctx, app, vec![], vec2(1200., 760.));
    object_search_frame(ctx, app, vec![], vec2(1200., 760.))
}

#[test]
fn palette_navigation_keeps_each_origin_instead_of_using_the_modal_or_no_focus() {
    let labels = [
        "Feuille de dessin",
        "Afficher les propriétés",
        "Pages (1)",
        "Fichier",
        "Réduire la bibliothèque",
    ];
    for origin in 0..5 {
        for backwards in [false, true] {
            let (ctx, mut app, id) = panel_setup();
            ctx.enable_accesskit();
            app.selected = vec![id];
            stable_panel(&ctx, &mut app, vec2(1200., 760.));
            // Establish real canvas focus, then visit every area from it.
            focus_key(&ctx, &mut app, Key::F6, false);
            focus_key(&ctx, &mut app, Key::F6, true);
            for _ in 0..origin {
                focus_key(&ctx, &mut app, Key::F6, false);
            }
            let before = app.doc.clone();
            let out = palette_region_command(&ctx, &mut app, backwards);
            let next = (origin + if backwards { 4 } else { 1 }) % 5;
            assert_eq!(
                focused_label(&out),
                labels[next],
                "origin {origin}, backwards {backwards}"
            );
            assert_eq!(app.doc, before);
            assert_eq!(app.selected, vec![id]);
            assert!(!app.history.can_undo());
        }
    }
}

#[test]
fn palette_navigation_recognizes_fields_and_secondary_buttons_as_their_workspace_area() {
    for source in [
        "title",
        "library_button",
        "file_button",
        "page_button",
        "zoom_button",
    ] {
        for backwards in [false, true] {
            let (ctx, mut app, _) = panel_setup();
            ctx.enable_accesskit();
            app.show_inspector = source == "title";
            stable_panel(&ctx, &mut app, vec2(1200., 760.));
            let (next, previous) = match source {
                "title" => {
                    ctx.memory_mut(|m| m.request_focus(egui::Id::new("inspector_document_title")));
                    panel_frame(
                        &ctx,
                        &mut app,
                        vec2(1200., 760.),
                        vec![egui::Event::Text(" champ".into())],
                    );
                    ("Pages (1)", "Feuille de dessin")
                }
                "library_button" => {
                    focus_key(&ctx, &mut app, Key::F6, true);
                    let out = focus_key(&ctx, &mut app, Key::Tab, false);
                    assert_eq!(focused_label(&out), "Modèles");
                    ("Feuille de dessin", "Fichier")
                }
                "file_button" => {
                    for _ in 0..3 {
                        focus_key(&ctx, &mut app, Key::F6, false);
                    }
                    let out = focus_key(&ctx, &mut app, Key::Tab, false);
                    assert_eq!(focused_label(&out), "Enregistrer · Ctrl/⌘ S");
                    ("Réduire la bibliothèque", "Pages (1)")
                }
                "page_button" => {
                    for _ in 0..2 {
                        focus_key(&ctx, &mut app, Key::F6, false);
                    }
                    let out = focus_key(&ctx, &mut app, Key::Tab, false);
                    assert_eq!(focused_label(&out), "Ajouter une page");
                    ("Fichier", "Afficher les propriétés")
                }
                _ => {
                    focus_key(&ctx, &mut app, Key::F6, false);
                    focus_key(&ctx, &mut app, Key::F6, true);
                    let mut reached = false;
                    for _ in 0..10 {
                        let out = focus_key(&ctx, &mut app, Key::Tab, false);
                        let tree = out.platform_output.accesskit_update.unwrap();
                        if tree.nodes.iter().any(|(id, node)| {
                            *id == tree.focus && node.label() == Some("Réduire le zoom")
                        }) {
                            reached = true;
                            break;
                        }
                    }
                    assert!(reached, "Tab must reach the zoom button");
                    ("Afficher les propriétés", "Réduire la bibliothèque")
                }
            };
            let before = app.doc.clone();
            let undo = app.history.can_undo();
            let out = palette_region_command(&ctx, &mut app, backwards);
            assert_eq!(
                focused_label(&out),
                if backwards { previous } else { next },
                "{source}"
            );
            assert_eq!(app.doc, before);
            assert_eq!(app.history.can_undo(), undo);
        }
    }
}

#[test]
fn cancelling_a_palette_does_not_leave_a_focus_request_or_reuse_its_origin() {
    for cancel in ["escape", "shortcut", "button"] {
        let (ctx, mut app, _) = panel_setup();
        ctx.enable_accesskit();
        stable_panel(&ctx, &mut app, vec2(1200., 760.));
        focus_key(&ctx, &mut app, Key::F6, true);
        object_key(&ctx, &mut app, Key::K, egui::Modifiers::COMMAND);
        object_search_frame(
            &ctx,
            &mut app,
            vec![egui::Event::Text("zone suivante".into())],
            vec2(1200., 760.),
        );
        match cancel {
            "escape" => {
                object_key(&ctx, &mut app, Key::Escape, egui::Modifiers::NONE);
            }
            "shortcut" => {
                object_key(&ctx, &mut app, Key::K, egui::Modifiers::COMMAND);
            }
            _ => {
                object_key(&ctx, &mut app, Key::Tab, egui::Modifiers::SHIFT);
                object_key(&ctx, &mut app, Key::Enter, egui::Modifiers::NONE);
            }
        }
        assert!(!app.command_palette.open);
        for _ in 0..3 {
            object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
        }
        assert!(
            ctx.memory(|m| m.focused()).is_none(),
            "{cancel}: cancellation must not activate a workspace anchor"
        );
        let out = focus_key(&ctx, &mut app, Key::F6, false);
        assert_eq!(focused_label(&out), "Afficher les propriétés");
        let out = palette_region_command(&ctx, &mut app, false);
        assert_eq!(
            focused_label(&out),
            "Pages (1)",
            "{cancel}: a new palette must capture its new origin"
        );
    }
}

#[test]
fn f6_immediately_after_tab_to_a_clipped_library_card_keeps_the_library_origin() {
    for backwards in [false, true] {
        let (ctx, mut app, _) = panel_setup();
        ctx.enable_accesskit();
        ctx.style_mut(|style| style.scroll_animation = egui::style::ScrollAnimation::none());
        app.library_panel = LibraryPanel::Models;
        app.show_quick_toolbar = false;
        stable_panel(&ctx, &mut app, vec2(1200., 760.));
        let title = app.builtin_templates[5].title.clone();
        focus_key(&ctx, &mut app, Key::F6, true);
        let mut reached = false;
        for _ in 0..40 {
            let out = focus_key(&ctx, &mut app, Key::Tab, false);
            let tree = out.platform_output.accesskit_update.unwrap();
            if tree
                .nodes
                .iter()
                .any(|(id, node)| *id == tree.focus && node.label() == Some(title.as_str()))
            {
                reached = true;
                break;
            }
        }
        assert!(reached, "Tab must reach the requested template");
        let response = ctx
            .read_response(ctx.memory(|memory| memory.focused()).unwrap())
            .unwrap();
        assert!(
            response.rect.center().y > app.canvas.bottom(),
            "the stored card response must still be below the viewport"
        );
        let before = app.doc.clone();
        // No idle/repaint frame here: the scroll request has only just been
        // issued and the stored response may still be vertically clipped.
        let out = focus_key(&ctx, &mut app, Key::F6, backwards);
        assert_eq!(
            focused_label(&out),
            if backwards {
                "Fichier"
            } else {
                "Feuille de dessin"
            }
        );
        assert_eq!(app.doc, before);
    }
}

#[test]
fn horizontal_page_clipping_keeps_the_page_bar_origin() {
    for backwards in [false, true] {
        let (ctx, mut app, _) = panel_setup();
        ctx.enable_accesskit();
        for _ in 0..19 {
            app.doc.add_page(false).unwrap();
        }
        app.commit();
        stable_panel(&ctx, &mut app, vec2(1200., 760.));
        focus_key(&ctx, &mut app, Key::F6, false);
        focus_key(&ctx, &mut app, Key::F6, false);
        let mut reached = false;
        for _ in 0..30 {
            let out = focus_key(&ctx, &mut app, Key::Tab, false);
            let tree = out.platform_output.accesskit_update.unwrap();
            if tree
                .nodes
                .iter()
                .any(|(id, node)| *id == tree.focus && node.label() == Some("20 · Page 20"))
            {
                reached = true;
                break;
            }
        }
        assert!(reached);
        let response = ctx
            .read_response(ctx.memory(|memory| memory.focused()).unwrap())
            .unwrap();
        assert!(response.rect.center().x > app.canvas.right());
        let before = app.doc.clone();
        let out = focus_key(&ctx, &mut app, Key::F6, backwards);
        assert_eq!(
            focused_label(&out),
            if backwards {
                "Afficher les propriétés"
            } else {
                "Fichier"
            }
        );
        assert_eq!(app.doc, before);
    }
}
