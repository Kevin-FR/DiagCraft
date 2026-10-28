//! Pixel-exact page/selection PNG exports from the same SVG as PDF and canvas.
use crate::{
    document::{Document, Point},
    routing::RoutingContext,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
pub const MAX_OUTPUT_PIXELS: u64 = 16_000_000;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PngOptions {
    pub page: usize,
    pub selection: Vec<u64>,
    pub width: u32,
    pub height: u32,
    pub dpi: f32,
    pub transparent: bool,
}
impl Default for PngOptions {
    fn default() -> Self {
        Self {
            page: 0,
            selection: vec![],
            width: 1280,
            height: 900,
            dpi: 96.,
            transparent: false,
        }
    }
}
impl PngOptions {
    pub fn validate(&self) -> Result<(), String> {
        if self.width == 0
            || self.height == 0
            || self.width > 8192
            || self.height > 8192
            || u64::from(self.width) * u64::from(self.height) > MAX_OUTPUT_PIXELS
        {
            return Err(
                "Export PNG trop grand : maximum 16 millions de pixels et 8192 px par côté.".into(),
            );
        }
        if !self.dpi.is_finite() || !(36. ..=1200.).contains(&self.dpi) {
            return Err("Résolution PNG invalide : 36 à 1200 ppp requis.".into());
        }
        if self.selection.len() > 10_000 {
            return Err("Sélection PNG trop volumineuse.".into());
        }
        Ok(())
    }
}
fn selection(doc: &Document, ids: &[u64]) -> Result<Option<HashSet<u64>>, String> {
    if ids.is_empty() {
        return Ok(None);
    }
    let view = doc.organization_view();
    if ids.iter().any(|id| !view.is_object_visible(*id)) {
        return Err("Sélection PNG absente ou masquée.".into());
    }
    Ok(Some(view.expand_groups(ids).into_iter().collect()))
}
pub fn region(doc: &Document, ids: &[u64]) -> Result<[f32; 4], String> {
    let Some(ids) = selection(doc, ids)? else {
        return Ok([0., 0., doc.page_size[0], doc.page_size[1]]);
    };
    let mut b = [
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    let mut include = |p: Point| {
        b[0] = b[0].min(p[0]);
        b[1] = b[1].min(p[1]);
        b[2] = b[2].max(p[0]);
        b[3] = b[3].max(p[1]);
    };
    for n in &doc.nodes {
        if ids.contains(&n.id) {
            let r = crate::appearance_svg::node_bounds(n);
            include([n.position[0] + r[0], n.position[1] + r[1]]);
            include([n.position[0] + r[2], n.position[1] + r[3]]);
        }
    }
    for c in &doc.charts {
        if ids.contains(&c.id) {
            include(c.position);
            include([c.position[0] + c.size[0], c.position[1] + c.size[1]]);
        }
    }
    for d in &doc.drawings {
        if ids.contains(&d.id) {
            let pad = 12. + d.width * 4.;
            for p in d.controls() {
                include([p[0] - pad, p[1] - pad]);
                include([p[0] + pad, p[1] + pad]);
            }
        }
    }
    let routing = RoutingContext::new(doc);
    for e in &doc.connectors {
        if ids.contains(&e.id) {
            let r = routing.route(e);
            let pad = (12. + e.width * 4.) * e.appearance.start_scale.max(e.appearance.end_scale);
            for p in &r.points {
                include([p[0] - pad, p[1] - pad]);
                include([p[0] + pad, p[1] + pad]);
            }
            if !e.label.is_empty() {
                let at = r.label_position(e.label_t);
                let half = e.label.chars().count() as f32 * 4. + 8.;
                include([at[0] - half, at[1] - 20.]);
                include([at[0] + half, at[1] + 8.]);
            }
        }
    }
    if b.iter().any(|v| !v.is_finite()) {
        return Err("Sélection PNG vide.".into());
    }
    Ok([
        b[0] - 4.,
        b[1] - 4.,
        (b[2] - b[0] + 8.).max(1.),
        (b[3] - b[1] + 8.).max(1.),
    ])
}
pub fn export(doc: &Document, options: &PngOptions) -> Result<Vec<u8>, String> {
    options.validate()?;
    doc.validate()?;
    let page = doc
        .page_document(options.page)
        .ok_or("Page PNG introuvable.")?;
    let bounds = region(&page, &options.selection)?;
    let selected = selection(&page, &options.selection)?;
    let source = crate::svg::export_view(&page, bounds, !options.transparent, selected.as_ref())?;
    let mut config = resvg::usvg::Options::default();
    crate::pdf::load_fonts(&mut config);
    let tree = resvg::usvg::Tree::from_str(&source, &config).map_err(|e| e.to_string())?;
    let mut pixels = resvg::tiny_skia::Pixmap::new(options.width, options.height)
        .ok_or("Allocation PNG impossible.")?;
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::from_scale(
            options.width as f32 / tree.size().width(),
            options.height as f32 / tree.size().height(),
        ),
        &mut pixels.as_mut(),
    );
    let rgba: Vec<u8> = pixels
        .pixels()
        .iter()
        .flat_map(|p| {
            let c = p.demultiply();
            [c.red(), c.green(), c.blue(), c.alpha()]
        })
        .collect();
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, options.width, options.height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let ppm = (options.dpi / 0.0254).round() as u32;
        encoder.set_pixel_dims(Some(png::PixelDimensions {
            xppu: ppm,
            yppu: ppm,
            unit: png::Unit::Meter,
        }));
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&rgba)
            .map_err(|e| e.to_string())?;
    }
    Ok(bytes)
}
