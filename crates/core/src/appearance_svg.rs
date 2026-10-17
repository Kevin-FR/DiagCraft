//! Shared SVG bodies for styled shapes, native previews and portable exports.
use crate::{
    appearance::*,
    document::{Color, Connector, Node, Point, ShapeKind},
    svg::hex,
};
use std::fmt::Write;

fn gradient(
    id: &str,
    a: Color,
    b: Color,
    angle: f32,
    radial: bool,
    bounds: Option<[f32; 4]>,
) -> String {
    if radial {
        format!(
            "<radialGradient id=\"{id}\"><stop stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></radialGradient>",
            hex(a),
            hex(b)
        )
    } else {
        let t = angle.to_radians();
        let (dx, dy) = (t.cos() / 2., t.sin() / 2.);
        let [x, y, w, h] = bounds
            .map(|b| [b[0], b[1], (b[2] - b[0]).max(1.), (b[3] - b[1]).max(1.)])
            .unwrap_or([0., 0., 1., 1.]);
        let units = if bounds.is_some() {
            " gradientUnits=\"userSpaceOnUse\""
        } else {
            ""
        };
        format!(
            "<linearGradient id=\"{id}\"{units} x1=\"{}\" y1=\"{}\" x2=\"{}\" y2=\"{}\"><stop stop-color=\"{}\"/><stop offset=\"1\" stop-color=\"{}\"/></linearGradient>",
            x + (0.5 - dx) * w,
            y + (0.5 - dy) * h,
            x + (0.5 + dx) * w,
            y + (0.5 + dy) * h,
            hex(a),
            hex(b)
        )
    }
}
fn paint(
    defs: &mut String,
    id: &str,
    mode: PaintMode,
    base: Color,
    secondary: Color,
    tint: f32,
    angle: f32,
) -> String {
    let a = tinted(base, tint);
    match mode {
        PaintMode::None => "none".into(),
        PaintMode::Linear | PaintMode::Radial | PaintMode::Monochrome => {
            let b = if mode == PaintMode::Monochrome {
                tinted(a, 65.)
            } else {
                tinted(secondary, tint)
            };
            defs.push_str(&gradient(id, a, b, angle, mode == PaintMode::Radial, None));
            format!("url(#{id})")
        }
        _ => hex(a),
    }
}
fn fill(defs: &mut String, id: &str, style: &Fill, color: Color) -> String {
    if !matches!(style.mode, PaintMode::Pattern | PaintMode::Texture) {
        return paint(
            defs,
            id,
            style.mode,
            color,
            style.secondary,
            style.tint,
            style.angle,
        );
    }
    let size = style.spacing;
    let second = hex(tinted(style.secondary, style.tint));
    let detail = if style.mode == PaintMode::Pattern {
        match style.pattern {
            PatternKind::Diagonal => {
                format!("<path d=\"M 0 {size} L {size} 0\" stroke=\"{second}\"/>")
            }
            PatternKind::Cross => {
                format!("<path d=\"M 0 0 H {size} M 0 0 V {size}\" stroke=\"{second}\"/>")
            }
            PatternKind::Horizontal => format!("<path d=\"M 0 0 H {size}\" stroke=\"{second}\"/>"),
            PatternKind::Dots => format!(
                "<circle cx=\"{}\" cy=\"{}\" r=\"1.5\" fill=\"{second}\"/>",
                size / 2.,
                size / 2.
            ),
        }
    } else {
        match style.texture {
            TextureKind::Canvas => format!(
                "<path d=\"M 0 2 H {size} M 0 5 H {size} M 2 0 V {size} M 5 0 V {size}\" stroke=\"{second}\" opacity=\"0.22\"/>"
            ),
            TextureKind::Paper | TextureKind::Grain => (0..24)
                .map(|i| {
                    let x = ((i * 37 + 11) % 97) as f32 / 97. * size;
                    let y = ((i * 53 + 7) % 89) as f32 / 89. * size;
                    format!(
                        "<circle cx=\"{x}\" cy=\"{y}\" r=\"{}\" fill=\"{second}\" opacity=\"{}\"/>",
                        if style.texture == TextureKind::Grain {
                            0.7
                        } else {
                            0.35
                        },
                        if style.texture == TextureKind::Grain {
                            0.45
                        } else {
                            0.2
                        }
                    )
                })
                .collect(),
        }
    };
    let _ = write!(
        defs,
        "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{size}\" height=\"{size}\" patternTransform=\"rotate({})\">{detail}</pattern>",
        style.angle
    );
    format!("url(#{id})")
}
pub fn path(points: &[Point], closed: bool) -> String {
    let mut s = String::new();
    for (i, p) in points.iter().enumerate() {
        let _ = write!(s, "{} {} {} ", if i == 0 { "M" } else { "L" }, p[0], p[1]);
    }
    if closed {
        s.push('Z');
    }
    s
}
fn node_path(n: &Node) -> String {
    let [w, h] = n.size;
    match n.kind {
        ShapeKind::Rectangle | ShapeKind::Rounded => {
            let r = n
                .appearance
                .line
                .corner_radius
                .unwrap_or(if n.kind == ShapeKind::Rounded {
                    20.
                } else {
                    2.
                })
                .min(w / 2.)
                .min(h / 2.);
            format!(
                "M {r} 0 H {} Q {w} 0 {w} {r} V {} Q {w} {h} {} {h} H {r} Q 0 {h} 0 {} V {r} Q 0 0 {r} 0 Z",
                w - r,
                h - r,
                w - r,
                h - r
            )
        }
        ShapeKind::Ellipse => format!(
            "M 0 {} A {} {} 0 1 0 {w} {} A {} {} 0 1 0 0 {} Z",
            h / 2.,
            w / 2.,
            h / 2.,
            h / 2.,
            w / 2.,
            h / 2.,
            h / 2.
        ),
        _ => path(
            &crate::geometry::shape(n.kind)
                .outline
                .iter()
                .map(|p| [p[0] * w, p[1] * h])
                .collect::<Vec<_>>(),
            true,
        ),
    }
}
fn line(
    defs: &mut String,
    id: &str,
    geometry: (&str, [f32; 4]),
    style: &Line,
    color: Color,
    width: f32,
    pattern: crate::stroke_style::LinePattern,
) -> String {
    if style.mode == PaintMode::None || width == 0. {
        return String::new();
    }
    let (d, bounds) = geometry;
    let a = tinted(color, style.tint);
    let paint = if matches!(style.mode, PaintMode::Linear | PaintMode::Monochrome) {
        let b = if style.mode == PaintMode::Monochrome {
            tinted(a, 65.)
        } else {
            tinted(style.secondary, style.tint)
        };
        defs.push_str(&gradient(
            &format!("{id}-color"),
            a,
            b,
            style.angle,
            false,
            Some(bounds),
        ));
        format!("url(#{id}-color)")
    } else {
        hex(a)
    };
    let cap = match style.cap {
        Cap::Flat => "butt",
        Cap::Round => "round",
        Cap::Square => "square",
    };
    let join = match style.join {
        Join::Miter => "miter",
        Join::Round => "round",
        Join::Bevel => "bevel",
    };
    let dash = pattern
        .dash(width)
        .map(|mut d| {
            // Zero-length SVG dashes disappear with flat caps.
            if d[0] == 0. && style.cap != Cap::Round {
                d[0] = if style.cap == Cap::Flat {
                    width
                } else {
                    width * 0.01
                };
                d[1] -= d[0];
            }
            d
        })
        .map(|v| format!(" stroke-dasharray=\"{} {}\"", v[0], v[1]))
        .unwrap_or_default();
    let attrs = format!("fill=\"none\" stroke-linecap=\"{cap}\" stroke-linejoin=\"{join}\"{dash}");
    let mask = if style.compound == Compound::Single {
        String::new()
    } else {
        let _ = write!(
            defs,
            "<mask id=\"{id}-compound\" maskUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"><path d=\"{d}\" stroke=\"white\" stroke-width=\"{width}\" {attrs}/><path d=\"{d}\" stroke=\"black\" stroke-width=\"{}\" {attrs}/>",
            bounds[0] - width * 2.,
            bounds[1] - width * 2.,
            (bounds[2] - bounds[0]).max(1.) + width * 4.,
            (bounds[3] - bounds[1]).max(1.) + width * 4.,
            width * 0.45
        );
        if style.compound == Compound::Triple {
            let _ = write!(
                defs,
                "<path d=\"{d}\" stroke=\"white\" stroke-width=\"{}\" {attrs}/>",
                width * 0.16
            );
        }
        defs.push_str("</mask>");
        format!(" mask=\"url(#{id}-compound)\"")
    };
    format!(
        "<path d=\"{d}\" stroke=\"{paint}\" stroke-width=\"{width}\" opacity=\"{}\" {attrs}{mask}/>",
        1. - style.transparency / 100.
    )
}
fn shadow_transform(n: &Node) -> String {
    let s = &n.appearance.shadow;
    let [w, h] = n.size;
    let scale = s.scale / 100.;
    let cy = if s.kind == ShadowKind::Perspective {
        h
    } else {
        h / 2.
    };
    let sy = if s.kind == ShadowKind::Perspective {
        scale * 0.25
    } else {
        scale
    };
    format!(
        "translate({} {}) rotate({}) scale({scale} {sy}) translate({} {})",
        w / 2. + s.offset[0],
        cy + s.offset[1],
        s.angle,
        -w / 2.,
        -cy
    )
}
pub fn node_bounds(n: &Node) -> [f32; 4] {
    let pad = n.stroke_width * 2. + 2.;
    let [w, h] = n.size;
    let mut b = [-pad, -pad, w + pad, h + pad];
    let s = &n.appearance.shadow;
    if matches!(s.kind, ShadowKind::Outer | ShadowKind::Perspective) {
        let cy = if s.kind == ShadowKind::Perspective {
            h
        } else {
            h / 2.
        };
        let scale = s.scale / 100.;
        let sy = if s.kind == ShadowKind::Perspective {
            scale * 0.25
        } else {
            scale
        };
        let a = s.angle.to_radians();
        let blur = s.blur * 3. + pad * scale;
        for [x, y] in [[0., 0.], [w, 0.], [w, h], [0., h]] {
            let x = (x - w / 2.) * scale;
            let y = (y - cy) * sy;
            let x2 = x * a.cos() - y * a.sin() + w / 2. + s.offset[0];
            let y2 = x * a.sin() + y * a.cos() + cy + s.offset[1];
            b[0] = b[0].min(x2 - blur);
            b[1] = b[1].min(y2 - blur);
            b[2] = b[2].max(x2 + blur);
            b[3] = b[3].max(y2 + blur);
        }
    }
    b
}
pub fn node(n: &Node) -> String {
    if n.kind == ShapeKind::Text {
        return String::new();
    }
    let id = format!("appearance-node-{}", n.id);
    let a = &n.appearance;
    let mut defs = String::new();
    let d = node_path(n);
    let f = fill(&mut defs, &format!("{id}-fill"), &a.fill, n.fill);
    let mut body = if matches!(a.fill.mode, PaintMode::Pattern | PaintMode::Texture) {
        // One continuous base avoids PDF tiling seams. Group opacity keeps
        // marks and background at the same alpha, including at tile boundaries.
        format!(
            "<g opacity=\"{}\"><path d=\"{d}\" fill=\"{}\"/><path d=\"{d}\" fill=\"{f}\"/></g>",
            1. - a.fill.transparency / 100.,
            hex(tinted(n.fill, a.fill.tint))
        )
    } else {
        format!(
            "<path d=\"{d}\" fill=\"{f}\" fill-opacity=\"{}\"/>",
            1. - a.fill.transparency / 100.
        )
    };
    body.push_str(&line(
        &mut defs,
        &format!("{id}-line"),
        (&d, [0., 0., n.size[0], n.size[1]]),
        &a.line,
        n.stroke,
        n.stroke_width,
        a.line.pattern,
    ));
    if !matches!(
        n.kind,
        ShapeKind::Rectangle | ShapeKind::Rounded | ShapeKind::Ellipse | ShapeKind::Diamond
    ) {
        let g = crate::geometry::shape(n.kind);
        for (i, detail) in g.details.iter().enumerate() {
            let points = detail
                .iter()
                .map(|p| [p[0] * n.size[0], p[1] * n.size[1]])
                .collect::<Vec<_>>();
            body.push_str(&line(
                &mut defs,
                &format!("{id}-detail-{i}"),
                (&path(&points, false), [0., 0., n.size[0], n.size[1]]),
                &a.line,
                n.stroke,
                n.stroke_width,
                a.line.pattern,
            ));
        }
        if a.line.mode != PaintMode::None {
            for polygon in g.fills {
                let points = polygon
                    .iter()
                    .map(|p| [p[0] * n.size[0], p[1] * n.size[1]])
                    .collect::<Vec<_>>();
                let _ = write!(
                    body,
                    "<path d=\"{}\" fill=\"{}\" opacity=\"{}\"/>",
                    path(&points, true),
                    hex(tinted(n.stroke, a.line.tint)),
                    1. - a.line.transparency / 100.
                );
            }
        }
    }
    let s = &a.shadow;
    let mut content = String::new();
    if s.kind != ShadowKind::None {
        let b = node_bounds(n);
        let x = b[0];
        let y = b[1];
        let w = b[2] - x;
        let h = b[3] - y;
        let extra = s.blur * 3. + 2.;
        let filter_bounds = format!(
            "filterUnits=\"userSpaceOnUse\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" color-interpolation-filters=\"sRGB\"",
            x - extra,
            y - extra,
            w + extra * 2.,
            h + extra * 2.
        );
        // Shape alpha, not its bounding rectangle, determines every shadow.
        let _ = write!(
            defs,
            "<g id=\"{id}-body\">{body}</g><filter id=\"{id}-shadow\" {filter_bounds}><feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"blurred\"/><feFlood flood-color=\"{}\" flood-opacity=\"{}\"/><feComposite in2=\"blurred\" operator=\"in\"/></filter>",
            s.blur / 2.,
            hex(s.color),
            1. - s.transparency / 100.
        );
        if s.kind == ShadowKind::Inner {
            let _ = write!(
                defs,
                "<filter id=\"{id}-black\" {filter_bounds}><feGaussianBlur in=\"SourceAlpha\" stdDeviation=\"{}\" result=\"alpha\"/><feFlood flood-color=\"black\"/><feComposite in2=\"alpha\" operator=\"in\"/></filter><filter id=\"{id}-white\" {filter_bounds}><feFlood flood-color=\"white\"/><feComposite in2=\"SourceAlpha\" operator=\"in\"/></filter><mask id=\"{id}-clip\" maskUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"><use href=\"#{id}-body\" filter=\"url(#{id}-white)\"/></mask><mask id=\"{id}-inner\" maskUnits=\"userSpaceOnUse\" x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"white\"/><g filter=\"url(#{id}-black)\"><use href=\"#{id}-body\" transform=\"{}\"/></g></mask>",
                s.blur / 2.,
                shadow_transform(n)
            );
            let _ = write!(
                content,
                "<use href=\"#{id}-body\"/><g mask=\"url(#{id}-clip)\"><rect x=\"{x}\" y=\"{y}\" width=\"{w}\" height=\"{h}\" fill=\"{}\" opacity=\"{}\" mask=\"url(#{id}-inner)\"/></g>",
                hex(s.color),
                1. - s.transparency / 100.
            );
        } else {
            let _ = write!(
                content,
                "<g filter=\"url(#{id}-shadow)\"><use href=\"#{id}-body\" transform=\"{}\"/></g><use href=\"#{id}-body\"/>",
                shadow_transform(n)
            );
        }
    } else {
        content = body;
    }
    format!(
        "<g transform=\"translate({} {})\"><defs>{defs}</defs>{content}</g>\n",
        n.position[0], n.position[1]
    )
}
pub fn node_image(n: &Node) -> (String, [f32; 4]) {
    let mut local = n.clone();
    local.id = 1;
    local.position = [0., 0.];
    let b = node_bounds(&local);
    let source = wrap(&node(&local), b);
    (source, b)
}
pub fn wrap(content: &str, b: [f32; 4]) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"{} {} {} {}\">{content}</svg>",
        b[2] - b[0],
        b[3] - b[1],
        b[0],
        b[1],
        b[2] - b[0],
        b[3] - b[1]
    )
}

