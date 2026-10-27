use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fs, io::Write, path::Path};

pub type Point = [f32; 2];
pub type Color = [u8; 3];
pub const FORMAT_VERSION: u32 = 7;
pub const MAX_FILE_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextAlign {
    Left,
    #[default]
    Center,
    Right,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextStyle {
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
    pub monospace: bool,
    pub align: TextAlign,
    pub wrap: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub auto_fit: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GridStyle {
    #[default]
    Dots,
    Lines,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageStyle {
    pub background: Color,
    pub grid_spacing: f32,
    pub margin: f32,
    pub watermark: String,
    pub grid_style: GridStyle,
    pub export_grid: bool,
}
impl Default for PageStyle {
    fn default() -> Self {
        Self {
            background: [255, 255, 255],
            grid_spacing: 20.,
            margin: 40.,
            watermark: String::new(),
            grid_style: GridStyle::Dots,
            export_grid: false,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DrawingKind {
    Line,
    Bezier,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drawing {
    pub id: u64,
    pub kind: DrawingKind,
    pub start: Point,
    pub end: Point,
    pub bend: f32,
    pub color: Color,
    pub width: f32,
    pub arrow: bool,
    #[serde(default, skip_serializing_if = "crate::appearance::Line::is_default")]
    pub appearance: crate::appearance::Line,
}
impl Drawing {
    pub fn controls(&self) -> [Point; 4] {
        let dx = self.end[0] - self.start[0];
        let dy = self.end[1] - self.start[1];
        let len = (dx * dx + dy * dy).sqrt().max(0.001);
        let offset = [-dy / len * self.bend, dx / len * self.bend];
        [
            self.start,
            [
                self.start[0] + dx / 3. + offset[0],
                self.start[1] + dy / 3. + offset[1],
            ],
            [
                self.start[0] + dx * 2. / 3. + offset[0],
                self.start[1] + dy * 2. / 3. + offset[1],
            ],
            self.end,
        ]
    }
    pub fn points(&self) -> Vec<Point> {
        if self.kind == DrawingKind::Line {
            return vec![self.start, self.end];
        }
        let c = self.controls();
        (0..=40)
            .map(|i| {
                let t = i as f32 / 40.;
                let u = 1. - t;
                [0, 1].map(|a| {
                    u * u * u * c[0][a]
                        + 3. * u * u * t * c[1][a]
                        + 3. * u * t * t * c[2][a]
                        + t * t * t * c[3][a]
                })
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeKind {
    Rectangle,
    Rounded,
    Ellipse,
    Diamond,
    Text,
    Document,
    Data,
    PredefinedProcess,
    Database,
    Hexagon,
    Trapezoid,
    ManualInput,
    ManualOperation,
    OffPage,
    InternalStorage,
    Delay,
    Display,
    Terminator,
    Note,
    Callout,
    Star,
    ArrowRight,
    ArrowLeft,
    ArrowUp,
    ArrowDown,
    DoubleArrow,
    Chevron,
    NotchedArrow,
    ElbowArrow,
    Triangle,
    RightTriangle,
    Pentagon,
    Octagon,
    Plus,
    Heart,
    Cloud,
    Shield,
    Drop,
    Star4,
    Star8,
    Gear,
    Frame,
    Swimlane,
    TitleBlock,
    UmlClass,
    Component,
    Package,
    Actor,
    Lifeline,
    Fork,
    InitialState,
    FinalState,
    UseCase,
    Cube,
    Cone,
    Pyramid,
    BraceLeft,
    BraceRight,
    Cross,
    Lightning,
}

impl ShapeKind {
    pub const FLOWCHART: [Self; 18] = [
        Self::Rectangle,
        Self::Rounded,
        Self::Diamond,
        Self::Document,
        Self::Data,
        Self::Terminator,
        Self::PredefinedProcess,
        Self::InternalStorage,
        Self::Ellipse,
        Self::Database,
        Self::Hexagon,
        Self::Trapezoid,
        Self::ManualInput,
        Self::ManualOperation,
        Self::Delay,
        Self::Display,
        Self::OffPage,
        Self::Note,
    ];
    pub const ARROWS: [Self; 8] = [
        Self::ArrowRight,
        Self::ArrowLeft,
        Self::ArrowDown,
        Self::ArrowUp,
        Self::DoubleArrow,
        Self::Chevron,
        Self::NotchedArrow,
        Self::ElbowArrow,
    ];
    pub const ANNOTATIONS: [Self; 3] = [Self::Text, Self::Callout, Self::Star];
    pub const ALL: [Self; 5] = [
        Self::Rectangle,
        Self::Rounded,
        Self::Ellipse,
        Self::Diamond,
        Self::Text,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Rectangle => "Rectangle",
            Self::Rounded => "Arrondi",
            Self::Ellipse => "Ellipse",
            Self::Diamond => "Décision",
            Self::Text => "Texte",
            Self::Document => "Document",
            Self::Data => "Données",
            Self::PredefinedProcess => "Sous-processus",
            Self::Database => "Base de données",
            Self::Hexagon => "Préparation",
            Self::Trapezoid => "Trapèze",
            Self::ManualInput => "Saisie manuelle",
            Self::ManualOperation => "Opération manuelle",
            Self::OffPage => "Renvoi hors page",
            Self::InternalStorage => "Stockage interne",
            Self::Delay => "Délai",
            Self::Display => "Affichage",
            Self::Terminator => "Début / fin",
            Self::Note => "Note",
            Self::Callout => "Légende",
            Self::Star => "Étoile",
            Self::ArrowRight => "Flèche droite",
            Self::ArrowLeft => "Flèche gauche",
            Self::ArrowUp => "Flèche haute",
            Self::ArrowDown => "Flèche basse",
            Self::DoubleArrow => "Double flèche",
            Self::Chevron => "Chevron",
            Self::NotchedArrow => "Flèche à encoche",
            Self::ElbowArrow => "Flèche coudée",
            Self::Triangle => "Triangle",
            Self::RightTriangle => "Triangle rectangle",
            Self::Pentagon => "Pentagone",
            Self::Octagon => "Octogone",
            Self::Plus => "Croix",
            Self::Heart => "Cœur",
            Self::Cloud => "Nuage",
            Self::Shield => "Bouclier",
            Self::Drop => "Goutte",
            Self::Star4 => "Étoile à quatre branches",
            Self::Star8 => "Étoile à huit branches",
            Self::Gear => "Engrenage",
            Self::Frame => "Cadre",
            Self::Swimlane => "Couloir horizontal",
            Self::TitleBlock => "Cartouche",
            Self::UmlClass => "Classe UML",
            Self::Component => "Composant UML",
            Self::Package => "Paquet UML",
            Self::Actor => "Acteur UML",
            Self::Lifeline => "Ligne de vie",
            Self::Fork => "Synchronisation",
            Self::InitialState => "État initial",
            Self::FinalState => "État final",
            Self::UseCase => "Cas d’utilisation",
            Self::Cube => "Cube",
            Self::Cone => "Cône",
            Self::Pyramid => "Pyramide",
            Self::BraceLeft => "Accolade gauche",
            Self::BraceRight => "Accolade droite",
            Self::Cross => "Croix diagonale",
            Self::Lightning => "Éclair",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: u64,
    pub kind: ShapeKind,
    pub position: Point,
    pub size: Point,
    pub text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub text_runs: Vec<crate::rich_text::TextRun>,
    pub fill: Color,
    pub stroke: Color,
    pub text_color: Color,
    pub stroke_width: f32,
    pub font_size: f32,
    #[serde(default)]
    pub text_style: TextStyle,
    #[serde(
        default,
        skip_serializing_if = "crate::appearance::Appearance::is_default"
    )]
    pub appearance: crate::appearance::Appearance,
}

impl Node {
    pub fn center(&self) -> Point {
        [
            self.position[0] + self.size[0] / 2.,
            self.position[1] + self.size[1] / 2.,
        ]
    }
    pub fn contains(&self, p: Point) -> bool {
        let c = self.center();
        let x = (p[0] - c[0]).abs() / (self.size[0] / 2.);
        let y = (p[1] - c[1]).abs() / (self.size[1] / 2.);
        if self.kind == ShapeKind::Actor {
            let local = [
                (p[0] - self.position[0]) / self.size[0],
                (p[1] - self.position[1]) / self.size[1],
            ];
            if ((local[0] - 0.5) / 0.13).powi(2) + ((local[1] - 0.14) / 0.13).powi(2) <= 1. {
                return true;
            }
        }
        match self.kind {
            ShapeKind::Diamond => x + y <= 1.,
            ShapeKind::Ellipse => x * x + y * y <= 1.,
            ShapeKind::Rectangle | ShapeKind::Rounded | ShapeKind::Text => x <= 1. && y <= 1.,
            _ => crate::geometry::contains(
                &crate::geometry::shape(self.kind).outline,
                [
                    (p[0] - self.position[0]) / self.size[0],
                    (p[1] - self.position[1]) / self.size[1],
                ],
            ),
        }
    }
    /// Intersection of a ray from the center with the shape boundary.
    pub fn boundary_towards(&self, p: Point) -> Point {
        if !ShapeKind::ALL.contains(&self.kind) {
            let b = crate::geometry::boundary(
                &crate::geometry::shape(self.kind).outline,
                [
                    (p[0] - self.position[0]) / self.size[0],
                    (p[1] - self.position[1]) / self.size[1],
                ],
            );
            return [
                self.position[0] + b[0] * self.size[0],
                self.position[1] + b[1] * self.size[1],
            ];
        }
        let c = self.center();
        let dx = p[0] - c[0];
        let dy = p[1] - c[1];
        let rx = self.size[0] / 2.;
        let ry = self.size[1] / 2.;
        let denominator = match self.kind {
            ShapeKind::Ellipse => ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt(),
            ShapeKind::Diamond => (dx / rx).abs() + (dy / ry).abs(),
            _ => (dx / rx).abs().max((dy / ry).abs()),
        };
        if denominator < 0.0001 {
            c
        } else {
            [c[0] + dx / denominator, c[1] + dy / denominator]
        }
    }
    pub fn display_text(&self) -> String {
        self.text_layout().text()
    }
    pub fn port_position(&self, port: Port, towards: Point) -> Point {
        let c = self.center();
        self.boundary_towards(match port {
            Port::Auto => towards,
            Port::Left => [c[0] - self.size[0], c[1]],
            Port::Right => [c[0] + self.size[0], c[1]],
            Port::Top => [c[0], c[1] - self.size[1]],
            Port::Bottom => [c[0], c[1] + self.size[1]],
        })
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Port {
    #[default]
    Auto,
    Left,
    Right,
    Top,
    Bottom,
}
impl Port {
    pub const ALL: [Self; 5] = [Self::Auto, Self::Left, Self::Right, Self::Top, Self::Bottom];
    pub fn label(self) -> &'static str {
        match self {
            Self::Auto => "Automatique",
            Self::Left => "Gauche",
            Self::Right => "Droite",
            Self::Top => "Haut",
            Self::Bottom => "Bas",
        }
    }
}
fn middle_label() -> f32 {
    0.5
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorKind {
    Straight,
    Elbow,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Connector {
    pub id: u64,
    pub from: u64,
    pub to: u64,
    #[serde(default)]
    pub free_start: Option<Point>,
    #[serde(default)]
    pub free_end: Option<Point>,
    pub kind: ConnectorKind,
    pub label: String,
    pub color: Color,
    pub width: f32,
    pub arrow: bool,
    #[serde(default)]
    pub arrow_start: bool,
    #[serde(default)]
    pub from_port: Port,
    #[serde(default)]
    pub to_port: Port,
    #[serde(default)]
    pub waypoints: Vec<Point>,
    #[serde(default = "middle_label")]
    pub label_t: f32,
    #[serde(default)]
    pub pattern: crate::stroke_style::LinePattern,
    #[serde(default)]
    pub start_head: crate::stroke_style::ArrowHead,
    #[serde(default)]
    pub end_head: crate::stroke_style::ArrowHead,
    #[serde(default, skip_serializing_if = "crate::appearance::Line::is_default")]
    pub appearance: crate::appearance::Line,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Document {
    pub format_version: u32,
    pub title: String,
    #[serde(default = "default_page_name")]
    pub page_name: String,
    #[serde(default)]
    pub page_layout: crate::pages::PageLayout,
    #[serde(default)]
    pub pages: Vec<crate::pages::Page>,
    #[serde(skip)]
    pub page_index: usize,
    pub page_size: Point,
    pub nodes: Vec<Node>,
    pub connectors: Vec<Connector>,
    #[serde(default)]
    pub drawings: Vec<Drawing>,
    #[serde(default)]
    pub page_style: PageStyle,
    #[serde(default)]
    pub charts: Vec<crate::chart::Chart>,
    #[serde(default)]
    pub organization: crate::organization::Organization,
}

fn default_page_name() -> String {
    "Page 1".into()
}
impl Default for Document {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            title: "Sans titre".into(),
            page_name: default_page_name(),
            page_layout: Default::default(),
            pages: vec![],
            page_index: 0,
            page_size: [1280., 900.],
            nodes: vec![],
            connectors: vec![],
            drawings: vec![],
            page_style: PageStyle::default(),
            charts: vec![],
            organization: Default::default(),
        }
    }
}

impl Document {
    pub fn edit_bounds(&self) -> Point {
        if self.page_layout.auto_size {
            [100_000., 100_000.]
        } else {
            self.page_size
        }
    }
    pub fn object_count(&self) -> usize {
        self.nodes.len()
            + self.connectors.len()
            + self.drawings.len()
            + self.charts.len()
            + self
                .pages
                .iter()
                .map(|p| p.nodes.len() + p.connectors.len() + p.drawings.len() + p.charts.len())
                .sum::<usize>()
    }
    pub fn add_chart(&mut self, kind: crate::chart::ChartKind, center: Point) -> u64 {
        let id = self.next_id();
        let mut c = crate::chart::Chart::sample(id, kind, [center[0] - 240., center[1] - 160.]);
        c.position[0] = c.position[0].clamp(0., (self.page_size[0] - c.size[0]).max(0.));
        c.position[1] = c.position[1].clamp(0., (self.page_size[1] - c.size[1]).max(0.));
        self.charts.push(c);
        id
    }
    pub fn hit_chart(&self, p: Point) -> Option<u64> {
        let organization = self.organization_view();
        self.charts
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                organization.is_object_visible(c.id)
                    && p[0] >= c.position[0]
                    && p[0] <= c.position[0] + c.size[0]
                    && p[1] >= c.position[1]
                    && p[1] <= c.position[1] + c.size[1]
            })
            .max_by_key(|(index, c)| {
                (
                    organization.layer_order(organization.object_layer(c.id)),
                    *index,
                )
            })
            .map(|(_, c)| c.id)
    }
    pub fn next_id(&self) -> u64 {
        let ids: HashSet<_> = self
            .nodes
            .iter()
            .map(|n| n.id)
            .chain(self.connectors.iter().map(|c| c.id))
            .chain(self.drawings.iter().map(|d| d.id))
            .chain(self.charts.iter().map(|c| c.id))
            .collect();
        if let Some(next) = ids
            .iter()
            .max()
            .copied()
            .unwrap_or(0)
            .checked_add(1)
            .filter(|id| *id < u64::MAX)
        {
            return next;
        }
        // Imported IDs may be close to u64::MAX. Reuse an unused low ID safely.
        (1..u64::MAX)
            .find(|id| !ids.contains(id))
            .expect("available document ID")
    }
    pub fn add_node(&mut self, kind: ShapeKind, center: Point) -> u64 {
        let id = self.next_id();
        let size = match kind {
            ShapeKind::Diamond => [200., 120.],
            ShapeKind::Text => [300., 48.],
            _ => [220., 84.],
        };
        self.nodes.push(Node {
            id,
            kind,
            position: [center[0] - size[0] / 2., center[1] - size[1] / 2.],
            size,
            text: if ShapeKind::ARROWS.contains(&kind) {
                String::new()
            } else if kind == ShapeKind::Text {
                "Votre texte".into()
            } else {
                kind.label().into()
            },
            fill: [233, 246, 241],
            stroke: [39, 128, 107],
            text_color: [32, 53, 64],
            stroke_width: 1.5,
            font_size: 18.,
            text_style: TextStyle::default(),
            text_runs: vec![],
            appearance: Default::default(),
        });
        id
    }
    pub fn connect(&mut self, from: u64, to: u64, kind: ConnectorKind) -> Option<u64> {
        if from == to || self.node(from).is_none() || self.node(to).is_none() {
            return None;
        }
        if self.connectors.iter().any(|c| c.from == from && c.to == to) {
            return None;
        }
        let id = self.next_id();
        self.connectors.push(Connector {
            id,
            from,
            to,
            free_start: None,
            free_end: None,
            kind,
            label: String::new(),
            color: [111, 128, 139],
            width: 1.8,
            arrow: true,
            arrow_start: false,
            from_port: Port::Auto,
            to_port: Port::Auto,
            waypoints: vec![],
            label_t: 0.5,
            pattern: Default::default(),
            start_head: Default::default(),
            end_head: Default::default(),
            appearance: Default::default(),
        });
        Some(id)
    }
    pub fn node(&self, id: u64) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn node_mut(&mut self, id: u64) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }
    pub fn hit_node(&self, p: Point) -> Option<u64> {
        let organization = self.organization_view();
        self.nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| organization.is_object_visible(n.id) && n.contains(p))
            .max_by_key(|(index, n)| {
                (
                    organization.layer_order(organization.object_layer(n.id)),
                    *index,
                )
            })
            .map(|(_, n)| n.id)
    }
    pub fn remove(&mut self, ids: &[u64]) {
        let existing: HashSet<_> = self.object_ids().into_iter().collect();
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| existing.contains(id))
            .collect();
        if !ids.is_empty() {
            let _ = self.try_remove(&ids);
        }
    }
    pub fn try_remove(&mut self, ids: &[u64]) -> Result<(), String> {
        self.validate()?;
        let ids: HashSet<_> = self.requested_ids(ids, true)?.into_iter().collect();
        let mut next = self.clone();
        next.nodes.retain(|n| !ids.contains(&n.id));
        next.connectors
            .retain(|c| !ids.contains(&c.id) && !ids.contains(&c.from) && !ids.contains(&c.to));
        next.drawings.retain(|d| !ids.contains(&d.id));
        next.charts.retain(|c| !ids.contains(&c.id));
        next.prune_organization();
        self.check_locked_changes(&next)?;
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn duplicate_nodes(&mut self, ids: &[u64]) -> Vec<u64> {
        let existing: HashSet<_> = self.object_ids().into_iter().collect();
        let ids: Vec<_> = ids
            .iter()
            .copied()
            .filter(|id| existing.contains(id))
            .collect();
        if ids.is_empty() {
            return vec![];
        }
        self.duplicate_objects(&ids, [24., 24.]).unwrap_or_default()
    }
    pub fn add_drawing(&mut self, kind: DrawingKind, start: Point, end: Point) -> u64 {
        let id = self.next_id();
        self.drawings.push(Drawing {
            id,
            kind,
            start,
            end,
            bend: if kind == DrawingKind::Bezier { 60. } else { 0. },
            color: [70, 86, 105],
            width: 2.,
            arrow: false,
            appearance: Default::default(),
        });
        id
    }
    pub fn hit_drawing(&self, p: Point, tolerance: f32) -> Option<u64> {
        let organization = self.organization_view();
        self.drawings
            .iter()
            .enumerate()
            .filter(|(_, d)| {
                organization.is_object_visible(d.id)
                    && d.points()
                        .windows(2)
                        .any(|s| segment_distance(p, s[0], s[1]) <= tolerance)
            })
            .max_by_key(|(index, d)| {
                (
                    organization.layer_order(organization.object_layer(d.id)),
                    *index,
                )
            })
            .map(|(_, d)| d.id)
    }
    pub fn selection(&self, ids: &[u64]) -> Self {
        let ids: HashSet<_> = self.expand_groups(ids).into_iter().collect();
        let mut fragment = Self {
            nodes: self
                .nodes
                .iter()
                .filter(|n| ids.contains(&n.id))
                .cloned()
                .collect(),
            connectors: self
                .connectors
                .iter()
                .filter(|e| {
                    ids.contains(&e.id)
                        || ((e.from == 0 || ids.contains(&e.from))
                            && (e.to == 0 || ids.contains(&e.to))
                            && (ids.contains(&e.from) || ids.contains(&e.to)))
                })
                .map(|e| {
                    let mut e = e.clone();
                    let route = self.route(&e);
                    if e.from != 0 && !ids.contains(&e.from) {
                        e.from = 0;
                        e.free_start = route.first().copied();
                        e.from_port = Port::Auto;
                    }
                    if e.to != 0 && !ids.contains(&e.to) {
                        e.to = 0;
                        e.free_end = route.last().copied();
                        e.to_port = Port::Auto;
                    }
                    e
                })
                .collect(),
            drawings: self
                .drawings
                .iter()
                .filter(|d| ids.contains(&d.id))
                .cloned()
                .collect(),
            title: "Sélection DiagCraft".into(),
            charts: self
                .charts
                .iter()
                .filter(|c| ids.contains(&c.id))
                .cloned()
                .collect(),
            ..Self::default()
        };
        fragment.organization =
            self.selected_organization(&fragment.object_ids().into_iter().collect());
        fragment
    }
    pub fn duplicate_objects(&mut self, ids: &[u64], offset: Point) -> Result<Vec<u64>, String> {
        let ids = self.requested_ids(ids, false)?;
        let view = self.organization_view();
        if ids.iter().any(|id| {
            !view.is_object_visible(*id)
                || self
                    .organization
                    .layers
                    .iter()
                    .any(|l| l.id == self.object_layer(*id) && l.locked)
        }) {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let fragment = self.selection(&ids);
        self.paste_with_layers(&fragment, offset, true)
    }
    pub fn paste(&mut self, fragment: &Self, offset: Point) -> Result<Vec<u64>, String> {
        self.paste_with_layers(fragment, offset, false)
    }
    fn paste_with_layers(
        &mut self,
        fragment: &Self,
        offset: Point,
        reuse_layers: bool,
    ) -> Result<Vec<u64>, String> {
        fragment.validate()?;
        if fragment.page_count() != 1 {
            return Err("Copiez les objets d’une seule page.".into());
        }
        if !offset.iter().all(|v| v.is_finite() && v.abs() <= 100_000.) {
            return Err("Décalage invalide.".into());
        }
        if self.object_count() + fragment.object_count() > 10_000 {
            return Err("Le document dépasse 10 000 objets.".into());
        }
        let mut target = self.clone();
        let mut map = std::collections::HashMap::new();
        let mut selected = vec![];
        for mut n in fragment.nodes.clone() {
            let old = n.id;
            n.id = target.next_id();
            map.insert(old, n.id);
            selected.push(n.id);
            for (a, v) in offset.iter().enumerate() {
                n.position[a] += v;
            }
            target.nodes.push(n);
        }
        for mut e in fragment.connectors.clone() {
            let old = e.id;
            e.id = target.next_id();
            map.insert(old, e.id);
            if e.from != 0 {
                e.from = map[&e.from];
            }
            if e.to != 0 {
                e.to = map[&e.to];
            }
            for p in [&mut e.free_start, &mut e.free_end].into_iter().flatten() {
                for a in 0..2 {
                    p[a] += offset[a];
                }
            }
            if fragment.nodes.is_empty() {
                selected.push(e.id);
            }
            for p in &mut e.waypoints {
                for a in 0..2 {
                    p[a] += offset[a];
                }
            }
            target.connectors.push(e);
        }
        for mut d in fragment.drawings.clone() {
            let old = d.id;
            d.id = target.next_id();
            map.insert(old, d.id);
            selected.push(d.id);
            for p in [&mut d.start, &mut d.end] {
                for (a, v) in offset.iter().enumerate() {
                    p[a] += v;
                }
            }
            target.drawings.push(d);
        }
        for mut c in fragment.charts.clone() {
            let old = c.id;
            c.id = target.next_id();
            map.insert(old, c.id);
            selected.push(c.id);
            for (a, v) in offset.iter().enumerate() {
                c.position[a] += v;
            }
            target.charts.push(c);
        }
        target.paste_organization(fragment, &map, reuse_layers)?;
        target.validate()?;
        *self = target;
        Ok(selected)
    }
    pub fn fit_page_to_drawing(&mut self) -> Result<(), String> {
        self.validate()?;
        let before = self.clone();
        if self.nodes.is_empty()
            && self.drawings.is_empty()
            && self.charts.is_empty()
            && self.connectors.is_empty()
        {
            return Ok(());
        }
        let mut min: Point = [f32::INFINITY; 2];
        let mut max: Point = [f32::NEG_INFINITY; 2];
        for n in &self.nodes {
            let bounds = if n.appearance.is_default() {
                [0., 0., n.size[0], n.size[1]]
            } else {
                crate::appearance_svg::node_bounds(n)
            };
            for a in 0..2 {
                min[a] = min[a].min(n.position[a] + bounds[a]);
                max[a] = max[a].max(n.position[a] + bounds[a + 2]);
            }
        }
        for d in &self.drawings {
            for p in d.controls() {
                for a in 0..2 {
                    min[a] = min[a].min(p[a]);
                    max[a] = max[a].max(p[a]);
                }
            }
        }
        for c in &self.charts {
            for a in 0..2 {
                min[a] = min[a].min(c.position[a]);
                max[a] = max[a].max(c.position[a] + c.size[a]);
            }
        }
        for e in &self.connectors {
            for p in self.route(e) {
                for a in 0..2 {
                    min[a] = min[a].min(p[a]);
                    max[a] = max[a].max(p[a]);
                }
            }
        }
        let offset = [
            self.page_style.margin - min[0],
            self.page_style.margin - min[1],
        ];
        let size = [0, 1].map(|a| (max[a] - min[a] + 2. * self.page_style.margin).max(100.));
        if size.iter().any(|v| *v > 100_000.) {
            return Err("Le dessin dépasse les dimensions maximales d’une page.".into());
        }
        for n in &mut self.nodes {
            for (a, v) in offset.iter().enumerate() {
                n.position[a] += v;
            }
        }
        for d in &mut self.drawings {
            for p in [&mut d.start, &mut d.end] {
                for (a, v) in offset.iter().enumerate() {
                    p[a] += v;
                }
            }
        }
        self.page_size = size;
        for c in &mut self.charts {
            for (a, v) in offset.iter().enumerate() {
                c.position[a] += v;
            }
        }
        for e in &mut self.connectors {
            for p in [&mut e.free_start, &mut e.free_end].into_iter().flatten() {
                for a in 0..2 {
                    p[a] += offset[a];
                }
            }
            for p in &mut e.waypoints {
                for a in 0..2 {
                    p[a] += offset[a];
                }
            }
        }
        if let Err(error) = before.check_locked_changes(self) {
            *self = before;
            return Err(error);
        }
        Ok(())
    }
    pub fn route(&self, edge: &Connector) -> Vec<Point> {
        let a = self.node(edge.from);
        let b = self.node(edge.to);
        let (Some(ac), Some(bc)) = (
            a.map(Node::center).or(edge.free_start),
            b.map(Node::center).or(edge.free_end),
        ) else {
            return vec![];
        };
        if edge.from_port != Port::Auto || edge.to_port != Port::Auto || !edge.waypoints.is_empty()
        {
            let start = a.map(|a| a.port_position(edge.from_port, bc)).unwrap_or(ac);
            let end = b.map(|b| b.port_position(edge.to_port, ac)).unwrap_or(bc);
            if edge.kind == ConnectorKind::Straight {
                return vec![start, end];
            }
            if !edge.waypoints.is_empty() {
                let mut points = vec![start];
                for p in edge.waypoints.iter().chain(std::iter::once(&end)) {
                    let prev = *points.last().unwrap();
                    if (prev[0] - p[0]).abs() > 0.001 && (prev[1] - p[1]).abs() > 0.001 {
                        points.push([p[0], prev[1]]);
                    }
                    points.push(*p);
                }
                return simplify_route(points);
            }
            return simplify_route(crate::routing::orthogonal(
                a,
                b,
                ac,
                bc,
                edge.from_port,
                edge.to_port,
            ));
        }
        if edge.kind == ConnectorKind::Straight {
            return vec![
                a.map(|a| a.boundary_towards(bc)).unwrap_or(ac),
                b.map(|b| b.boundary_towards(ac)).unwrap_or(bc),
            ];
        }
        if (bc[0] - ac[0]).abs() > (bc[1] - ac[1]).abs() {
            let start = a.map(|a| a.boundary_towards([bc[0], ac[1]])).unwrap_or(ac);
            let end = b.map(|b| b.boundary_towards([ac[0], bc[1]])).unwrap_or(bc);
            let mid = (start[0] + end[0]) / 2.;
            vec![start, [mid, start[1]], [mid, end[1]], end]
        } else {
            let start = a.map(|a| a.boundary_towards([ac[0], bc[1]])).unwrap_or(ac);
            let end = b.map(|b| b.boundary_towards([bc[0], ac[1]])).unwrap_or(bc);
            let mid = (start[1] + end[1]) / 2.;
            vec![start, [start[0], mid], [end[0], mid], end]
        }
    }
    pub fn hit_connector(&self, p: Point, tolerance: f32) -> Option<u64> {
        let organization = self.organization_view();
        self.connectors
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                organization.is_object_visible(c.id)
                    && self
                        .route(c)
                        .windows(2)
                        .any(|s| segment_distance(p, s[0], s[1]) <= tolerance)
            })
            .max_by_key(|(index, c)| {
                (
                    organization.layer_order(organization.object_layer(c.id)),
                    *index,
                )
            })
            .map(|(_, c)| c.id)
    }
    pub fn edge_label_point(&self, edge: &Connector) -> Point {
        let route = self.route(edge);
        if route.len() < 2 {
            return [0., 0.];
        }
        let total: f32 = route
            .windows(2)
            .map(|p| (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]))
            .sum();
        let mut remain = total * edge.label_t;
        for pair in route.windows(2) {
            let a = pair[0];
            let b = pair[1];
            let len = (b[0] - a[0]).hypot(b[1] - a[1]);
            if remain <= len && len > 0.001 {
                let t = remain / len;
                return [
                    a[0] + (b[0] - a[0]) * t + if (b[0] - a[0]).abs() < 0.01 { 12. } else { 0. },
                    a[1] + (b[1] - a[1]) * t - 10.,
                ];
            }
            remain -= len;
        }
        route[route.len() - 1]
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != FORMAT_VERSION {
            return Err(format!(
                "Version de document non prise en charge : {}",
                self.format_version
            ));
        }
        if self.pages.len() >= 100 || self.page_index >= self.page_count() {
            return Err("Liste de pages invalide.".into());
        }
        if self.page_name.trim().is_empty() || self.page_name.len() > 800 {
            return Err("Nom de page invalide.".into());
        }
        for page in &self.pages {
            Self::from_page(page.clone(), &self.title).validate()?;
        }
        if self.object_count() > 10_000 {
            return Err("Le document dépasse 10 000 objets.".into());
        }
        if self.title.len() > 4096 {
            return Err("Titre trop long.".into());
        }
        if !self.page_style.grid_spacing.is_finite()
            || !(5.0..=200.0).contains(&self.page_style.grid_spacing)
            || !self.page_style.margin.is_finite()
            || !(0.0..=1000.0).contains(&self.page_style.margin)
            || self.page_style.watermark.len() > 1000
        {
            return Err("Paramètres de page invalides.".into());
        }
        if !self
            .page_size
            .iter()
            .all(|v| v.is_finite() && (100.0..=100_000.0).contains(v))
        {
            return Err("Dimensions de page invalides.".into());
        }
        let mut ids = HashSet::new();
        for n in &self.nodes {
            n.appearance.validate()?;
            n.validate_text_runs()?;
            if n.id == 0 || n.id == u64::MAX || !ids.insert(n.id) {
                return Err("Identifiant de forme invalide ou dupliqué.".into());
            }
            if !n
                .position
                .iter()
                .all(|v| v.is_finite() && v.abs() <= 100_000.)
                || !n
                    .size
                    .iter()
                    .all(|v| v.is_finite() && (24.0..=20_000.0).contains(v))
            {
                return Err("Géométrie de forme invalide.".into());
            }
            if !n.font_size.is_finite()
                || !(6.0..=200.0).contains(&n.font_size)
                || !n.stroke_width.is_finite()
                || !(0.0..=20.0).contains(&n.stroke_width)
                || n.text.len() > 65_536
            {
                return Err("Style ou texte de forme invalide.".into());
            }
        }
        let node_ids: HashSet<_> = self.nodes.iter().map(|n| n.id).collect();
        for c in &self.charts {
            if c.id == 0 || c.id == u64::MAX || !ids.insert(c.id) {
                return Err("Identifiant de graphique invalide.".into());
            }
            c.validate()?;
        }
        for d in &self.drawings {
            if d.id == 0
                || d.id == u64::MAX
                || !ids.insert(d.id)
                || !d
                    .start
                    .iter()
                    .chain(d.end.iter())
                    .all(|v| v.is_finite() && v.abs() <= 100_000.)
                || !d.bend.is_finite()
                || d.bend.abs() > 10_000.
                || !d.width.is_finite()
                || !(0.5..=20.).contains(&d.width)
            {
                return Err("Tracé invalide.".into());
            }
            d.appearance.validate()?;
        }
        for c in &self.connectors {
            c.appearance.validate()?;
            if c.id == 0
                || c.id == u64::MAX
                || !ids.insert(c.id)
                || (c.from != 0 && c.from == c.to)
                || !valid_endpoint(c.from, c.free_start, &node_ids)
                || !valid_endpoint(c.to, c.free_end, &node_ids)
            {
                return Err("Connecteur invalide ou extrémité absente.".into());
            }
            if !c.width.is_finite() || !(0.5..=20.).contains(&c.width) || c.label.len() > 4096 {
                return Err("Style de connecteur invalide.".into());
            }
            if !c.label_t.is_finite()
                || !(0.0..=1.0).contains(&c.label_t)
                || c.waypoints.len() > 16
                || c.waypoints
                    .iter()
                    .flatten()
                    .any(|v| !v.is_finite() || v.abs() > 100_000.)
            {
                return Err("Points de connecteur invalides.".into());
            }
        }
        self.validate_organization()?;
        Ok(())
    }
    pub fn upgrade_legacy(&mut self) {
        if matches!(self.format_version, 1..=6) {
            self.format_version = FORMAT_VERSION;
        }
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        use std::io::Read;
        let file = fs::File::open(path).map_err(|e| e.to_string())?;
        if file.metadata().map_err(|e| e.to_string())?.len() > MAX_FILE_BYTES {
            return Err("Fichier trop volumineux (maximum 16 Mio).".into());
        }
        let mut bytes = Vec::new();
        file.take(MAX_FILE_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Fichier trop volumineux.".into());
        }
        let mut doc: Self =
            serde_json::from_slice(&bytes).map_err(|e| format!("Document JSON invalide : {e}"))?;
        doc.upgrade_legacy();
        doc.validate()?;
        Ok(doc)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Document trop volumineux (maximum 16 Mio).".into());
        }
        atomic_write(path, &bytes)
    }
    pub fn demo() -> Self {
        let mut d = Self {
            title: "De l’idée au diagramme".into(),
            ..Self::default()
        };
        let header = d.add_node(ShapeKind::Text, [640., 80.]);
        {
            let n = d.node_mut(header).unwrap();
            n.text = "Donnez forme à vos idées".into();
            n.size = [600., 52.];
            n.position = [340., 50.];
            n.font_size = 30.;
        }
        let subtitle = d.add_node(ShapeKind::Text, [640., 115.]);
        {
            let n = d.node_mut(subtitle).unwrap();
            n.text = "Un parcours simple, prêt à devenir le vôtre.".into();
            n.size = [600., 36.];
            n.position = [340., 105.];
            n.font_size = 16.;
            n.text_color = [132, 143, 153];
        }
        let a = d.add_node(ShapeKind::Rounded, [540., 230.]);
        d.node_mut(a).unwrap().text = "Une idée".into();
        let b = d.add_node(ShapeKind::Rectangle, [540., 390.]);
        {
            let n = d.node_mut(b).unwrap();
            n.text = "Dessiner le parcours".into();
            n.fill = [237, 241, 254];
            n.stroke = [113, 131, 207];
        }
        let c = d.add_node(ShapeKind::Diamond, [540., 565.]);
        {
            let n = d.node_mut(c).unwrap();
            n.text = "Prêt à partager ?".into();
            n.size = [240., 140.];
            n.position = [420., 495.];
            n.fill = [255, 246, 225];
            n.stroke = [202, 158, 66];
            n.font_size = 17.;
        }
        let e = d.add_node(ShapeKind::Rectangle, [930., 565.]);
        {
            let n = d.node_mut(e).unwrap();
            n.text = "Ajuster les détails".into();
            n.fill = [252, 237, 236];
            n.stroke = [202, 140, 132];
        }
        let f = d.add_node(ShapeKind::Rounded, [540., 750.]);
        d.node_mut(f).unwrap().text = "Exporter en SVG".into();
        d.connect(a, b, ConnectorKind::Elbow);
        d.connect(b, c, ConnectorKind::Elbow);
        let yes = d.connect(c, f, ConnectorKind::Elbow).unwrap();
        d.connectors.iter_mut().find(|e| e.id == yes).unwrap().label = "Oui".into();
        let no = d.connect(c, e, ConnectorKind::Elbow).unwrap();
        d.connectors.iter_mut().find(|e| e.id == no).unwrap().label = "À peaufiner".into();
        d.connect(e, b, ConnectorKind::Elbow);
        d
    }
}

fn valid_endpoint(id: u64, free: Option<Point>, nodes: &HashSet<u64>) -> bool {
    if id == 0 {
        free.is_some_and(|p| p.iter().all(|v| v.is_finite() && v.abs() <= 100_000.))
    } else {
        free.is_none() && nodes.contains(&id)
    }
}
fn simplify_route(mut points: Vec<Point>) -> Vec<Point> {
    points.dedup_by(|a, b| (a[0] - b[0]).abs() + (a[1] - b[1]).abs() < 0.001);
    let mut out: Vec<Point> = vec![];
    for p in points {
        while out.len() >= 2 {
            let a = out[out.len() - 2];
            let b = out[out.len() - 1];
            if ((b[0] - a[0]) * (p[1] - b[1]) - (b[1] - a[1]) * (p[0] - b[0])).abs() < 0.001
                && (b[0] - a[0]) * (p[0] - b[0]) + (b[1] - a[1]) * (p[1] - b[1]) >= 0.
            {
                out.pop();
            } else {
                break;
            }
        }
        out.push(p);
    }
    out
}

pub fn segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let dx = b[0] - a[0];
    let dy = b[1] - a[1];
    let len = dx * dx + dy * dy;
    if len < 0.00001 {
        return ((p[0] - a[0]).powi(2) + (p[1] - a[1]).powi(2)).sqrt();
    }
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len).clamp(0., 1.);
    ((p[0] - a[0] - t * dx).powi(2) + (p[1] - a[1] - t * dy).powi(2)).sqrt()
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    tmp.write_all(bytes)
        .and_then(|_| tmp.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    tmp.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
