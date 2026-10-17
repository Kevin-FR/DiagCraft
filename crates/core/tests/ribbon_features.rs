use diagcraft::{
    document::{Document, DrawingKind, FORMAT_VERSION, TextAlign},
    svg,
};

#[test]
fn version_one_files_migrate_without_losing_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ancien.draw");
    let mut value = serde_json::to_value(Document::demo()).unwrap();
    value["format_version"] = 1.into();
    value.as_object_mut().unwrap().remove("page_style");
    value.as_object_mut().unwrap().remove("drawings");
    for n in value["nodes"].as_array_mut().unwrap() {
        n.as_object_mut().unwrap().remove("text_style");
    }
    std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    let loaded = Document::load(&path).unwrap();
    assert_eq!(loaded.format_version, FORMAT_VERSION);
    assert_eq!(loaded.nodes, Document::demo().nodes);
    assert_eq!(loaded.connectors, Document::demo().connectors);
    loaded.validate().unwrap();
    loaded.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), loaded);
}

#[test]
fn styled_text_page_and_curves_survive_save_and_export() {
    let mut d = Document::demo();
    let n = &mut d.nodes[0];
    n.text = "Titre & sous-titre".into();
    n.text_style.bold = true;
    n.text_style.italic = true;
    n.text_style.underline = true;
    n.text_style.strikethrough = true;
    n.text_style.align = TextAlign::Left;
    d.page_style.background = [240, 242, 250];
    d.page_style.grid_spacing = 35.;
    d.page_style.margin = 65.;
    d.page_style.watermark = "Brouillon <interne>".into();
    let id = d.add_drawing(DrawingKind::Bezier, [100., 100.], [500., 300.]);
    d.drawings[0].arrow = true;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nouveau.draw");
    d.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), d);
    let out = svg::export(&d).unwrap();
    assert!(out.contains("fill=\"#f0f2fa\""));
    assert!(out.contains("Brouillon &lt;interne&gt;"));
    assert!(out.contains(
        "font-weight=\"700\" font-style=\"italic\" text-decoration=\"underline line-through\""
    ));
    assert!(out.contains("text-anchor=\"start\""));
    assert!(out.contains(" C "));
    assert!(d.drawings.iter().any(|d| d.id == id));
}

#[test]
fn curves_have_exact_endpoints_and_can_be_selected_on_their_path() {
    let mut d = Document::default();
    let id = d.add_drawing(DrawingKind::Bezier, [20., 40.], [500., 230.]);
    let path = &d.drawings[0];
    let points = path.points();
    assert_eq!(points.first(), Some(&path.start));
    assert_eq!(points.last(), Some(&path.end));
    assert_eq!(d.hit_drawing(points[20], 0.1), Some(id));
    assert_ne!(points[20], [(20. + 500.) / 2., (40. + 230.) / 2.]);
    d.drawings[0].bend = 0.;
    assert!((d.drawings[0].points()[20][0] - 260.).abs() < 0.001);
    d.drawings[0].kind = DrawingKind::Line;
    assert_eq!(d.drawings[0].points(), vec![[20., 40.], [500., 230.]]);
}

#[test]
fn pasted_selections_remap_edges_and_preserve_all_styles() {
    let mut d = Document::demo();
    d.nodes[2].text_style.bold = true;
    let id = d.add_drawing(DrawingKind::Line, [100., 200.], [300., 400.]);
    let ids: Vec<_> = d.nodes.iter().map(|n| n.id).chain([id]).collect();
    let copied = d.selection(&ids);
    let before = d.clone();
    let pasted = d.paste(&copied, [24., 24.]).unwrap();
    assert_eq!(pasted.len(), copied.nodes.len() + copied.drawings.len());
    assert_eq!(d.connectors.len(), before.connectors.len() * 2);
    assert_eq!(d.drawings[1].start, [124., 224.]);
    assert_eq!(
        d.nodes[before.nodes.len() + 2].text_style,
        copied.nodes[2].text_style
    );
    for edge in d.connectors.iter().skip(before.connectors.len()) {
        assert!(pasted.contains(&edge.from) && pasted.contains(&edge.to));
    }
    d.validate().unwrap();
}

