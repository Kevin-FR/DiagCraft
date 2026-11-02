use diagcraft::{
    auto_layout::{Direction, Spacing, preview, preview_with_spacing},
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

#[test]
fn custom_spacing_controls_levels_and_siblings_independently_in_both_directions() {
    let (doc, ids) = flow();
    let before = doc.clone();
    for (axis, direction) in [(0, Direction::Horizontal), (1, Direction::Vertical)] {
        for spacing in [
            Spacing {
                levels: 20.,
                blocks: 300.,
            },
            Spacing {
                levels: 300.,
                blocks: 20.,
            },
            Spacing {
                levels: 85.,
                blocks: 35.,
            },
        ] {
            let p = preview_with_spacing(&doc, None, direction, spacing).unwrap();
            let a = p.document.node(ids[0]).unwrap();
            let b = p.document.node(ids[1]).unwrap();
            let c = p.document.node(ids[2]).unwrap();
            assert_eq!(
                b.position[axis] - a.position[axis] - a.size[axis],
                spacing.levels
            );
            assert_eq!(
                c.position[1 - axis] - b.position[1 - axis] - b.size[1 - axis],
                spacing.blocks
            );
            assert_eq!(p.document.connectors, doc.connectors);
            let repeated = preview_with_spacing(&p.document, None, direction, spacing).unwrap();
            assert_eq!(repeated.moved, 0);
            p.document.validate().unwrap();
        }
    }
    assert_eq!(doc, before);
    assert_eq!(
        preview(&doc, None, Direction::Horizontal).unwrap().document,
        preview_with_spacing(&doc, None, Direction::Horizontal, Spacing::default())
            .unwrap()
            .document
    );
}

#[test]
fn invalid_spacing_is_rejected_without_mutating_the_source() {
    let (doc, _) = flow();
    let before = doc.clone();
    for value in [
        f32::NAN,
        f32::INFINITY,
        f32::NEG_INFINITY,
        -1.,
        0.,
        19.,
        301.,
    ] {
        for spacing in [
            Spacing {
                levels: value,
                blocks: 60.,
            },
            Spacing {
                levels: 60.,
                blocks: value,
            },
        ] {
            assert!(preview_with_spacing(&doc, None, Direction::Vertical, spacing).is_err());
            assert_eq!(doc, before);
        }
    }
}

#[test]
fn crossing_branches_are_reordered_without_changing_connections_or_source() {
    let mut doc = Document::default();
    let ids: Vec<_> = (0..4)
        .map(|i| doc.add_node(ShapeKind::Rectangle, [300. + i as f32 * 160., 400.]))
        .collect();
    doc.connect(ids[0], ids[3], ConnectorKind::Elbow).unwrap();
    doc.connect(ids[1], ids[2], ConnectorKind::Elbow).unwrap();
    doc.connectors[0].waypoints = vec![[700., 200.], [900., 200.]];
    let before = doc.clone();
    for (cross, direction) in [(1, Direction::Horizontal), (0, Direction::Vertical)] {
        let p = preview(&doc, None, direction).unwrap();
        assert_eq!((p.crossings_before, p.crossings_after), (1, 0));
        assert!(
            p.document.node(ids[3]).unwrap().position[cross]
                < p.document.node(ids[2]).unwrap().position[cross]
        );
        assert_eq!(p.document.connectors, doc.connectors);
        assert_eq!(preview(&p.document, None, direction).unwrap().moved, 0);
        let mut shuffled = doc.clone();
        shuffled.connectors.reverse();
        let reordered = preview(&shuffled, None, direction).unwrap();
        for n in &p.document.nodes {
            assert_eq!(reordered.document.node(n.id).unwrap().position, n.position);
        }
    }
    assert_eq!(doc, before);
}

#[test]
fn fixed_obstacles_do_not_reverse_the_optimized_order_of_unequal_blocks() {
    let mut doc = Document::default();
    let ids: Vec<_> = (0..4)
        .map(|_| doc.add_node(ShapeKind::Rectangle, [800., 600.]))
        .collect();
    doc.node_mut(ids[2]).unwrap().size = [100., 100.];
    doc.node_mut(ids[3]).unwrap().size = [300., 100.];
    doc.connect(ids[0], ids[3], ConnectorKind::Elbow).unwrap();
    doc.connect(ids[1], ids[2], ConnectorKind::Elbow).unwrap();
    let obstacle = doc.add_node(ShapeKind::Text, [500., 40.]);
    let n = doc.node_mut(obstacle).unwrap();
    n.position = [500., 40.];
    n.size = [80., 250.];
    let before = doc.clone();
    let p = preview(&doc, None, Direction::Horizontal).unwrap();
    assert_eq!((p.crossings_before, p.crossings_after), (1, 0));
    let first = p.document.node(ids[3]).unwrap();
    let second = p.document.node(ids[2]).unwrap();
    assert!(first.position[1] >= 350.);
    assert!(second.position[1] >= first.position[1] + first.size[1] + 60.);
    assert_eq!(
        p.document.node(obstacle).unwrap(),
        doc.node(obstacle).unwrap()
    );
    assert_eq!(p.document.connectors, doc.connectors);
    assert_eq!(doc, before);
    assert_eq!(
        preview(&p.document, None, Direction::Horizontal)
            .unwrap()
            .moved,
        0
    );
}

fn grouped_flow() -> (Document, Vec<u64>, u64) {
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rectangle, [500., 400.]);
    let b = doc.add_node(ShapeKind::Ellipse, [700., 550.]);
    let text = doc.add_node(ShapeKind::Text, [600., 300.]);
    let edge = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    doc.connectors[0].waypoints = vec![[300., 220.], [900., 680.]];
    doc.connectors[0].label = "Internal".into();
    doc.group_objects(&[a, b, text, edge]).unwrap();
    let tail = doc.add_node(ShapeKind::Rectangle, [1100., 800.]);
    doc.connect(b, tail, ConnectorKind::Elbow).unwrap();
    doc.connectors[1].waypoints = vec![[1100., 700.]];
    (doc, vec![a, b, text, tail], edge)
}

