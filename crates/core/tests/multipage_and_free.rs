use diagcraft::{
    document::{ConnectorKind, Document, FORMAT_VERSION, Port, ShapeKind},
    history::History,
    pages::{PageLayout, Unit},
    pdf, svg,
};

#[test]
fn navigation_is_not_an_edit_and_serialization_is_canonical() {
    let mut d = Document::demo();
    d.page_name = "Accueil".into();
    d.add_page(false).unwrap();
    d.page_name = "Détails".into();
    d.page_size = [600., 800.];
    d.add_node(ShapeKind::Diamond, [300., 300.]);
    let before = d.clone();
    let bytes = serde_json::to_vec(&d).unwrap();
    d.switch_page(0).unwrap();
    assert_eq!(d, before);
    assert_eq!(serde_json::to_vec(&d).unwrap(), bytes);
    let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(v["page_name"], "Accueil");
    assert_eq!(v["pages"][0]["page_name"], "Détails");
    let loaded: Document = serde_json::from_slice(&bytes).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded, before);
    assert_eq!(loaded.page_index, 0);
    let mut history = History::new(&d);
    d.switch_page(1).unwrap();
    history.record(&d);
    assert!(!history.can_undo());
    d.nodes[0].text = "Édition".into();
    history.record(&d);
    assert_eq!(history.undo().unwrap(), before);
}
#[test]
fn page_duplicate_reorder_delete_and_sizes_are_independent_and_atomic() {
    let mut d = Document::demo();
    d.page_name = "Première".into();
    d.add_page(true).unwrap();
    d.page_name = "Deuxième".into();
    d.nodes[0].text = "Copie modifiée".into();
    d.resize_page(
        "Deuxième".into(),
        [800., 1200.],
        PageLayout {
            unit: Unit::Millimeters,
            auto_size: false,
        },
    )
    .unwrap();
    assert_ne!(d.page(0).unwrap().nodes[0].text, d.nodes[0].text);
    assert_ne!(d.page(0).unwrap().page_size, d.page_size);
    d.move_page(1, 0).unwrap();
    assert_eq!(d.page_index, 0);
    assert_eq!(d.page_names(), vec!["Deuxième", "Première"]);
    d.remove_page(0).unwrap();
    assert_eq!(d.page_name, "Première");
    let before = d.clone();
    assert!(d.remove_page(0).is_err());
    assert_eq!(d, before);
    assert!(
        d.resize_page(" ".into(), [800., 1200.], Default::default())
            .is_err()
    );
    assert_eq!(d, before);
    assert!(
        d.resize_page("Nom".into(), [f32::NAN, 1200.], Default::default())
            .is_err()
    );
    assert_eq!(d, before);
    assert!(d.move_page(0, 99).is_err());
    assert_eq!(d, before);
}
#[test]
fn page_validation_checks_inactive_pages_and_global_object_limits() {
    let mut d = Document::demo();
    d.add_page(false).unwrap();
    d.page_size = [-10., 500.];
    d.switch_page(0).unwrap();
    assert!(d.validate().is_err());
    let mut d = Document::default();
    for _ in 1..100 {
        d.add_page(false).unwrap();
    }
    let before = d.clone();
    assert!(d.add_page(false).is_err());
    assert_eq!(d, before);
    let mut d = Document::demo();
    d.add_page(true).unwrap();
    let n = d.nodes[0].clone();
    d.nodes = (1..=9999)
        .map(|id| {
            let mut n = n.clone();
            n.id = id;
            n
        })
        .collect();
    d.connectors.clear();
    assert!(d.validate().is_err());
}
#[test]
fn free_routes_stay_attached_at_one_end_and_support_manual_edits_and_copy() {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rounded, [200., 250.]);
    let id = d
        .connect_endpoints(a, 0, None, Some([800., 600.]), ConnectorKind::Elbow)
        .unwrap();
    let end = *d.route(&d.connectors[0]).last().unwrap();
    assert_eq!(end, [800., 600.]);
    d.node_mut(a).unwrap().position[0] += 80.;
    assert_eq!(*d.route(&d.connectors[0]).last().unwrap(), end);
    d.set_automatic_route(id, false).unwrap();
    d.add_connector_waypoint(id, [500., 400.]).unwrap();
    d.validate().unwrap();
    let copy = d.selection(&[id]);
    assert_eq!(copy.connectors[0].from, 0);
    assert!(copy.nodes.is_empty());
    copy.validate().unwrap();
    let ids = d.paste(&copy, [20., 30.]).unwrap();
    assert_eq!(ids.len(), 1);
    let e = d.connectors.last().unwrap();
    assert_eq!(e.free_end, Some([820., 630.]));
    assert!(e.waypoints.contains(&[520., 430.]));
    assert!(svg::export(&d).unwrap().contains("<polyline"));
    let temp = tempfile::tempdir().unwrap();
    let p = temp.path().join("free.draw");
    d.save(&p).unwrap();
    assert_eq!(Document::load(&p).unwrap(), d);
}
#[test]
fn both_free_endpoints_can_attach_detach_and_split_without_losing_styles() {
    let mut d = Document::default();
    let edge = d
        .connect_endpoints(
            0,
            0,
            Some([200., 400.]),
            Some([1000., 400.]),
            ConnectorKind::Elbow,
        )
        .unwrap();
    d.connectors[0].pattern = diagcraft::stroke_style::LinePattern::Dotted;
    let n = d
        .insert_shape_on_connector(edge, ShapeKind::Diamond, [600., 400.])
        .unwrap();
    assert_eq!(d.connectors.len(), 2);
    assert_eq!(d.connectors[0].free_start, Some([200., 400.]));
    assert_eq!(d.connectors[1].free_end, Some([1000., 400.]));
    assert_eq!(d.connectors[0].to, n);
    assert_eq!(d.connectors[1].from, n);
    let a = d.add_node(ShapeKind::Rectangle, [200., 200.]);
    d.reconnect_connector(edge, true, a, Port::Bottom).unwrap();
    assert!(d.connectors[0].free_start.is_none());
    d.release_connector_end(edge, true, [200., 350.]).unwrap();
    assert_eq!(d.connectors[0].from, 0);
    d.validate().unwrap();
    let before = d.clone();
    assert!(
        d.release_connector_end(edge, false, [f32::INFINITY, 0.])
            .is_err()
    );
    assert_eq!(d, before);
    assert!(
        d.connect_endpoints(0, 0, None, Some([5., 5.]), ConnectorKind::Straight)
            .is_err()
    );
    assert_eq!(d, before);
    assert!(
        d.connect_endpoints(
            0,
            0,
            Some([1., 1.]),
            Some([1., 1.]),
            ConnectorKind::Straight
        )
        .is_err()
    );
    assert_eq!(d, before);
}
#[test]
fn fit_and_auto_size_include_free_endpoints_and_units_match_paper_sizes() {
    let mut d = Document::default();
    d.connect_endpoints(
        0,
        0,
        Some([-50., -20.]),
        Some([1800., 1200.]),
        ConnectorKind::Straight,
    )
    .unwrap();
    d.fit_page_to_drawing().unwrap();
    assert_eq!(d.connectors[0].free_start, Some([40., 40.]));
    assert_eq!(d.connectors[0].free_end, Some([1890., 1260.]));
    assert_eq!(d.page_size, [1930., 1300.]);
    d.page_size = [500., 500.];
    d.page_layout.auto_size = true;
    d.grow_page_to_content();
    assert_eq!(d.page_size, [1930., 1300.]);
    for unit in Unit::ALL {
        assert!((unit.from_px(unit.to_px(123.4)) - 123.4).abs() < 0.001);
    }
    let a4 = &diagcraft::pages::presets()[1];
    assert!((a4.size[0] * 0.75 - 595.2756).abs() < 0.001);
    assert!((a4.size[1] * 0.75 - 841.8898).abs() < 0.001);
}
#[test]
fn legacy_files_migrate_to_one_page_without_overwriting() {
    let temp = tempfile::tempdir().unwrap();
    for name in [
        "parcours-v1.draw",
        "ruban.draw",
        "support.draw",
        "contexte.draw",
        "support-v5.draw",
        "connecteurs-v6.draw",
    ] {
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples")
                .join(name),
        )
        .unwrap();
        let path = temp.path().join(name);
        std::fs::write(&path, &bytes).unwrap();
        let d = Document::load(&path).unwrap();
        assert_eq!(d.format_version, FORMAT_VERSION);
        assert_eq!(d.page_count(), 1);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn multipage_pdf_contains_each_page_and_current_page_export_remains_single() {
    let mut d = Document::demo();
    d.add_page(false).unwrap();
    d.page_size = [800., 1200.];
    let n = d.add_node(ShapeKind::Text, [400., 100.]);
    d.node_mut(n).unwrap().text = "Deuxième page".into();
    let pdf = pdf::export(&d, pdf::PageFormat::Drawing).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("/Count 2"));
    assert_eq!(text.matches("/Type /Page\n").count(), 2);
    assert!(text.contains("600 900"));
    let single = pdf::export_current(&d, pdf::PageFormat::Drawing).unwrap();
    assert!(String::from_utf8_lossy(&single).contains("/Count 1"));
}
