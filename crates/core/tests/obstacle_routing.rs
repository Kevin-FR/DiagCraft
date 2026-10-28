use diagcraft::{
    document::{ConnectorKind, Document, Port, ShapeKind},
    routing::RouteStatus,
};
fn node(d: &mut Document, p: [f32; 2], size: [f32; 2]) -> u64 {
    let id = d.add_node(ShapeKind::Rectangle, p);
    let n = d.node_mut(id).unwrap();
    n.position = p;
    n.size = size;
    id
}
fn fixture() -> (Document, u64, u64, u64) {
    let mut d = Document::default();
    let a = node(&mut d, [100., 260.], [120., 80.]);
    let b = node(&mut d, [800., 260.], [120., 80.]);
    let obstacle = node(&mut d, [420., 200.], [160., 200.]);
    d.connect(a, b, ConnectorKind::Elbow).unwrap();
    (d, a, b, obstacle)
}
fn avoids(d: &Document, points: &[[f32; 2]], ignore: &[u64]) {
    for p in points.windows(2) {
        assert!(p[0][0] == p[1][0] || p[0][1] == p[1][1], "diagonal {p:?}");
        for n in &d.nodes {
            if ignore.contains(&n.id) {
                continue;
            }
            let r = [
                n.position[0] + 0.02,
                n.position[1] + 0.02,
                n.position[0] + n.size[0] - 0.02,
                n.position[1] + n.size[1] - 0.02,
            ];
            let cross = if p[0][0] == p[1][0] {
                p[0][0] > r[0]
                    && p[0][0] < r[2]
                    && p[0][1].min(p[1][1]) < r[3]
                    && p[0][1].max(p[1][1]) > r[1]
            } else {
                p[0][1] > r[1]
                    && p[0][1] < r[3]
                    && p[0][0].min(p[1][0]) < r[2]
                    && p[0][0].max(p[1][0]) > r[0]
            };
            assert!(!cross, "crosses {}: {points:?}", n.id);
        }
    }
}
#[test]
fn automatic_links_detour_without_mutating_documents_and_are_order_independent() {
    let (mut d, _, _, _) = fixture();
    let before = d.clone();
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    avoids(&d, &r.points, &[]);
    assert_eq!(d, before);
    d.nodes.reverse();
    assert_eq!(d.route(&d.connectors[0]), r.points);
}
#[test]
fn manual_anchors_are_exact_ordered_constraints_even_when_collinear() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].waypoints = vec![[300., 450.], [600., 450.], [700., 450.]];
    d.connectors[0].from_port = Port::Right;
    d.connectors[0].to_port = Port::Left;
    let before = d.connectors[0].clone();
    let r = d.route_details(&before);
    assert_eq!(r.status, RouteStatus::Clear);
    avoids(&d, &r.points, &[]);
    let mut at = 0;
    for p in &before.waypoints {
        at += r.points[at..]
            .iter()
            .position(|q| q == p)
            .expect("manual anchor lost")
            + 1;
    }
    assert_eq!(d.connectors[0], before);
    d.node_mut(before.from).unwrap().position[1] += 25.;
    let r = d.route(&before);
    for p in &before.waypoints {
        assert!(r.contains(p));
    }
    avoids(&d, &r, &[]);
}
#[test]
fn free_ends_and_explicit_ports_survive_detours() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].from_port = Port::Top;
    d.connectors[0].to = 0;
    d.connectors[0].free_end = Some([860., 300.]);
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Constrained); // free end inside former target
    assert_eq!(*r.points.last().unwrap(), [860., 300.]);
    assert!(r.points[1][1] < r.points[0][1]);
    avoids(&d, &r.points, &[d.nodes[1].id]);
}
#[test]
fn impossible_waypoint_is_preserved_and_reported_without_rewriting_source() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].waypoints = vec![[500., 300.]];
    let original = d.clone();
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Constrained);
    assert!(r.points.contains(&[500., 300.]));
    assert_eq!(d, original);
}
#[test]
fn dense_maze_and_cycles_remain_orthogonal_and_clear() {
    let mut d = Document::default();
    let a = node(&mut d, [50., 300.], [80., 60.]);
    let b = node(&mut d, [1000., 300.], [80., 60.]);
    for (x, y, h) in [
        (220., 0., 420.),
        (420., 240., 400.),
        (620., 0., 420.),
        (820., 240., 400.),
    ] {
        node(&mut d, [x, y], [60., h]);
    }
    d.connect(a, b, ConnectorKind::Elbow).unwrap();
    d.connectors[0].from_port = Port::Right;
    d.connectors[0].to_port = Port::Left;
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    avoids(&d, &r.points, &[]);
    assert!(r.points.len() >= 6);
    d.connect(b, a, ConnectorKind::Elbow).unwrap();
    avoids(&d, &d.route(&d.connectors[1]), &[]);
}
#[test]
fn route_and_label_are_identical_in_inspection_and_svg() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].label = "Verified route".into();
    d.connectors[0].waypoints = vec![[340., 460.], [650., 460.]];
    let r = d.route_details(&d.connectors[0]);
    let inspection = diagcraft::agent::inspect(&d).unwrap();
    assert_eq!(
        inspection["pages"][0]["connectors"][0]["route"],
        serde_json::json!(r.points)
    );
    assert_eq!(
        inspection["pages"][0]["connectors"][0]["label_position"],
        serde_json::json!(r.label_position(d.connectors[0].label_t))
    );
    let coords = r
        .points
        .iter()
        .map(|p| format!("{},{}", p[0], p[1]))
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        diagcraft::svg::export(&d)
            .unwrap()
            .contains(&format!("points=\"{coords}\""))
    );
}
#[test]
fn routing_dense_documents_has_a_fixed_search_budget() {
    let mut d = Document::default();
    for y in 0..20 {
        for x in 0..40 {
            node(&mut d, [x as f32 * 90., y as f32 * 70.], [45., 35.]);
        }
    }
    for y in 0..20 {
        d.connect_endpoints(
            0,
            0,
            Some([-40., y as f32 * 70. + 18.]),
            Some([3650., y as f32 * 70. + 18.]),
            ConnectorKind::Elbow,
        )
        .unwrap();
    }
    let start = std::time::Instant::now();
    let context = diagcraft::routing::RoutingContext::new(&d);
    for e in &d.connectors {
        let r = context.route(e);
        assert_eq!(r.status, RouteStatus::Clear);
        avoids(&d, &r.points, &[]);
    }
    assert!(
        start.elapsed() < std::time::Duration::from_secs(5),
        "routing took {:?}",
        start.elapsed()
    );
}