#[test]
fn groups_move_rigidly_and_external_connections_determine_levels_in_both_directions() {
    let (doc, ids, internal) = grouped_flow();
    for (axis, direction) in [(0, Direction::Horizontal), (1, Direction::Vertical)] {
        let result = preview(&doc, None, direction).unwrap();
        let next = &result.document;
        let delta = [
            next.node(ids[0]).unwrap().position[0] - doc.node(ids[0]).unwrap().position[0],
            next.node(ids[0]).unwrap().position[1] - doc.node(ids[0]).unwrap().position[1],
        ];
        for id in &ids[..3] {
            let original = doc.node(*id).unwrap();
            let mut expected = original.clone();
            for (axis, delta) in delta.iter().enumerate() {
                expected.position[axis] += delta;
            }
            assert_eq!(next.node(*id).unwrap(), &expected);
        }
        let mut expected = doc.connectors[0].clone();
        assert_eq!(expected.id, internal);
        for p in &mut expected.waypoints {
            for axis in 0..2 {
                p[axis] += delta[axis];
            }
        }
        assert_eq!(next.connectors[0], expected);
        assert_eq!(next.connectors[1], doc.connectors[1]);
        assert!(next.node(ids[3]).unwrap().position[axis] > [900., 680.][axis] + delta[axis]); // group's envelope includes the far waypoint
        assert_eq!(next.organization, doc.organization);
        assert_eq!(result.moved, 4);
        assert_eq!(result.fixed, 0);
        assert_eq!(preview(next, None, direction).unwrap().moved, 0);
        assert_eq!(preview(&doc, None, direction).unwrap().document, *next);
        let mut shuffled = doc.clone();
        shuffled.nodes.reverse();
        shuffled.connectors.reverse();
        shuffled.organization.groups[0].members.reverse();
        let reordered = preview(&shuffled, None, direction).unwrap();
        for node in &next.nodes {
            assert_eq!(reordered.document.node(node.id).unwrap(), node);
        }
    }
}

