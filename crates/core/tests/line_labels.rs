use diagcraft::{
    agent,
    document::{ConnectorKind, Document, DrawingKind, Point, ShapeKind},
    raster::{self, PngOptions},
    routing, svg, text_layout,
};
use serde_json::json;

fn scene() -> (Document, u64, u64) {
    let mut doc = Document {
        page_size: [800., 600.],
        ..Default::default()
    };
    let a = doc.add_node(ShapeKind::Rectangle, [150., 300.]);
    let b = doc.add_node(ShapeKind::Rectangle, [650., 300.]);
    let edge = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    let drawing = doc.add_drawing(DrawingKind::Bezier, [100., 100.], [500., 100.]);
    (doc, edge, drawing)
}
fn close(a: Point, b: Point) {
    for axis in 0..2 {
        assert!((a[axis] - b[axis]).abs() < 0.02, "{a:?} vs {b:?}");
    }
}

#[test]
fn optional_label_defaults_preserve_canonical_legacy_documents_and_revisions() {
    let (doc, _, _) = scene();
    let canonical = serde_json::to_value(&doc).unwrap();
    assert!(canonical["connectors"][0].get("label_offset").is_none());
    let drawing = &canonical["drawings"][0];
    for key in ["label", "label_t", "label_offset"] {
        assert!(drawing.get(key).is_none());
    }
    let mut explicit = canonical.clone();
    explicit["connectors"][0]["label_offset"] = json!([0., 0.]);
    explicit["drawings"][0]["label"] = json!("");
    explicit["drawings"][0]["label_t"] = json!(0.5);
    explicit["drawings"][0]["label_offset"] = json!([0., 0.]);
    let loaded: Document = serde_json::from_value(explicit).unwrap();
    assert_eq!(loaded, doc);
    assert_eq!(serde_json::to_value(&loaded).unwrap(), canonical);
    assert_eq!(agent::revision(&loaded), agent::revision(&doc));
    for file in ["parcours-v1.draw", "connecteurs-v6.draw", "dossier-v7.draw"] {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples")
            .join(file);
        let loaded = Document::load(&path).unwrap();
        assert!(loaded.connectors.iter().all(|e| e.label_offset == [0.; 2]));
        assert!(
            loaded
                .drawings
                .iter()
                .all(|d| d.label.is_empty() && d.label_t == 0.5 && d.label_offset == [0.; 2])
        );
        loaded.validate().unwrap();
    }
}

#[test]
fn labels_round_trip_on_all_pages_and_invalid_values_fail_validation() {
    let (mut doc, _, _) = scene();
    doc.connectors[0].label = "Route\n中文".into();
    doc.connectors[0].label_t = 0.25;
    doc.connectors[0].label_offset = [17., -39.];
    doc.drawings[0].label = "Courbe\nfrançaise".into();
    doc.drawings[0].label_t = 0.8;
    doc.drawings[0].label_offset = [-45., 63.];
    doc.add_page(true).unwrap();
    let folder = tempfile::tempdir().unwrap();
    let path = folder.path().join("labels.draw");
    doc.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), doc);
    let (source, _, _) = scene();
    for invalid in [f32::NAN, f32::INFINITY, -0.01, 1.01] {
        let mut bad = source.clone();
        bad.drawings[0].label_t = invalid;
        assert!(bad.validate().is_err());
        let mut bad = source.clone();
        bad.connectors[0].label_t = invalid;
        assert!(bad.validate().is_err());
    }
    for invalid in [f32::NAN, f32::INFINITY, -100_001., 100_001.] {
        let mut bad = source.clone();
        bad.drawings[0].label_offset = [invalid, 0.];
        assert!(bad.validate().is_err());
        let mut bad = source.clone();
        bad.connectors[0].label_offset = [0., invalid];
        assert!(bad.validate().is_err());
    }
    let mut accepted = source.clone();
    accepted.drawings[0].label = "中".repeat(1365);
    accepted.validate().unwrap();
    accepted.drawings[0].label.push('中');
    assert!(accepted.validate().is_err());
    let mut accepted = source;
    accepted.connectors[0].label = "x".repeat(4096);
    accepted.validate().unwrap();
    accepted.connectors[0].label.push('x');
    assert!(accepted.validate().is_err());
}

