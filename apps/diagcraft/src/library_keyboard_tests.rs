use super::*;

const WINDOW: Vec2 = vec2(1200., 760.);

fn key(ctx: &egui::Context, app: &mut DrawApp, key: Key, shift: bool) -> egui::FullOutput {
    panel_frame(
        ctx,
        app,
        WINDOW,
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

fn focused_name(output: &egui::FullOutput) -> Option<&str> {
    let tree = output.platform_output.accesskit_update.as_ref()?;
    tree.nodes
        .iter()
        .find(|(id, _)| *id == tree.focus)
        .and_then(|(_, node)| node.label())
}

fn tab_to(ctx: &egui::Context, app: &mut DrawApp, label: &str, backwards: bool) {
    for _ in 0..100 {
        key(ctx, app, Key::Tab, backwards);
        // egui applies Shift+Tab's preceding target at the next pass. A real
        // key release allows that transition to finish before testing its name.
        let out = panel_frame(
            ctx,
            app,
            WINDOW,
            vec![nudge_key(
                Key::Tab,
                false,
                false,
                if backwards {
                    egui::Modifiers::SHIFT
                } else {
                    egui::Modifiers::NONE
                },
            )],
        );
        if focused_name(&out) == Some(label) {
            return;
        }
    }
    panic!("Tab did not reach {label}");
}

fn keyboard_library(panel: LibraryPanel) -> (egui::Context, DrawApp) {
    let (ctx, mut app, _) = panel_setup();
    ctx.enable_accesskit();
    ctx.style_mut(|style| style.scroll_animation = egui::style::ScrollAnimation::none());
    app.library_panel = panel;
    app.show_quick_toolbar = false;
    stable_panel(&ctx, &mut app, WINDOW);
    (ctx, app)
}

fn saved_symbols(app: &mut DrawApp, count: u64) -> Node {
    app.library.visible_libraries = vec!["personal".into()];
    let mut source = app.doc.nodes[0].clone();
    source.kind = ShapeKind::Rectangle;
    source.fill = [173, 204, 229];
    source.size = [180., 110.];
    app.library.symbols = (0..count)
        .map(|i| {
            let mut node = source.clone();
            node.id = i + 1;
            node.text = format!("Symbole {} 中文", i + 1);
            node
        })
        .collect();
    source
}

fn assert_visible_focus(ctx: &egui::Context, out: &egui::FullOutput) {
    let id = ctx.memory(|memory| memory.focused()).expect("focused card");
    let response = ctx.read_response(id).unwrap();
    assert!(
        out.shapes
            .iter()
            .any(|shape| matches!(&shape.shape, Shape::Rect(rect)
        if rect.stroke.width == 2.
        && rect.stroke.color == Color32::from_rgb(0, 83, 180)
        && (rect.rect.width() - response.rect.shrink(2.).width()).abs() < 0.1
        && shape.clip_rect.contains_rect(rect.rect))),
        "focused card must be revealed inside its clip: {:?}",
        response.rect
    );
}

fn painted_focus(out: &egui::FullOutput) -> Option<(Rect, Rect)> {
    out.shapes.iter().find_map(|shape| {
        if let Shape::Rect(rect) = &shape.shape
            && rect.stroke.width == 2.
            && rect.stroke.color == Color32::from_rgb(0, 83, 180)
        {
            Some((rect.rect, shape.clip_rect))
        } else {
            None
        }
    })
}

#[test]
fn personal_symbols_and_charts_have_localized_accessible_names_in_seven_languages() {
    for language in Language::ALL {
        let (ctx, mut app) = keyboard_library(LibraryPanel::Symbols);
        i18n::activate(language);
        let mut first = app.doc.nodes[0].clone();
        first.kind = ShapeKind::Rectangle;
        first.text = "Facture 中文".into();
        first.id = 1;
        let mut second = first.clone();
        second.id = 2;
        second.text = "Devis 中文".into();
        app.library.symbols = vec![first, second];
        app.library.visible_libraries = vec!["personal".into()];
        let original = app.doc.clone();
        let out = stable_panel(&ctx, &mut app, WINDOW);
        let tree = out.platform_output.accesskit_update.unwrap();
        for text in ["Facture 中文", "Devis 中文"] {
            let expected = format!("{} · {}", text, t(ShapeKind::Rectangle.label()));
            assert!(
                tree.nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some(expected.as_str())),
                "{}: {expected}",
                language.code()
            );
        }
        app.library_panel = LibraryPanel::Charts;
        let out = stable_panel(&ctx, &mut app, WINDOW);
        let tree = out.platform_output.accesskit_update.unwrap();
        for kind in ChartKind::ALL {
            assert!(
                tree.nodes
                    .iter()
                    .any(|(_, node)| node.label() == Some(t(kind.label()).as_str())),
                "{}: {}",
                language.code(),
                kind.label()
            );
        }
        assert_eq!(app.doc, original);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn tab_reveals_personal_symbols_in_both_directions_and_keeps_saved_contents_when_placing() {
    let (ctx, mut app) = keyboard_library(LibraryPanel::Symbols);
    let source = saved_symbols(&mut app, 64);
    let original = app.doc.clone();
    stable_panel(&ctx, &mut app, WINDOW);
    key(&ctx, &mut app, Key::F6, true);
    tab_to(&ctx, &mut app, "Symbole 64 中文 · Rectangle", false);
    let out = stable_panel(&ctx, &mut app, WINDOW);
    assert_visible_focus(&ctx, &out);
    tab_to(&ctx, &mut app, "Symbole 1 中文 · Rectangle", true);
    let out = stable_panel(&ctx, &mut app, WINDOW);
    assert_visible_focus(&ctx, &out);
    assert_eq!(app.doc, original);
    assert!(!app.history.can_undo());
    key(&ctx, &mut app, Key::Enter, false);
    assert!(matches!(app.tool, Tool::Shape(ShapeKind::Rectangle)));
    key(&ctx, &mut app, Key::F6, false);
    assert!(app.focus_ui.canvas_focused(&ctx));
    key(&ctx, &mut app, Key::Enter, false);
    let placed = app.doc.nodes.last().unwrap();
    assert_eq!(placed.text, "Symbole 1 中文");
    assert_eq!(placed.fill, source.fill);
    assert_eq!(placed.size, source.size);
    assert_ne!(placed.id, source.id);
    key(&ctx, &mut app, Key::Escape, false);
    app.undo();
    assert_eq!(app.doc, original);
}

#[test]
fn offscreen_chart_card_is_revealed_then_enter_inserts_one_undoable_chart() {
    let (ctx, mut app) = keyboard_library(LibraryPanel::Charts);
    let original = app.doc.clone();
    key(&ctx, &mut app, Key::F6, true);
    tab_to(
        &ctx,
        &mut app,
        t(ChartKind::StepArea.label()).as_str(),
        false,
    );
    let out = stable_panel(&ctx, &mut app, WINDOW);
    assert_visible_focus(&ctx, &out);
    assert_eq!(app.doc, original);
    assert!(!app.history.can_undo());
    key(&ctx, &mut app, Key::Enter, false);
    assert_eq!(app.doc.charts.len(), 1);
    assert_eq!(app.doc.charts[0].kind, ChartKind::StepArea);
    assert!(app.show_inspector);
    app.undo();
    assert_eq!(app.doc, original);
}

#[test]
fn offscreen_template_card_is_revealed_and_enter_preserves_the_original_unsaved_tab() {
    let (ctx, mut app) = keyboard_library(LibraryPanel::Models);
    app.doc.title = "Mon dessin modifié".into();
    app.commit();
    let original = app.doc.clone();
    let template = app.builtin_templates.last().unwrap().clone();
    stable_panel(&ctx, &mut app, WINDOW);
    key(&ctx, &mut app, Key::F6, true);
    tab_to(&ctx, &mut app, &template.title, false);
    let out = stable_panel(&ctx, &mut app, WINDOW);
    assert_visible_focus(&ctx, &out);
    assert_eq!(app.doc, original);
    key(&ctx, &mut app, Key::Enter, false);
    assert_eq!(app.doc, template.document);
    assert_eq!(app.tabs.len(), 2);
    app.switch_tab(0);
    assert_eq!(app.doc, original);
    assert!(app.dirty());
}

#[test]
fn focused_card_survives_window_and_pane_reflow_without_fighting_later_pointer_scrolling() {
    let (ctx, mut app) = keyboard_library(LibraryPanel::Symbols);
    saved_symbols(&mut app, 64);
    app.library_width = 400.;
    stable_panel(&ctx, &mut app, WINDOW);
    key(&ctx, &mut app, Key::F6, true);
    tab_to(&ctx, &mut app, "Symbole 64 中文 · Rectangle", false);
    let out = stable_panel(&ctx, &mut app, WINDOW);
    assert_visible_focus(&ctx, &out);
    let focused = ctx.memory(|memory| memory.focused()).unwrap();
    let original = app.doc.clone();
    let smaller = vec2(1200., 680.);
    let out = stable_panel(&ctx, &mut app, smaller);
    assert_eq!(ctx.memory(|memory| memory.focused()), Some(focused));
    assert_visible_focus(&ctx, &out);
    app.library_width = 240.;
    let out = stable_panel(&ctx, &mut app, smaller);
    assert_eq!(ctx.memory(|memory| memory.focused()), Some(focused));
    assert_visible_focus(&ctx, &out);
    let before = painted_focus(&out).unwrap().0;
    let hover = pos2(app.canvas.left() - 100., app.canvas.top() + 140.);
    let mut scrolled = panel_frame(
        &ctx,
        &mut app,
        smaller,
        vec![
            egui::Event::PointerMoved(hover),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: vec2(0., 400.),
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    for _ in 0..30 {
        scrolled = panel_frame(&ctx, &mut app, smaller, vec![]);
    }
    let after = painted_focus(&scrolled);
    assert!(
        after.is_none_or(
            |(rect, clip)| rect.top() > before.top() + 100. && !clip.contains_rect(rect)
        ),
        "manual scrolling must not snap back to the focused card: {before:?} -> {after:?}"
    );
    assert_eq!(focused_name(&scrolled), Some("Symbole 64 中文 · Rectangle"));
    assert_eq!(ctx.memory(|memory| memory.focused()), Some(focused));
    assert_eq!(app.doc, original);
    assert!(!app.history.can_undo());
}
