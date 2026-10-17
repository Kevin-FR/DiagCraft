//! Vector decorations shared by the native canvas and SVG/PDF exports.
use crate::document::Point;
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinePattern {
    #[default]
    Solid,
    Dashed,
    Dotted,
}
impl LinePattern {
    pub const ALL: [Self; 3] = [Self::Solid, Self::Dashed, Self::Dotted];
    pub fn label(self) -> &'static str {
        match self {
            Self::Solid => "Continu",
            Self::Dashed => "Tirets",
            Self::Dotted => "Pointillés",
        }
    }
    pub fn dash(self, width: f32) -> Option<[f32; 2]> {
        match self {
            Self::Solid => None,
            Self::Dashed => Some([6. * width, 4. * width]),
            Self::Dotted => Some([0., 4. * width]),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArrowHead {
    #[default]
    Triangle,
    Open,
    Diamond,
    Circle,
}
impl ArrowHead {
    pub const ALL: [Self; 4] = [Self::Triangle, Self::Open, Self::Diamond, Self::Circle];
    pub fn label(self) -> &'static str {
        match self {
            Self::Triangle => "Triangle",
            Self::Open => "Ouverte",
            Self::Diamond => "Losange",
            Self::Circle => "Cercle",
        }
    }
}
pub struct Marker {
    pub points: Vec<Point>,
    pub closed: bool,
    pub filled: bool,
}
pub fn marker(points: &[Point], kind: ArrowHead, size: f32) -> Option<Marker> {
    let tip = *points.last()?;
    let prior = points
        .iter()
        .rev()
        .find(|p| (p[0] - tip[0]).hypot(p[1] - tip[1]) > 0.001)?;
    let length = (tip[0] - prior[0]).hypot(tip[1] - prior[1]);
    let u = [(tip[0] - prior[0]) / length, (tip[1] - prior[1]) / length];
    let map = |back: f32, side: f32| {
        [
            tip[0] - u[0] * back - u[1] * side,
            tip[1] - u[1] * back + u[0] * side,
        ]
    };
    let outline = match kind {
        ArrowHead::Triangle => vec![tip, map(size, size * 0.45), map(size, -size * 0.45)],
        ArrowHead::Open => vec![map(size, size * 0.45), tip, map(size, -size * 0.45)],
        ArrowHead::Diamond => vec![
            tip,
            map(size * 0.7, size * 0.4),
            map(size * 1.4, 0.),
            map(size * 0.7, -size * 0.4),
        ],
        ArrowHead::Circle => (0..40)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 40.;
                map(size / 2. + a.cos() * size / 2., a.sin() * size / 2.)
            })
            .collect(),
    };
    Some(Marker {
        points: outline,
        closed: kind != ArrowHead::Open,
        filled: kind == ArrowHead::Triangle,
    })
}