#[test]
fn hidden_blocks_and_container_frames_do_not_obstruct_routes() {
    let (mut d, _, _, obstacle) = fixture();
    let layer = d.create_layer("Hidden".into()).unwrap();
    d.move_objects_to_layer(&[obstacle], layer).unwrap();
    d.update_layer(layer, None, Some(false), None).unwrap();
    let frame = node(&mut d, [40., 120.], [950., 400.]);
    d.node_mut(frame).unwrap().kind = ShapeKind::Frame;
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    assert!(r.points.iter().all(|p| p[1] == 300.));
    d.update_layer(layer, None, Some(true), None).unwrap();
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    avoids(&d, &r.points, &[frame]);
}
#[test]
fn a_chart_is_an_obstacle_and_direct_lines_keep_their_original_behavior() {
    let (mut d, _, _, obstacle) = fixture();
    d.nodes.retain(|n| n.id != obstacle);
    let chart = d.add_chart(diagcraft::chart::ChartKind::Columns, [500., 300.]);
    let c = d.charts.iter_mut().find(|c| c.id == chart).unwrap();
    c.position = [420., 200.];
    c.size = [160., 200.];
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    assert!(r.points.iter().any(|p| p[1] <= 200. || p[1] >= 400.));
    d.connectors[0].kind = ConnectorKind::Straight;
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Direct);
    assert_eq!(r.points.len(), 2);
}
#[test]
fn adding_one_waypoint_does_not_pin_automatic_bends_and_uses_actual_path_order() {
    let (mut d, _, _, _) = fixture();
    let id = d.connectors[0].id;
    let p = [340., 460.];
    d.add_connector_waypoint(id, p).unwrap();
    assert_eq!(d.connectors[0].waypoints, vec![p]);
    let q = [700., 460.];
    d.add_connector_waypoint(id, q).unwrap();
    assert_eq!(d.connectors[0].waypoints, vec![p, q]);
    avoids(&d, &d.route(&d.connectors[0]), &[]);
    for i in 2..16 {
        d.add_connector_waypoint(id, [700. + i as f32, 460.])
            .unwrap();
    }
    let before = d.clone();
    assert!(d.add_connector_waypoint(id, [760., 460.]).is_err());
    assert_eq!(d, before);
}
#[test]
fn tiny_manual_offsets_negative_zero_and_conflicts_have_truthful_diagnostics() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].waypoints = vec![[340., 460.], [340.005, 460.], [500., 300.]];
    let r = d.route_details(&d.connectors[0]);
    for p in &d.connectors[0].waypoints {
        assert!(r.points.contains(p), "lost {p:?}: {:?}", r.points);
    }
    let inspection = diagcraft::agent::inspect(&d).unwrap();
    assert_eq!(
        inspection["pages"][0]["connectors"][0]["routing_status"],
        "constrained"
    );
    assert!(
        diagcraft::agent::diagnostics(&d)
            .iter()
            .any(|w| w["code"] == "route_constrained")
    );
    let mut d = Document::default();
    node(&mut d, [-10., 100.], [30., 100.]);
    d.connect_endpoints(
        0,
        0,
        Some([-0., 0.]),
        Some([0., 300.]),
        ConnectorKind::Elbow,
    )
    .unwrap();
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    avoids(&d, &r.points, &[]);
}
#[test]
fn small_drags_do_not_flip_a_symmetric_detour_and_undo_restores_exact_geometry() {
    let (mut d, _, _, obstacle) = fixture();
    d.connectors[0].from_port = Port::Right;
    d.connectors[0].to_port = Port::Left;
    let original = d.route(&d.connectors[0]);
    let top = original.iter().any(|p| p[1] < 200.);
    for delta in [1., -1., 2., -2.] {
        let mut shifted = d.clone();
        shifted.node_mut(obstacle).unwrap().position[1] += delta;
        let route = shifted.route(&shifted.connectors[0]);
        assert_eq!(
            route.iter().any(|p| p[1] < 202.),
            top,
            "route flipped: {route:?}"
        );
        avoids(&shifted, &route, &[]);
    }
    let mut history = diagcraft::history::History::new(&d);
    d.node_mut(obstacle).unwrap().position[0] += 80.;
    history.record(&d);
    d = history.undo().unwrap();
    assert_eq!(d.route(&d.connectors[0]), original);
}

#[test]
fn thick_connectors_keep_space_for_their_stroke() {
    let (mut d, _, _, _) = fixture();
    d.connectors[0].width = 40.;
    let r = d.route_details(&d.connectors[0]);
    assert_eq!(r.status, RouteStatus::Clear);
    assert!(r.points.iter().any(|p| p[1] <= 172. || p[1] >= 428.));
    avoids(&d, &r.points, &[]);
}
