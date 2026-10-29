use diagcraft::{
    auto_layout::{Direction, preview},
    document::{ConnectorKind, Document, ShapeKind},
};
fn flow() -> (Document, Vec<u64>) {
    let mut d = Document::default();
    let ids: Vec<_> = (0..4)
        .map(|i| d.add_node(ShapeKind::Rectangle, [800. - i as f32 * 180., 500.]))
        .collect();
    for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
        d.connect(ids[a], ids[b], ConnectorKind::Elbow).unwrap();
    }
    (d, ids)
}
#[test]
fn branching_flow_is_deterministic_disjoint_and_keeps_everything_except_positions() {
    let (d, ids) = flow();
    for (axis, direction) in [(0, Direction::Horizontal), (1, Direction::Vertical)] {
        let p = preview(&d, None, direction).unwrap();
        assert_eq!(p.document, preview(&d, None, direction).unwrap().document);
        let repeated = preview(&p.document, None, direction).unwrap();
        assert_eq!(repeated.moved, 0);
        assert_eq!(repeated.fixed, d.nodes.len());
        assert_eq!(p.document.connectors, d.connectors);
        for (a, b) in [(0, 1), (0, 2), (1, 3), (2, 3)] {
            let a = p.document.node(ids[a]).unwrap();
            let b = p.document.node(ids[b]).unwrap();
            assert!(a.position[axis] + a.size[axis] < b.position[axis]);
        }
        for a in &p.document.nodes {
            for b in &p.document.nodes {
                if a.id != b.id {
                    assert!((0..2).any(|i| a.position[i] + a.size[i] <= b.position[i]
                        || b.position[i] + b.size[i] <= a.position[i]));
                }
            }
        }
        let mut normalized = p.document;
        normalized.page_size = d.page_size;
        for n in &mut normalized.nodes {
            n.position = d.node(n.id).unwrap().position;
        }
        assert_eq!(normalized, d);
    }
}
#[test]
fn cycles_disconnected_nodes_and_manual_waypoints_remain_valid() {
    let (mut d, ids) = flow();
    d.connect(ids[3], ids[0], ConnectorKind::Elbow).unwrap();
    d.add_node(ShapeKind::Ellipse, [900., 700.]);
    d.connectors[0].waypoints = vec![[700., 650.], [900., 650.]];
    let p = preview(&d, None, Direction::Vertical).unwrap();
    p.document.validate().unwrap();
    assert_eq!(p.document.connectors, d.connectors);
    assert!(p.moved > 0);
}
#[test]
fn selection_locks_hidden_groups_and_text_are_preserved() {
    let (mut d, ids) = flow();
    d.set_objects_locked(&[ids[0]], true).unwrap();
    d.group_objects(&ids[1..3]).unwrap();
    let text = d.add_node(ShapeKind::Text, [40., 40.]);
    let hidden = d.add_node(ShapeKind::Rectangle, [40., 400.]);
    let layer = d.create_layer("Hidden".into()).unwrap();
    d.move_objects_to_layer(&[hidden], layer).unwrap();
    d.update_layer(layer, None, Some(false), None).unwrap();
    let p = preview(&d, Some(&[ids[3], text, hidden]), Direction::Horizontal).unwrap();
    for n in &d.nodes {
        if n.id != ids[3] {
            assert_eq!(p.document.node(n.id).unwrap(), n);
        }
    }
    assert_eq!(p.document.organization, d.organization);
    assert!(p.fixed >= 5);
    // Text at the origin acts as a fixed obstacle.
    assert!(p.document.node(ids[3]).unwrap().position[1] > 40.);
}
#[test]
fn other_pages_and_source_survive_and_limits_fail_without_mutation() {
    let (mut d, _) = flow();
    let first = d.page(0).unwrap();
    d.add_page(false).unwrap();
    d.add_node(ShapeKind::Rectangle, [800., 600.]);
    let before = d.clone();
    let p = preview(&d, None, Direction::Horizontal).unwrap();
    assert_eq!(p.document.page(0).unwrap(), first);
    assert_eq!(d, before);
    assert!(preview(&d, Some(&[]), Direction::Horizontal).is_err());
    for _ in 0..500 {
        d.add_node(ShapeKind::Rectangle, [200., 200.]);
    }
    let before = d.clone();
    assert!(preview(&d, None, Direction::Horizontal).is_err());
    assert_eq!(d, before);
}
