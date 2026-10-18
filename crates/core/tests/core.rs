use diagcraft::{
    document::{
        ConnectorKind, Document, FORMAT_VERSION, MAX_FILE_BYTES, Node, Point, ShapeKind,
        atomic_write, segment_distance,
    },
    history::History,
    svg,
};
use std::{collections::HashSet, fs};

fn pair(kind: ConnectorKind) -> (Document, u64, u64, u64) {
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    let b = doc.add_node(ShapeKind::Rectangle, [600., 200.]);
    let edge = doc.connect(a, b, kind).unwrap();
    (doc, a, b, edge)
}

fn point_close(actual: Point, expected: Point) {
    assert!(
        (actual[0] - expected[0]).abs() < 0.001 && (actual[1] - expected[1]).abs() < 0.001,
        "expected {expected:?}, got {actual:?}"
    );
}

fn normalized_boundary(node: &Node, point: Point) -> f32 {
    let center = node.center();
    let x = (point[0] - center[0]).abs() / (node.size[0] / 2.);
    let y = (point[1] - center[1]).abs() / (node.size[1] / 2.);
    match node.kind {
        ShapeKind::Ellipse => x * x + y * y,
        ShapeKind::Diamond => x + y,
        _ => x.max(y),
    }
}

#[test]
fn defaults_and_demo_are_valid() {
    assert_eq!(Document::default().format_version, FORMAT_VERSION);
    Document::default().validate().unwrap();
    let demo = Document::demo();
    demo.validate().unwrap();
    assert!(demo.nodes.iter().any(|n| n.kind == ShapeKind::Text));
    assert!(!demo.connectors.is_empty());
}

#[test]
fn ids_are_unique_across_shapes_and_connectors() {
    let (mut doc, a, _, edge) = pair(ConnectorKind::Straight);
    let third = doc.add_node(ShapeKind::Ellipse, [300., 400.]);
    assert!(third > edge);
    assert_eq!(doc.next_id(), third + 1);
    doc.validate().unwrap();

    for invalid in [0, u64::MAX, a] {
        let mut bad = doc.clone();
        bad.nodes.last_mut().unwrap().id = invalid;
        assert!(bad.validate().is_err(), "accepted shape id {invalid}");
    }
    for invalid in [0, u64::MAX, a] {
        let mut bad = doc.clone();
        bad.connectors[0].id = invalid;
        assert!(bad.validate().is_err(), "accepted connector id {invalid}");
    }
}

#[test]
fn editing_a_valid_document_with_large_ids_keeps_ids_valid() {
    let mut doc = Document::default();
    doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    doc.nodes[0].id = u64::MAX - 1;
    doc.validate().unwrap();
    let fresh = doc.add_node(ShapeKind::Ellipse, [600., 200.]);
    assert!(fresh > 0 && fresh < u64::MAX);
    assert_ne!(fresh, u64::MAX - 1);
    doc.validate().unwrap();
    let edge = doc
        .connect(u64::MAX - 1, fresh, ConnectorKind::Straight)
        .unwrap();
    assert!(edge > 0 && edge < u64::MAX && edge != fresh);
    doc.validate().unwrap();
}

#[test]
fn rejects_unsupported_versions_and_invalid_page_dimensions() {
    for version in [0, FORMAT_VERSION + 1, u32::MAX] {
        let doc = Document {
            format_version: version,
            ..Document::default()
        };
        assert!(doc.validate().is_err());
    }
    for value in [f32::NAN, f32::INFINITY, -1., 99.9, 100_001.] {
        for axis in 0..2 {
            let mut doc = Document::default();
            doc.page_size[axis] = value;
            assert!(doc.validate().is_err(), "accepted page dimension {value}");
        }
    }
    let doc = Document {
        page_size: [100., 100_000.],
        ..Document::default()
    };
    doc.validate().unwrap();
}

