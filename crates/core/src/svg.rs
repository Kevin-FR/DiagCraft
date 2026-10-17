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
    doc.validate()?;
    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\" role=\"img\">\n<title>{}</title>\n<rect width=\"100%\" height=\"100%\" fill=\"{}\"/>\n",
        doc.page_size[0],
        doc.page_size[1],
        doc.page_size[0],
        doc.page_size[1],
        escape(&doc.title),
        hex(doc.page_style.background)
    );
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
    for drawing in &doc.drawings {
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
    }
    for edge in &doc.connectors {
        let points = doc.route(edge);
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
                    &mut out,
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
                    &mut out,
                    &reverse,
                    &color,
                    edge.width,
                    edge.start_head,
                    doc.page_style.background,
                );
            }
        }
        if !edge.label.is_empty() && points.len() >= 2 {
            let pos = doc.edge_label_point(edge);
            let label_width = edge.label.chars().count() as f32 * 8. + 8.;
            let _ = writeln!(
                out,
                "<rect x=\"{}\" y=\"{}\" width=\"{label_width}\" height=\"20\" rx=\"2\" fill=\"white\"/>",
                pos[0] - label_width / 2.,
                pos[1] - 15.
            );
            let _ = writeln!(
                out,
                "<text x=\"{}\" y=\"{}\" text-anchor=\"middle\" font-family=\"'Source Sans 3',Arial,sans-serif\" font-size=\"14\" fill=\"{color}\">{}</text>",
                pos[0],
                pos[1],
                escape(&edge.label)
            );
        }
    }
    for n in &doc.nodes {
        let [x, y] = n.position;
        let [w, h] = n.size;
        let c = n.center();
        let style = format!(
            "fill=\"{}\" stroke=\"{}\" stroke-width=\"{}\"",
            hex(n.fill),
            hex(n.stroke),
            n.stroke_width
        );
        if !n.appearance.is_default() {
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
        let display = n.display_text();
        let lines: Vec<_> = display.split('\n').collect();
        let start = c[1] - (lines.len() as f32 - 1.) * n.font_size * 0.625;
        let (anchor, text_x) = match n.text_style.align {
            TextAlign::Left => ("start", x + 12.),
            TextAlign::Center => ("middle", c[0]),
            TextAlign::Right => ("end", x + w - 12.),
        };
        let decoration = match (n.text_style.underline, n.text_style.strikethrough) {
            (true, true) => "underline line-through",
            (true, false) => "underline",
            (false, true) => "line-through",
            _ => "none",
        };
        let _ = write!(
            out,
            "<text text-anchor=\"{anchor}\" dominant-baseline=\"central\" font-family=\"{}\" font-size=\"{}\" fill=\"{}\" font-weight=\"{}\" font-style=\"{}\" text-decoration=\"{decoration}\">",
            if n.text_style.monospace {
                "monospace"
            } else {
                "'Source Sans 3',Arial,sans-serif"
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
        for (i, line) in lines.iter().enumerate() {
            let _ = write!(
                out,
                "<tspan x=\"{}\" y=\"{}\">{}</tspan>",
                text_x,
                start + i as f32 * n.font_size * 1.25,
                escape(line)
            );
        }
        out.push_str("</text>\n");
    }
    for chart in &doc.charts {
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
                        "<text x=\"{}\" y=\"{}\" text-anchor=\"{}\" dominant-baseline=\"central\" font-family=\"'Source Sans 3',Arial,sans-serif\" font-size=\"{size}\" fill=\"{}\">{}</text>",
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
    out.push_str("</svg>\n");
    Ok(out)
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
