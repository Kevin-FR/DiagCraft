//! Optional visual styles. Defaults serialize away to preserve existing v7 revisions.
use crate::{
    document::{Color, Point},
    stroke_style::LinePattern,
};
use serde::{Deserialize, Serialize};

macro_rules! choices {
    ($name:ident, $default:ident, $( $variant:ident => $label:literal ),+ $(,)?) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all="snake_case")]
        pub enum $name { $( $variant ),+ }
        impl Default for $name { fn default()->Self { Self::$default } }
        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub fn label(self)-> &'static str { match self { $( Self::$variant => $label ),+ } }
        }
    }
}
choices!(PaintMode, Solid, None=>"Aucun", Solid=>"Uni", Linear=>"Dégradé linéaire", Radial=>"Dégradé radial", Monochrome=>"Dégradé monochrome", Pattern=>"Motif", Texture=>"Texture");
choices!(PatternKind, Diagonal, Diagonal=>"Diagonales", Cross=>"Quadrillage", Dots=>"Points", Horizontal=>"Hachures");
choices!(TextureKind, Paper, Paper=>"Papier", Canvas=>"Toile", Grain=>"Grain");
choices!(Cap, Round, Flat=>"Plat", Round=>"Rond", Square=>"Carré");
choices!(Join, Round, Miter=>"Angle vif", Round=>"Arrondi", Bevel=>"Biseauté");
choices!(Compound, Single, Single=>"Simple", Double=>"Double", Triple=>"Triple");
choices!(ShadowKind, None, None=>"Aucune ombre", Outer=>"Ombre portée", Inner=>"Ombre intérieure", Perspective=>"Ombre en perspective");

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Fill {
    pub mode: PaintMode,
    pub secondary: Color,
    pub tint: f32,
    pub transparency: f32,
    pub angle: f32,
    pub pattern: PatternKind,
    pub texture: TextureKind,
    pub spacing: f32,
}
impl Default for Fill {
    fn default() -> Self {
        Self {
            mode: PaintMode::Solid,
            secondary: [90, 165, 250],
            tint: 0.,
            transparency: 0.,
            angle: 0.,
            pattern: Default::default(),
            texture: Default::default(),
            spacing: 12.,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Line {
    pub mode: PaintMode,
    pub secondary: Color,
    pub tint: f32,
    pub transparency: f32,
    pub angle: f32,
    pub pattern: LinePattern,
    pub cap: Cap,
    pub join: Join,
    pub compound: Compound,
    pub corner_radius: Option<f32>,
    pub start_scale: f32,
    pub end_scale: f32,
}
impl Default for Line {
    fn default() -> Self {
        Self {
            mode: PaintMode::Solid,
            secondary: [90, 165, 250],
            tint: 0.,
            transparency: 0.,
            angle: 0.,
            pattern: Default::default(),
            cap: Default::default(),
            join: Default::default(),
            compound: Default::default(),
            corner_radius: None,
            start_scale: 1.,
            end_scale: 1.,
        }
    }
}
impl Line {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<(), String> {
        if !matches!(
            self.mode,
            PaintMode::None | PaintMode::Solid | PaintMode::Linear | PaintMode::Monochrome
        ) {
            return Err("Type de contour invalide.".into());
        }
        for (v, min, max) in [
            (self.tint, -100., 100.),
            (self.transparency, 0., 100.),
            (self.angle, -180., 180.),
            (self.start_scale, 0.5, 3.),
            (self.end_scale, 0.5, 3.),
        ] {
            bounded(v, min, max)?;
        }
        if let Some(radius) = self.corner_radius {
            bounded(radius, 0., 100.)?;
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Shadow {
    pub kind: ShadowKind,
    pub color: Color,
    pub transparency: f32,
    pub scale: f32,
    pub blur: f32,
    pub angle: f32,
    pub offset: Point,
}
impl Default for Shadow {
    fn default() -> Self {
        Self {
            kind: ShadowKind::None,
            color: [25, 38, 55],
            transparency: 72.,
            scale: 100.,
            blur: 6.,
            angle: 0.,
            offset: [6., 8.],
        }
    }
}
impl Shadow {
    pub fn preset(kind: ShadowKind, index: usize) -> Self {
        let x = (index % 3) as f32 - 1.;
        let y = (index / 3) as f32 - 1.;
        Self {
            kind,
            offset: [x * 9., y * 9.],
            blur: if kind == ShadowKind::Inner { 4. } else { 6. },
            angle: if kind == ShadowKind::Perspective {
                x * 15.
            } else {
                0.
            },
            scale: if kind == ShadowKind::Perspective {
                110.
            } else {
                100.
            },
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Appearance {
    pub fill: Fill,
    pub line: Line,
    pub shadow: Shadow,
}
impl Appearance {
    pub fn is_default(&self) -> bool {
        self == &Self::default()
    }
    pub fn validate(&self) -> Result<(), String> {
        let f = &self.fill;
        let s = &self.shadow;
        for (v, min, max) in [
            (f.tint, -100., 100.),
            (f.transparency, 0., 100.),
            (f.angle, -180., 180.),
            (f.spacing, 4., 64.),
            (s.transparency, 0., 100.),
            (s.scale, 10., 300.),
            (s.blur, 0., 64.),
            (s.angle, -180., 180.),
            (s.offset[0], -200., 200.),
            (s.offset[1], -200., 200.),
        ] {
            bounded(v, min, max)?;
        }
        self.line.validate()
    }
}
fn bounded(v: f32, min: f32, max: f32) -> Result<(), String> {
    if v.is_finite() && (min..=max).contains(&v) {
        Ok(())
    } else {
        Err("Paramètre d’apparence hors limites.".into())
    }
}
pub fn tinted(color: Color, tint: f32) -> Color {
    color.map(|c| {
        {
            let c = c as f32;
            if tint >= 0. {
                c + (255. - c) * tint / 100.
            } else {
                c * (1. + tint / 100.)
            }
        }
        .round()
        .clamp(0., 255.) as u8
    })
}

/// A portable review document covering fills, shadows and compound contours.
pub fn demo() -> crate::document::Document {
    use crate::document::{Document, ShapeKind};
    let mut d = Document {
        title: "Remplir, ligne et ombre".into(),
        page_size: [1180., 880.],
        ..Document::default()
    };
    for (i, label) in [
        "Dégradé linéaire",
        "Dégradé radial",
        "Motif diagonal",
        "Ombre portée",
        "Ombre intérieure",
        "Perspective",
        "Contour dégradé",
        "Double contour",
        "Texture papier",
    ]
    .iter()
    .enumerate()
    {
        let id = d.add_node(
            if i == 1 || i == 4 {
                ShapeKind::Diamond
            } else {
                ShapeKind::Rounded
            },
            [210. + i as f32 % 3. * 370., 160. + (i / 3) as f32 * 270.],
        );
        let n = d.node_mut(id).unwrap();
        n.size = [240., 112.];
        n.text = (*label).into();
        n.fill = [225, 240, 255];
        n.stroke = [45, 120, 220];
        n.appearance.fill.secondary = [230, 220, 255];
        match i {
            0 => {
                n.appearance.fill.mode = PaintMode::Linear;
                n.appearance.fill.angle = 25.;
            }
            1 => n.appearance.fill.mode = PaintMode::Radial,
            2 => {
                n.appearance.fill.mode = PaintMode::Pattern;
                n.appearance.fill.secondary = [165, 194, 230];
            }
            3..=5 => {
                n.appearance.shadow = Shadow::preset(
                    [
                        ShadowKind::Outer,
                        ShadowKind::Inner,
                        ShadowKind::Perspective,
                    ][i - 3],
                    8,
                );
                n.appearance.shadow.transparency = 45.;
            }
            6 => {
                n.appearance.line.mode = PaintMode::Linear;
                n.appearance.line.secondary = [205, 85, 170];
                n.stroke_width = 7.;
            }
            7 => {
                n.appearance.line.compound = Compound::Double;
                n.stroke_width = 8.;
            }
            _ => {
                n.appearance.fill.mode = PaintMode::Texture;
                n.appearance.fill.secondary = [80, 110, 150];
            }
        }
    }
    d
}