#[test]
fn rejects_invalid_shape_geometry_and_styles() {
    let (doc, _, _, _) = pair(ConnectorKind::Straight);
    for value in [f32::NAN, f32::INFINITY, -100_001., 100_001.] {
        for axis in 0..2 {
            let mut bad = doc.clone();
            bad.nodes[0].position[axis] = value;
            assert!(bad.validate().is_err());
        }
    }
    for value in [f32::NAN, f32::INFINITY, 0., 23.9, 20_001.] {
        for axis in 0..2 {
            let mut bad = doc.clone();
            bad.nodes[0].size[axis] = value;
            assert!(bad.validate().is_err());
        }
    }
    for value in [f32::NAN, f32::INFINITY, 5.9, 200.1] {
        let mut bad = doc.clone();
        bad.nodes[0].font_size = value;
        assert!(bad.validate().is_err());
    }
    for value in [f32::NAN, f32::INFINITY, -0.1, 20.1] {
        let mut bad = doc.clone();
        bad.nodes[0].stroke_width = value;
        assert!(bad.validate().is_err());
    }
    let mut boundaries = doc;
    boundaries.nodes[0].position = [-100_000., 100_000.];
    boundaries.nodes[0].size = [24., 20_000.];
    boundaries.nodes[0].font_size = 6.;
    boundaries.nodes[0].stroke_width = 0.;
    boundaries.validate().unwrap();
}

#[test]
fn validates_connector_endpoints_and_styles() {
    let (doc, a, _, _) = pair(ConnectorKind::Elbow);
    for endpoints in [(a, a), (0, a), (a, 999), (999, 998)] {
        let mut bad = doc.clone();
        (bad.connectors[0].from, bad.connectors[0].to) = endpoints;
        assert!(bad.validate().is_err());
    }
    for width in [f32::NAN, f32::INFINITY, 0., 0.49, 20.01] {
        let mut bad = doc.clone();
        bad.connectors[0].width = width;
        assert!(bad.validate().is_err());
    }
    for width in [0.5, 20.] {
        let mut good = doc.clone();
        good.connectors[0].width = width;
        good.validate().unwrap();
    }
}

#[test]
fn validates_text_and_object_limits() {
    let (doc, _, _, _) = pair(ConnectorKind::Straight);
    let mut bad = doc.clone();
    bad.title = "t".repeat(4097);
    assert!(bad.validate().is_err());
    let mut bad = doc.clone();
    bad.nodes[0].text = "n".repeat(65_537);
    assert!(bad.validate().is_err());
    let mut bad = doc.clone();
    bad.connectors[0].label = "e".repeat(4097);
    assert!(bad.validate().is_err());
    let mut at_limit = doc;
    at_limit.title = "t".repeat(4096);
    at_limit.nodes[0].text = "n".repeat(65_536);
    at_limit.connectors[0].label = "e".repeat(4096);
    at_limit.validate().unwrap();

    let template = at_limit.nodes[1].clone();
    let mut many = Document {
        nodes: (1..=10_000)
            .map(|id| Node {
                id,
                ..template.clone()
            })
            .collect(),
        ..Document::default()
    };
    many.validate().unwrap();
    many.nodes.push(Node {
        id: 10_001,
        ..template
    });
    assert!(many.validate().is_err());
}

#[test]
fn connect_rejects_invalid_and_duplicate_edges_without_mutation() {
    let (mut doc, a, b, _) = pair(ConnectorKind::Straight);
    let before = doc.clone();
    assert_eq!(doc.connect(a, a, ConnectorKind::Elbow), None);
    assert_eq!(doc.connect(a, 999, ConnectorKind::Elbow), None);
    assert_eq!(doc.connect(999, b, ConnectorKind::Elbow), None);
    assert_eq!(doc.connect(a, b, ConnectorKind::Elbow), None);
    assert_eq!(doc, before);
    assert!(doc.connect(b, a, ConnectorKind::Elbow).is_some());
    doc.validate().unwrap();
}

#[test]
fn hit_testing_obeys_shape_outline_and_front_to_back_order() {
    let mut doc = Document::default();
    let rectangle = doc.add_node(ShapeKind::Rectangle, [100., 100.]);
    let ellipse = doc.add_node(ShapeKind::Ellipse, [100., 100.]);
    let diamond = doc.add_node(ShapeKind::Diamond, [100., 100.]);
    for id in [rectangle, ellipse, diamond] {
        let node = doc.node_mut(id).unwrap();
        node.position = [0., 0.];
        node.size = [200., 100.];
    }
    assert_eq!(doc.hit_node([100., 50.]), Some(diamond));
    assert_eq!(doc.hit_node([180., 80.]), Some(ellipse));
    assert_eq!(doc.hit_node([199., 99.]), Some(rectangle));
    assert_eq!(doc.hit_node([201., 50.]), None);
    for kind in ShapeKind::ALL {
        let node = Node {
            kind,
            ..doc.nodes[0].clone()
        };
        assert!(node.contains([100., 50.]));
        assert!(node.contains([100., 0.]));
        assert!(node.contains([200., 50.]));
        assert!(!node.contains([200.1, 50.]));
        assert!(!node.contains([100., -0.1]));
        assert_eq!(
            node.contains([0., 0.]),
            !matches!(kind, ShapeKind::Ellipse | ShapeKind::Diamond)
        );
    }
}

