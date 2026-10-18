//! Shared line breaks, font metrics and fitting, independent of the GUI and zoom.
use crate::document::{Document, Node, Point, ShapeKind, TextStyle};
use std::sync::OnceLock;
use ttf_parser::{Face, GlyphId};
use unicode_segmentation::UnicodeSegmentation;

pub const PADDING: f32 = 12.;

fn faces() -> &'static [Face<'static>; 6] {
    static FACES: OnceLock<[Face<'static>; 6]> = OnceLock::new();
    FACES.get_or_init(|| {
        [
            include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/SourceSans3-Bold.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/SourceSans3-It.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/SourceSans3-BoldIt.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/Hack-Regular.ttf").as_slice(),
            include_bytes!("../../../assets/fonts/NotoSansSC-Regular.otf").as_slice(),
        ]
        .map(|data| Face::parse(data, 0).expect("validated bundled font"))
    })
}

fn primary(style: &TextStyle) -> usize {
    if style.monospace {
        4
    } else {
        usize::from(style.bold) + 2 * usize::from(style.italic)
    }
}

fn glyph(style: &TextStyle, ch: char) -> (usize, GlyphId) {
    let first = primary(style);
    if let Some(id) = faces()[first].glyph_index(ch) {
        (first, id)
    } else if let Some(id) = faces()[5].glyph_index(ch) {
        (5, id)
    } else {
        (first, GlyphId(0))
    }
}

fn advance(style: &TextStyle, text: &str, size: f32) -> f32 {
    text.chars()
        .map(|ch| {
            if ch.is_control() || matches!(ch, '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{fe0f}') {
                return 0.;
            }
            let (font, id) = glyph(style, ch);
            let face = &faces()[font];
            if id.0 == 0 {
                return size * 1.3;
            }
            f32::from(face.glyph_hor_advance(id).unwrap_or(face.units_per_em()))
                / f32::from(face.units_per_em())
                * size
        })
        .sum()
}

/// Fractions reserve a central text region away from sloped or curved outlines.
pub fn region(kind: ShapeKind) -> Point {
    use ShapeKind::*;
    match kind {
        Rectangle | Rounded | Text | PredefinedProcess | InternalStorage | Frame | UmlClass => {
            [1., 1.]
        }
        Diamond => [0.5, 0.5],
        Ellipse | UseCase | InitialState | FinalState => [0.7, 0.7],
        Triangle | RightTriangle | Cone | Pyramid => [0.3, 0.3],
        ArrowRight | ArrowLeft | DoubleArrow | NotchedArrow | Chevron => [0.5, 0.35],
        ArrowUp | ArrowDown => [0.35, 0.5],
        Star | Star4 | Star8 | Plus | Cross | Lightning => [0.35, 0.35],
        _ => [0.75, 0.65],
    }
}

/// Central text area in document coordinates, also used by SVG and the canvas.
pub fn area(node: &Node) -> (Point, Point) {
    let fraction = region(node.kind);
    let size = [0, 1].map(|axis| (node.size[axis] * fraction[axis] - 2. * PADDING).max(1.));
    let center = node.center();
    ([center[0] - size[0] / 2., center[1] - size[1] / 2.], size)
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub lines: Vec<String>,
    pub width: f32,
    pub height: f32,
    pub line_height: f32,
    pub ascent: f32,
    pub descent: f32,
}

impl Layout {
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }
    pub fn first_baseline(&self, center_y: f32) -> f32 {
        center_y - self.height / 2. + (self.line_height + self.ascent + self.descent) / 2.
    }
}

fn break_paragraph(
    paragraph: &str,
    max_width: f32,
    style: &TextStyle,
    size: f32,
    lines: &mut Vec<String>,
) {
    let units: Vec<_> = paragraph
        .graphemes(true)
        .map(|s| (s, advance(style, s, size)))
        .collect();
    if units.is_empty() {
        lines.push(String::new());
        return;
    }
    let (mut start, mut at, mut width, mut word_break) = (0, 0, 0., None);
    while at < units.len() {
        if at > start && width + units[at].1 > max_width {
            let end = word_break.filter(|end| *end > start).unwrap_or(at);
            lines.push(units[start..end].iter().map(|unit| unit.0).collect());
            start = end;
            at = end;
            width = 0.;
            word_break = None;
        } else {
            width += units[at].1;
            if matches!(units[at].0, " " | "\u{3000}" | "\u{200b}") {
                word_break = Some(at + 1);
            }
            at += 1;
        }
    }
    lines.push(units[start..].iter().map(|unit| unit.0).collect());
}

