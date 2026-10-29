use super::*;

const WINDOW: Vec2 = vec2(1200., 680.);

fn map_setup() -> (egui::Context, DrawApp, u64) {
    let (ctx, mut app, id) = panel_setup();
    ctx.enable_accesskit();
    app.doc.page_size = [4000., 2200.];
    app.show_minimap = true;
    app.show_quick_toolbar = false;
    app.selected = vec![id];
    app.history = History::new(&app.doc);
    stable_panel(&ctx, &mut app, WINDOW);
    app.zoom = 1.5;
    app.pan = app.canvas.size() / 2. - vec2(300., 250.) * app.zoom;
    app.fit_next = false;
    stable_panel(&ctx, &mut app, WINDOW);
    (ctx, app, id)
}

fn close_enough(actual: Point, expected: Point) {
    assert!(
        (actual[0] - expected[0]).abs() < 0.2 && (actual[1] - expected[1]).abs() < 0.2,
        "world position {actual:?}, expected {expected:?}"
    );
}

fn map_response(ctx: &egui::Context) -> egui::Response {
    ctx.read_response(egui::Id::new(minimap_ui::MAP_ID))
        .expect("the mini-map must have a real response")
}

fn map_label(output: &egui::FullOutput) -> Option<String> {
    output
        .platform_output
        .accesskit_update
        .as_ref()
        .and_then(|tree| {
            tree.nodes
                .iter()
                .find(|(id, _)| *id == tree.focus)
                .and_then(|(_, node)| node.label())
                .map(str::to_owned)
        })
}

#[test]
fn minimap_preferences_are_local_compatible_and_reset_without_discarding_history() {
    let (ctx, mut app, id) = panel_setup();
    assert!(!app.show_minimap);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("interface-v1.json");
    app.interface_path = Some(path.clone());
    app.show_minimap = true;
    app.save_interface().unwrap();
    let (_, mut reopened, _) = panel_setup();
    reopened.interface_path = Some(path.clone());
    reopened.restore_interface();
    assert!(reopened.show_minimap);
    app.doc.node_mut(id).unwrap().text = "Keep this edit".into();
    app.commit();
    let before = app.doc.clone();
    app.selected = vec![id];
    app.run_command(commands_ui::Command::ResetInterface, &ctx);
    assert!(!app.show_minimap);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    app.undo();
    assert_ne!(app.doc, before);

    // The old preference schema is still valid and keeps the uncluttered default.
    std::fs::write(&path, br#"{"version":1,"library":false}"#).unwrap();
    reopened.restore_interface();
    assert!(!reopened.show_minimap);
    assert!(!reopened.show_library);
    assert_eq!(
        std::fs::read(&path).unwrap(),
        br#"{"version":1,"library":false}"#
    );
}

#[test]
fn minimap_click_in_seven_languages_only_changes_the_camera_and_keeps_redo() {
    for language in Language::ALL {
        let (ctx, mut app, id) = map_setup();
        i18n::activate(language);
        app.doc.title = "Earlier change".into();
        app.commit();
        let before = app.doc.clone();
        app.doc.title = "Redo branch".into();
        app.commit();
        let redo = app.doc.clone();
        app.undo();
        app.selected = vec![id];
        app.zoom = 1.5;
        app.pan = app.canvas.size() / 2. - vec2(300., 250.) * app.zoom;
        app.fit_next = false;
        stable_panel(&ctx, &mut app, WINDOW);
        let zoom = app.zoom;
        let transform = app.minimap_transform().unwrap();
        let target = [3200., 1760.];
        let point = transform.project(target);
        assert!(map_response(&ctx).rect.contains(point));
        panel_click(&ctx, &mut app, WINDOW, point);
        close_enough(app.world(app.canvas.center()), target);
        assert_eq!(app.zoom, zoom);
        assert_eq!(app.selected, vec![id]);
        assert_eq!(app.doc, before);
        assert!(app.history.can_redo());
        app.redo();
        assert_eq!(app.doc, redo);
    }
    i18n::activate(Language::French);
}

#[test]
fn minimap_drag_keeps_its_grab_offset_projection_and_captures_release_outside() {
    let (ctx, mut app, id) = map_setup();
    let before = app.doc.clone();
    app.zoom = 1.3;
    app.pan = app.canvas.size() / 2. - vec2(1400., 1000.) * app.zoom;
    stable_panel(&ctx, &mut app, WINDOW);
    let transform = app.minimap_transform().unwrap();
    let start = transform.project([1400., 1000.]) + vec2(2., 1.);
    panel_pointer(&ctx, &mut app, WINDOW, start, true);
    close_enough(app.world(app.canvas.center()), [1400., 1000.]);
    let end = transform.project([1800., 1160.]) + vec2(2., 1.);
    panel_frame(&ctx, &mut app, WINDOW, vec![egui::Event::PointerMoved(end)]);
    close_enough(app.world(app.canvas.center()), [1800., 1160.]);
    assert!(app.minimap_ui.is_dragging());
    assert_eq!(app.minimap_transform().unwrap().world, transform.world);
    assert_eq!(app.zoom, 1.3);
    let outside = app.canvas.right_top() + vec2(20., -20.);
    panel_pointer(&ctx, &mut app, WINDOW, outside, false);
    assert!(!app.minimap_ui.is_dragging());
    let pan = app.pan;
    let after_release = outside + vec2(50., 30.);
    panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![egui::Event::PointerMoved(after_release)],
    );
    assert_eq!(app.pan, pan);
    assert_eq!(app.doc, before);
    assert_eq!(app.selected, vec![id]);
    assert!(!app.history.can_undo());
    assert!(app.drag.is_none());
}

