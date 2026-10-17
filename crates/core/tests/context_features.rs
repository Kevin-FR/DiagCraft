use diagcraft::{
    document::{ConnectorKind, Document, FORMAT_VERSION, Port, ShapeKind},
    library::Library,
    svg,
};

fn linked() -> (Document, u64, u64, u64) {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [200., 300.]);
    let b = d.add_node(ShapeKind::Rectangle, [1000., 300.]);
    let id = d.connect(a, b, ConnectorKind::Elbow).unwrap();
    (d, a, b, id)
}
#[test]
fn start_and_end_arrowheads_survive_save_and_vector_export() {
    let (mut d, _, _, _) = linked();
    d.connectors[0].arrow_start = true;
    d.connectors[0].arrow = false;
    let start = svg::export(&d).unwrap();
    assert_eq!(start.matches("<polygon").count(), 1);
    let p = d.route(&d.connectors[0])[0];
    assert!(start.contains(&format!("<polygon points=\"{},{} ", p[0], p[1])));
    d.connectors[0].arrow = true;
    assert_eq!(svg::export(&d).unwrap().matches("<polygon").count(), 2);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("arrows.draw");
    d.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), d);
}
#[test]
fn manual_routing_preserves_ports_geometry_and_can_be_reset() {
    let (mut d, a, b, id) = linked();
    d.node_mut(b).unwrap().position[1] += 200.;
    let route = d.route(&d.connectors[0]);
    d.set_automatic_route(id, false).unwrap();
    assert_eq!(d.connectors[0].from_port, Port::Right);
    assert_eq!(d.connectors[0].to_port, Port::Left);
    assert_eq!(d.route(&d.connectors[0]), route);
    d.node_mut(a).unwrap().position[1] += 60.;
    assert_ne!(d.route(&d.connectors[0]), route);
    assert!(!d.connectors[0].waypoints.is_empty());
    d.set_automatic_route(id, true).unwrap();
    assert!(d.connectors[0].waypoints.is_empty());
    d.validate().unwrap();
}
#[test]
fn waypoints_insert_near_clicked_segment_and_invalid_edits_are_atomic() {
    let (mut d, _, _, id) = linked();
    d.set_automatic_route(id, false).unwrap();
    let middle = d.connectors[0].waypoints[0];
    d.add_connector_waypoint(id, [400., 300.]).unwrap();
    assert_eq!(d.connectors[0].waypoints[0], [400., 300.]);
    assert!(d.connectors[0].waypoints.contains(&middle));
    let before = d.clone();
    assert!(d.add_connector_waypoint(id, [f32::NAN, 0.]).is_err());
    assert_eq!(d, before);
    d.connectors[0].waypoints.resize(16, [600., 400.]);
    let before = d.clone();
    assert!(d.add_connector_waypoint(id, [700., 400.]).is_err());
    assert_eq!(d, before);
}
#[test]
fn linked_shapes_use_requested_ports_and_expand_page_when_needed() {
    for port in [Port::Left, Port::Right, Port::Top, Port::Bottom] {
        let mut d = Document::default();
        let source = d.add_node(ShapeKind::Rounded, [600., 600.]);
        let id = d
            .add_linked_shape(source, ShapeKind::Document, port)
            .unwrap();
        let edge = &d.connectors[0];
        assert_eq!(edge.from, source);
        assert_eq!(edge.to, id);
        assert_eq!(edge.from_port, port);
        assert!(d.node(id).unwrap().position.iter().all(|v| *v >= 0.));
        let n = d.node(id).unwrap();
        for a in 0..2 {
            assert!(n.position[a] + n.size[a] <= d.page_size[a]);
        }
        d.validate().unwrap();
    }
    let mut d = Document::default();
    let source = d.add_node(ShapeKind::Rectangle, [120., 80.]);
    let before = d.clone();
    assert!(
        d.add_linked_shape(source, ShapeKind::Rectangle, Port::Top)
            .is_err()
    );
    assert_eq!(d, before);
}
#[test]
fn inserting_a_shape_splits_link_and_preserves_original_label_and_style() {
    let (mut d, a, b, edge_id) = linked();
    d.connectors[0].label = "Contrôler".into();
    d.connectors[0].arrow_start = true;
    d.connectors[0].color = [12, 34, 56];
    d.connectors[0].width = 3.;
    let id = d
        .insert_shape_on_connector(edge_id, ShapeKind::Diamond, [600., 320.])
        .unwrap();
    assert_eq!(d.node(id).unwrap().center(), [600., 300.]);
    assert_eq!(d.nodes.len(), 3);
    assert_eq!(d.connectors.len(), 2);
    let first = d.connectors.iter().find(|e| e.id == edge_id).unwrap();
    assert_eq!((first.from, first.to), (a, id));
    assert_eq!(first.label, "Contrôler");
    assert!(first.arrow_start);
    let second = d.connectors.iter().find(|e| e.id != edge_id).unwrap();
    assert_eq!((second.from, second.to), (id, b));
    assert!(!second.arrow_start);
    assert!(second.label.is_empty());
    assert_eq!(second.color, [12, 34, 56]);
    assert_eq!(second.width, 3.);
    d.validate().unwrap();
}
#[test]
fn insertion_without_room_and_object_limit_fail_without_mutation() {
    let (mut d, a, _, edge) = linked();
    let before = d.clone();
    assert!(
        d.insert_shape_on_connector(edge, ShapeKind::Rectangle, [205., 300.])
            .is_err()
    );
    assert_eq!(d, before);
    let n = d.nodes[0].clone();
    d.nodes = (1..=9998)
        .map(|id| {
            let mut n = n.clone();
            n.id = id;
            n
        })
        .collect();
    d.connectors.clear();
    d.validate().unwrap();
    d.nodes.push({
        let mut n = n.clone();
        n.id = 9999;
        n
    });
    let before = d.clone();
    assert!(
        d.add_linked_shape(a, ShapeKind::Rectangle, Port::Right)
            .is_err()
    );
    assert_eq!(d, before);
}
#[test]
fn new_connector_properties_are_preserved_by_clipboard_remapping() {
    let (mut d, a, b, _) = linked();
    d.connectors[0].arrow_start = true;
    d.set_automatic_route(d.connectors[0].id, false).unwrap();
    let fragment = d.selection(&[a, b]);
    d.paste(&fragment, [24., 24.]).unwrap();
    assert!(d.connectors[1].arrow_start);
    assert_ne!(d.connectors[0].from, d.connectors[1].from);
    assert_eq!(
        d.connectors[1].waypoints[0],
        [
            d.connectors[0].waypoints[0][0] + 24.,
            d.connectors[0].waypoints[0][1] + 24.
        ]
    );
    d.validate().unwrap();
}
#[test]
fn version_three_documents_and_personal_templates_migrate_without_rewriting_source() {
    let (dir_doc, _, _, _) = linked();
    let mut value = serde_json::to_value(&dir_doc).unwrap();
    value["format_version"] = 3.into();
    value["connectors"][0]
        .as_object_mut()
        .unwrap()
        .remove("arrow_start");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v3.draw");
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let loaded = Document::load(&path).unwrap();
    assert_eq!(loaded.format_version, FORMAT_VERSION);
    assert!(!loaded.connectors[0].arrow_start);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let path = dir.path().join("library-v3.json");
    let value = serde_json::json!({"templates":[{"name":"Ancien modèle","category":"IT","document":value}],"symbols":[],"recent":["support"]});
    let bytes = serde_json::to_vec(&value).unwrap();
    std::fs::write(&path, &bytes).unwrap();
    let lib = Library::load(&path).unwrap();
    assert_eq!(lib.templates[0].document.format_version, FORMAT_VERSION);
    assert_eq!(lib.recent, vec!["support"]);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}