#[test]
fn anchor_projection_and_label_positions_follow_live_geometry_with_constant_offset() {
    assert_eq!(routing::project_label(&[], [2., 3.]), 0.5);
    assert_eq!(routing::project_label(&[[1., 1.], [1., 1.]], [2., 3.]), 0.5);
    let route = [[0., 0.], [0., 0.], [100., 0.], [100., 100.]];
    assert_eq!(routing::project_label(&route, [80., 20.]), 0.4);
    assert_eq!(routing::project_label(&route, [120., 50.]), 0.75);
    assert_eq!(routing::project_label(&route, [-20., 0.]), 0.);
    assert_eq!(routing::project_label(&route, [100., 200.]), 1.);
    let (mut doc, _, _) = scene();
    doc.connectors[0].label_t = 0.25;
    doc.connectors[0].label_offset = [30., 40.];
    for shift in [[0., 0.], [70., -50.]] {
        doc.nodes[0].position[0] += shift[0];
        doc.nodes[0].position[1] += shift[1];
        let route = doc.route(&doc.connectors[0]);
        let expected = routing::label_position(&route, 0.25);
        close(
            doc.edge_label_point(&doc.connectors[0]),
            [expected[0] + 30., expected[1] + 40.],
        );
    }
    for kind in [DrawingKind::Line, DrawingKind::Bezier] {
        doc.drawings[0].kind = kind;
        doc.drawings[0].label_t = 0.7;
        doc.drawings[0].label_offset = [-50., 80.];
        let route = doc.drawings[0].points();
        let at = routing::label_position(&route, 0.7);
        close(doc.drawings[0].label_position(), [at[0] - 50., at[1] + 80.]);
        let before = doc.drawings[0].label_position();
        let drawing = &mut doc.drawings[0];
        for p in [&mut drawing.start, &mut drawing.end] {
            p[0] += 25.;
            p[1] -= 16.;
        }
        close(
            doc.drawings[0].label_position(),
            [before[0] + 25., before[1] - 16.],
        );
    }
}