#[test]
fn selecting_any_group_member_including_text_or_connector_expands_to_whole_group() {
    let (doc, ids, internal) = grouped_flow();
    for selected in [ids[0], ids[1], ids[2], internal] {
        let result = preview(&doc, Some(&[selected]), Direction::Horizontal).unwrap();
        assert_eq!(result.moved, 3);
        assert_eq!(result.fixed, 1);
        assert_eq!(result.document.node(ids[3]), doc.node(ids[3]));
        for id in &ids[..3] {
            assert_ne!(result.document.node(*id), doc.node(*id));
        }
        assert_ne!(result.document.connectors[0], doc.connectors[0]);
        assert_eq!(result.document.connectors[1], doc.connectors[1]);
    }
}

#[test]
fn protection_of_any_member_or_layer_keeps_whole_group_fixed() {
    let (doc, ids, internal) = grouped_flow();
    for lock in [ids[0], ids[2], internal] {
        let mut locked = doc.clone();
        locked.set_objects_locked(&[lock], true).unwrap();
        let next = preview(&locked, None, Direction::Horizontal)
            .unwrap()
            .document;
        for id in &ids[..3] {
            assert_eq!(next.node(*id), locked.node(*id));
        }
        assert_eq!(next.connectors, locked.connectors);
        assert!(preview(&locked, Some(&[ids[0]]), Direction::Horizontal).is_err());
    }
    for (visible, locked) in [(false, false), (true, true)] {
        let mut protected = doc.clone();
        let layer = protected.create_layer("Group".into()).unwrap();
        protected.move_objects_to_layer(&[ids[0]], layer).unwrap();
        protected
            .update_layer(layer, None, Some(visible), Some(locked))
            .unwrap();
        let next = preview(&protected, None, Direction::Horizontal)
            .unwrap()
            .document;
        for id in &ids[..3] {
            assert_eq!(next.node(*id), protected.node(*id));
        }
        assert_eq!(next.connectors, protected.connectors);
    }
}

#[test]
fn unsupported_mixed_external_free_and_text_only_groups_stay_fixed() {
    use diagcraft::{chart::ChartKind, document::DrawingKind};
    for variant in 0..5 {
        let mut doc = Document::default();
        let first_kind = if variant == 4 {
            ShapeKind::Text
        } else {
            ShapeKind::Rectangle
        };
        let first = doc.add_node(first_kind, [400., 400.]);
        let extra = match variant {
            0 => doc.add_chart(ChartKind::Bars, [600., 400.]),
            1 => doc.add_drawing(DrawingKind::Line, [500., 400.], [600., 500.]),
            2 | 3 => {
                let external = doc.add_node(ShapeKind::Rectangle, [700., 400.]);
                let edge = doc.connect(first, external, ConnectorKind::Elbow).unwrap();
                if variant == 3 {
                    doc.connectors[0].to = 0;
                    doc.connectors[0].free_end = Some([700., 400.]);
                }
                edge
            }
            _ => doc.add_node(ShapeKind::Text, [600., 400.]),
        };
        doc.group_objects(&[first, extra]).unwrap();
        doc.add_node(ShapeKind::Rectangle, [1000., 800.]);
        let result = preview(&doc, None, Direction::Horizontal).unwrap();
        assert_eq!(result.document.node(first), doc.node(first));
        assert_eq!(result.document.connectors, doc.connectors);
        assert_eq!(result.document.charts, doc.charts);
        assert_eq!(result.document.drawings, doc.drawings);
        assert!(preview(&doc, Some(&[first]), Direction::Vertical).is_err());
    }
}