#[test]
fn rays_intersect_the_correct_shape_boundary() {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rectangle, [100., 50.]);
    let source = doc.node(id).unwrap();
    for kind in ShapeKind::ALL {
        let node = Node {
            kind,
            ..source.clone()
        };
        let center = node.center();
        point_close(node.boundary_towards(center), center);
        for target in [[500., 300.], [-300., -200.], [100., 1000.], [1000., 50.]] {
            let p = node.boundary_towards(target);
            assert!((normalized_boundary(&node, p) - 1.).abs() < 0.001);
            let direction = [target[0] - center[0], target[1] - center[1]];
            let ray = [p[0] - center[0], p[1] - center[1]];
            assert!((direction[0] * ray[1] - direction[1] * ray[0]).abs() < 0.05);
            assert!(direction[0] * ray[0] + direction[1] * ray[1] > 0.);
        }
    }
}

#[test]
fn straight_connector_tracks_moved_and_resized_shapes() {
    let (mut doc, a, b, _) = pair(ConnectorKind::Straight);
    let initial = doc.route(&doc.connectors[0]);
    point_close(initial[0], [310., 200.]);
    point_close(initial[1], [490., 200.]);

    let target = doc.node_mut(b).unwrap();
    target.position = [550., 320.];
    target.size = [300., 160.];
    let updated = doc.route(&doc.connectors[0]);
    assert_ne!(initial, updated);
    assert_eq!(updated.len(), 2);
    let start = doc.node(a).unwrap();
    let end = doc.node(b).unwrap();
    point_close(updated[0], start.boundary_towards(end.center()));
    point_close(updated[1], end.boundary_towards(start.center()));
    assert!((normalized_boundary(start, updated[0]) - 1.).abs() < 0.001);
    assert!((normalized_boundary(end, updated[1]) - 1.).abs() < 0.001);
}

#[test]
fn elbow_routes_remain_orthogonal_and_attached_after_edits() {
    for kind in [ShapeKind::Rectangle, ShapeKind::Ellipse, ShapeKind::Diamond] {
        let (mut doc, a, b, _) = pair(ConnectorKind::Elbow);
        doc.node_mut(a).unwrap().kind = kind;
        doc.node_mut(b).unwrap().kind = kind;
        for (position, size) in [
            ([500., 300.], [200., 100.]),
            ([100., 700.], [120., 200.]),
            ([-300., 150.], [240., 80.]),
        ] {
            let target = doc.node_mut(b).unwrap();
            target.position = position;
            target.size = size;
            let route = doc.route(&doc.connectors[0]);
            assert_eq!(route.len(), 4);
            for segment in route.windows(2) {
                assert!(segment[0][0] == segment[1][0] || segment[0][1] == segment[1][1]);
            }
            assert!((normalized_boundary(doc.node(a).unwrap(), route[0]) - 1.).abs() < 0.001);
            assert!((normalized_boundary(doc.node(b).unwrap(), route[3]) - 1.).abs() < 0.001);
        }
    }
}

#[test]
fn connector_hit_testing_uses_segments_and_tolerance() {
    let (mut doc, a, b, edge) = pair(ConnectorKind::Straight);
    assert_eq!(doc.hit_connector([400., 200.], 0.), Some(edge));
    assert_eq!(doc.hit_connector([400., 203.], 3.), Some(edge));
    assert_eq!(doc.hit_connector([400., 203.1], 3.), None);
    assert_eq!(doc.hit_connector([200., 200.], 10.), None);
    let reverse = doc.connect(b, a, ConnectorKind::Straight).unwrap();
    assert_eq!(doc.hit_connector([400., 200.], 1.), Some(reverse));
    let mut dangling = doc.connectors[0].clone();
    dangling.to = 999;
    assert!(doc.route(&dangling).is_empty());

    assert_eq!(segment_distance([3., 4.], [0., 0.], [0., 0.]), 5.);
    assert_eq!(segment_distance([15., 0.], [0., 0.], [10., 0.]), 5.);
    assert_eq!(segment_distance([-3., 4.], [0., 0.], [10., 0.]), 5.);
    assert_eq!(segment_distance([5., 4.], [0., 0.], [10., 0.]), 4.);
}