#[test]
fn minimap_keyboard_pans_and_fits_without_nudging_the_selected_object() {
    for language in Language::ALL {
        let (ctx, mut app, id) = map_setup();
        i18n::activate(language);
        let before = app.doc.clone();
        let point = app.minimap_transform().unwrap().project([2000., 1100.]);
        panel_click(&ctx, &mut app, WINDOW, point);
        let output = stable_panel(&ctx, &mut app, WINDOW);
        assert_eq!(
            map_label(&output).as_deref(),
            Some(t("Naviguer dans la mini-carte").as_str())
        );
        let original = app.world(app.canvas.center());
        let step_x = app.canvas.width() / app.zoom * 0.1;
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![command_key(Key::ArrowRight, egui::Modifiers::NONE)],
        );
        close_enough(
            app.world(app.canvas.center()),
            [original[0] + step_x, original[1]],
        );
        let old = app.world(app.canvas.center());
        let step_y = app.canvas.height() / app.zoom * 0.25;
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![command_key(Key::ArrowDown, egui::Modifiers::SHIFT)],
        );
        close_enough(app.world(app.canvas.center()), [old[0], old[1] + step_y]);
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![command_key(Key::Home, egui::Modifiers::NONE)],
        );
        stable_panel(&ctx, &mut app, WINDOW);
        close_enough(app.world(app.canvas.center()), [2000., 1100.]);
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![id]);
        assert!(!app.history.can_undo());
        assert!(app.keyboard_move.is_none());
    }
    i18n::activate(Language::French);
}