pub fn connector(edge: &Connector, points: &[Point], background: Color) -> String {
    let mut defs = String::new();
    let id = format!("appearance-edge-{}", edge.id);
    let s = &edge.appearance;
    let bounds = points.iter().fold(
        [
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ],
        |mut b, p| {
            b[0] = b[0].min(p[0]);
            b[1] = b[1].min(p[1]);
            b[2] = b[2].max(p[0]);
            b[3] = b[3].max(p[1]);
            b
        },
    );
    let mut content = line(
        &mut defs,
        &id,
        (&path(points, false), bounds),
        s,
        edge.color,
        edge.width,
        edge.pattern,
    );
    if s.mode != PaintMode::None {
        let color = if matches!(s.mode, PaintMode::Linear | PaintMode::Monochrome) {
            format!("url(#{id}-color)")
        } else {
            hex(tinted(edge.color, s.tint))
        };
        for (enabled, head, scale, pts) in [
            (edge.arrow, edge.end_head, s.end_scale, points.to_vec()),
            (
                edge.arrow_start,
                edge.start_head,
                s.start_scale,
                points.iter().rev().copied().collect(),
            ),
        ] {
            if enabled
                && let Some(m) = crate::stroke_style::marker(&pts, head, (8. + edge.width) * scale)
            {
                let d = path(&m.points, m.closed);
                let fill = if m.filled {
                    color.clone()
                } else if m.closed {
                    hex(background)
                } else {
                    "none".into()
                };
                let _ = write!(
                    content,
                    "<path d=\"{d}\" fill=\"{fill}\" stroke=\"{color}\" stroke-width=\"{}\" opacity=\"{}\" stroke-linejoin=\"round\"/>",
                    if m.filled { 0. } else { edge.width },
                    1. - s.transparency / 100.
                );
            }
        }
    }
    format!("<defs>{defs}</defs>{content}")
}

pub fn drawing_edge(d: &crate::document::Drawing) -> Connector {
    Connector {
        id: d.id,
        from: 0,
        to: 0,
        free_start: Some(d.start),
        free_end: Some(d.end),
        kind: crate::document::ConnectorKind::Straight,
        label: String::new(),
        color: d.color,
        width: d.width,
        arrow: d.arrow,
        arrow_start: false,
        from_port: Default::default(),
        to_port: Default::default(),
        waypoints: vec![],
        label_t: 0.5,
        pattern: d.appearance.pattern,
        start_head: Default::default(),
        end_head: Default::default(),
        appearance: d.appearance.clone(),
    }
}
