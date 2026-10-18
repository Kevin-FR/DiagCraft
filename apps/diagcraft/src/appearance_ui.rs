//! Bounded cache of SVG bodies, shared by canvas and style preset previews.
use super::*;
use diagcraft::{appearance::*, appearance_svg};
use std::{
    cell::RefCell,
    collections::HashMap,
    hash::{Hash, Hasher},
};

#[derive(Clone, Copy, Default, PartialEq)]
pub(super) enum StyleTab {
    #[default]
    Fill,
    Line,
    Shadow,
}
struct Cached {
    texture: egui::TextureHandle,
    pixels: usize,
    used: u64,
}
#[derive(Default)]
pub(super) struct StyleCache {
    items: HashMap<u64, Cached>,
    tick: u64,
    pixels: usize,
}
impl StyleCache {
    fn paint(&mut self, p: &egui::Painter, source: &str, rect: Rect) -> bool {
        if !rect.intersects(p.clip_rect()) {
            return true;
        }
        let desired = rect.size() * p.ctx().pixels_per_point();
        let scale = (2048. / desired.max_elem())
            .min((2_000_000. / (desired.x * desired.y)).sqrt())
            .min(1.);
        let size = (desired * scale).max(vec2(1., 1.));
        // Bucket zoom sizes to reuse textures through slow pinch movements.
        let width = ((size.x / 32.).ceil() * 32.).clamp(1., 2048.) as u32;
        let height = ((size.y / 32.).ceil() * 32.).clamp(1., 2048.) as u32;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut hash);
        width.hash(&mut hash);
        height.hash(&mut hash);
        let key = hash.finish();
        self.tick += 1;
        if !self.items.contains_key(&key) {
            let Ok(tree) = resvg::usvg::Tree::from_str(source, &resvg::usvg::Options::default())
            else {
                return false;
            };
            let Some(mut pixmap) = resvg::tiny_skia::Pixmap::new(width, height) else {
                return false;
            };
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::from_scale(
                    width as f32 / tree.size().width(),
                    height as f32 / tree.size().height(),
                ),
                &mut pixmap.as_mut(),
            );
            let image = egui::ColorImage::from_rgba_premultiplied(
                [width as usize, height as usize],
                pixmap.data(),
            );
            let pixels = width as usize * height as usize;
            while (self.pixels + pixels > 16_000_000 || self.items.len() >= 128)
                && !self.items.is_empty()
            {
                let oldest = *self.items.iter().min_by_key(|(_, v)| v.used).unwrap().0;
                self.pixels -= self.items.remove(&oldest).unwrap().pixels;
            }
            self.pixels += pixels;
            self.items.insert(
                key,
                Cached {
                    texture: p.ctx().load_texture(
                        format!("appearance-{key}"),
                        image,
                        egui::TextureOptions::LINEAR,
                    ),
                    pixels,
                    used: self.tick,
                },
            );
        }
        let item = self.items.get_mut(&key).unwrap();
        item.used = self.tick;
        p.image(
            item.texture.id(),
            rect,
            Rect::from_min_max(Pos2::ZERO, pos2(1., 1.)),
            Color32::WHITE,
        );
        true
    }
    pub(super) fn node(&mut self, p: &egui::Painter, n: &Node, origin: Pos2, zoom: f32) -> bool {
        let (source, b) = appearance_svg::node_image(n);
        self.paint(
            p,
            &source,
            Rect::from_min_max(
                origin + vec2(b[0], b[1]) * zoom,
                origin + vec2(b[2], b[3]) * zoom,
            ),
        )
    }
    pub(super) fn edge(
        &mut self,
        p: &egui::Painter,
        e: &document::Connector,
        points: &[Point],
        origin: Pos2,
        zoom: f32,
        background: document::Color,
    ) -> bool {
        let pad = (8. + e.width) * e.appearance.start_scale.max(e.appearance.end_scale) * 1.5
            + e.width * 2.;
        let mut b = [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        for v in points {
            b[0] = b[0].min(v[0] - pad);
            b[1] = b[1].min(v[1] - pad);
            b[2] = b[2].max(v[0] + pad);
            b[3] = b[3].max(v[1] + pad);
        }
        let source = appearance_svg::wrap(&appearance_svg::connector(e, points, background), b);
        self.paint(
            p,
            &source,
            Rect::from_min_max(
                origin + vec2(b[0], b[1]) * zoom,
                origin + vec2(b[2], b[3]) * zoom,
            ),
        )
    }
}
pub(super) type SharedCache = RefCell<StyleCache>;

