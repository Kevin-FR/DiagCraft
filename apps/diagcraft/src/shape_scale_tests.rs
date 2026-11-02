use super::*;

fn shape_scale_setup(page_size: Point, zoom: f32) -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let doc = Document {
        page_size,
        ..Default::default()
    };
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.fit_next = false;
    app.zoom = zoom;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.snap = false;
    app.smart_guides = false;
    app.show_library = false;
    app.show_inspector = false;
    app.show_quick_toolbar = false;
    app.ribbon_expanded = Some(false);
    object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
    object_search_frame(&ctx, &mut app, vec![], vec2(1200., 760.));
    (ctx, app)
}

fn shape_scale_click(ctx: &egui::Context, app: &mut DrawApp, point: Point) {
    let pos = app.screen(point);
    assert!(app.canvas.contains(pos));
    for pressed in [true, false] {
        object_search_frame(
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
            vec2(1200., 760.),
        );
    }
}

#[test]
fn a4_insertions_keep_document_scale_for_click_keyboard_and_builtin_symbols() {
    let portrait = diagcraft::pages::presets()
        .into_iter()
        .find(|preset| preset.label.starts_with("A4"))
        .unwrap()
        .size;
    for page_size in [portrait, [portrait[1], portrait[0]]] {
        for zoom in [0.5, 1., 2.] {
            for path in ["click", "keyboard", "library"] {
                let (ctx, mut app) = shape_scale_setup(page_size, zoom);
                for (index, kind) in [ShapeKind::Rectangle, ShapeKind::Rounded, ShapeKind::Diamond]
                    .into_iter()
                    .enumerate()
                {
                    if path == "library" {
                        app.choose_symbol(symbols::preset(kind));
                    } else {
                        app.set_tool(Tool::Shape(kind));
                    }
                    if path == "keyboard" {
                        object_key(&ctx, &mut app, Key::Enter, egui::Modifiers::NONE);
                        assert!(app.rich_editor.is_some());
                        assert!(app.finish_inline_text(&ctx, true));
                    } else {
                        shape_scale_click(&ctx, &mut app, [160. + index as f32 * 180., 160.]);
                    }
                    assert_eq!(app.doc.nodes.len(), index + 1, "{path} at zoom {zoom}");
                    let node = app.doc.nodes.last().unwrap();
                    assert_eq!(
                        node.size,
                        if kind == ShapeKind::Diamond {
                            [160., 120.]
                        } else {
                            [120., 60.]
                        },
                        "{path} at zoom {zoom}"
                    );
                    assert_eq!(node.font_size, 12., "{path} at zoom {zoom}");
                }
                assert_eq!(app.doc.page_size, page_size);
                // Three ordinary process blocks, two 40 px gaps and page margins fit A4.
                let required = app.doc.nodes.iter().map(|node| node.size[0]).sum::<f32>()
                    + 2. * 40.
                    + 2. * 40.;
                assert!(required < page_size[0]);
                app.doc.validate().unwrap();
            }
        }
    }
}

#[test]
fn personal_symbol_dimensions_and_font_survive_click_and_keyboard_at_each_zoom() {
    for zoom in [0.5, 1., 2.] {
        for keyboard in [false, true] {
            let (ctx, mut app) = shape_scale_setup([794., 1123.], zoom);
            let mut source = symbols::preset(ShapeKind::Rounded);
            source.size = [331., 137.];
            source.font_size = 27.;
            source.text = "Mon format personnel".into();
            source.text_style.auto_fit = false;
            app.choose_symbol(source.clone());
            if keyboard {
                object_key(&ctx, &mut app, Key::Enter, egui::Modifiers::NONE);
                assert!(app.finish_inline_text(&ctx, true));
            } else {
                shape_scale_click(&ctx, &mut app, [330., 220.]);
            }
            let placed = app.doc.nodes.last().unwrap();
            assert_eq!(placed.size, source.size);
            assert_eq!(placed.font_size, source.font_size);
            assert_eq!(placed.text, source.text);
            assert_eq!(placed.text_runs, source.text_runs);
            let after = app.doc.clone();
            app.undo();
            assert!(app.doc.nodes.is_empty());
            assert!(!app.history.can_undo());
            app.redo();
            assert_eq!(app.doc, after);
        }
    }
}

#[test]
fn compact_shape_can_be_enlarged_manually_in_one_undo_step() {
    let (ctx, mut app) = shape_scale_setup([794., 1123.], 1.);
    app.set_tool(Tool::Shape(ShapeKind::Rectangle));
    shape_scale_click(&ctx, &mut app, [350., 250.]);
    let id = app.doc.nodes[0].id;
    assert_eq!(app.doc.nodes[0].size, [120., 60.]);
    let before = app.doc.clone();
    let (start, _) = resize_handles(app.selected_node_rect().unwrap())[4];
    panel_pointer(&ctx, &mut app, vec2(1200., 760.), start, true);
    assert!(matches!(app.drag, Some(Drag::Resize { .. })));
    panel_pointer(
        &ctx,
        &mut app,
        vec2(1200., 760.),
        start + vec2(100., 60.),
        false,
    );
    assert_eq!(app.doc.node(id).unwrap().size, [220., 120.]);
    let enlarged = app.doc.clone();
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc, enlarged);
}

#[test]
fn contextual_linked_and_inserted_shapes_keep_compact_defaults_and_wrap() {
    for insert in [false, true] {
        let (ctx, mut app) = shape_scale_setup([794., 1123.], 1.);
        let a = app.doc.add_node(ShapeKind::Rectangle, [180., 300.]);
        let b = app.doc.add_node(ShapeKind::Rectangle, [650., 300.]);
        let edge = app.doc.connect(a, b, ConnectorKind::Straight).unwrap();
        app.commit();
        app.selected = vec![if insert { edge } else { a }];
        app.context_position = [420., 300.];
        let before = app.doc.clone();
        app.perform_action(
            if insert {
                context_menu::Action::Insert(ShapeKind::Rounded)
            } else {
                context_menu::Action::Linked(ShapeKind::Rounded, Port::Right)
            },
            &ctx,
        );
        assert!(app.error.is_none());
        let node = app.doc.node(app.selected[0]).unwrap();
        assert_eq!(node.size, [120., 60.]);
        assert_eq!(node.font_size, 12.);
        assert!(node.text_style.wrap);
        assert_eq!(app.doc.nodes.len(), 3);
        app.undo();
        assert_eq!(app.doc, before);
    }
}