#[test]
fn deleting_shapes_cascades_only_to_incident_connectors() {
    let (mut doc, a, b, ab) = pair(ConnectorKind::Straight);
    let c = doc.add_node(ShapeKind::Diamond, [600., 500.]);
    let bc = doc.connect(b, c, ConnectorKind::Elbow).unwrap();
    let ca = doc.connect(c, a, ConnectorKind::Elbow).unwrap();
    doc.remove(&[a, 999]);
    assert!(doc.node(a).is_none());
    assert!(doc.node(b).is_some() && doc.node(c).is_some());
    assert_eq!(
        doc.connectors.iter().map(|e| e.id).collect::<Vec<_>>(),
        [bc]
    );
    assert!(!doc.connectors.iter().any(|e| e.id == ab || e.id == ca));
    doc.validate().unwrap();
    doc.remove(&[bc]);
    assert_eq!(doc.nodes.len(), 2);
    assert!(doc.connectors.is_empty());
    let before = doc.clone();
    doc.remove(&[]);
    assert_eq!(doc, before);
}

#[test]
fn duplication_copies_internal_edges_and_preserves_originals() {
    let (mut doc, a, b, _) = pair(ConnectorKind::Elbow);
    doc.connectors[0].label = "condition".into();
    doc.connectors[0].color = [10, 20, 30];
    doc.connectors[0].width = 3.;
    doc.connectors[0].arrow = false;
    let external = doc.add_node(ShapeKind::Ellipse, [900., 200.]);
    doc.connect(b, external, ConnectorKind::Straight).unwrap();
    let original = doc.clone();
    let copies = doc.duplicate_nodes(&[a, b, a, 999]);
    assert_eq!(copies.len(), 2);
    assert_eq!(
        &doc.nodes[..original.nodes.len()],
        original.nodes.as_slice()
    );
    assert_eq!(&doc.connectors[..2], original.connectors.as_slice());
    for (old, new) in [a, b].into_iter().zip(copies.iter().copied()) {
        let source = original.node(old).unwrap();
        let copied = doc.node(new).unwrap();
        let mut expected = source.clone();
        expected.id = new;
        expected.position[0] += 24.;
        expected.position[1] += 24.;
        assert_eq!(*copied, expected);
    }
    assert_eq!(doc.connectors.len(), 3);
    let mut expected_edge = original.connectors[0].clone();
    expected_edge.id = doc.connectors[2].id;
    expected_edge.from = copies[0];
    expected_edge.to = copies[1];
    assert_eq!(doc.connectors[2], expected_edge);
    let ids: HashSet<_> = doc
        .nodes
        .iter()
        .map(|n| n.id)
        .chain(doc.connectors.iter().map(|e| e.id))
        .collect();
    assert_eq!(ids.len(), doc.nodes.len() + doc.connectors.len());
    doc.validate().unwrap();

    let before = doc.clone();
    assert!(doc.duplicate_nodes(&[999]).is_empty());
    assert_eq!(doc, before);
}

#[test]
fn history_restores_document_snapshots_and_discards_redo_on_branch() {
    let (mut doc, a, b, _) = pair(ConnectorKind::Straight);
    let initial = doc.clone();
    let mut history = History::new(&doc);
    assert!(!history.can_undo() && !history.can_redo());
    assert_eq!(history.undo(), None);
    assert_eq!(history.redo(), None);
    history.record(&doc);
    assert!(!history.can_undo());

    doc.node_mut(a).unwrap().text = "edited".into();
    history.record(&doc);
    let edit = doc.clone();
    doc.remove(&[b]);
    history.record(&doc);
    let deleted = doc.clone();
    doc.title = "unsaved live mutation".into();
    assert_eq!(history.undo(), Some(edit.clone()));
    assert_eq!(history.undo(), Some(initial.clone()));
    assert_eq!(history.undo(), None);
    assert_eq!(history.redo(), Some(edit.clone()));
    history.record(&edit);
    assert!(
        history.can_redo(),
        "recording an unchanged undo state must retain redo"
    );
    assert_eq!(history.redo(), Some(deleted));
    assert_eq!(history.undo(), Some(edit.clone()));
    let mut branch = edit;
    branch.node_mut(a).unwrap().position[0] += 100.;
    history.record(&branch);
    assert!(!history.can_redo());
    assert_eq!(history.redo(), None);
    let mut restored = history.undo().unwrap();
    restored.nodes.clear();
    assert_eq!(history.redo(), Some(branch));
    assert_eq!(
        history.undo().unwrap().nodes.len(),
        2,
        "returned snapshots must not alias history"
    );
}

