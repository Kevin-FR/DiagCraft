use diagcraft::document::{ConnectorKind, Document, Port, ShapeKind};
fn sign(p: Port) -> [f32; 2] {
    match p {
        Port::Top => [0., -1.],
        Port::Bottom => [0., 1.],
        Port::Left => [-1., 0.],
        Port::Right => [1., 0.],
        _ => unreachable!(),
    }
}
#[test]
fn automatic_elbows_leave_and_enter_each_chosen_side_without_crossing_endpoint_blocks() {
    for target in [
        [850., 650.],
        [150., 650.],
        [850., 150.],
        [150., 150.],
        [850., 400.],
    ] {
        for from in [Port::Top, Port::Right, Port::Bottom, Port::Left] {
            for to in [Port::Top, Port::Right, Port::Bottom, Port::Left, Port::Auto] {
                let mut d = Document::default();
                let a = d.add_node(ShapeKind::Rectangle, [500., 400.]);
                let b = d.add_node(ShapeKind::Rectangle, target);
                d.connect(a, b, ConnectorKind::Elbow).unwrap();
                d.connectors[0].from_port = from;
                d.connectors[0].to_port = to;
                let route = d.route(&d.connectors[0]);
                assert!(route.len() >= 2);
                let v = sign(from);
                let delta = [route[1][0] - route[0][0], route[1][1] - route[0][1]];
                assert!(
                    delta[0] * v[0] + delta[1] * v[1] > 0.,
                    "from {from:?}, to {to:?}, route {route:?}"
                );
                if to != Port::Auto {
                    let v = sign(to);
                    let end = route.len() - 1;
                    let delta = [
                        route[end - 1][0] - route[end][0],
                        route[end - 1][1] - route[end][1],
                    ];
                    assert!(delta[0] * v[0] + delta[1] * v[1] > 0.);
                }
                for segment in route.windows(2) {
                    assert!(
                        (segment[0][0] - segment[1][0]).abs() < 0.01
                            || (segment[0][1] - segment[1][1]).abs() < 0.01
                    );
                    for n in &d.nodes {
                        let x0 = n.position[0] + 0.01;
                        let x1 = n.position[0] + n.size[0] - 0.01;
                        let y0 = n.position[1] + 0.01;
                        let y1 = n.position[1] + n.size[1] - 0.01;
                        let vertical = (segment[0][0] - segment[1][0]).abs() < 0.01;
                        let crosses = if vertical {
                            segment[0][0] > x0
                                && segment[0][0] < x1
                                && segment[0][1].min(segment[1][1]) < y1
                                && segment[0][1].max(segment[1][1]) > y0
                        } else {
                            segment[0][1] > y0
                                && segment[0][1] < y1
                                && segment[0][0].min(segment[1][0]) < x1
                                && segment[0][0].max(segment[1][0]) > x0
                        };
                        assert!(
                            !crosses,
                            "crossing block {}: {from:?} {to:?} {route:?}",
                            n.id
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn tight_facing_blocks_and_free_ends_keep_cardinal_attachments() {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [250., 300.]);
    let b = d.add_node(ShapeKind::Rectangle, [490., 300.]);
    d.connect(a, b, ConnectorKind::Elbow).unwrap();
    d.connectors[0].from_port = Port::Right;
    d.connectors[0].to_port = Port::Left;
    assert_eq!(d.route(&d.connectors[0]), vec![[360., 300.], [380., 300.]]);
    let id = d
        .connect_endpoints(a, 0, None, Some([100., 650.]), ConnectorKind::Elbow)
        .unwrap();
    let edge = d.connectors.iter_mut().find(|e| e.id == id).unwrap();
    edge.from_port = Port::Top;
    let route = d.route(d.connectors.last().unwrap());
    assert_eq!(route[0], [250., 258.]);
    assert_eq!(*route.last().unwrap(), [100., 650.]);
    assert!(route[1][1] < route[0][1]);
}
