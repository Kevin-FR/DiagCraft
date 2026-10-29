use super::*;

const WINDOW: Vec2 = vec2(1200., 680.);
const MIN_TEXT_CONTRAST: f64 = 4.5;

fn luminance(rgb: [f64; 3]) -> f64 {
    rgb.into_iter()
        .zip([0.2126, 0.7152, 0.0722])
        .map(|(value, weight)| {
            weight
                * if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
        })
        .sum()
}

fn ratio(a: [f64; 3], b: [f64; 3]) -> f64 {
    let a = luminance(a);
    let b = luminance(b);
    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

fn composite(color: Color32, opacity: f32, background: [f64; 3]) -> [f64; 3] {
    // Color32 stores premultiplied sRGBA; the painter can additionally fade a
    // TextShape. Measure that output against the fills painted behind it.
    let rgba = color.to_array();
    let opacity = f64::from(opacity);
    let alpha = f64::from(rgba[3]) / 255. * opacity;
    [0, 1, 2].map(|axis| {
        (f64::from(rgba[axis]) / 255. * opacity + background[axis] * (1. - alpha)).clamp(0., 1.)
    })
}

fn text_shape<'a>(
    output: &'a egui::FullOutput,
    label: &str,
) -> (usize, &'a egui::epaint::TextShape) {
    output
        .shapes
        .iter()
        .enumerate()
        .find_map(|(index, clipped)| {
            if let Shape::Text(text) = &clipped.shape {
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                (text.galley.job.text == label && rect.intersect(clipped.clip_rect).is_positive())
                    .then_some((index, text))
            } else {
                None
            }
        })
        .unwrap_or_else(|| panic!("missing painted text: {label}"))
}

fn background_at(output: &egui::FullOutput, before: usize, position: Pos2) -> [f64; 3] {
    let mut background = [1., 1., 1.];
    for clipped in &output.shapes[..before] {
        if let Shape::Rect(rect) = &clipped.shape
            && rect.rect.contains(position)
            && clipped.clip_rect.contains(position)
            && rect.fill.a() > 0
        {
            background = composite(rect.fill, 1., background);
        }
    }
    background
}

fn painted_colors(output: &egui::FullOutput, label: &str) -> ([f64; 3], Vec<[f64; 3]>) {
    let (index, text) = text_shape(output, label);
    painted_colors_at(output, index, text)
}

fn painted_colors_at(
    output: &egui::FullOutput,
    index: usize,
    text: &egui::epaint::TextShape,
) -> ([f64; 3], Vec<[f64; 3]>) {
    let visible =
        Rect::from_min_size(text.pos, text.galley.size()).intersect(output.shapes[index].clip_rect);
    let background = background_at(output, index, visible.center());
    let colors = text
        .galley
        .rows
        .iter()
        .flat_map(|row| {
            let visuals = &row.visuals;
            visuals.mesh.vertices[visuals.glyph_vertex_range.clone()].iter()
        })
        .map(|vertex| {
            let color = text.override_text_color.unwrap_or_else(|| {
                if vertex.color == Color32::PLACEHOLDER {
                    text.fallback_color
                } else {
                    vertex.color
                }
            });
            composite(color, text.opacity_factor, background)
        })
        .collect::<Vec<_>>();
    assert!(
        !colors.is_empty(),
        "text must have painted glyphs: {}",
        text.galley.job.text
    );
    (background, colors)
}

fn assert_text_contrast(output: &egui::FullOutput, label: &str) {
    let (background, colors) = painted_colors(output, label);
    for color in colors {
        let contrast = ratio(color, background);
        assert!(
            contrast >= MIN_TEXT_CONTRAST,
            "{label}: foreground={color:?}, background={background:?}, ratio={contrast}"
        );
    }
}