#[test]
fn history_is_bounded_to_one_hundred_snapshots() {
    let mut doc = Document {
        title: "0".into(),
        ..Document::default()
    };
    let mut history = History::new(&doc);
    for step in 1..=120 {
        doc.title = step.to_string();
        history.record(&doc);
    }
    let mut count = 0;
    let mut oldest = None;
    while let Some(state) = history.undo() {
        count += 1;
        oldest = Some(state);
    }
    assert_eq!(count, 99);
    assert_eq!(oldest.unwrap().title, "21");
    let mut redo_count = 0;
    while let Some(state) = history.redo() {
        redo_count += 1;
        assert_eq!(state.title, (21 + redo_count).to_string());
    }
    assert_eq!(redo_count, 99);
}

#[test]
fn save_load_round_trip_and_atomic_overwrite_preserve_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dessin.rdraw");
    let mut doc = Document::demo();
    doc.title = "Été & diagramme 日本語".into();
    doc.nodes[0].text = "Ligne 1\nLigne 2 < & >".into();
    doc.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), doc);
    let first_bytes = fs::read(&path).unwrap();
    doc.nodes[2].position = [17., -24.];
    doc.connectors[0].label = "version suivante".into();
    doc.save(&path).unwrap();
    assert_ne!(fs::read(&path).unwrap(), first_bytes);
    assert_eq!(Document::load(&path).unwrap(), doc);
    assert_eq!(
        fs::read_dir(dir.path()).unwrap().count(),
        1,
        "atomic write must clean temporary files"
    );

    let preserved = fs::read(&path).unwrap();
    doc.nodes[0].size = [0., 0.];
    assert!(doc.save(&path).is_err());
    assert_eq!(
        fs::read(&path).unwrap(),
        preserved,
        "invalid saves must not damage existing files"
    );
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn atomic_write_replaces_existing_bytes_and_cleans_failed_temporary_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("export.svg");
    atomic_write(&path, b"first bytes").unwrap();
    atomic_write(&path, b"new").unwrap();
    assert_eq!(fs::read(&path).unwrap(), b"new");
    assert!(atomic_write(&dir.path().join("missing").join("file"), b"x").is_err());
    let target_directory = dir.path().join("folder");
    fs::create_dir(&target_directory).unwrap();
    assert!(atomic_write(&target_directory, b"x").is_err());
    assert!(target_directory.is_dir());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 2);
}

#[test]
fn loading_rejects_malformed_unsupported_and_oversized_files() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.rdraw");
    for bytes in [
        b"".as_slice(),
        b"{",
        b"{}",
        b"null",
        b"[]",
        b"not JSON",
        &[0xff, 0xfe],
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(Document::load(&path).is_err());
    }
    let (mut bad, _, _, _) = pair(ConnectorKind::Straight);
    bad.format_version = FORMAT_VERSION + 1;
    fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(Document::load(&path).is_err());
    bad.format_version = FORMAT_VERSION;
    bad.connectors[0].to = 999;
    fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
    assert!(Document::load(&path).is_err());
    let valid_json = serde_json::to_string(&Document::demo()).unwrap();
    fs::write(
        &path,
        valid_json.replace("\"rounded\"", "\"unknown_shape\""),
    )
    .unwrap();
    assert!(Document::load(&path).is_err());

    let file = fs::File::create(&path).unwrap();
    file.set_len(MAX_FILE_BYTES + 1).unwrap();
    assert!(Document::load(&path).unwrap_err().contains("volumineux"));
    assert!(Document::load(&dir.path().join("missing.rdraw")).is_err());
}

