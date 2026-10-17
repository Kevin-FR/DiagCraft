use diagcraft::{
    chart::{Chart, ChartKind, Primitive},
    document::{ConnectorKind, Document, FORMAT_VERSION, Port, ShapeKind},
    geometry,
    library::Library,
    svg, templates,
};

fn area(p: &[[f32; 2]]) -> f32 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>()
        .abs()
        / 2.
}

#[test]
fn every_symbol_triangulates_without_missing_or_overlapping_area() {
    for kind in ShapeKind::FLOWCHART
        .into_iter()
        .chain(ShapeKind::ARROWS)
        .chain(ShapeKind::ANNOTATIONS)
    {
        let g = geometry::shape(kind);
        let triangles = geometry::triangulate(&g.outline);
        let total: f32 = triangles
            .iter()
            .map(|t| area(&[g.outline[t[0]], g.outline[t[1]], g.outline[t[2]]]))
            .sum();
        assert!(
            (total - area(&g.outline)).abs() < 0.0001,
            "{kind:?}: {total} vs {}",
            area(&g.outline)
        );
        assert!(geometry::contains(&g.outline, [0.5, 0.5]), "{kind:?}");
    }
}
#[test]
fn new_symbols_attach_connectors_to_actual_outline() {
    for kind in ShapeKind::FLOWCHART.into_iter().chain(ShapeKind::ARROWS) {
        let mut d = Document::default();
        let id = d.add_node(kind, [400., 400.]);
        let n = d.node(id).unwrap();
        for target in [
            [800., 400.],
            [400., 800.],
            [0., 400.],
            [400., 0.],
            [800., 800.],
        ] {
            let p = n.boundary_towards(target);
            assert!(p.iter().all(|v| v.is_finite()));
            if !matches!(kind, ShapeKind::Ellipse | ShapeKind::Rounded) {
                let local = [
                    (p[0] - n.position[0]) / n.size[0],
                    (p[1] - n.position[1]) / n.size[1],
                ];
                let g = geometry::shape(kind);
                let nearest = g
                    .outline
                    .iter()
                    .zip(g.outline.iter().cycle().skip(1))
                    .take(g.outline.len())
                    .map(|(a, b)| diagcraft::document::segment_distance(local, *a, *b))
                    .fold(f32::INFINITY, f32::min);
                assert!(nearest < 0.0001, "{kind:?}");
            }
        }
    }
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::ArrowRight, [400., 400.]);
    let n = d.node(id).unwrap();
    assert!(!n.contains(n.position));
    assert!(n.contains(n.center()));
}
#[test]
fn all_chart_types_have_finite_geometry_and_valid_svg() {
    for kind in ChartKind::ALL {
        let mut d = Document::default();
        d.add_chart(kind, [500., 400.]);
        d.validate().unwrap();
        let c = &d.charts[0];
        for primitive in c.primitives() {
            match primitive {
                Primitive::Rect { position, size, .. } => {
                    assert!(position.into_iter().chain(size).all(f32::is_finite))
                }
                Primitive::Path { points, width, .. } => {
                    assert!(width.is_finite());
                    assert!(points.iter().flatten().all(|v| v.is_finite()));
                }
                Primitive::Text { position, size, .. } => {
                    assert!(position.iter().all(|v| v.is_finite()));
                    assert!(size > 0.);
                }
                Primitive::Circle {
                    position, radius, ..
                } => {
                    assert!(position.iter().all(|v| v.is_finite()));
                    assert!(radius > 0.);
                }
            }
        }
        let output = svg::export(&d).unwrap();
        assert!(output.contains("<g transform=\"translate("));
        assert!(!output.contains("NaN"));
        assert!(!output.contains("inf"));
    }
}
#[test]
fn percent_charts_handle_all_zero_data() {
    for kind in [
        ChartKind::PercentColumns,
        ChartKind::PercentBars,
        ChartKind::PercentLine,
    ] {
        let mut c = Chart::sample(1, kind, [0., 0.]);
        for s in &mut c.series {
            s.values.fill(0.);
        }
        let mut d = Document::default();
        d.charts.push(c);
        let output = svg::export(&d).unwrap();
        assert!(output.contains("100%"));
        assert!(!output.contains("NaN"));
    }
}
#[test]
fn chart_stacks_encode_actual_values() {
    let mut c = Chart::sample(1, ChartKind::StackedColumns, [0., 0.]);
    c.categories = vec!["A".into()];
    c.series[0].values = vec![25.];
    c.series[1].values = vec![75.];
    let bars: Vec<_> = c
        .primitives()
        .into_iter()
        .filter_map(|p| {
            if let Primitive::Rect {
                position,
                size,
                fill,
            } = p
            {
                if fill == c.series[0].color || fill == c.series[1].color {
                    Some((position, size))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .take(2)
        .collect();
    assert_eq!(bars[0].0[0], bars[1].0[0]);
    assert!((bars[1].1[1] / bars[0].1[1] - 3.).abs() < 0.001);
    assert!((bars[1].0[1] + bars[1].1[1] - bars[0].0[1]).abs() < 0.001);
}
#[test]
fn chart_data_roundtrip_and_svg_escape() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("chart.draw");
    let mut d = Document::default();
    d.add_chart(ChartKind::StepArea, [500., 400.]);
    d.charts[0].title = "<Titre & données>".into();
    d.charts[0].categories[0] = "<T1>".into();
    d.charts[0].series[0].name = "A&B".into();
    d.charts[0].series[0].values[0] = 123.5;
    d.save(&path).unwrap();
    assert_eq!(d, Document::load(&path).unwrap());
    let output = svg::export(&d).unwrap();
    assert!(output.contains("&lt;Titre &amp; données&gt;"));
    assert!(output.contains("A&amp;B"));
    assert!(output.contains("&lt;T1&gt;"));
}
#[test]
fn invalid_chart_data_is_rejected_before_render_and_save() {
    let base = Chart::sample(1, ChartKind::Line, [0., 0.]);
    for invalid in 0..6 {
        let mut c = base.clone();
        match invalid {
            0 => c.series[0].values[0] = f32::NAN,
            1 => c.series[0].values[0] = -1.,
            2 => c.categories.clear(),
            3 => c.series[0].values.pop().map(|_| ()).unwrap(),
            4 => c.size[0] = 0.,
            _ => c.series.clear(),
        };
        assert!(c.validate().is_err());
        assert!(c.primitives().is_empty());
        let d = Document {
            charts: vec![c],
            ..Default::default()
        };
        assert!(svg::export(&d).is_err());
    }
}
#[test]
fn chart_copy_paste_delete_and_fit_preserve_document_invariants() {
    let mut d = Document::default();
    let chart = d.add_chart(ChartKind::Bars, [500., 400.]);
    let node = d.add_node(ShapeKind::Document, [850., 400.]);
    let fragment = d.selection(&[chart, node]);
    let ids = d.paste(&fragment, [30., 40.]).unwrap();
    assert_eq!(ids.len(), 2);
    assert_eq!(
        d.charts[1].position,
        [d.charts[0].position[0] + 30., d.charts[0].position[1] + 40.]
    );
    assert_ne!(d.charts[0].id, d.charts[1].id);
    d.remove(&[chart]);
    assert_eq!(d.charts.len(), 1);
    d.charts[0].position = [-100., -200.];
    d.fit_page_to_drawing().unwrap();
    assert!(d.charts[0].position.iter().all(|v| *v >= 0.));
    d.validate().unwrap();
}
#[test]
fn manual_routes_preserve_ports_waypoints_and_label_position() {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [200., 200.]);
    let b = d.add_node(ShapeKind::Diamond, [700., 500.]);
    let id = d.connect(a, b, ConnectorKind::Elbow).unwrap();
    let e = &mut d.connectors[0];
    e.from_port = Port::Bottom;
    e.to_port = Port::Right;
    e.waypoints = vec![[200., 400.], [900., 400.], [900., 500.]];
    e.label_t = 0.2;
    e.label = "Oui".into();
    let route = d.route(&d.connectors[0]);
    assert_eq!(
        route[0],
        d.node(a).unwrap().port_position(Port::Bottom, [700., 500.])
    );
    assert_eq!(
        *route.last().unwrap(),
        d.node(b).unwrap().port_position(Port::Right, [200., 200.])
    );
    assert!(
        route
            .windows(2)
            .all(|p| (p[0][0] - p[1][0]).abs() < 0.001 || (p[0][1] - p[1][1]).abs() < 0.001)
    );
    let label = d.edge_label_point(&d.connectors[0]);
    d.connectors[0].label_t = 0.8;
    assert_ne!(label, d.edge_label_point(&d.connectors[0]));
    let fragment = d.selection(&[a, b]);
    let ids = d.paste(&fragment, [20., 30.]).unwrap();
    let pasted = d.connectors.iter().find(|e| e.id != id).unwrap();
    assert!(ids.contains(&pasted.from) && ids.contains(&pasted.to));
    assert_eq!(pasted.waypoints[0], [220., 430.]);
    d.validate().unwrap();
}
#[test]
fn connector_controls_are_validated() {
    let mut d = templates::support();
    d.connectors[0].label_t = 1.1;
    assert!(d.validate().is_err());
    d.connectors[0].label_t = 0.5;
    d.connectors[0].waypoints = vec![[f32::INFINITY, 0.]];
    assert!(d.validate().is_err());
}
#[test]
fn template_catalog_is_valid_and_support_keeps_branches() {
    let items = templates::catalog();
    assert_eq!(items.len(), 9);
    for t in items {
        t.document.validate().unwrap();
        svg::export(&t.document).unwrap();
    }
    let d = templates::support();
    let decision = d
        .nodes
        .iter()
        .find(|n| n.kind == ShapeKind::Diamond)
        .unwrap();
    let exits: Vec<_> = d
        .connectors
        .iter()
        .filter(|c| c.from == decision.id)
        .collect();
    assert_eq!(exits.len(), 2);
    assert!(exits.iter().any(|c| c.label == "Oui"));
    assert!(exits.iter().any(|c| c.label == "Non"));
    let retry = exits.iter().find(|c| c.label == "Non").unwrap().to;
    assert!(d.connectors.iter().any(|c| c.from == retry));
    let s = svg::export(&d).unwrap();
    assert!(s.contains("Décrire le besoin"));
    assert!(!s.contains("diagcraft-grid"));
}
#[test]
fn wrapping_preserves_explicit_breaks_and_unicode() {
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Rectangle, [200., 200.]);
    let n = d.node_mut(id).unwrap();
    n.text = "Éléments à organiser\n第二行".into();
    n.size[0] = 110.;
    n.text_style.wrap = true;
    let wrapped = n.display_text();
    assert!(wrapped.contains("\n第二行"));
    assert!(wrapped.lines().count() > 2);
    assert_eq!(
        wrapped.replace([' ', '\n'], ""),
        n.text.replace([' ', '\n'], "")
    );
    n.text_style.wrap = false;
    assert_eq!(n.display_text(), n.text);
}
#[test]
fn personal_library_roundtrip_and_recent_order() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("nested/library.json");
    let mut lib = Library::default();
    let d = templates::support();
    lib.add_template("Mon modèle", "IT", &d).unwrap();
    lib.add_symbol(&d.nodes[1]).unwrap();
    lib.remember("support");
    lib.remember("algorithm");
    lib.remember("support");
    assert_eq!(lib.recent, vec!["support", "algorithm"]);
    lib.save(&p).unwrap();
    let loaded = Library::load(&p).unwrap();
    assert_eq!(loaded.templates[0].document, d);
    assert_eq!(loaded.symbols.len(), 1);
    assert_eq!(loaded.recent, lib.recent);
}
#[test]
fn failed_library_additions_preserve_existing_data() {
    let mut lib = Library::default();
    lib.add_template("Original", "IT", &Document::default())
        .unwrap();
    let before = serde_json::to_string(&lib).unwrap();
    assert!(
        lib.add_template("Original", "IT", &Document::default())
            .is_err()
    );
    assert!(lib.add_template(" ", "IT", &Document::default()).is_err());
    let invalid = Document {
        page_size: [0., 0.],
        ..Default::default()
    };
    assert!(lib.add_template("Invalide", "IT", &invalid).is_err());
    assert_eq!(before, serde_json::to_string(&lib).unwrap());
}
#[test]
fn version_two_documents_gain_safe_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("old.draw");
    let mut d = Document::demo();
    d.format_version = 2;
    let mut value = serde_json::to_value(&d).unwrap();
    value.as_object_mut().unwrap().remove("charts");
    for e in value["connectors"].as_array_mut().unwrap() {
        for key in ["from_port", "to_port", "waypoints", "label_t"] {
            e.as_object_mut().unwrap().remove(key);
        }
    }
    std::fs::write(&p, serde_json::to_vec(&value).unwrap()).unwrap();
    let loaded = Document::load(&p).unwrap();
    assert_eq!(loaded.format_version, FORMAT_VERSION);
    assert!(loaded.charts.is_empty());
    assert_eq!(loaded.connectors[0].from_port, Port::Auto);
    assert_eq!(loaded.connectors[0].label_t, 0.5);
}