#[test]
fn active_status_field_labels_and_library_help_are_readable_in_seven_languages() {
    for language in Language::ALL {
        let (ctx, mut app, _) = panel_setup();
        i18n::activate(language);
        app.show_inspector = true;
        app.library_panel = LibraryPanel::Projects;
        let before = app.doc.clone();
        let status = t(&app.status);
        let output = stable_panel(&ctx, &mut app, WINDOW);
        for label in [
            status,
            t("NOM DU DIAGRAMME"),
            t("Sélectionnez un objet pour modifier son texte, ses couleurs et sa taille."),
            t("Enregistrez un dessin avec Ctrl/⌘ S pour l’ajouter à la galerie."),
        ] {
            assert_text_contrast(&output, &label);
        }
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn actual_search_placeholders_remain_readable_before_typing_in_seven_languages() {
    for language in Language::ALL {
        let (ctx, mut app, _) = panel_setup();
        i18n::activate(language);
        let before = app.doc.clone();
        let output = stable_panel(&ctx, &mut app, WINDOW);
        assert_text_contrast(&output, &t("Rechercher…"));
        ctx.style_mut(|style| style.animation_time = 0.);
        app.open_object_search();
        let mut output = object_search_frame(&ctx, &mut app, vec![], WINDOW);
        for _ in 0..2 {
            output = object_search_frame(&ctx, &mut app, vec![], WINDOW);
        }
        assert_text_contrast(&output, &t("Texte, type ou #identifiant…"));
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn object_result_details_keep_contrast_in_normal_hover_selected_and_focused_states() {
    for language in Language::ALL {
        let (ctx, mut app, _) = panel_setup();
        ctx.enable_accesskit();
        i18n::activate(language);
        app.doc.add_node(ShapeKind::Rectangle, [600., 500.]);
        app.history = History::new(&app.doc);
        let before = app.doc.clone();
        ctx.style_mut(|style| style.animation_time = 0.);
        app.open_object_search();
        let mut output = object_search_frame(&ctx, &mut app, vec![], WINDOW);
        for _ in 0..2 {
            output = object_search_frame(&ctx, &mut app, vec![], WINDOW);
        }
        let selected = app.object_search.entries[0].detail.clone();
        let detail = app.object_search.entries[1].detail.clone();
        let accessible = format!("{} · {}", app.object_search.entries[1].label, detail);
        assert_text_contrast(&output, &selected);
        assert_text_contrast(&output, &detail);
        let (_, text) = text_shape(&output, &detail);
        let hover = text.pos + text.galley.size() / 2.;
        output = object_search_frame(
            &ctx,
            &mut app,
            vec![egui::Event::PointerMoved(hover)],
            WINDOW,
        );
        assert_text_contrast(&output, &detail);
        let mut focused = false;
        for _ in 0..40 {
            output = object_search_frame(
                &ctx,
                &mut app,
                vec![command_key(Key::Tab, egui::Modifiers::NONE)],
                WINDOW,
            );
            let tree = output.platform_output.accesskit_update.as_ref().unwrap();
            if tree
                .nodes
                .iter()
                .any(|(id, node)| *id == tree.focus && node.label() == Some(accessible.as_str()))
            {
                focused = true;
                break;
            }
        }
        assert!(focused, "keyboard must reach the second result");
        assert_text_contrast(&output, &detail);
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn canvas_help_has_a_wrapped_opaque_backdrop_over_a_dark_page() {
    for language in Language::ALL {
        let (ctx, mut app, _) = panel_setup();
        i18n::activate(language);
        app.show_inspector = true;
        app.library_width = 400.;
        app.inspector_width = 400.;
        app.doc.page_style.background = [12, 22, 34];
        app.fit_next = false;
        app.pan = vec2(-150., -150.);
        app.zoom = 1.;
        let before = app.doc.clone();
        let output = stable_panel(&ctx, &mut app, WINDOW);
        let hint =
            t("Glisser dans le vide : sélectionner · Deux doigts : déplacer · Pincer : zoomer");
        let (index, text) = text_shape(&output, &hint);
        let text_rect = Rect::from_min_size(text.pos, text.galley.size());
        assert!(text.galley.size().x <= app.canvas.width() - 40. + 0.1);
        assert!(app.canvas.contains_rect(text_rect));
        assert!(
            output.shapes[index]
                .clip_rect
                .intersect(app.canvas)
                .contains_rect(text_rect)
        );
        let (background, _) = painted_colors(&output, &hint);
        assert!(
            background.iter().all(|channel| *channel > 0.9),
            "help needs its own light backdrop"
        );
        assert_text_contrast(&output, &hint);
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
    i18n::activate(Language::French);
}

#[test]
fn empty_page_prompts_adapt_to_the_actual_page_fill_without_changing_the_document() {
    for language in Language::ALL {
        for background in [
            [255, 255, 255],
            [12, 22, 34],
            [119, 119, 119],
            [200, 110, 50],
            [0, 80, 180],
            [30, 160, 100],
        ] {
            let ctx = egui::Context::default();
            let mut doc = Document::default();
            doc.page_style.background = background;
            let mut app = DrawApp::from_context(&ctx, doc, None, None);
            i18n::activate(language);
            let before = app.doc.clone();
            let output = stable_panel(&ctx, &mut app, WINDOW);
            for label in [
                "Tout commence par une idée.",
                "Choisissez une forme à gauche, puis cliquez ici.",
            ] {
                assert_text_contrast(&output, &t(label));
            }
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn disabled_actions_stay_distinct_and_inert_while_focus_keeps_visible_contrast() {
    let (ctx, mut app, _) = panel_setup();
    app.ribbon_expanded = Some(true);
    let before = app.doc.clone();
    let output = stable_panel(&ctx, &mut app, WINDOW);
    let (_, disabled) = painted_colors(&output, &t("Copier"));
    let (_, enabled) = painted_colors(&output, &t("Outil Pointeur"));
    assert_ne!(disabled[0], enabled[0]);
    let (_, text) = text_shape(&output, &t("Copier"));
    let point = text.pos + text.galley.size() / 2.;
    panel_pointer(&ctx, &mut app, WINDOW, point, true);
    let output = panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![
            egui::Event::PointerMoved(point),
            egui::Event::PointerButton {
                pos: point,
                button: egui::PointerButton::Primary,
                pressed: false,
                modifiers: egui::Modifiers::NONE,
            },
        ],
    );
    assert!(
        !output
            .platform_output
            .commands
            .iter()
            .any(|command| matches!(command, egui::OutputCommand::CopyText(_)))
    );
    assert!(app.clipboard.is_none());
    let output = panel_frame(
        &ctx,
        &mut app,
        WINDOW,
        vec![command_key(Key::F6, egui::Modifiers::NONE)],
    );
    let focused = ctx
        .read_response(ctx.memory(|m| m.focused()).unwrap())
        .unwrap();
    let outline = output
        .shapes
        .iter()
        .enumerate()
        .find_map(|(index, clipped)| {
            if let Shape::Rect(rect) = &clipped.shape {
                (rect.stroke.width >= 2.
                    && rect.rect.center().distance(focused.rect.center()) < 0.1)
                    .then_some((index, rect))
            } else {
                None
            }
        })
        .expect("focused panel control must keep a visible outline");
    let background = background_at(&output, outline.0, outline.1.rect.center());
    let foreground = composite(outline.1.stroke.color, 1., background);
    assert!(ratio(foreground, background) >= 3.);
    assert_eq!(app.doc, before);
    assert!(!app.history.can_undo());

    // An unsaved document keeps its indicator against both active and inactive
    // tab fills; measure the actual glyphs rather than a palette constant.
    let (ctx, mut app, _) = panel_setup();
    app.doc.title.push_str(" modified");
    app.open_tab(Document::default(), None);
    app.doc.title.push_str(" modified");
    let before = app.doc.clone();
    let output = stable_panel(&ctx, &mut app, WINDOW);
    let dots = output
        .shapes
        .iter()
        .enumerate()
        .filter_map(|(index, clipped)| {
            if let Shape::Text(text) = &clipped.shape {
                (text.galley.job.text == "•" && text.pos.y < 38.).then_some((index, text))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        dots.len(),
        2,
        "both dirty tabs must have a painted indicator"
    );
    for (index, text) in dots {
        let (background, colors) = painted_colors_at(&output, index, text);
        assert!(
            colors
                .into_iter()
                .all(|color| ratio(color, background) >= 3.)
        );
    }
    assert_eq!(app.doc, before);
}

#[test]
fn real_connection_hints_stay_readable_and_inside_a_small_canvas() {
    for language in Language::ALL {
        for state in ["valid", "invalid", "free"] {
            let (ctx, mut app, source) = panel_setup();
            i18n::activate(language);
            let target = app.doc.add_node(ShapeKind::Rectangle, [900., 600.]);
            app.history = History::new(&app.doc);
            app.show_inspector = true;
            app.library_width = 400.;
            app.inspector_width = 400.;
            stable_panel(&ctx, &mut app, WINDOW);
            let before = app.doc.clone();
            app.begin_connection(source, Port::Auto, None, None);
            let (position, hint) = match state {
                "valid" => (
                    app.screen(app.doc.node(target).unwrap().center()),
                    "Cliquez pour relier",
                ),
                "invalid" => (
                    app.screen(app.doc.node(source).unwrap().center()),
                    "Choisissez un autre bloc",
                ),
                _ => (
                    app.canvas.right_bottom() - vec2(30., 80.),
                    "Cliquez pour une extrémité libre",
                ),
            };
            let output = panel_frame(
                &ctx,
                &mut app,
                WINDOW,
                vec![egui::Event::PointerMoved(position)],
            );
            let label = t(hint);
            let (index, text) = text_shape(&output, &label);
            let rect = Rect::from_min_size(text.pos, text.galley.size());
            assert!(
                app.canvas.contains_rect(rect),
                "{} {state}: {rect:?}",
                language.code()
            );
            assert!(output.shapes[index].clip_rect.contains_rect(rect));
            assert_text_contrast(&output, &label);
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
    }
    i18n::activate(Language::French);
}