fn choice<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut T,
    choices: &[T],
    name: impl Fn(T) -> &'static str,
) {
    ui.label(t(label));
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width() - 12.)
        .selected_text(t(name(*value)))
        .show_ui(ui, |ui| {
            for v in choices {
                ui.selectable_value(value, *v, t(name(*v)));
            }
        });
}
pub(super) fn scalar(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    suffix: &str,
) {
    ui.label(t(label));
    ui.add(
        egui::Slider::new(value, range)
            .suffix(suffix)
            .clamping(egui::SliderClamping::Edits),
    );
    ui.add_space(5.);
}
pub(super) fn color(ui: &mut egui::Ui, label: &str, value: &mut document::Color) {
    ui.horizontal(|ui| {
        ui.label(t(label));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.color_edit_button_srgb(value);
        });
    });
}

pub(super) fn line_controls(
    ui: &mut egui::Ui,
    s: &mut Line,
    base: &mut document::Color,
    width: &mut f32,
    pattern: Option<&mut LinePattern>,
    corners: Option<f32>,
    minimum: f32,
) {
    choice(
        ui,
        "Type de trait",
        &mut s.mode,
        &[
            PaintMode::None,
            PaintMode::Solid,
            PaintMode::Linear,
            PaintMode::Monochrome,
        ],
        PaintMode::label,
    );
    ui.add_enabled_ui(s.mode != PaintMode::None, |ui| {
        ui.add_space(8.);
        color(ui, "Couleur", base);
        if s.mode == PaintMode::Linear {
            color(ui, "Seconde couleur", &mut s.secondary);
        }
        if matches!(s.mode, PaintMode::Linear | PaintMode::Monochrome) {
            scalar(ui, "Angle du dégradé", &mut s.angle, -180.0..=180.0, "°");
        }
        scalar(ui, "Ombre / teinte", &mut s.tint, -100.0..=100.0, " %");
        scalar(ui, "Transparence", &mut s.transparency, 0.0..=100.0, " %");
        scalar(ui, "Épaisseur", width, minimum..=20.0, " px");
        choice(
            ui,
            "Type composé",
            &mut s.compound,
            Compound::ALL,
            Compound::label,
        );
        choice(
            ui,
            "Type de tiret",
            pattern.unwrap_or(&mut s.pattern),
            &LinePattern::ALL,
            LinePattern::label,
        );
        choice(ui, "Extrémités du trait", &mut s.cap, Cap::ALL, Cap::label);
        choice(ui, "Jonctions", &mut s.join, Join::ALL, Join::label);
        if let Some(default) = corners {
            let mut radius = s.corner_radius.unwrap_or(default);
            let before = radius;
            scalar(ui, "Rayon des coins", &mut radius, 0.0..=100.0, " px");
            if radius != before {
                s.corner_radius = Some(radius);
            }
        }
    });
}

pub(super) fn node_inspector(
    ui: &mut egui::Ui,
    n: &mut Node,
    tab: &mut StyleTab,
    cache: &SharedCache,
    focus: &mut Option<u64>,
    capture_presets: bool,
) -> (bool, bool, bool) {
    let mut fit_text = false;
    ui.label(RichText::new(t(n.kind.label())).size(20.).strong());
    ui.add_space(10.);
    if n.kind != ShapeKind::Text {
        node_controls(ui, n, tab, cache, capture_presets);
    }
    ui.separator();
    egui::CollapsingHeader::new(t("Texte et typographie"))
        .id_salt(("text", n.id))
        .default_open(n.kind == ShapeKind::Text)
        .open((*focus == Some(n.id)).then_some(true))
        .show(ui, |ui| {
            let edit = ui.add(
                egui::TextEdit::multiline(&mut n.text)
                    .id_salt(("node_text", n.id))
                    .desired_width(f32::INFINITY)
                    .desired_rows(3)
                    .char_limit(4000),
            );
            if *focus == Some(n.id) {
                edit.request_focus();
                edit.scroll_to_me(Some(Align::Center));
                *focus = None;
            }
            ui.checkbox(&mut n.text_style.wrap, t("Retour à la ligne automatique"));
            if ui
                .checkbox(
                    &mut n.text_style.auto_fit,
                    t("Adapter automatiquement au texte"),
                )
                .on_hover_text(t(
                    "Un redimensionnement manuel désactive l’ajustement automatique.",
                ))
                .changed()
                && n.text_style.auto_fit
            {
                n.text_style.wrap = true;
            }
            ui.label(
                RichText::new(t(
                    "Avec retour automatique, la largeur est conservée et la hauteur s’adapte.",
                ))
                .small()
                .color(MUTED),
            );
            fit_text = ui.button(t("Ajuster la forme au texte")).clicked();
            color(ui, "Couleur du texte", &mut n.text_color);
            ui.add(
                egui::Slider::new(&mut n.font_size, 8.0..=64.0)
                    .clamping(egui::SliderClamping::Edits)
                    .text(t("Taille")),
            );
        });
    let before_geometry = n.size;
    egui::CollapsingHeader::new(t("Position et dimensions"))
        .id_salt(("position", n.id))
        .show(ui, |ui| {
            egui::Grid::new("geometry")
                .num_columns(2)
                .spacing(vec2(6., 8.))
                .show(ui, |ui| {
                    for (i, label) in [(0, "X"), (1, "Y")] {
                        ui.horizontal(|ui| {
                            ui.label(t(label));
                            ui.add(
                                egui::DragValue::new(&mut n.position[i])
                                    .clamp_existing_to_range(false)
                                    .speed(1.)
                                    .range(0.0..=100_000.0),
                            );
                        });
                    }
                    ui.end_row();
                    for (i, label) in [(0, "L"), (1, "H")] {
                        ui.horizontal(|ui| {
                            ui.label(t(label));
                            ui.add(
                                egui::DragValue::new(&mut n.size[i])
                                    .clamp_existing_to_range(false)
                                    .speed(1.)
                                    .range(24.0..=20_000.0),
                            );
                        });
                    }
                });
        });
    if n.size != before_geometry {
        n.text_style.auto_fit = false;
    }
    ui.add_space(8.);
    (
        ui.button(t("Mettre au premier plan")).clicked(),
        ui.button(t("Mettre à l’arrière-plan")).clicked(),
        fit_text,
    )
}
pub(super) fn node_controls(
    ui: &mut egui::Ui,
    n: &mut Node,
    tab: &mut StyleTab,
    cache: &SharedCache,
    capture_presets: bool,
) {
    style_tabs(ui, tab, true);
    let corners = node_corners(n);
    style_body(ui, n, *tab, cache, capture_presets, corners, 0.);
}

