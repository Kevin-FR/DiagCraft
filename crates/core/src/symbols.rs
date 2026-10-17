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
pub fn preset(kind: ShapeKind) -> Node {
    let mut d = Document::default();
    let id = d.add_node(kind, [0., 0.]);
    let mut n = d.node(id).unwrap().clone();
    n.text_style.wrap = true;
    match kind {
        Triangle | RightTriangle | Pentagon | Octagon | Plus | Cross | Heart | Shield | Drop
        | Star4 | Star8 | Gear | Cube | Cone | Pyramid | Cloud => {
            n.size = [140., 140.];
            n.text.clear();
        }
        BraceLeft | BraceRight => {
            n.size = [50., 150.];
            n.text.clear();
        }
        Lightning => {
            n.size = [90., 140.];
            n.text.clear();
            n.fill = [255, 223, 80];
        }
        UmlClass => {
            n.size = [240., 150.];
            n.text = "Classe\n\n+ attribut : Type\n\n+ méthode()".into();
            n.font_size = 18.;
            n.text_style.monospace = true;
        }
        Component => {
            n.size = [230., 120.];
            n.text = "Composant".into();
        }
        Package => {
            n.size = [240., 150.];
            n.text = "Paquet".into();
        }
        Actor => {
            n.size = [90., 150.];
            n.text.clear();
        }
        Lifeline => {
            n.size = [160., 250.];
            n.text.clear();
        }
        Fork => {
            n.size = [240., 24.];
            n.fill = [35, 51, 65];
            n.text.clear();
        }
        InitialState | FinalState => {
            n.size = [48., 48.];
            n.fill = if kind == InitialState {
                [35, 51, 65]
            } else {
                [255, 255, 255]
            };
            n.text.clear();
        }
        UseCase => {
            n.size = [240., 100.];
            n.text = "Cas d’utilisation".into();
        }
        Frame | Swimlane | TitleBlock => {
            n.size = [360., 220.];
            n.fill = [255, 255, 255];
            n.text.clear();
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