#[test]
fn group_envelope_avoids_obstacles_and_extends_page_for_internal_waypoints() {
    let (mut doc, ids, _) = grouped_flow();
    let obstacle = doc.add_node(ShapeKind::Rectangle, [90., 90.]);
    doc.node_mut(obstacle).unwrap().position = [40., 40.];
    doc.node_mut(obstacle).unwrap().size = [200., 200.];
    let next = preview(&doc, Some(&[ids[0]]), Direction::Horizontal)
        .unwrap()
        .document;
    // Min waypoint began at [300, 220], so its new coordinate is the envelope origin.
    let envelope = next.connectors[0].waypoints[0];
    assert_eq!(envelope[0], 40.);
    assert_eq!(envelope[1], 300.);
    assert_eq!(next.node(obstacle), doc.node(obstacle));
    for p in &next.connectors[0].waypoints {
        for (axis, coordinate) in p.iter().enumerate() {
            assert!(*coordinate >= 0. && *coordinate + 40. <= next.page_size[axis]);
        }
    }
    next.validate().unwrap();
}

#[test]
fn large_groups_count_every_node_and_excessive_envelopes_fail_without_source_changes() {
    let mut doc = Document::default();
    let ids: Vec<_> = (0..501)
        .map(|_| doc.add_node(ShapeKind::Rectangle, [500., 500.]))
        .collect();
    doc.group_objects(&ids).unwrap();
    let before = doc.clone();
    assert!(
        preview(&doc, Some(&ids[..1]), Direction::Horizontal)
            .err()
            .unwrap()
            .contains("500")
    );
    assert_eq!(doc, before);
    let (mut doc, ids, _) = grouped_flow();
    doc.connectors[0].waypoints = vec![[-100_000., 0.], [100_000., 0.]];
    let before = doc.clone();
    assert!(preview(&doc, Some(&ids[..1]), Direction::Horizontal).is_err());
    assert_eq!(doc, before);
}

#[test]
fn fractional_group_coordinates_and_negative_waypoints_are_idempotent() {
    let (mut doc, ids, _) = grouped_flow();
    doc.node_mut(ids[0]).unwrap().position = [210.1, -35.75];
    doc.node_mut(ids[1]).unwrap().position = [357.37, 280.19];
    doc.node_mut(ids[2]).unwrap().position = [-10.02, 123.45];
    doc.connectors[0].waypoints = vec![[-780.23, -250.78], [950.51, 704.65]];
    let image = doc
        .add_image(
            diagcraft::images::ImageAsset::from_rgba(1, 1, &[10, 20, 30, 255]).unwrap(),
            [700.23, 510.48],
        )
        .unwrap();
    let mut group = doc.organization.groups[0].members.clone();
    group.push(image);
    doc.group_objects(&group).unwrap();
    let source = doc.clone();
    for direction in [Direction::Horizontal, Direction::Vertical] {
        let next = preview(&doc, None, direction).unwrap().document;
        assert_eq!(
            next.node(image).unwrap().image,
            doc.node(image).unwrap().image
        );
        assert_eq!(next.organization, doc.organization);
        let repeated = preview(&next, None, direction).unwrap();
        assert_eq!(repeated.moved, 0);
        assert_eq!(repeated.document, next);
    }
    assert_eq!(doc, source);
}

#[test]
fn rounded_group_envelopes_do_not_shift_successors_on_a_second_layout() {
    for (direction, connected) in [
        (Direction::Horizontal, true),
        (Direction::Vertical, true),
        (Direction::Horizontal, false),
        (Direction::Vertical, false),
    ] {
        let axis = usize::from(direction == Direction::Vertical);
        let mut doc = Document::default();
        let a = doc.add_node(ShapeKind::Rectangle, [0., 0.]);
        let b = doc.add_node(ShapeKind::Rectangle, [0., 0.]);
        let tail = doc.add_node(ShapeKind::Rectangle, [900., 900.]);
        doc.node_mut(a).unwrap().position = [-50.23; 2];
        doc.node_mut(b).unwrap().position = [310.13; 2];
        doc.node_mut(b).unwrap().size = [99.7; 2];
        doc.group_objects(&[a, b]).unwrap();
        if connected {
            doc.connect(b, tail, ConnectorKind::Elbow).unwrap();
        }
        let next = preview(&doc, None, direction).unwrap().document;
        let repeated = preview(&next, None, direction).unwrap();
        assert_eq!(repeated.moved, 0, "axis {axis}");
        assert_eq!(repeated.document, next);
    }
}