#[test]
fn minimap_opening_frame_intercepts_drawing_tools_space_middle_and_trackpad() {
    for tool in [Tool::Shape(ShapeKind::Rectangle), Tool::Connect] {
        let (ctx, mut app, _) = map_setup();
        app.set_tool(tool);
        app.show_minimap = false;
        stable_panel(&ctx, &mut app, WINDOW);
        app.show_minimap = true;
        let point = app.minimap_transform().unwrap().project([3000., 1500.]);
        let before = app.doc.clone();
        panel_pointer(&ctx, &mut app, WINDOW, point, true);
        panel_pointer(&ctx, &mut app, WINDOW, point + vec2(8., 5.), false);
        assert_eq!(app.doc, before);
        assert!(app.drag.is_none());
        assert!(app.connect_from.is_none());
        assert!(!app.history.can_undo());
    }
    for button in [egui::PointerButton::Primary, egui::PointerButton::Middle] {
        let (ctx, mut app, _) = map_setup();
        let point = app.minimap_transform().unwrap().map.center();
        let before = app.doc.clone();
        let events = |pressed| {
            vec![
                egui::Event::PointerMoved(point),
                nudge_key(Key::Space, pressed, false, egui::Modifiers::NONE),
                egui::Event::PointerButton {
                    pos: point,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        panel_frame(&ctx, &mut app, WINDOW, events(true));
        assert!(
            app.drag.is_none(),
            "the canvas must not start panning underneath the mini-map"
        );
        panel_frame(&ctx, &mut app, WINDOW, events(false));
        let pan = app.pan;
        let zoom = app.zoom;
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![
                egui::Event::PointerMoved(point),
                wheel(vec2(100., 60.), egui::Modifiers::NONE),
                egui::Event::Zoom(1.2),
            ],
        );
        assert_eq!(app.pan, pan);
        assert_eq!(app.zoom, zoom);
        assert_eq!(app.doc, before);
        assert!(app.drag.is_none());
    }
}

fn has_map_title(output: &egui::FullOutput) -> bool {
    output.shapes.iter().any(|clipped| {
        matches!(&clipped.shape, Shape::Text(text) if text.galley.job.text == t("Mini-carte"))
    })
}

#[test]
fn minimap_hides_during_inline_edit_modal_and_canvas_drag_without_stealing_release() {
    let (ctx, mut app, id) = map_setup();
    app.open_node_text(id);
    let before = app.doc.clone();
    let output = stable_panel(&ctx, &mut app, WINDOW);
    assert!(!has_map_title(&output));
    assert!(app.panel_rect().is_none());
    assert_eq!(app.doc, before);
    app.rich_editor = None;
    app.show_help = true;
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, WINDOW)),
            ..Default::default()
        },
        |ctx| {
            app.header(ctx);
            app.footer(ctx);
            app.pages_bar(ctx);
            app.side_panels(ctx);
            app.canvas(ctx, false);
            app.help_dialog(ctx);
        },
    );
    assert!(!has_map_title(&output));
    assert_eq!(app.doc, before);
    app.show_help = false;
    app.zoom = 1.;
    app.pan = app.canvas.size() / 2. - vec2(500., 400.);
    app.fit_next = false;
    stable_panel(&ctx, &mut app, WINDOW);
    let map_point = app.minimap_transform().unwrap().map.center();
    let source = app.screen(app.doc.node(id).unwrap().center());
    panel_pointer(&ctx, &mut app, WINDOW, source, true);
    assert!(app.drag.is_some());
    let pan = app.pan;
    let output = panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![egui::Event::PointerMoved(map_point)],
    );
    assert!(!has_map_title(&output));
    assert!(!app.minimap_ui.is_dragging());
    assert_eq!(app.pan, pan);
    panel_pointer(&ctx, &mut app, WINDOW, map_point, false);
    assert!(
        app.drag.is_none(),
        "a document drag released over the old mini-map location must end"
    );
    assert_ne!(
        app.doc, before,
        "the document gesture must keep running across the hidden map"
    );
    app.undo();
    assert_eq!(app.doc, before);
    assert!(has_map_title(&stable_panel(&ctx, &mut app, WINDOW)));
}

fn overview_shapes(output: &egui::FullOutput, map: Rect) -> Vec<Shape> {
    let clip = map.expand(2.);
    output
        .shapes
        .iter()
        .filter_map(|clipped| (clipped.clip_rect == clip).then_some(clipped.shape.clone()))
        .collect()
}

