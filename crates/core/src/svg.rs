use crate::document::{Color, Document, DrawingKind, ShapeKind, TextAlign};
use std::fmt::Write;

fn escape(s: &str) -> String {
    s.chars()
        .filter(|&c| {
            c == '\n' || c == '\t' || c == '\r' || (c >= ' ' && c != '\u{fffe}' && c != '\u{ffff}')
        })
        .map(|c| match c {
            '&' => "&amp;".into(),
            '<' => "&lt;".into(),
            '>' => "&gt;".into(),
            '"' => "&quot;".into(),
            '\'' => "&apos;".into(),
            _ => c.to_string(),
        })
        .collect()
}
pub fn hex(c: Color) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}

/// Export vector geometry using the same computed routes as the native canvas.
pub fn export(doc: &Document) -> Result<String, String> {
    export_view(
        doc,
        [0., 0., doc.page_size[0], doc.page_size[1]],
        true,
        None,
    )
}
pub(crate) fn export_view(
    doc: &Document,
    region: [f32; 4],
    background: bool,
    selected: Option<&std::collections::HashSet<u64>>,
) -> Result<String, String> {
    doc.validate()?;
    let organization = doc.organization_view();
    let [x, y, w, h] = region;
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{x} {y} {w} {h}\" role=\"img\">\n<title>{}</title>\n",
        escape(&doc.title)
    );
    // The root is exposed as a single image to assistive technology, so nested
    // image descriptions must also be included in the root description. Keep
    // the same visibility, selection and viewport boundaries as the drawing.
    let mut description_started = false;
    for node in &doc.nodes {
        if !organization.is_object_visible(node.id)
            || selected.is_some_and(|ids| !ids.contains(&node.id))
            || node.position[0] >= x + w
            || node.position[1] >= y + h
            || node.position[0] + node.size[0] <= x
            || node.position[1] + node.size[1] <= y
        {
            continue;
        }
        let Some(image) = &node.image else {
            continue;
        };
        let description = escape(image.alt.trim());
        if description.trim().is_empty() {
            continue;
        }
        if !description_started {
            out.push_str("<desc>");
            description_started = true;
        } else {
            out.push('\n');
        }
        out.push_str(&description);
    }
    if description_started {
        out.push_str("</desc>\n");
    }
    if background {
        let _ = writeln!(
            out,
            "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\"/>",
            hex(doc.page_style.background)
        );
    }
    if doc.page_style.export_grid {
        let step = doc.page_style.grid_spacing;
        let content = match doc.page_style.grid_style {
            crate::document::GridStyle::Dots => {
                "<circle cx=\"1\" cy=\"1\" r=\"0.8\" fill=\"#d8dce2\"/>".to_owned()
            }
            crate::document::GridStyle::Lines => format!(
                "<path d=\"M {step} 0 L 0 0 0 {step}\" fill=\"none\" stroke=\"#dfe1e5\" stroke-width=\"1\"/>"
            ),
        };
        let _ = writeln!(
            out,
            "<defs><pattern id=\"diagcraft-grid\" width=\"{step}\" height=\"{step}\" patternUnits=\"userSpaceOnUse\">{content}</pattern></defs><rect width=\"100%\" height=\"100%\" fill=\"url(#diagcraft-grid)\"/>"
        );
    }
    if !doc.page_style.watermark.is_empty() {
        let _ = writeln!(
            out,
            "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" fill=\"#8796a8\" opacity=\"0.15\" font-family=\"sans-serif\" font-size=\"64\">{}</text>",
            doc.page_size[0] / 2.,
            doc.page_size[1] / 2.,
            escape(&doc.page_style.watermark)
        );
    }
    let routing = crate::routing::RoutingContext::new(doc);
    for layer in doc.layer_ids() {
        render_layer(doc, &organization, &routing, selected, layer, &mut out);
    }
    out.push_str("</svg>\n");
    Ok(out)
}

fn line_label(out: &mut String, label: &str, center: crate::document::Point, color: Color) {
    let layout = crate::text_layout::line_label(label);
    let bounds = [
        center[0] - layout.width / 2. - 4.,
        center[1] - layout.height / 2. - 3.,
        center[0] + layout.width / 2. + 4.,
        center[1] + layout.height / 2. + 3.,
    ];
    let _ = writeln!(
        out,
        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"white\"/>",
        bounds[0],
        bounds[1],
        bounds[2] - bounds[0],
        bounds[3] - bounds[1]
    );
    let _ = write!(
        out,
        "<text text-anchor=\"middle\" xml:space=\"preserve\" font-family=\"'Source Sans 3','Noto Sans SC',Arial,sans-serif\" font-size=\"14\" fill=\"{}\">",
        hex(color)
    );
    let baseline = layout.first_baseline(center[1]);
    for (i, line) in layout.lines.iter().enumerate() {
        let _ = write!(
            out,
            "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
            center[0],
            baseline + i as f32 * layout.line_height,
            escape(line)
        );
    }
    out.push_str("</text>\n");
}