#[test]
fn svg_escapes_all_text_and_strips_invalid_xml_controls() {
    let (mut doc, _, _, _) = pair(ConnectorKind::Straight);
    let text = "Été 日本語 <script> & \"double\" 'simple'\u{0}\u{1}\u{b}\u{1f}\u{fffe}\u{ffff}\t\r";
    doc.title = text.into();
    doc.nodes[0].text = text.into();
    doc.connectors[0].label = text.into();
    let out = svg::export(&doc).unwrap();
    let escaped = "Été 日本語 &lt;script&gt; &amp; &quot;double&quot; &apos;simple&apos;\t\r";
    assert_eq!(
        out.matches(escaped).count(),
        2,
        "title and connector keep their whitespace"
    );
    assert!(
        out.contains(&escaped.replace('\t', "    ").replace('\r', "")),
        "node tabs are expanded and CR is a line break"
    );
    assert_eq!(
        doc.nodes[0].text, text,
        "display normalization never edits the source"
    );
    assert!(!out.contains("<script>"));
    for forbidden in ['\u{0}', '\u{1}', '\u{b}', '\u{1f}', '\u{fffe}', '\u{ffff}'] {
        assert!(!out.contains(forbidden));
    }
    assert!(out.starts_with("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
    assert!(out.contains("xmlns=\"http://www.w3.org/2000/svg\""));
    assert!(out.ends_with("</svg>\n"));
}

#[test]
fn svg_exports_page_shape_geometry_styles_and_multiline_text() {
    let mut doc = Document {
        page_size: [800., 600.],
        ..Document::default()
    };
    for (i, kind) in ShapeKind::ALL.into_iter().enumerate() {
        let id = doc.add_node(kind, [100., 100. + i as f32 * 100.]);
        let node = doc.node_mut(id).unwrap();
        node.position = [10., 20. + i as f32 * 100.];
        node.size = [200., 80.];
        node.text = if kind == ShapeKind::Text {
            "Première\nDeuxième".into()
        } else {
            String::new()
        };
        node.font_size = 20.;
        node.fill = [1, 2, 3];
        node.stroke = [4, 5, 6];
        node.text_color = [7, 8, 9];
        node.stroke_width = 2.;
    }
    let out = svg::export(&doc).unwrap();
    assert!(out.contains("width=\"800\" height=\"600\" viewBox=\"0 0 800 600\""));
    assert!(out.contains("<rect x=\"10\" y=\"20\" width=\"200\" height=\"80\" rx=\"2\""));
    assert!(out.contains("<rect x=\"10\" y=\"120\" width=\"200\" height=\"80\" rx=\"20\""));
    assert!(out.contains("<ellipse cx=\"110\" cy=\"260\" rx=\"100\" ry=\"40\""));
    assert!(out.contains("<polygon points=\"110,320 210,360 110,400 10,360\""));
    assert!(out.contains("fill=\"#010203\" stroke=\"#040506\" stroke-width=\"2\""));
    assert!(out.contains("font-size=\"20\" fill=\"#070809\""));
    assert!(out.contains(
        "<tspan x=\"110\" y=\"453.24\">Première</tspan><tspan x=\"110\" y=\"480.24\">Deuxième</tspan>"
    ));
    assert_eq!(
        out.matches("<rect ").count(),
        3,
        "text shape must have no filled rectangle"
    );
    assert_eq!(svg::hex([0, 15, 255]), "#000fff");
}

#[test]
fn svg_exports_computed_connector_routes_and_optional_arrowheads() {
    let (mut doc, _, b, _) = pair(ConnectorKind::Elbow);
    doc.node_mut(b).unwrap().position[1] += 150.;
    let route = doc.route(&doc.connectors[0]);
    let coords = route
        .iter()
        .map(|p| format!("{},{}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ");
    let out = svg::export(&doc).unwrap();
    assert!(out.contains(&format!("<polyline points=\"{coords}\"")));
    assert_eq!(out.matches("<polygon ").count(), 1);
    assert!(!out.contains("NaN") && !out.contains("inf"));
    doc.connectors[0].arrow = false;
    let out = svg::export(&doc).unwrap();
    assert!(!out.contains("<polygon "));

    doc.connectors[0].arrow = true;
    let same_position = doc.nodes[0].position;
    let same_size = doc.nodes[0].size;
    doc.nodes[1].position = same_position;
    doc.nodes[1].size = same_size;
    let out = svg::export(&doc).unwrap();
    assert!(!out.contains("NaN") && !out.contains("inf"));
    assert!(
        !out.contains("<polygon "),
        "coincident centers must not create a degenerate arrowhead"
    );
}

#[test]
fn svg_refuses_invalid_documents() {
    let (mut doc, _, _, _) = pair(ConnectorKind::Straight);
    doc.nodes[0].position[0] = f32::NAN;
    assert!(svg::export(&doc).is_err());
    doc.nodes[0].position[0] = 0.;
    doc.connectors[0].from = 999;
    assert!(svg::export(&doc).is_err());
}