pub(super) fn node_corners(n: &Node) -> Option<f32> {
    match n.kind {
        ShapeKind::Rounded => Some(20.),
        ShapeKind::Rectangle => Some(2.),
        _ => None,
    }
}

pub(super) fn style_tabs(ui: &mut egui::Ui, tab: &mut StyleTab, has_shapes: bool) {
    if !has_shapes {
        *tab = StyleTab::Line;
    }
    ui.horizontal(|ui| {
        for (candidate, label) in [
            (StyleTab::Fill, "Remplir"),
            (StyleTab::Line, "Ligne"),
            (StyleTab::Shadow, "Ombre"),
        ] {
            ui.add_enabled_ui(has_shapes || candidate == StyleTab::Line, |ui| {
                ui.selectable_value(tab, candidate, t(label));
            });
        }
    });
    ui.separator();
    ui.add_space(8.);
}

/// Returns true for an explicit preset or reset, including unchanged exemplar values.
pub(super) fn style_body(
    ui: &mut egui::Ui,
    n: &mut Node,
    tab: StyleTab,
    cache: &SharedCache,
    capture_presets: bool,
    corners: Option<f32>,
    minimum: f32,
) -> bool {
    let mut replace_effect = false;
    match tab {
        StyleTab::Fill => {
            let s = &mut n.appearance.fill;
            choice(
                ui,
                "Remplissage",
                &mut s.mode,
                PaintMode::ALL,
                PaintMode::label,
            );
            ui.add_enabled_ui(s.mode != PaintMode::None, |ui| {
                ui.add_space(8.);
                color(ui, "Couleur", &mut n.fill);
                ui.horizontal_wrapped(|ui| {
                    for c in PALETTE {
                        if swatch(ui, c, n.fill == c).clicked() {
                            n.fill = c;
                        }
                    }
                });
                if matches!(
                    s.mode,
                    PaintMode::Linear | PaintMode::Radial | PaintMode::Pattern | PaintMode::Texture
                ) {
                    color(ui, "Seconde couleur", &mut s.secondary);
                }
                if s.mode == PaintMode::Pattern {
                    choice(
                        ui,
                        "Motif",
                        &mut s.pattern,
                        PatternKind::ALL,
                        PatternKind::label,
                    );
                }
                if s.mode == PaintMode::Texture {
                    choice(
                        ui,
                        "Texture",
                        &mut s.texture,
                        TextureKind::ALL,
                        TextureKind::label,
                    );
                }
                if matches!(s.mode, PaintMode::Pattern | PaintMode::Texture) {
                    scalar(ui, "Espacement", &mut s.spacing, 4.0..=64.0, " px");
                }
                if !matches!(
                    s.mode,
                    PaintMode::Solid | PaintMode::Radial | PaintMode::Texture
                ) {
                    scalar(ui, "Angle", &mut s.angle, -180.0..=180.0, "°");
                }
                scalar(ui, "Ombre / teinte", &mut s.tint, -100.0..=100.0, " %");
                scalar(ui, "Transparence", &mut s.transparency, 0.0..=100.0, " %");
            });
        }
        StyleTab::Line => line_controls(
            ui,
            &mut n.appearance.line,
            &mut n.stroke,
            &mut n.stroke_width,
            None,
            corners,
            minimum,
        ),
        StyleTab::Shadow => {
            let s = &mut n.appearance.shadow;
            let menu = ui.menu_button(t("Préréglages d’ombre"), |ui| {
                egui::ScrollArea::vertical()
                    .max_height(400.)
                    .show(ui, |ui| {
                        ui.set_max_width(230.);
                        if ui.button(t("Aucune ombre")).clicked() {
                            s.kind = ShadowKind::None;
                            replace_effect = true;
                            ui.close();
                        }
                        for kind in [
                            ShadowKind::Outer,
                            ShadowKind::Inner,
                            ShadowKind::Perspective,
                        ] {
                            ui.label(t(kind.label()));
                            egui::Grid::new(kind.label())
                                .num_columns(3)
                                .spacing(vec2(6., 6.))
                                .show(ui, |ui| {
                                    for i in 0..9 {
                                        let preset = Shadow::preset(kind, i);
                                        let (r, response) =
                                            ui.allocate_exact_size(vec2(58., 50.), Sense::click());
                                        if response.hovered() {
                                            ui.painter().rect_filled(r, 5, SURFACE);
                                        }
                                        let mut sample = symbols::preset(ShapeKind::Rectangle);
                                        sample.size = [32., 28.];
                                        sample.fill = [0, 125, 250];
                                        sample.stroke = [0, 100, 225];
                                        sample.appearance.shadow = preset.clone();
                                        sample.appearance.shadow.offset =
                                            sample.appearance.shadow.offset.map(|v| v * 0.35);
                                        sample.appearance.shadow.blur *= 0.35;
                                        sample.stroke_width = 1.;
                                        cache.borrow_mut().node(
                                            ui.painter(),
                                            &sample,
                                            r.min + vec2(12., 10.),
                                            1.,
                                        );
                                        if response
                                            .on_hover_text(format!(
                                                "{} · X {} · Y {}",
                                                kind.label(),
                                                preset.offset[0],
                                                preset.offset[1]
                                            ))
                                            .clicked()
                                        {
                                            *s = preset;
                                            replace_effect = true;
                                            ui.close();
                                        }
                                        if i % 3 == 2 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        }
                    });
            });
            if capture_presets {
                egui::Popup::open_id(ui.ctx(), egui::Popup::default_response_id(&menu.response));
            }
            choice(
                ui,
                "Position de l’ombre",
                &mut s.kind,
                ShadowKind::ALL,
                ShadowKind::label,
            );
            ui.add_enabled_ui(s.kind != ShadowKind::None, |ui| {
                ui.add_space(8.);
                color(ui, "Couleur", &mut s.color);
                scalar(ui, "Transparence", &mut s.transparency, 0.0..=100.0, " %");
                scalar(ui, "Taille", &mut s.scale, 10.0..=300.0, " %");
                scalar(ui, "Flou", &mut s.blur, 0.0..=64.0, " px");
                scalar(ui, "Angle", &mut s.angle, -180.0..=180.0, "°");
                scalar(ui, "Décalage X", &mut s.offset[0], -200.0..=200.0, " px");
                scalar(ui, "Décalage Y", &mut s.offset[1], -200.0..=200.0, " px");
            });
        }
    }
    ui.add_space(12.);
    if ui.small_button(t("Réinitialiser cet onglet")).clicked() {
        replace_effect = true;
        match tab {
            StyleTab::Fill => n.appearance.fill = Default::default(),
            StyleTab::Line => n.appearance.line = Default::default(),
            StyleTab::Shadow => n.appearance.shadow = Default::default(),
        }
    }
    replace_effect
}

pub(super) fn edge_controls(ui: &mut egui::Ui, e: &mut document::Connector) {
    line_controls(
        ui,
        &mut e.appearance,
        &mut e.color,
        &mut e.width,
        Some(&mut e.pattern),
        None,
        0.5,
    );
    ui.add_space(8.);
    field_label(ui, "FLÈCHES");
    contextual_toolbar::edge_style_controls(ui, e);
    ui.add(egui::Slider::new(&mut e.appearance.start_scale, 0.5..=3.0).text(t("Taille départ")));
    ui.add(egui::Slider::new(&mut e.appearance.end_scale, 0.5..=3.0).text(t("Taille arrivée")));
}
