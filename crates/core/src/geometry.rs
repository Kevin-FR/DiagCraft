//! Shared, original vector geometry for the symbol palette, canvas and SVG.
use crate::document::{Point, ShapeKind};

pub struct Geometry {
    pub outline: Vec<Point>,
    pub details: Vec<Vec<Point>>,
    pub fills: Vec<Vec<Point>>,
}
pub fn shape(kind: ShapeKind) -> Geometry {
    use ShapeKind::*;
    let mut details = vec![];
    let mut fills = vec![];
    let outline = match kind {
        Diamond => vec![[0.5, 0.], [1., 0.5], [0.5, 1.], [0., 0.5]],
        Data => vec![[0.18, 0.], [1., 0.], [0.82, 1.], [0., 1.]],
        Hexagon => vec![
            [0.2, 0.],
            [0.8, 0.],
            [1., 0.5],
            [0.8, 1.],
            [0.2, 1.],
            [0., 0.5],
        ],
        Trapezoid => vec![[0.15, 0.], [0.85, 0.], [1., 1.], [0., 1.]],
        ManualOperation => vec![[0., 0.], [1., 0.], [0.85, 1.], [0.15, 1.]],
        ManualInput => vec![[0., 0.25], [1., 0.], [1., 1.], [0., 1.]],
        OffPage => vec![[0., 0.], [1., 0.], [1., 0.65], [0.5, 1.], [0., 0.65]],
        Document => {
            let mut p = vec![[0., 0.], [1., 0.], [1., 0.86]];
            for i in 1..=24 {
                let x = 1. - i as f32 / 24.;
                p.push([x, 0.86 + 0.1 * (x * std::f32::consts::TAU).sin()]);
            }
            p
        }
        Database => {
            let mut p = vec![];
            for i in 0..=24 {
                let a = std::f32::consts::PI + i as f32 / 24. * std::f32::consts::PI;
                p.push([0.5 + 0.5 * a.cos(), 0.14 + 0.14 * a.sin()]);
            }
            for i in 0..=24 {
                let a = i as f32 / 24. * std::f32::consts::PI;
                p.push([0.5 + 0.5 * a.cos(), 0.86 + 0.14 * a.sin()]);
            }
            details.push(
                (0..=24)
                    .map(|i| {
                        let a = i as f32 / 24. * std::f32::consts::PI;
                        [0.5 + 0.5 * a.cos(), 0.14 + 0.14 * a.sin()]
                    })
                    .collect(),
            );
            p
        }
        PredefinedProcess => {
            details.push(vec![[0.12, 0.], [0.12, 1.]]);
            details.push(vec![[0.88, 0.], [0.88, 1.]]);
            rect()
        }
        InternalStorage => {
            details.push(vec![[0.14, 0.], [0.14, 1.]]);
            details.push(vec![[0., 0.18], [1., 0.18]]);
            rect()
        }
        Delay => {
            let mut p = vec![[0., 0.], [0.65, 0.]];
            for i in 0..=24 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 24. * std::f32::consts::PI;
                p.push([0.65 + 0.35 * a.cos(), 0.5 + 0.5 * a.sin()]);
            }
            p.push([0., 1.]);
            p
        }
        Display => {
            let mut p = vec![[0.2, 0.], [0.8, 0.]];
            for i in 0..=24 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 24. * std::f32::consts::PI;
                p.push([0.8 + 0.2 * a.cos(), 0.5 + 0.5 * a.sin()]);
            }
            p.extend([[0.2, 1.], [0., 0.5]]);
            p
        }
        Terminator => {
            let mut p = vec![];
            for i in 0..=20 {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 / 20. * std::f32::consts::PI;
                p.push([0.75 + 0.25 * a.cos(), 0.5 + 0.5 * a.sin()]);
            }
            for i in 0..=20 {
                let a = std::f32::consts::FRAC_PI_2 + i as f32 / 20. * std::f32::consts::PI;
                p.push([0.25 + 0.25 * a.cos(), 0.5 + 0.5 * a.sin()]);
            }
            p
        }
        Note => {
            details.push(vec![[0.8, 0.], [0.8, 0.2], [1., 0.2]]);
            vec![[0., 0.], [0.8, 0.], [1., 0.2], [1., 1.], [0., 1.]]
        }
        Callout => vec![
            [0., 0.],
            [1., 0.],
            [1., 0.78],
            [0.4, 0.78],
            [0.15, 1.],
            [0.23, 0.78],
            [0., 0.78],
        ],
        Star => (0..10)
            .map(|i| {
                let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.;
                let r = if i % 2 == 0 { 0.5 } else { 0.22 };
                [0.5 + r * a.cos(), 0.5 + r * a.sin()]
            })
            .collect(),
        ArrowRight | ArrowLeft | ArrowUp | ArrowDown => {
            let p = right_arrow();
            p.into_iter()
                .map(|[x, y]| match kind {
                    ArrowLeft => [1. - x, y],
                    ArrowUp => [y, 1. - x],
                    ArrowDown => [y, x],
                    _ => [x, y],
                })
                .collect()
        }
        DoubleArrow => vec![
            [0., 0.5],
            [0.25, 0.],
            [0.25, 0.3],
            [0.75, 0.3],
            [0.75, 0.],
            [1., 0.5],
            [0.75, 1.],
            [0.75, 0.7],
            [0.25, 0.7],
            [0.25, 1.],
        ],
        Chevron => vec![
            [0., 0.],
            [0.62, 0.],
            [1., 0.5],
            [0.62, 1.],
            [0., 1.],
            [0.38, 0.5],
        ],
        NotchedArrow => vec![
            [0., 0.27],
            [0.62, 0.27],
            [0.62, 0.],
            [1., 0.5],
            [0.62, 1.],
            [0.62, 0.73],
            [0., 0.73],
            [0.18, 0.5],
        ],
        ElbowArrow => vec![
            [0., 0.],
            [0.28, 0.],
            [0.28, 0.45],
            [0.65, 0.45],
            [0.65, 0.2],
            [1., 0.6],
            [0.65, 1.],
            [0.65, 0.75],
            [0., 0.75],
        ],
        Triangle | Pyramid => {
            if kind == Pyramid {
                details.push(vec![[0.5, 0.], [0.55, 0.82], [0., 1.]]);
                details.push(vec![[0.55, 0.82], [1., 1.]]);
            }
            vec![[0.5, 0.], [1., 1.], [0., 1.]]
        }
        RightTriangle => vec![[0., 0.], [1., 1.], [0., 1.]],
        Pentagon => radial(5, 0.5, 0.5),
        Octagon => radial(8, 0.5, 0.5),
        Star4 => radial(8, 0.5, 0.16),
        Star8 => radial(16, 0.5, 0.29),
        Gear => {
            details.push(ellipse([0.5, 0.5], [0.18, 0.18]));
            radial(32, 0.5, 0.4)
        }
        Plus | Cross => {
            let p = vec![
                [0.35, 0.],
                [0.65, 0.],
                [0.65, 0.35],
                [1., 0.35],
                [1., 0.65],
                [0.65, 0.65],
                [0.65, 1.],
                [0.35, 1.],
                [0.35, 0.65],
                [0., 0.65],
                [0., 0.35],
                [0.35, 0.35],
            ];
            if kind == Cross {
                p.into_iter()
                    .map(|[x, y]| [0.5 + (x - y) * 0.5, 0.5 + (x + y - 1.) * 0.5])
                    .collect()
            } else {
                p
            }
        }
        Heart => (0..64)
            .map(|i| {
                let t = i as f32 / 64. * std::f32::consts::TAU;
                [
                    0.5 + 0.5 * t.sin().powi(3),
                    0.5 - (13. * t.cos()
                        - 5. * (2. * t).cos()
                        - 2. * (3. * t).cos()
                        - (4. * t).cos())
                        / 34.,
                ]
            })
            .collect(),
        Cloud => (0..80)
            .map(|i| {
                let t = i as f32 / 80. * std::f32::consts::TAU;
                let r = 0.44 + 0.055 * (7. * t).cos();
                [0.5 + r * t.cos(), 0.5 + r * t.sin()]
            })
            .collect(),
        Shield => vec![
            [0., 0.],
            [1., 0.],
            [0.94, 0.55],
            [0.78, 0.82],
            [0.5, 1.],
            [0.22, 0.82],
            [0.06, 0.55],
        ],
        Drop => {
            let mut p = vec![[0.5, 0.]];
            p.extend((0..=32).map(|i| {
                let a = i as f32 / 32. * std::f32::consts::PI;
                [0.5 + 0.5 * a.cos(), 0.55 + 0.45 * a.sin()]
            }));
            p
        }
        Frame => {
            details.push(vec![
                [0.1, 0.12],
                [0.9, 0.12],
                [0.9, 0.88],
                [0.1, 0.88],
                [0.1, 0.12],
            ]);
            rect()
        }
        Swimlane => {
            details.push(vec![[0.16, 0.], [0.16, 1.]]);
            rect()
        }
        TitleBlock => {
            details.push(vec![[0., 0.35], [1., 0.35]]);
            details.push(vec![[0.5, 0.35], [0.5, 1.]]);
            rect()
        }
        UmlClass => {
            details.push(vec![[0., 0.33], [1., 0.33]]);
            details.push(vec![[0., 0.67], [1., 0.67]]);
            rect()
        }
        Component => {
            details.push(vec![
                [0.05, 0.23],
                [0.25, 0.23],
                [0.25, 0.4],
                [0.05, 0.4],
                [0.05, 0.23],
            ]);
            details.push(vec![
                [0.05, 0.6],
                [0.25, 0.6],
                [0.25, 0.77],
                [0.05, 0.77],
                [0.05, 0.6],
            ]);
            rect()
        }
        Package => {
            details.push(vec![[0., 0.2], [1., 0.2]]);
            vec![
                [0., 0.],
                [0.4, 0.],
                [0.4, 0.2],
                [1., 0.2],
                [1., 1.],
                [0., 1.],
            ]
        }
        Actor => {
            details.push(ellipse([0.5, 0.14], [0.13, 0.13]));
            vec![
                [0.44, 0.29],
                [0.56, 0.29],
                [0.56, 0.38],
                [0.95, 0.38],
                [0.95, 0.45],
                [0.56, 0.45],
                [0.56, 0.66],
                [0.82, 0.97],
                [0.74, 1.],
                [0.5, 0.74],
                [0.26, 1.],
                [0.18, 0.97],
                [0.44, 0.66],
                [0.44, 0.45],
                [0.05, 0.45],
                [0.05, 0.38],
                [0.44, 0.38],
            ]
        }
        Lifeline => vec![
            [0., 0.],
            [1., 0.],
            [1., 0.22],
            [0.52, 0.22],
            [0.52, 1.],
            [0.48, 1.],
            [0.48, 0.22],
            [0., 0.22],
        ],
        Fork => rect(),
        InitialState | FinalState | UseCase => {
            if kind == FinalState {
                fills.push(ellipse([0.5, 0.5], [0.33, 0.33]));
            }
            ellipse([0.5, 0.5], [0.5, 0.5])
        }
        Cube => {
            details.push(vec![[0., 0.22], [0.72, 0.22], [1., 0.]]);
            details.push(vec![[0.72, 0.22], [0.72, 1.]]);
            vec![
                [0.28, 0.],
                [1., 0.],
                [1., 0.78],
                [0.72, 1.],
                [0., 1.],
                [0., 0.22],
            ]
        }
        Cone => {
            details.push(
                (0..=24)
                    .map(|i| {
                        let t = i as f32 / 24. * std::f32::consts::PI;
                        [0.5 + 0.5 * t.cos(), 0.86 - 0.14 * t.sin()]
                    })
                    .collect(),
            );
            let mut p = vec![[0.5, 0.], [1., 0.86]];
            p.extend((0..=24).map(|i| {
                let t = i as f32 / 24. * std::f32::consts::PI;
                [0.5 + 0.5 * t.cos(), 0.86 + 0.14 * t.sin()]
            }));
            p
        }
        BraceLeft | BraceRight => {
            let p = vec![
                [0.9, 0.],
                [0.6, 0.],
                [0.45, 0.1],
                [0.45, 0.35],
                [0.15, 0.5],
                [0.45, 0.65],
                [0.45, 0.9],
                [0.6, 1.],
                [0.9, 1.],
                [0.9, 0.88],
                [0.6, 0.88],
                [0.6, 0.6],
                [0.7, 0.5],
                [0.6, 0.4],
                [0.6, 0.12],
                [0.9, 0.12],
            ];
            if kind == BraceRight {
                p.into_iter().map(|[x, y]| [1. - x, y]).collect()
            } else {
                p
            }
        }
        Lightning => vec![
            [0.6, 0.],
            [0.1, 0.58],
            [0.47, 0.58],
            [0.35, 1.],
            [0.9, 0.4],
            [0.53, 0.4],
        ],
        _ => rect(),
    };
    Geometry {
        outline: clean(outline),
        details,
        fills,
    }
}
fn radial(count: usize, outer: f32, inner: f32) -> Vec<Point> {
    (0..count)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::TAU / count as f32;
            let r = if i % 2 == 0 { outer } else { inner };
            [0.5 + r * a.cos(), 0.5 + r * a.sin()]
        })
        .collect()
}
fn ellipse(c: Point, r: Point) -> Vec<Point> {
    (0..=48)
        .map(|i| {
            let a = i as f32 / 48. * std::f32::consts::TAU;
            [c[0] + r[0] * a.cos(), c[1] + r[1] * a.sin()]
        })
        .collect()
}
fn rect() -> Vec<Point> {
    vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]]
}
fn right_arrow() -> Vec<Point> {
    vec![
        [0., 0.27],
        [0.62, 0.27],
        [0.62, 0.],
        [1., 0.5],
        [0.62, 1.],
        [0.62, 0.73],
        [0., 0.73],
    ]
}
fn clean(mut p: Vec<Point>) -> Vec<Point> {
    p.dedup_by(|a, b| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() < 0.00001);
    if p.len() > 1
        && (p[0][0] - p[p.len() - 1][0]).abs() + (p[0][1] - p[p.len() - 1][1]).abs() < 0.00001
    {
        p.pop();
    }
    p
}
pub fn contains(points: &[Point], p: Point) -> bool {
    let mut inside = false;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        if crate::document::segment_distance(p, *a, *b) < 0.00001 {
            return true;
        }
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            inside = !inside;
        }
    }
    inside
}
pub fn boundary(points: &[Point], towards: Point) -> Point {
    let center = if points.len() == 3
        && points
            .iter()
            .zip(points.iter().cycle().skip(1))
            .take(3)
            .any(|(a, b)| crate::document::segment_distance([0.5, 0.5], *a, *b) < 0.00001)
    {
        [
            points.iter().map(|p| p[0]).sum::<f32>() / 3.,
            points.iter().map(|p| p[1]).sum::<f32>() / 3.,
        ]
    } else {
        [0.5, 0.5]
    };
    let d = [towards[0] - center[0], towards[1] - center[1]];
    let cross = |a: Point, b: Point| a[0] * b[1] - a[1] * b[0];
    let mut best = f32::INFINITY;
    for (a, b) in points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
    {
        let e = [b[0] - a[0], b[1] - a[1]];
        let denom = cross(d, e);
        if denom.abs() < 0.000001 {
            continue;
        }
        let v = [a[0] - center[0], a[1] - center[1]];
        let t = cross(v, e) / denom;
        let u = cross(v, d) / denom;
        if t >= 0. && (-0.00001..=1.00001).contains(&u) {
            best = best.min(t);
        }
    }
    if best.is_finite() {
        [center[0] + best * d[0], center[1] + best * d[1]]
    } else {
        center
    }
}
/// Ear clipping handles the concave arrows and document waves without fill artifacts.
pub fn triangulate(p: &[Point]) -> Vec<[usize; 3]> {
    if p.len() < 3 {
        return vec![];
    }
    let cross = |a: Point, b: Point, c: Point| {
        (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
    };
    let area: f32 = p
        .iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum();
    let mut ids: Vec<_> = if area >= 0. {
        (0..p.len()).collect()
    } else {
        (0..p.len()).rev().collect()
    };
    let mut out = vec![];
    while ids.len() > 3 {
        let mut ear = None;
        for i in 0..ids.len() {
            let a = ids[(i + ids.len() - 1) % ids.len()];
            let b = ids[i];
            let c = ids[(i + 1) % ids.len()];
            if cross(p[a], p[b], p[c]) <= 0.0000001 {
                continue;
            }
            let contains = ids
                .iter()
                .filter(|&&j| j != a && j != b && j != c)
                .any(|&j| {
                    cross(p[a], p[b], p[j]) >= -0.0000001
                        && cross(p[b], p[c], p[j]) >= -0.0000001
                        && cross(p[c], p[a], p[j]) >= -0.0000001
                });
            if !contains {
                ear = Some((i, [a, b, c]));
                break;
            }
        }
        if let Some((i, triangle)) = ear {
            out.push(triangle);
            ids.remove(i);
        } else {
            break;
        }
    }
    if ids.len() == 3 {
        out.push([ids[0], ids[1], ids[2]]);
    }
    out
}
