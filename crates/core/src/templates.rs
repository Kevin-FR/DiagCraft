use crate::document::{ConnectorKind, Document, GridStyle, Port, ShapeKind};

#[derive(Clone)]
pub struct Template {
    pub id: String,
    pub title: String,
    pub category: String,
    pub document: Document,
}
pub fn catalog() -> Vec<Template> {
    [
        ("support", "Traitement d’une demande", "IT", support()),
        (
            "validation",
            "Validation d’un document",
            "Général",
            workflow(
                "Validation d’un document",
                &[
                    "Préparer le document",
                    "Vérifier les informations",
                    "Valider et diffuser",
                ],
                true,
            ),
        ),
        (
            "algorithm",
            "Algorithme de calcul",
            "Ingénierie",
            algorithm(),
        ),
        (
            "deployment",
            "Déploiement logiciel",
            "IT",
            workflow(
                "Déploiement logiciel",
                &[
                    "Préparer la version",
                    "Exécuter les tests",
                    "Déployer en production",
                ],
                true,
            ),
        ),
        (
            "onboarding",
            "Accueil d’un collaborateur",
            "Entreprise",
            workflow(
                "Accueil d’un collaborateur",
                &[
                    "Préparer l’arrivée",
                    "Créer les accès",
                    "Présenter l’équipe",
                    "Planifier la formation",
                ],
                false,
            ),
        ),
        (
            "organization",
            "Organigramme d’équipe",
            "Entreprise",
            organization(),
        ),
        (
            "course",
            "Parcours d’apprentissage",
            "Éducation",
            workflow(
                "Parcours d’apprentissage",
                &["Découvrir le sujet", "S’exercer", "Évaluer les acquis"],
                true,
            ),
        ),
        ("project", "Chronologie de projet", "Général", timeline()),
        (
            "process",
            "De l’idée au diagramme",
            "Général",
            Document::demo(),
        ),
    ]
    .into_iter()
    .map(|(id, title, category, document)| Template {
        id: id.into(),
        title: title.into(),
        category: category.into(),
        document,
    })
    .collect()
}
fn node(d: &mut Document, kind: ShapeKind, text: &str, position: [f32; 2], size: [f32; 2]) -> u64 {
    let id = d.add_node(kind, [0., 0.]);
    let n = d.node_mut(id).unwrap();
    n.position = position;
    n.size = size;
    n.text = text.into();
    n.font_size = 22.;
    n.text_style.wrap = true;
    n.stroke_width = 1.2;
    n.text_color = [32, 36, 42];
    let (fill, stroke) = match kind {
        ShapeKind::Diamond => ([233, 248, 245], [110, 213, 201]),
        ShapeKind::Rounded | ShapeKind::Terminator => ([236, 238, 255], [148, 165, 252]),
        _ => ([255, 247, 234], [248, 208, 126]),
    };
    n.fill = fill;
    n.stroke = stroke;
    id
}
fn title(d: &mut Document, text: &str) {
    let id = node(
        d,
        ShapeKind::Text,
        text,
        [35., 25.],
        [d.page_size[0] - 70., 85.],
    );
    let n = d.node_mut(id).unwrap();
    n.font_size = 34.;
    n.text_style.bold = true;
    n.text_style.align = crate::document::TextAlign::Left;
}
fn link(
    d: &mut Document,
    a: u64,
    b: u64,
    from: Port,
    to: Port,
    label: &str,
    waypoints: Vec<[f32; 2]>,
) {
    let id = d
        .connect(
            a,
            b,
            if waypoints.is_empty()
                && ((matches!(from, Port::Left | Port::Right)
                    && matches!(to, Port::Left | Port::Right))
                    || (matches!(from, Port::Top | Port::Bottom)
                        && matches!(to, Port::Top | Port::Bottom)))
            {
                ConnectorKind::Straight
            } else {
                ConnectorKind::Elbow
            },
        )
        .unwrap();
    let e = d.connectors.iter_mut().find(|e| e.id == id).unwrap();
    e.from_port = from;
    e.to_port = to;
    e.label = label.into();
    e.waypoints = waypoints;
    e.color = [78, 83, 91];
    e.width = 1.5;
}
/// A project-authored request workflow, independent of any external template.
pub fn support() -> Document {
    let mut d = workflow(
        "Une demande, une réponse",
        &[
            "Décrire le besoin",
            "Choisir le bon interlocuteur",
            "Préparer une proposition",
            "Recueillir un retour",
        ],
        true,
    );
    d.page_style.grid_style = GridStyle::Dots;
    d.page_style.export_grid = false;
    d
}
fn workflow(name: &str, steps: &[&str], decision: bool) -> Document {
    let mut d = Document {
        title: name.into(),
        page_size: [1280., 1100.],
        ..Document::default()
    };
    title(&mut d, name);
    let start = node(
        &mut d,
        ShapeKind::Terminator,
        "Début",
        [440., 130.],
        [240., 80.],
    );
    let mut previous = start;
    for (i, step) in steps.iter().enumerate() {
        let id = node(
            &mut d,
            ShapeKind::Rectangle,
            step,
            [420., 270. + i as f32 * 150.],
            [280., 90.],
        );
        link(&mut d, previous, id, Port::Bottom, Port::Top, "", vec![]);
        previous = id;
    }
    let y = 270. + steps.len() as f32 * 150.;
    if decision {
        let check = node(
            &mut d,
            ShapeKind::Diamond,
            "Tout est prêt ?",
            [430., y],
            [260., 140.],
        );
        link(&mut d, previous, check, Port::Bottom, Port::Top, "", vec![]);
        let retry = node(
            &mut d,
            ShapeKind::Rectangle,
            "Ajuster et\nrecommencer",
            [880., y + 20.],
            [240., 100.],
        );
        link(&mut d, check, retry, Port::Right, Port::Left, "Non", vec![]);
        link(&mut d, retry, previous, Port::Top, Port::Right, "", vec![]);
        let end = node(
            &mut d,
            ShapeKind::Terminator,
            "Terminé",
            [440., y + 220.],
            [240., 80.],
        );
        link(&mut d, check, end, Port::Bottom, Port::Top, "Oui", vec![]);
        d.page_size[1] = (y + 360.).max(1100.);
    } else {
        let end = node(
            &mut d,
            ShapeKind::Terminator,
            "Terminé",
            [440., y],
            [240., 80.],
        );
        link(&mut d, previous, end, Port::Bottom, Port::Top, "", vec![]);
    }
    d
}
fn algorithm() -> Document {
    let mut d = Document {
        title: "Algorithme de calcul".into(),
        page_size: [1200., 1150.],
        ..Document::default()
    };
    title(&mut d, "Algorithme de calcul d’une somme");
    let a = node(
        &mut d,
        ShapeKind::Terminator,
        "Début",
        [430., 130.],
        [250., 80.],
    );
    let b = node(&mut d, ShapeKind::Data, "Lire n", [430., 290.], [250., 90.]);
    let c = node(
        &mut d,
        ShapeKind::Rectangle,
        "somme = 0\ni = 1",
        [430., 460.],
        [250., 100.],
    );
    let e = node(
        &mut d,
        ShapeKind::Diamond,
        "i ≤ n ?",
        [430., 650.],
        [250., 140.],
    );
    let f = node(
        &mut d,
        ShapeKind::PredefinedProcess,
        "somme = somme + i\ni = i + 1",
        [840., 660.],
        [280., 110.],
    );
    let g = node(
        &mut d,
        ShapeKind::Data,
        "Afficher la somme",
        [430., 900.],
        [250., 90.],
    );
    link(&mut d, a, b, Port::Bottom, Port::Top, "", vec![]);
    link(&mut d, b, c, Port::Bottom, Port::Top, "", vec![]);
    link(&mut d, c, e, Port::Bottom, Port::Top, "", vec![]);
    link(&mut d, e, f, Port::Right, Port::Left, "Oui", vec![]);
    link(&mut d, f, c, Port::Top, Port::Right, "", vec![]);
    link(&mut d, e, g, Port::Bottom, Port::Top, "Non", vec![]);
    d
}
fn organization() -> Document {
    let mut d = Document {
        title: "Organigramme d’équipe".into(),
        ..Document::default()
    };
    title(&mut d, "Une équipe, des responsabilités claires");
    let lead = node(
        &mut d,
        ShapeKind::Rounded,
        "Direction",
        [480., 180.],
        [280., 100.],
    );
    for (i, name) in ["Produit", "Ingénierie", "Opérations"]
        .into_iter()
        .enumerate()
    {
        let id = node(
            &mut d,
            ShapeKind::Rectangle,
            name,
            [100. + i as f32 * 400., 440.],
            [250., 100.],
        );
        link(&mut d, lead, id, Port::Bottom, Port::Top, "", vec![]);
        let team = node(
            &mut d,
            ShapeKind::Rounded,
            "Équipe",
            [100. + i as f32 * 400., 690.],
            [250., 100.],
        );
        link(&mut d, id, team, Port::Bottom, Port::Top, "", vec![]);
    }
    d
}
fn timeline() -> Document {
    let mut d = Document {
        title: "Chronologie de projet".into(),
        ..Document::default()
    };
    title(&mut d, "Du cadrage à la livraison");
    let mut previous = None;
    for (i, label) in ["Cadrer", "Concevoir", "Réaliser", "Livrer"]
        .into_iter()
        .enumerate()
    {
        let id = node(
            &mut d,
            ShapeKind::Rounded,
            label,
            [70. + i as f32 * 300., 390.],
            [230., 100.],
        );
        if let Some(prev) = previous {
            link(&mut d, prev, id, Port::Right, Port::Left, "", vec![]);
        }
        previous = Some(id);
    }
    d
}