#[test]
fn drawing_and_connector_labels_export_multiline_escaped_and_selection_encloses_offset() {
    let (mut doc, edge, drawing) = scene();
    doc.connectors[0].label = "A & B\n中文".into();
    doc.connectors[0].label_offset = [30., 170.];
    doc.drawings[0].label = "Courbe <test>\nDeuxième ligne".into();
    doc.drawings[0].label_offset = [-20., 120.];
    let source = doc.clone();
    let exported = svg::export(&doc).unwrap();
    for text in ["A &amp; B", "中文", "Courbe &lt;test&gt;", "Deuxième ligne"] {
        assert!(exported.contains(&format!(">{text}</tspan>")));
    }
    for (id, label, center) in [
        (
            edge,
            doc.connectors[0].label.as_str(),
            doc.edge_label_point(&doc.connectors[0]),
        ),
        (
            drawing,
            doc.drawings[0].label.as_str(),
            doc.drawings[0].label_position(),
        ),
    ] {
        let region = raster::region(&doc, &[id]).unwrap();
        let label_bounds = text_layout::line_label_bounds(label, center);
        assert!(region[0] <= label_bounds[0] && region[1] <= label_bounds[1]);
        assert!(
            region[0] + region[2] >= label_bounds[2] && region[1] + region[3] >= label_bounds[3]
        );
        let bytes = raster::export(
            &doc,
            &PngOptions {
                selection: vec![id],
                width: 800,
                height: 600,
                transparent: true,
                ..Default::default()
            },
        )
        .unwrap();
        let image = image::load_from_memory(&bytes).unwrap().to_rgba8();
        let y = ((center[1] - region[1]) / region[3] * 600.) as u32;
        assert!(
            image
                .enumerate_pixels()
                .any(|(_, py, p)| py.abs_diff(y) < 15 && p[3] > 0)
        );
    }
    assert!(
        diagcraft::pdf::export(&doc, diagcraft::pdf::PageFormat::Drawing)
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(doc, source);
    let inspect = agent::inspect(&doc).unwrap();
    assert_eq!(
        inspect["pages"][0]["connectors"][0]["label_position"],
        json!(doc.edge_label_point(&doc.connectors[0]))
    );
    assert_eq!(
        inspect["pages"][0]["drawings"][0]["label_position"],
        json!(doc.drawings[0].label_position())
    );
}

#[test]
fn label_agent_edits_are_atomic_revision_checked_and_respect_locks() {
    let (doc, edge, drawing) = scene();
    let patch = |doc: &Document, ops: serde_json::Value| {
        agent::parse_patch(
            &json!({"api_version":1,"expected_revision":agent::revision(doc),"operations":ops})
                .to_string(),
        )
        .unwrap()
    };
    let ops = json!([
        {"op":"update_connector","page":0,"target":edge,"changes":{"label":"Link","label_t":0.3,"label_offset":[20,50]}},
        {"op":"update_drawing","page":0,"target":drawing,"start":[100,100],"end":[500,100],"bend":60,"label":"Drawing","label_t":0.75,"label_offset":[-25,80]},
        {"op":"add_drawing","page":0,"alias":"new","kind":"line","start":[80,400],"end":[500,400],"label":"New","label_t":0.9,"label_offset":[20,-30]}
    ]);
    let preview = agent::preview(&doc, &patch(&doc, ops.clone())).unwrap();
    assert_eq!(preview.document.connectors[0].label_offset, [20., 50.]);
    assert_eq!(preview.document.drawings[0].label, "Drawing");
    assert_eq!(preview.document.drawings[1].label, "New");
    let before = doc.clone();
    let invalid = json!([
        {"op":"update_connector","page":0,"target":edge,"changes":{"label":"changed"}},
        {"op":"update_drawing","page":0,"target":drawing,"start":[100,100],"end":[500,100],"bend":60,"label_offset":[100001,0]}
    ]);
    assert!(agent::preview(&doc, &patch(&doc, invalid)).is_err());
    assert_eq!(doc, before);
    let mut locked = doc.clone();
    locked.set_objects_locked(&[drawing], true).unwrap();
    assert!(agent::preview(&locked, &patch(&locked, ops)).is_err());
    let mut stale = doc.clone();
    stale.title = "new revision".into();
    assert!(agent::preview(&stale,&patch(&doc,json!([{"op":"update_connector","page":0,"target":edge,"changes":{"label":"stale"}}]))).is_err());
}

#[test]
fn splitting_and_duplication_preserve_one_label_and_relative_offset() {
    let (mut doc, edge, _) = scene();
    doc.connectors[0].label = "Original".into();
    doc.connectors[0].label_offset = [20., 30.];
    doc.insert_shape_on_connector(edge, ShapeKind::Rectangle, [400., 300.])
        .unwrap();
    assert_eq!(doc.connectors[0].label, "Original");
    assert_eq!(doc.connectors[0].label_offset, [20., 30.]);
    assert!(doc.connectors[1].label.is_empty());
    assert_eq!(doc.connectors[1].label_offset, [0.; 2]);
    let drawing = doc.drawings[0].id;
    doc.drawings[0].label = "Duplicated".into();
    doc.drawings[0].label_offset = [40., 50.];
    let at = doc.drawings[0].label_position();
    let duplicate = doc.duplicate_objects(&[drawing], [24., 24.]).unwrap();
    let copy = doc.drawings.iter().find(|d| d.id == duplicate[0]).unwrap();
    assert_eq!(copy.label, "Duplicated");
    assert_eq!(copy.label_offset, [40., 50.]);
    close(copy.label_position(), [at[0] + 24., at[1] + 24.]);
}

#[test]
fn fit_page_and_diagnostics_include_labels_displaced_beyond_page_edges() {
    let (mut doc, _, _) = scene();
    doc.drawings[0].label = "Far drawing".into();
    doc.drawings[0].label_offset = [-1000., -1000.];
    doc.connectors[0].label = "Far connector".into();
    doc.connectors[0].label_offset = [1000., 1000.];
    assert!(
        agent::diagnostics(&doc)
            .iter()
            .filter(|w| w["code"] == "outside_page")
            .count()
            >= 2
    );
    doc.fit_page_to_drawing().unwrap();
    for (label, center) in [
        (&doc.drawings[0].label, doc.drawings[0].label_position()),
        (
            &doc.connectors[0].label,
            doc.edge_label_point(&doc.connectors[0]),
        ),
    ] {
        let bounds = text_layout::line_label_bounds(label, center);
        assert!(
            bounds[0] >= 0.
                && bounds[1] >= 0.
                && bounds[2] <= doc.page_size[0]
                && bounds[3] <= doc.page_size[1]
        );
    }
}