#[test]
fn minimap_excludes_hidden_objects_and_attached_links_while_locked_objects_stay_visible() {
    let (ctx, mut app, source) = map_setup();
    let second = app.doc.add_node(ShapeKind::Rectangle, [950., 350.]);
    app.doc
        .connect(source, second, ConnectorKind::Elbow)
        .unwrap();
    app.doc.set_objects_locked(&[second], true).unwrap();
    app.doc.add_chart(ChartKind::Columns, [1700., 600.]);
    app.doc
        .add_drawing(DrawingKind::Line, [1800., 900.], [2500., 1300.]);
    app.history = History::new(&app.doc);
    let output = stable_panel(&ctx, &mut app, WINDOW);
    let transform = app.minimap_transform().unwrap();
    let baseline = overview_shapes(&output, transform.map);
    assert!(
        baseline.len() >= 7,
        "the actual miniature must paint the page, boxes, paths and viewport"
    );
    let locked_center = transform.project(app.doc.node(second).unwrap().center());
    assert!(baseline.iter().any(|shape| matches!(shape, Shape::Rect(rect)
        if rect.rect.center().distance(locked_center) < 0.1 && rect.rect.width() < transform.map.width()/4.)));

    let layer = app.doc.create_layer("Hidden fixture".into()).unwrap();
    let hidden = app.doc.add_node(ShapeKind::Diamond, [2500., 1700.]);
    app.doc.node_mut(hidden).unwrap().position = [9000., 9000.];
    let drawing = app
        .doc
        .add_drawing(DrawingKind::Line, [10_000., 10_000.], [15_000., 15_000.]);
    app.doc
        .connect(source, hidden, ConnectorKind::Elbow)
        .unwrap();
    app.doc
        .move_objects_to_layer(&[hidden, drawing], layer)
        .unwrap();
    app.doc
        .update_layer(layer, None, Some(false), None)
        .unwrap();
    app.history = History::new(&app.doc);
    let before = app.doc.clone();
    let output = stable_panel(&ctx, &mut app, WINDOW);
    assert_eq!(app.minimap_transform().unwrap(), transform);
    assert_eq!(overview_shapes(&output, transform.map), baseline);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    app.doc.update_layer(layer, None, Some(true), None).unwrap();
    let output = stable_panel(&ctx, &mut app, WINDOW);
    let expanded = app.minimap_transform().unwrap();
    assert!(expanded.world.right() > 15_000. && expanded.world.bottom() > 15_000.);
    assert_ne!(overview_shapes(&output, expanded.map), baseline);
}

#[test]
fn minimap_projection_and_controls_stay_bounded_on_extreme_pages_and_short_windows() {
    for language in Language::ALL {
        for page in [
            [100., 100.],
            [100_000., 100.],
            [100., 100_000.],
            [100_000., 100_000.],
        ] {
            let ctx = egui::Context::default();
            let doc = Document {
                page_size: page,
                ..Default::default()
            };
            doc.validate().unwrap();
            let mut app = DrawApp::from_context(&ctx, doc, None, None);
            i18n::activate(language);
            app.show_minimap = true;
            app.show_library = true;
            app.show_inspector = true;
            app.library_width = 400.;
            app.inspector_width = 400.;
            let before = app.doc.clone();
            for size in [vec2(1600., 960.), WINDOW, vec2(1280., 720.)] {
                let output = stable_panel(&ctx, &mut app, size);
                let panel = app.panel_rect().unwrap();
                assert!(
                    app.canvas.contains_rect(panel),
                    "{}: {panel:?} / {:?}",
                    language.code(),
                    app.canvas
                );
                let map = app.minimap_transform().unwrap();
                assert!(map.map.is_finite() && map.map.is_positive());
                assert!(panel.contains_rect(map.map));
                assert!(
                    (map.map.width() / map.world.width() - map.map.height() / map.world.height())
                        .abs()
                        < 0.0001
                );
                assert!(has_map_title(&output));
                for clipped in &output.shapes {
                    if let Shape::Text(text) = &clipped.shape
                        && text.galley.job.text.ends_with(" %")
                        && text.pos.y > app.canvas.bottom() - 65.
                    {
                        assert!(
                            !panel.intersects(Rect::from_min_size(text.pos, text.galley.size()))
                        );
                    }
                }
            }
            let transform = app.minimap_transform().unwrap();
            app.zoom = 2.5;
            app.pan = vec2(1_000_000., 1_000_000.);
            app.fit_next = false;
            let output = stable_panel(&ctx, &mut app, vec2(1280., 720.));
            assert_eq!(
                app.minimap_transform().unwrap(),
                transform,
                "off-page panning must not rescale the overview"
            );
            let shapes = overview_shapes(&output, transform.map);
            assert!(
                shapes
                    .iter()
                    .any(|shape| matches!(shape, Shape::Circle(circle)
                if circle.radius==3. && transform.map.contains(circle.center)))
            );
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn minimap_page_switch_cancels_drag_without_panning_or_editing_the_new_page() {
    let (ctx, mut app, _) = map_setup();
    app.doc.add_page(false).unwrap();
    app.doc.page_size = [700., 1800.];
    app.switch_page(0);
    app.history = History::new(&app.doc);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = app.canvas.size() / 2. - vec2(1400., 1000.);
    stable_panel(&ctx, &mut app, WINDOW);
    let transform = app.minimap_transform().unwrap();
    let start = transform.project([1400., 1000.]);
    let moved = transform.project([2000., 1200.]);
    panel_pointer(&ctx, &mut app, WINDOW, start, true);
    panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![egui::Event::PointerMoved(moved)],
    );
    assert!(app.minimap_ui.is_dragging());
    app.switch_page(1);
    app.fit_next = false;
    app.pan = vec2(23., 41.);
    let before = app.doc.clone();
    let output = panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![egui::Event::PointerMoved(moved + vec2(40., 20.))],
    );
    assert!(has_map_title(&output));
    assert!(!app.minimap_ui.is_dragging());
    assert_eq!(app.pan, vec2(23., 41.));
    panel_pointer(&ctx, &mut app, WINDOW, moved, false);
    assert_eq!(app.pan, vec2(23., 41.));
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());
    let next = app.minimap_transform().unwrap();
    assert_ne!(next.world, transform.world);
    assert!(
        next.world.width() < next.world.height(),
        "the overview must use the active portrait page"
    );
}

