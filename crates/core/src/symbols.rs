//! Original symbols shared by the native palette, files and vector exports.
use crate::document::{Document, Node, ShapeKind};
use ShapeKind::*;
pub struct SymbolLibrary {
    pub id: &'static str,
    pub title: &'static str,
    pub category: &'static str,
    pub kinds: &'static [ShapeKind],
}
pub const LIBRARIES: &[SymbolLibrary] = &[
    SymbolLibrary {
        id: "basic",
        title: "Formes simples",
        category: "Base",
        kinds: &[
            Rectangle,
            Rounded,
            Ellipse,
            Diamond,
            Triangle,
            RightTriangle,
            Pentagon,
            Hexagon,
            Octagon,
            Plus,
            Cross,
            Heart,
            Cloud,
            Shield,
            Drop,
            Star,
            Star4,
            Star8,
            Gear,
            Lightning,
        ],
    },
    SymbolLibrary {
        id: "flowchart",
        title: "Diagramme de flux",
        category: "Base",
        kinds: &ShapeKind::FLOWCHART,
    },
    SymbolLibrary {
        id: "arrows",
        title: "Formes de flèche",
        category: "Base",
        kinds: &ShapeKind::ARROWS,
    },
    SymbolLibrary {
        id: "containers",
        title: "Conteneurs et cadres",
        category: "Base",
        kinds: &[Frame, Swimlane, TitleBlock, Package],
    },
    SymbolLibrary {
        id: "annotations",
        title: "Annotations",
        category: "Base",
        kinds: &[Text, Note, Callout, Star, BraceLeft, BraceRight],
    },
    SymbolLibrary {
        id: "blocks",
        title: "Blocs en perspective",
        category: "Base",
        kinds: &[Cube, Cone, Pyramid, Database],
    },
    SymbolLibrary {
        id: "mindmap",
        title: "Carte mentale",
        category: "Idées",
        kinds: &[Rounded, Ellipse, Cloud, Callout, Star, Heart, Lightning],
    },
    SymbolLibrary {
        id: "uml",
        title: "Modélisation UML",
        category: "Logiciel",
        kinds: &[
            UmlClass,
            Component,
            Package,
            Actor,
            Lifeline,
            Fork,
            InitialState,
            FinalState,
            UseCase,
            Note,
            Diamond,
        ],
    },
];
pub fn default_visible() -> Vec<String> {
    ["personal", "flowchart", "arrows", "annotations"]
        .map(str::to_owned)
        .to_vec()
}
pub fn known(id: &str) -> bool {
    id == "personal" || LIBRARIES.iter().any(|l| l.id == id)
}
pub fn all_kinds() -> Vec<ShapeKind> {
    let mut kinds = vec![];
    for l in LIBRARIES {
        for &k in l.kinds {
            if !kinds.contains(&k) {
                kinds.push(k)
            }
        }
    }
    kinds
}
/// New-object dimensions in document pixels (96 dpi), shared by every insertion path.
/// Page format and interface zoom never rescale these defaults or existing objects.
pub(crate) fn default_size(kind: ShapeKind) -> crate::document::Point {
    match kind {
        Text => [300., 48.],
        Diamond => [160., 120.],
        Hexagon | Document | Data | Terminator | Database | Trapezoid | ManualInput
        | ManualOperation | Delay | Display | Note | Callout => [120., 90.],
        OffPage => [150., 90.],
        Triangle | RightTriangle | Pentagon | Octagon | Plus | Cross | Heart | Shield | Drop
        | Star | Star4 | Star8 | Gear | Cube | Cone | Pyramid | Cloud => [80., 80.],
        BraceLeft | BraceRight => [30., 90.],
        Lightning => [50., 80.],
        UmlClass => [180., 120.],
        Component => [140., 80.],
        Package => [160., 100.],
        Actor => [54., 90.],
        Lifeline => [100., 160.],
        Fork => [140., 24.],
        InitialState | FinalState => [32., 32.],
        UseCase => [160., 90.],
        Frame | Swimlane | TitleBlock => [300., 180.],
        _ => [120., 60.],
    }
}
/// Decorative shapes begin without a caption in both direct and palette creation.
/// Explicit text in files, agent operations and personal symbols is unaffected.
pub(crate) fn default_caption(kind: ShapeKind) -> &'static str {
    if ShapeKind::ARROWS.contains(&kind) {
        return "";
    }
    match kind {
        Triangle | RightTriangle | Pentagon | Octagon | Plus | Cross | Heart | Shield | Drop
        | Star | Star4 | Star8 | Gear | Cube | Cone | Pyramid | Cloud | BraceLeft | BraceRight
        | Lightning | Actor | Lifeline | Fork | InitialState | FinalState | Frame | Swimlane
        | TitleBlock => "",
        Text => "Votre texte",
        _ => kind.label(),
    }
}
pub fn preset(kind: ShapeKind) -> Node {
    let mut d = Document::default();
    let id = d.add_node(kind, [0., 0.]);
    let mut n = d.node(id).unwrap().clone();
    n.text_style.wrap = true;
    match kind {
        Lightning => {
            n.fill = [255, 223, 80];
        }
        UmlClass => {
            n.text = "Classe\n\n+ attribut : Type\n\n+ méthode()".into();
            n.font_size = 12.;
            n.text_style.monospace = true;
        }
        Component => {
            n.text = "Composant".into();
        }
        Package => {
            n.text = "Paquet".into();
        }
        Fork => {
            n.fill = [35, 51, 65];
        }
        InitialState | FinalState => {
            n.fill = if kind == InitialState {
                [35, 51, 65]
            } else {
                [255, 255, 255]
            };
        }
        UseCase => {
            n.text = "Cas d’utilisation".into();
        }
        Frame | Swimlane | TitleBlock => {
            n.fill = [255, 255, 255];
        }
        _ => {}
    }
    n
}
pub fn demo() -> Document {
    let kinds = all_kinds();
    let mut d = Document {
        title: "Bibliothèque de symboles".into(),
        page_size: [1440., (kinds.len().div_ceil(6) * 180 + 100) as f32],
        ..Default::default()
    };
    for (i, k) in kinds.into_iter().enumerate() {
        let x = 120. + (i % 6) as f32 * 240.;
        let y = 90. + (i / 6) as f32 * 180.;
        let id = d.add_node(k, [x, y]);
        let mut n = preset(k);
        n.id = id;
        let scale = (130. / n.size[0]).min(90. / n.size[1]);
        n.size = n.size.map(|v| (v * scale).max(24.));
        n.position = [x - n.size[0] / 2., y - n.size[1] / 2.];
        n.text = if k == Text { "T".into() } else { String::new() };
        *d.node_mut(id).unwrap() = n;
        let label = d.add_node(Text, [x, y + 65.]);
        let n = d.node_mut(label).unwrap();
        n.size = [226., 35.];
        n.position = [x - 113., y + 50.];
        n.text = k.label().into();
        n.font_size = 15.;
    }
    d
}