pub fn layout(node: &Node) -> Layout {
    let size = if node.font_size.is_finite() {
        node.font_size.clamp(6., 200.)
    } else {
        18.
    };
    let display = node
        .text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");
    let mut ascent = 0_f32;
    let mut descent = 0_f32;
    let mut line_height = size * 1.25;
    for font in std::iter::once(primary(&node.text_style))
        .chain(display.chars().map(|ch| glyph(&node.text_style, ch).0))
    {
        let face = &faces()[font];
        let scale = size / f32::from(face.units_per_em());
        ascent = ascent.max(f32::from(face.ascender()) * scale);
        descent = descent.min(f32::from(face.descender()) * scale);
        line_height = line_height.max(f32::from(face.height() + face.line_gap()) * scale);
    }
    line_height = line_height.max(ascent - descent).ceil();
    let mut lines = vec![];
    // Slack covers rounding and small kerning/italic differences between renderers.
    let limit = (area(node).1[0] - size * 0.1 - 2.).max(1.);
    for paragraph in display.split('\n') {
        if node.text_style.wrap {
            break_paragraph(paragraph, limit, &node.text_style, size, &mut lines);
        } else {
            lines.push(paragraph.into());
        }
    }
    let width = lines
        .iter()
        .map(|line| advance(&node.text_style, line, size))
        .fold(0_f32, f32::max)
        + size * 0.1
        + 2.;
    Layout {
        height: lines.len() as f32 * line_height,
        lines,
        width,
        line_height,
        ascent,
        descent,
    }
}

impl Node {
    pub fn text_layout(&self) -> Layout {
        layout(self)
    }

    /// With wrapping, preserve width and fit height. Otherwise fit both dimensions.
    /// On failure, neither the text, style, position nor size is changed.
    pub fn fitted_text_size(&self) -> Result<Point, String> {
        if !self.font_size.is_finite()
            || !(6.0..=200.).contains(&self.font_size)
            || self.text.len() > 65_536
            || !self
                .size
                .iter()
                .all(|v| v.is_finite() && (24.0..=20_000.).contains(v))
        {
            return Err("Style ou géométrie invalide pour l’ajustement au texte.".into());
        }
        let fraction = region(self.kind);
        let mut probe = self.clone();
        if self.text_style.wrap {
            let widest = self
                .text
                .graphemes(true)
                .map(|g| advance(&self.text_style, g, self.font_size))
                .fold(0_f32, f32::max);
            probe.size[0] = self.size[0]
                .max(((widest + self.font_size * 0.1 + 3. + 2. * PADDING) / fraction[0]).ceil());
        }
        let text = layout(&probe);
        let width = if self.text_style.wrap {
            probe.size[0]
        } else {
            ((text.width + 2. * PADDING) / fraction[0]).ceil().max(24.)
        };
        let height = ((text.height + 2. * PADDING) / fraction[1]).ceil().max(24.);
        if width > 20_000. || height > 20_000. {
            return Err("Texte trop grand pour une forme (maximum 20 000 px). Réduisez le texte ou sa taille.".into());
        }
        Ok([width, height])
    }

    pub fn fit_to_text(&mut self) -> Result<(), String> {
        self.size = self.fitted_text_size()?;
        Ok(())
    }
}

impl Document {
    /// Fit compatible nodes atomically; position, connectors and selection IDs stay intact.
    pub fn fit_nodes_to_text(&mut self, ids: &[u64]) -> Result<usize, String> {
        let sizes = self
            .nodes
            .iter()
            .filter(|n| ids.contains(&n.id))
            .map(|n| Ok((n.id, n.fitted_text_size()?)))
            .collect::<Result<Vec<_>, String>>()?;
        let changed = sizes
            .iter()
            .filter(|(id, size)| self.node(*id).is_some_and(|n| n.size != *size))
            .count();
        for (id, size) in sizes {
            self.node_mut(id).unwrap().size = size;
        }
        Ok(changed)
    }

    pub fn fit_automatic_text(&mut self) -> Result<usize, String> {
        let ids = self
            .nodes
            .iter()
            .filter(|n| n.text_style.auto_fit)
            .map(|n| n.id)
            .collect::<Vec<_>>();
        self.fit_nodes_to_text(&ids)
    }
}