#[test]
fn malformed_clipboard_is_rejected_atomically() {
    let mut d = Document::demo();
    let original = d.clone();
    let mut bad = d.clone();
    bad.connectors[0].from = u64::MAX;
    assert!(d.paste(&bad, [24., 24.]).is_err());
    assert_eq!(d, original);
    assert!(d.paste(&original, [f32::NAN, 0.]).is_err());
    assert_eq!(d, original);
    assert!(d.paste(&original, [100_000., 100_000.]).is_err());
    assert_eq!(d, original);
}

#[test]
fn deleting_and_duplicating_paths_respects_ids_and_geometry() {
    let mut d = Document::default();
    let first = d.add_drawing(DrawingKind::Bezier, [10., 20.], [200., 300.]);
    let second = d.add_drawing(DrawingKind::Line, [30., 40.], [400., 500.]);
    let ids = d.duplicate_nodes(&[first]);
    assert_eq!(ids.len(), 1);
    assert_eq!(d.drawings[2].start, [34., 44.]);
    assert_eq!(d.drawings[2].kind, DrawingKind::Bezier);
    d.remove(&[first, second]);
    assert_eq!(d.drawings.len(), 1);
    assert_eq!(d.drawings[0].id, ids[0]);
    d.validate().unwrap();
}

#[test]
fn rejects_invalid_page_styles_and_path_geometry() {
    for v in [f32::NAN, f32::INFINITY, 0., 201.] {
        let mut d = Document::default();
        d.page_style.grid_spacing = v;
        assert!(d.validate().is_err());
    }
    for v in [f32::NAN, f32::INFINITY, -1., 1001.] {
        let mut d = Document::default();
        d.page_style.margin = v;
        assert!(d.validate().is_err());
    }
    let mut d = Document::default();
    let a = d.add_node(diagcraft::document::ShapeKind::Rectangle, [200., 200.]);
    d.add_drawing(DrawingKind::Line, [0., 0.], [100., 100.]);
    d.drawings[0].id = a;
    assert!(d.validate().is_err());
    d.drawings[0].id = a + 1;
    d.drawings[0].end[1] = f32::INFINITY;
    assert!(d.validate().is_err());
    d.drawings[0].end = [100., 100.];
    d.drawings[0].bend = 10_001.;
    assert!(d.validate().is_err());
}

#[test]
fn fit_page_encloses_negative_geometry_and_preserves_relative_positions() {
    let mut d = Document::default();
    let id = d.add_node(diagcraft::document::ShapeKind::Rectangle, [-200., -100.]);
    d.add_drawing(DrawingKind::Bezier, [-250., 200.], [600., 500.]);
    let delta = [
        d.drawings[0].start[0] - d.node(id).unwrap().position[0],
        d.drawings[0].start[1] - d.node(id).unwrap().position[1],
    ];
    d.fit_page_to_drawing().unwrap();
    d.validate().unwrap();
    assert_eq!(
        [
            d.drawings[0].start[0] - d.node(id).unwrap().position[0],
            d.drawings[0].start[1] - d.node(id).unwrap().position[1]
        ],
        delta
    );
    for p in d.drawings[0].points() {
        assert!(p[0] >= 0. && p[1] >= 0. && p[0] <= d.page_size[0] && p[1] <= d.page_size[1]);
    }
    let n = d.node(id).unwrap();
    assert!(n.position[0] >= 0. && n.position[1] >= 0.);
    assert!(n.position[0] + n.size[0] <= d.page_size[0]);
    let mut huge = Document::default();
    huge.add_drawing(DrawingKind::Line, [-100_000., 0.], [100_000., 100.]);
    let before = huge.clone();
    assert!(huge.fit_page_to_drawing().is_err());
    assert_eq!(huge, before);
}