#[test]
fn minimap_opening_secondary_and_middle_clicks_do_not_leak_after_leaving() {
    for button in [egui::PointerButton::Secondary, egui::PointerButton::Middle] {
        let (ctx, mut app, id) = map_setup();
        app.show_minimap = false;
        stable_panel(&ctx, &mut app, WINDOW);
        app.show_minimap = true;
        let edge = app.panel_rect().unwrap().left_center() + vec2(1., 0.);
        let before = app.doc.clone();
        let pan = app.pan;
        let click = |position, pressed| {
            vec![
                egui::Event::PointerMoved(position),
                egui::Event::PointerButton {
                    pos: position,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        panel_frame(&ctx, &mut app, WINDOW, click(edge, true));
        panel_frame(&ctx, &mut app, WINDOW, click(edge - vec2(3., 0.), false));
        assert!(
            !app.context_open,
            "a click started in the opening mini-map must not open the canvas context"
        );
        assert!(
            app.drag.is_none(),
            "a middle click started in the opening mini-map must not pan the canvas"
        );
        assert_eq!(app.pan, pan);
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![id]);
        assert!(!app.history.can_undo());
    }
}

#[test]
fn minimap_close_returns_focus_to_its_opener_and_enter_can_reopen_it() {
    for language in Language::ALL {
        let (ctx, mut app, id) = map_setup();
        i18n::activate(language);
        let before = app.doc.clone();
        let output = stable_panel(&ctx, &mut app, WINDOW);
        let panel = app.panel_rect().unwrap();
        let close = output
            .shapes
            .iter()
            .find_map(|clipped| {
                if let Shape::Text(text) = &clipped.shape {
                    let rect = Rect::from_min_size(text.pos, text.galley.size());
                    (text.galley.job.text == "×" && panel.contains_rect(rect))
                        .then_some(rect.center())
                } else {
                    None
                }
            })
            .expect("a visible close control belongs to the mini-map");
        panel_click(&ctx, &mut app, WINDOW, close);
        assert!(!app.show_minimap);
        let output = stable_panel(&ctx, &mut app, WINDOW);
        assert!(!has_map_title(&output));
        assert_eq!(
            map_label(&output).as_deref(),
            Some(t("Afficher la mini-carte").as_str())
        );
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![command_key(Key::Enter, egui::Modifiers::NONE)],
        );
        assert!(
            app.show_minimap,
            "Enter on the returned focus must reopen the mini-map"
        );
        let output = stable_panel(&ctx, &mut app, WINDOW);
        assert!(has_map_title(&output));
        panel_frame(
            &ctx,
            &mut app,
            WINDOW,
            vec![command_key(Key::ArrowRight, egui::Modifiers::NONE)],
        );
        assert_eq!(app.doc, before);
        assert_eq!(app.selected, vec![id]);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn minimap_keyboard_close_stays_closed_until_the_activation_key_is_released() {
    for language in Language::ALL {
        for activation in [Key::Enter, Key::Space] {
            for batched_release in [false, true] {
                let (ctx, mut app, id) = map_setup();
                i18n::activate(language);
                let point = app.minimap_transform().unwrap().project([2000., 1100.]);
                panel_click(&ctx, &mut app, WINDOW, point);
                let output = stable_panel(&ctx, &mut app, WINDOW);
                assert_eq!(
                    map_label(&output).as_deref(),
                    Some(t("Naviguer dans la mini-carte").as_str())
                );
                panel_frame(
                    &ctx,
                    &mut app,
                    WINDOW,
                    vec![nudge_key(Key::Tab, true, false, egui::Modifiers::SHIFT)],
                );
                let output = panel_frame(
                    &ctx,
                    &mut app,
                    WINDOW,
                    vec![nudge_key(Key::Tab, false, false, egui::Modifiers::NONE)],
                );
                assert_eq!(
                    map_label(&output).as_deref(),
                    Some(t("Masquer la mini-carte").as_str())
                );
                let focused = ctx
                    .read_response(ctx.memory(|m| m.focused()).unwrap())
                    .unwrap();
                assert!(
                    app.panel_rect().unwrap().contains_rect(focused.rect),
                    "Shift+Tab must reach the overview header Close, not its toolbar opener"
                );
                let before = app.doc.clone();
                let pan = app.pan;
                let zoom = app.zoom;
                let press = |repeat| {
                    let mut events =
                        vec![nudge_key(activation, true, repeat, egui::Modifiers::NONE)];
                    if activation == Key::Space {
                        events.push(egui::Event::Text(" ".into()));
                    }
                    events
                };
                let mut closing_events = press(false);
                if batched_release {
                    closing_events.push(nudge_key(activation, false, false, egui::Modifiers::NONE));
                }
                panel_frame(&ctx, &mut app, WINDOW, closing_events);
                assert!(
                    !app.show_minimap,
                    "{} {activation:?}: activating the header Close must not reopen through the returned focus",
                    language.code()
                );
                for _ in 0..3 {
                    let output = panel_frame(
                        &ctx,
                        &mut app,
                        WINDOW,
                        if batched_release { vec![] } else { press(true) },
                    );
                    assert!(
                        !app.show_minimap,
                        "{} {activation:?}: repeats from the closing key must not activate the opener",
                        language.code()
                    );
                    assert!(!has_map_title(&output));
                    assert_eq!(
                        map_label(&output).as_deref(),
                        Some(t("Afficher la mini-carte").as_str())
                    );
                }
                panel_frame(
                    &ctx,
                    &mut app,
                    WINDOW,
                    if batched_release {
                        vec![]
                    } else {
                        vec![nudge_key(activation, false, false, egui::Modifiers::NONE)]
                    },
                );
                assert!(
                    !app.show_minimap,
                    "releasing the closing key must keep the overview hidden"
                );
                panel_frame(&ctx, &mut app, WINDOW, press(false));
                assert!(
                    app.show_minimap,
                    "a fresh activation after release must reopen the overview"
                );
                panel_frame(
                    &ctx,
                    &mut app,
                    WINDOW,
                    vec![nudge_key(activation, false, false, egui::Modifiers::NONE)],
                );
                assert_eq!(app.pan, pan);
                assert_eq!(app.zoom, zoom);
                assert_eq!(app.doc, before);
                assert_eq!(app.selected, vec![id]);
                assert!(!app.history.can_undo());
            }
        }
    }
    i18n::activate(Language::French);
}