/// Editable chart examples, generated from local data only.
pub fn charts() -> Document {
    use crate::chart::ChartKind;
    let mut d = Document {
        title: "Graphiques – données modifiables".into(),
        page_size: [1100., 840.],
        ..Default::default()
    };
    for (kind, center) in [
        (ChartKind::Columns, [280., 220.]),
        (ChartKind::StackedBars, [820., 220.]),
        (ChartKind::Line, [280., 620.]),
        (ChartKind::Area, [820., 620.]),
    ] {
        d.add_chart(kind, center);
    }
    d
}

/// A small dossier illustrating independent page sizes and free connector ends.
pub fn multipage() -> Document {
    use crate::document::{ConnectorKind, Port};
    use crate::pages::{PageLayout, Unit};
    let mut d = support();
    d.title = "Dossier de diagrammes".into();
    d.page_name = "Demande interne".into();
    d.add_page(false).unwrap();
    d.page_name = "Connecteurs libres".into();
    d.page_size = [1600., 900.];
    let title = d.add_node(ShapeKind::Text, [800., 90.]);
    {
        let n = d.node_mut(title).unwrap();
        n.text = "Reliez vos idées, même en cours de dessin".into();
        n.size = [1400., 64.];
        n.position = [100., 50.];
        n.font_size = 32.;
        n.text_style.bold = true;
    }
    let a = d.add_node(ShapeKind::Rounded, [330., 300.]);
    d.node_mut(a).unwrap().text = "Point de départ".into();
    let b = d.add_node(ShapeKind::Diamond, [1050., 300.]);
    d.node_mut(b).unwrap().text = "À valider ?".into();
    let edge = d.connect(a, b, ConnectorKind::Elbow).unwrap();
    d.connectors
        .iter_mut()
        .find(|e| e.id == edge)
        .unwrap()
        .label = "Lien attaché".into();
    let edge = d
        .connect_endpoints(a, 0, None, Some([620., 620.]), ConnectorKind::Elbow)
        .unwrap();
    d.set_automatic_route(edge, false).unwrap();
    {
        let e = d.connectors.iter_mut().find(|e| e.id == edge).unwrap();
        e.label = "Extrémité libre".into();
        e.pattern = crate::stroke_style::LinePattern::Dashed;
        e.from_port = Port::Bottom;
        e.waypoints = vec![[330., 550.], [620., 550.]];
    }
    let edge = d
        .connect_endpoints(
            0,
            0,
            Some([850., 660.]),
            Some([1350., 660.]),
            ConnectorKind::Straight,
        )
        .unwrap();
    {
        let e = d.connectors.iter_mut().find(|e| e.id == edge).unwrap();
        e.label = "Deux points libres".into();
        e.pattern = crate::stroke_style::LinePattern::Dotted;
        e.arrow_start = true;
        e.start_head = crate::stroke_style::ArrowHead::Circle;
    }
    d.add_page(false).unwrap();
    d.page_name = "Bilan A4 portrait".into();
    d.page_size = crate::pages::presets()[1].size;
    d.page_layout = PageLayout {
        unit: Unit::Millimeters,
        auto_size: false,
    };
    let title = d.add_node(ShapeKind::Text, [397., 100.]);
    {
        let n = d.node_mut(title).unwrap();
        n.text = "Bilan de l’activité".into();
        n.size = [650., 60.];
        n.position = [70., 60.];
        n.font_size = 30.;
        n.text_style.bold = true;
    }
    d.add_chart(crate::chart::ChartKind::Columns, [397., 420.]);
    let note = d.add_node(ShapeKind::Text, [397., 800.]);
    {
        let n = d.node_mut(note).unwrap();
        n.text="Une page A4 indépendante, exportée avec ses dimensions.\nLes autres pages conservent leur propre format.".into();
        n.size = [640., 120.];
        n.position = [77., 740.];
        n.text_style.wrap = true;
    }
    d.switch_page(0).unwrap();
    d.validate().unwrap();
    d
}
