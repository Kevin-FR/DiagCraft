use crate::document::{Color, Point, TextAlign};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChartKind {
    Columns,
    StackedColumns,
    PercentColumns,
    Bars,
    StackedBars,
    PercentBars,
    Line,
    StepLine,
    PercentLine,
    Area,
    StepArea,
}
impl ChartKind {
    pub const ALL: [Self; 11] = [
        Self::Columns,
        Self::StackedColumns,
        Self::PercentColumns,
        Self::Bars,
        Self::StackedBars,
        Self::PercentBars,
        Self::Line,
        Self::StepLine,
        Self::PercentLine,
        Self::Area,
        Self::StepArea,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Columns => "Colonnes groupées",
            Self::StackedColumns => "Colonnes empilées",
            Self::PercentColumns => "Colonnes 100 %",
            Self::Bars => "Barres groupées",
            Self::StackedBars => "Barres empilées",
            Self::PercentBars => "Barres 100 %",
            Self::Line => "Courbes",
            Self::StepLine => "Courbes en escalier",
            Self::PercentLine => "Courbes 100 %",
            Self::Area => "Graphique de zone",
            Self::StepArea => "Zone en escalier",
        }
    }
    pub fn category(self) -> &'static str {
        match self {
            Self::Columns | Self::StackedColumns | Self::PercentColumns => "Histogramme",
            Self::Bars | Self::StackedBars | Self::PercentBars => "Graphique à barres",
            Self::Line | Self::StepLine | Self::PercentLine => "Graphique linéaire",
            _ => "Graphique de zone",
        }
    }
    fn percent(self) -> bool {
        matches!(
            self,
            Self::PercentColumns | Self::PercentBars | Self::PercentLine
        )
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Series {
    pub name: String,
    pub values: Vec<f32>,
    pub color: Color,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Chart {
    pub id: u64,
    pub kind: ChartKind,
    pub position: Point,
    pub size: Point,
    pub title: String,
    pub categories: Vec<String>,
    pub series: Vec<Series>,
    pub legend: bool,
}
#[derive(Clone, Debug)]
pub enum Primitive {
    Rect {
        position: Point,
        size: Point,
        fill: Color,
    },
    Path {
        points: Vec<Point>,
        fill: Option<Color>,
        stroke: Option<Color>,
        width: f32,
        closed: bool,
    },
    Text {
        position: Point,
        text: String,
        size: f32,
        color: Color,
        align: TextAlign,
    },
    Circle {
        position: Point,
        radius: f32,
        color: Color,
    },
}
impl Chart {
    pub fn sample(id: u64, kind: ChartKind, position: Point) -> Self {
        Self {
            id,
            kind,
            position,
            size: [480., 320.],
            title: kind.label().into(),
            categories: vec!["T1".into(), "T2".into(), "T3".into(), "T4".into()],
            series: vec![
                Series {
                    name: "Série A".into(),
                    values: vec![30., 55., 40., 75.],
                    color: [69, 161, 252],
                },
                Series {
                    name: "Série B".into(),
                    values: vec![20., 35., 60., 50.],
                    color: [172, 221, 250],
                },
            ],
            legend: true,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if !self
            .position
            .iter()
            .all(|v| v.is_finite() && v.abs() <= 100_000.)
            || !self
                .size
                .iter()
                .all(|v| v.is_finite() && (120.0..=20_000.0).contains(v))
        {
            return Err("Dimensions de graphique invalides.".into());
        }
        if self.title.len() > 1000
            || self.categories.is_empty()
            || self.categories.len() > 24
            || self.categories.iter().any(|s| s.len() > 120)
            || self.series.is_empty()
            || self.series.len() > 8
        {
            return Err("Le graphique accepte 1 à 24 catégories et 1 à 8 séries.".into());
        }
        if self.series.iter().any(|s| {
            s.name.len() > 120
                || s.values.len() != self.categories.len()
                || s.values
                    .iter()
                    .any(|v| !v.is_finite() || !(0.0..=1_000_000.0).contains(v))
        }) {
            return Err("Données de graphique invalides : valeurs positives requises.".into());
        }
        Ok(())
    }
    pub fn primitives(&self) -> Vec<Primitive> {
        if self.validate().is_err() {
            return vec![];
        }
        let mut out = vec![Primitive::Rect {
            position: [0., 0.],
            size: self.size,
            fill: [255, 255, 255],
        }];
        let scale = (self.size[0] / 480.).min(self.size[1] / 320.);
        let font = (12. * scale).max(4.);
        let left = 52. * scale;
        let top = if self.title.is_empty() {
            12. * scale
        } else {
            44. * scale
        };
        let bottom = self.size[1]
            - if self.legend {
                58. * scale
            } else {
                30. * scale
            };
        let right = self.size[0] - 20. * scale;
        let w = right - left;
        let h = bottom - top;
        let n = self.categories.len();
        let count = self.series.len();
        let percent = self.kind.percent();
        if !self.title.is_empty() {
            out.push(Primitive::Text {
                position: [self.size[0] / 2., 22. * scale],
                text: self.title.clone(),
                size: 18. * scale,
                color: [35, 51, 65],
                align: TextAlign::Center,
            });
        }
        let stacked = matches!(
            self.kind,
            ChartKind::StackedColumns
                | ChartKind::PercentColumns
                | ChartKind::StackedBars
                | ChartKind::PercentBars
                | ChartKind::Area
                | ChartKind::StepArea
        );
        let horizontal = matches!(
            self.kind,
            ChartKind::Bars | ChartKind::StackedBars | ChartKind::PercentBars
        );
        let max = if percent {
            100.
        } else if stacked {
            (0..n)
                .map(|i| self.series.iter().map(|s| s.values[i]).sum::<f32>())
                .fold(1., f32::max)
        } else {
            self.series
                .iter()
                .flat_map(|s| s.values.iter())
                .copied()
                .fold(1., f32::max)
        };
        let value = |series: usize, i: usize| {
            if percent {
                let sum: f32 = self.series.iter().map(|s| s.values[i]).sum();
                if sum > 0. {
                    self.series[series].values[i] * 100. / sum
                } else {
                    0.
                }
            } else {
                self.series[series].values[i]
            }
        };
        for tick in 0..=4 {
            let t = tick as f32 / 4.;
            let (a, b, label) = if horizontal {
                (
                    [left + t * w, top],
                    [left + t * w, bottom],
                    [left + t * w, bottom + 14. * scale],
                )
            } else {
                (
                    [left, bottom - t * h],
                    [right, bottom - t * h],
                    [left - 8. * scale, bottom - t * h],
                )
            };
            out.push(Primitive::Path {
                points: vec![a, b],
                fill: None,
                stroke: Some([229, 234, 241]),
                width: scale.max(0.5),
                closed: false,
            });
            out.push(Primitive::Text {
                position: label,
                text: format!("{:.0}{}", max * t, if percent { "%" } else { "" }),
                size: font,
                color: [126, 139, 154],
                align: if horizontal {
                    TextAlign::Center
                } else {
                    TextAlign::Right
                },
            });
        }
        for (i, label) in self.categories.iter().enumerate() {
            let pos = if horizontal {
                [left - 8. * scale, top + (i as f32 + 0.5) * h / n as f32]
            } else {
                [left + (i as f32 + 0.5) * w / n as f32, bottom + 17. * scale]
            };
            out.push(Primitive::Text {
                position: pos,
                text: label.clone(),
                size: font,
                color: [95, 111, 129],
                align: if horizontal {
                    TextAlign::Right
                } else {
                    TextAlign::Center
                },
            });
        }
        if matches!(
            self.kind,
            ChartKind::Columns
                | ChartKind::StackedColumns
                | ChartKind::PercentColumns
                | ChartKind::Bars
                | ChartKind::StackedBars
                | ChartKind::PercentBars
        ) {
            for i in 0..n {
                let mut cumulative = 0.;
                for s in 0..count {
                    let v = value(s, i);
                    let slot = if horizontal {
                        h / n as f32
                    } else {
                        w / n as f32
                    };
                    let band = slot * 0.7;
                    let thickness = if stacked { band } else { band / count as f32 };
                    let shift = if stacked { 0. } else { s as f32 * thickness };
                    let start = if stacked { cumulative } else { 0. };
                    let (position, size) = if horizontal {
                        (
                            [
                                left + start / max * w,
                                top + i as f32 * slot + slot * 0.15 + shift,
                            ],
                            [v / max * w, thickness.max(0.1)],
                        )
                    } else {
                        (
                            [
                                left + i as f32 * slot + slot * 0.15 + shift,
                                bottom - (start + v) / max * h,
                            ],
                            [thickness.max(0.1), v / max * h],
                        )
                    };
                    out.push(Primitive::Rect {
                        position,
                        size,
                        fill: self.series[s].color,
                    });
                    cumulative += v;
                }
            }
        } else {
            let mut base = vec![0.; n];
            for s in 0..count {
                let step = matches!(self.kind, ChartKind::StepLine | ChartKind::StepArea);
                let area = matches!(self.kind, ChartKind::Area | ChartKind::StepArea);
                let mut points: Vec<Point> = vec![];
                for (i, base_value) in base.iter().enumerate() {
                    let x = left + (i as f32 + 0.5) * w / n as f32;
                    let y = bottom - (value(s, i) + if area { *base_value } else { 0. }) / max * h;
                    if step && i > 0 {
                        points.push([x, points.last().unwrap()[1]]);
                    }
                    points.push([x, y]);
                }
                if area {
                    let mut polygon = points.clone();
                    for i in (0..n).rev() {
                        let x = left + (i as f32 + 0.5) * w / n as f32;
                        let y = bottom - base[i] / max * h;
                        if step && i + 1 < n {
                            polygon.push([x, polygon.last().unwrap()[1]]);
                        }
                        polygon.push([x, y]);
                    }
                    out.push(Primitive::Path {
                        points: polygon,
                        fill: Some(self.series[s].color),
                        stroke: None,
                        width: 0.,
                        closed: true,
                    });
                    for (i, v) in base.iter_mut().enumerate() {
                        *v += value(s, i);
                    }
                }
                out.push(Primitive::Path {
                    points: points.clone(),
                    fill: None,
                    stroke: Some(self.series[s].color),
                    width: (2. * scale).max(0.5),
                    closed: false,
                });
                if !area {
                    for point in points.iter().step_by(if step { 2 } else { 1 }) {
                        out.push(Primitive::Circle {
                            position: *point,
                            radius: (3. * scale).max(1.),
                            color: self.series[s].color,
                        });
                    }
                }
            }
        }
        if self.legend {
            for (i, s) in self.series.iter().enumerate() {
                let x = left + i as f32 * (w / count as f32);
                out.push(Primitive::Rect {
                    position: [x, self.size[1] - 20. * scale],
                    size: [10. * scale, 8. * scale],
                    fill: s.color,
                });
                out.push(Primitive::Text {
                    position: [x + 15. * scale, self.size[1] - 16. * scale],
                    text: s.name.clone(),
                    size: font,
                    color: [95, 111, 129],
                    align: TextAlign::Left,
                });
            }
        }
        out
    }
}