fn render_layer(
    doc: &Document,
    organization: &crate::organization::OrganizationView,
    routing: &crate::routing::RoutingContext<'_>,
    selected: Option<&std::collections::HashSet<u64>>,
    layer: u64,
    out: &mut String,
) {
    let include = |id: &u64| selected.is_none_or(|ids| ids.contains(id));
    for drawing in doc.drawings.iter().filter(|d| {
        organization.object_layer(d.id) == layer
            && organization.is_object_visible(d.id)
            && include(&d.id)
    }) {
        let [a, b, c, d] = drawing.controls();
        let color = hex(drawing.color);
        let path = match drawing.kind {
            DrawingKind::Line => format!("M {} {} L {} {}", a[0], a[1], d[0], d[1]),
            DrawingKind::Bezier => format!(
                "M {} {} C {} {}, {} {}, {} {}",
                a[0], a[1], b[0], b[1], c[0], c[1], d[0], d[1]
            ),
        };
        if !drawing.appearance.is_default() {
            out.push_str(&crate::appearance_svg::connector(
                &crate::appearance_svg::drawing_edge(drawing),
                &drawing.points(),
                doc.page_style.background,
            ));
        } else {
            let _ = writeln!(
                out,
                "<path d=\"{path}\" stroke=\"{color}\" stroke-width=\"{}\" fill=\"none\" stroke-linecap=\"round\"/>",
                drawing.width
            );
            if drawing.arrow {
                let pts = drawing.points();
                let tip = pts[pts.len() - 1];
                let before = pts[pts.len() - 2];
                let dx = tip[0] - before[0];
                let dy = tip[1] - before[1];
                let len = (dx * dx + dy * dy).sqrt();
                if len > 0.001 {
                    let (ux, uy) = (dx / len, dy / len);
                    let size = 8. + drawing.width;
                    let _ = writeln!(
                        out,
                        "<polygon points=\"{},{} {},{} {},{}\" fill=\"{color}\"/>",
                        tip[0],
                        tip[1],
                        tip[0] - ux * size - uy * size * 0.45,
                        tip[1] - uy * size + ux * size * 0.45,
                        tip[0] - ux * size + uy * size * 0.45,
                        tip[1] - uy * size - ux * size * 0.45
                    );
                }
            }
        }
        if !drawing.label.is_empty() {
            line_label(out, &drawing.label, drawing.label_position(), drawing.color);
        }
    }
    for edge in doc.connectors.iter().filter(|e| {
        organization.object_layer(e.id) == layer
            && organization.is_object_visible(e.id)
            && include(&e.id)
    }) {
        let routed = routing.route(edge);
        let points = routed.points;
        let color = hex(edge.color);
        let coords = points
            .iter()
            .map(|p| format!("{},{}", p[0], p[1]))
            .collect::<Vec<_>>()
            .join(" ");
        if !edge.appearance.is_default() {
            out.push_str(&crate::appearance_svg::connector(
                edge,
                &points,
                doc.page_style.background,
            ));
        } else {
            let dash = edge
                .pattern
                .dash(edge.width)
                .map(|d| format!(" stroke-dasharray=\"{} {}\"", d[0], d[1]))
                .unwrap_or_default();
            let _ = writeln!(
                out,
                "<polyline points=\"{coords}\" fill=\"none\" stroke=\"{color}\" stroke-width=\"{}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"{dash}/>",
                edge.width
            );
            if edge.arrow {
                styled_arrowhead(
                    out,
                    &points,
                    &color,
                    edge.width,
                    edge.end_head,
                    doc.page_style.background,
                );
            }
            if edge.arrow_start {
                let reverse: Vec<_> = points.iter().rev().copied().collect();
                styled_arrowhead(
                    out,
                    &reverse,
                    &color,
                    edge.width,
                    edge.start_head,
                    doc.page_style.background,
                );
            }
        }
        if !edge.label.is_empty() && points.len() >= 2 {
            line_label(out, &edge.label, edge.label_position(&points), edge.color);
        }
    }

    for n in doc.nodes.iter().filter(|n| {
        organization.object_layer(n.id) == layer
            && organization.is_object_visible(n.id)
            && include(&n.id)
    }) {
        let [x, y] = n.position;
        let [w, h] = n.size;
        let c = n.center();
        let style = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"",
            hex(n.fill),
            hex(n.stroke),
            n.stroke_width
        );
        if !n.appearance.is_default() || n.image.is_some() {
            out.push_str(&crate::appearance_svg::node(n));
        } else {
            match n.kind {
                ShapeKind::Rectangle | ShapeKind::Rounded => {
                    let _ = writeln!(
                        out,
                        "<rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" rx=\"{}\" {style}/>",
                        if n.kind == ShapeKind::Rounded {
                            20.
                        } else {
                            2.
                        }
                    );
                }
                ShapeKind::Ellipse => {
                    let _ = writeln!(
                        out,
                        "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" {style}/>",
                        c[0],
                        c[1],
                        w / 2.,
                        h / 2.
                    );
                }
                ShapeKind::Diamond => {
                    let _ = writeln!(
                        out,
                        "<polygon points=\"{},{} {},{} {},{} {},{}\" {style}/>",
                        c[0],
                        y,
                        x + w,
                        c[1],
                        c[0],
                        y + h,
                        x,
                        c[1]
                    );
                }
                ShapeKind::Text => {}
                _ => {
                    let geometry = crate::geometry::shape(n.kind);
                    let coords = geometry
                        .outline
                        .iter()
                        .map(|p| format!("{},{}", x + p[0] * w, y + p[1] * h))
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(out, "<polygon points=\"{coords}\" {style}/>");
                    for fill in geometry.fills {
                        let coords = fill
                            .iter()
                            .map(|p| format!("{},{}", x + p[0] * w, y + p[1] * h))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let _ = writeln!(
                            out,
                            "<polygon points=\"{coords}\" fill=\"{}\"/>",
                            hex(n.stroke)
                        );
                    }
                    for line in geometry.details {
                        let coords = line
                            .iter()
                            .map(|p| format!("{},{}", x + p[0] * w, y + p[1] * h))
                            .collect::<Vec<_>>()
                            .join(" ");
                        let _ = writeln!(
                            out,
                            "<polyline points=\"{coords}\" fill=\"none\" stroke=\"{}\" stroke-width=\"{}\"/>",
                            hex(n.stroke),
                            n.stroke_width
                        );
                    }
                }
            }
        }
        let text = n.text_layout();
        let (text_origin, text_size) = crate::text_layout::area(n);
        let clip_text = n.kind != ShapeKind::Text;
        let start = if clip_text && text.height > text_size[1] {
            // Keep the beginning readable when a block is shorter than its text.
            text_origin[1] + (text.line_height + text.ascent + text.descent) / 2.
        } else {
            text.first_baseline(c[1])
        };
        if clip_text {
            let _ = writeln!(
                out,
                "<defs><clipPath id=\"diagcraft-text-{}\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/></clipPath></defs><g clip-path=\"url(#diagcraft-text-{})\">",
                n.id, text_origin[0], text_origin[1], text_size[0], text_size[1], n.id
            );
        }
        let (anchor, text_x) = match n.text_style.align {
            TextAlign::Left => ("start", text_origin[0]),
            TextAlign::Center => ("middle", c[0]),
            TextAlign::Right => ("end", text_origin[0] + text_size[0]),
        };
        let decoration = match (
            n.text_runs.is_empty() && n.text_style.underline,
            n.text_runs.is_empty() && n.text_style.strikethrough,
        ) {
            (true, true) => "underline line-through",
            (true, false) => "underline",
            (false, true) => "line-through",
            _ => "none",
        };
        let _ = write!(
            out,
            "<text text-anchor=\"{anchor}\" xml:space=\"preserve\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" font-weight=\"{}\" font-style=\"{}\" text-decoration=\"{decoration}\">",
            if n.text_style.monospace {
                "'Hack','Noto Sans SC',monospace"
            } else {
                "'Source Sans 3','Noto Sans SC',Arial,sans-serif"
            },
            n.font_size,
            hex(n.text_color),
            if n.text_style.bold { "700" } else { "400" },
            if n.text_style.italic {
                "italic"
            } else {
                "normal"
            }
        );
        for (i, line) in text.lines.iter().enumerate() {
            let _ = write!(
                out,
                "<tspan x=\"{}\" y=\"{}\">",
                text_x,
                start + i as f32 * text.line_height
            );
            if n.text_runs.is_empty() {
                out.push_str(&escape(line));
            } else {
                for run in &text.runs[i] {
                    let style = &run.style;
                    let decoration =
                        match (style.text_style.underline, style.text_style.strikethrough) {
                            (true, true) => "underline line-through",
                            (true, false) => "underline",
                            (false, true) => "line-through",
                            _ => "none",
                        };
                    let family = if style.text_style.monospace {
                        "'Hack','Noto Sans SC',monospace"
                    } else {
                        "'Source Sans 3','Noto Sans SC',Arial,sans-serif"
                    };
                    let _ = write!(
                        out,
                        "<tspan font-family=\"{}\" font-size=\"{}\" fill=\"{}\" font-weight=\"{}\" font-style=\"{}\" text-decoration=\"{}\">{}</tspan>",
                        family,
                        style.font_size,
                        hex(style.color),
                        if style.text_style.bold { "700" } else { "400" },
                        if style.text_style.italic {
                            "italic"
                        } else {
                            "normal"
                        },
                        decoration,
                        escape(&run.text)
                    );
                }
            }
            out.push_str("</tspan>");
        }
        out.push_str("</text>\n");
        if clip_text {
            out.push_str("</g>\n");
        }
    }
    for chart in doc.charts.iter().filter(|c| {
        organization.object_layer(c.id) == layer
            && organization.is_object_visible(c.id)
            && include(&c.id)
    }) {
        let _ = writeln!(
            out,
            "<g transform=\"translate({} {})\">",
            chart.position[0], chart.position[1]
        );
        for primitive in chart.primitives() {
            use crate::chart::Primitive::*;
            match primitive {
                Rect {
                    position,
                    size,
                    fill,
                } => {
                    let _ = writeln!(
                        out,
                        "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                        position[0],
                        position[1],
                        size[0],
                        size[1],
                        hex(fill)
                    );
                }
                Circle {
                    position,
                    radius,
                    color,
                } => {
                    let _ = writeln!(
                        out,
                        "<circle cx=\"{}\" cy=\"{}\" r=\"{radius}\" fill=\"{}\"/>",
                        position[0],
                        position[1],
                        hex(color)
                    );
                }
                Text {
                    position,
                    text,
                    size,
                    color,
                    align,
                } => {
                    let _ = writeln!(
                        out,
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" dominant-baseline=\"central\" font-family=\"'Source Sans 3','Noto Sans SC',Arial,sans-serif\" font-size=\"{size}\" fill=\"{}\">{}</text>",
                        position[0],
                        position[1],
                        match align {
                            TextAlign::Center => "middle",
                            TextAlign::Right => "end",
                            TextAlign::Left => "start",
                        },
                        hex(color),
                        escape(&text)
                    );
                }
                Path {
                    points,
                    fill,
                    stroke,
                    width,
                    closed,
                } => {
                    let coords = points
                        .iter()
                        .enumerate()
                        .map(|(i, p)| {
                            format!("{} {} {}", if i == 0 { "M" } else { "L" }, p[0], p[1])
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    let _ = writeln!(
                        out,
                        "<path d=\"{coords}{}\" fill=\"{}\" stroke=\"{}\" stroke-width=\"{width}\"/>",
                        if closed { " Z" } else { "" },
                        fill.map(hex).unwrap_or("none".into()),
                        stroke.map(hex).unwrap_or("none".into())
                    );
                }
            }
        }
        out.push_str("</g>\n");
    }
}

fn styled_arrowhead(
    out: &mut String,
    points: &[[f32; 2]],
    color: &str,
    width: f32,
    kind: crate::stroke_style::ArrowHead,
    background: Color,
) {
    if let Some(marker) = crate::stroke_style::marker(points, kind, 8. + width) {
        let coords = marker
            .points
            .iter()
            .map(|p| format!("{},{}", p[0], p[1]))
            .collect::<Vec<_>>()
            .join(" ");
        let tag = if marker.closed { "polygon" } else { "polyline" };
        let fill = if marker.filled {
            color.to_owned()
        } else if marker.closed {
            hex(background)
        } else {
            "none".into()
        };
        let stroke = if marker.filled { "none" } else { color };
        let _ = writeln!(
            out,
            "<{tag} points=\"{coords}\" fill=\"{fill}\" stroke=\"{stroke}\" stroke-width=\"{width}\" stroke-linejoin=\"round\" stroke-linecap=\"round\"/>"
        );
    }
}

#[cfg(test)]
mod accessibility_tests {
    use super::*;
    use crate::images::ImageAsset;
    use std::collections::HashSet;

    fn described_images() -> (Document, u64, u64, u64, u64) {
        let asset = ImageAsset::from_rgba(2, 2, &[[40, 80, 120, 255]; 4].concat()).unwrap();
        let mut doc = Document::default();
        let visible = doc.add_image(asset.clone(), [100., 100.]).unwrap();
        doc.node_mut(visible).unwrap().image.as_mut().unwrap().alt =
            "Diagramme & <détail> \"中文\"\u{1}".into();
        let second = doc.add_image(asset.clone(), [400., 100.]).unwrap();
        doc.node_mut(second).unwrap().image.as_mut().unwrap().alt = "Deuxième image".into();
        let hidden = doc.add_image(asset.clone(), [600., 100.]).unwrap();
        doc.node_mut(hidden).unwrap().image.as_mut().unwrap().alt = "Secret masqué".into();
        let layer = doc.create_layer("Privé".into()).unwrap();
        doc.move_objects_to_layer(&[hidden], layer).unwrap();
        doc.update_layer(layer, None, Some(false), None).unwrap();
        let outside = doc.add_image(asset, [2000., 100.]).unwrap();
        doc.node_mut(outside).unwrap().image.as_mut().unwrap().alt = "Hors cadre".into();
        (doc, visible, second, hidden, outside)
    }

    #[test]
    fn svg_describes_only_visible_images_inside_the_page() {
        let (doc, _, _, _, _) = described_images();
        let result = export(&doc).unwrap();
        assert!(result.contains(
            "<desc>Diagramme &amp; &lt;détail&gt; &quot;中文&quot;\nDeuxième image</desc>"
        ));
        assert!(!result.contains("Secret masqué"));
        assert!(!result.contains("Hors cadre"));
        assert!(!result.contains('\u{1}'));
        assert!(result.contains("role=\"img\">\n<title>"));
    }

    #[test]
    fn selected_and_cropped_exports_do_not_disclose_other_descriptions() {
        let (doc, visible, second, hidden, _) = described_images();
        let selected = HashSet::from([visible, hidden]);
        let source = export_view(&doc, [0., 0., 1280., 900.], true, Some(&selected)).unwrap();
        assert!(source.contains("Diagramme &amp; &lt;détail&gt;"));
        assert!(!source.contains("Deuxième image"));
        assert!(!source.contains("Secret masqué"));
        let crop = export_view(&doc, [350., 0., 200., 300.], true, None).unwrap();
        assert!(crop.contains("<desc>Deuxième image</desc>"));
        assert!(!crop.contains("Diagramme &amp;"));
        let empty = export_view(
            &doc,
            [0., 0., 1280., 900.],
            true,
            Some(&HashSet::from([hidden, second + 100_000])),
        )
        .unwrap();
        assert!(!empty.contains("<desc>"));
    }

    #[test]
    fn blank_or_xml_forbidden_descriptions_do_not_create_empty_metadata() {
        let (mut doc, visible, second, hidden, outside) = described_images();
        for id in [visible, second, hidden, outside] {
            doc.node_mut(id).unwrap().image.as_mut().unwrap().alt = " \t\u{1} ".into();
        }
        let svg = export(&doc).unwrap();
        assert!(!svg.contains("<desc>"));
        assert!(!svg.contains('\u{1}'));
    }

    #[test]
    fn image_descriptions_do_not_change_png_pixels_and_pdf_still_exports() {
        let (mut doc, visible, second, hidden, outside) = described_images();
        let options = crate::raster::PngOptions {
            width: 128,
            height: 90,
            ..Default::default()
        };
        let with_descriptions = crate::raster::export(&doc, &options).unwrap();
        for id in [visible, second, hidden, outside] {
            doc.node_mut(id)
                .unwrap()
                .image
                .as_mut()
                .unwrap()
                .alt
                .clear();
        }
        let without_descriptions = crate::raster::export(&doc, &options).unwrap();
        assert_eq!(with_descriptions, without_descriptions);
        doc.node_mut(visible).unwrap().image.as_mut().unwrap().alt = "Accessible".into();
        let pdf = crate::pdf::export_current(&doc, crate::pdf::PageFormat::Drawing).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
    }
}
