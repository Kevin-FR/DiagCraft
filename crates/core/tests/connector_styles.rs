use diagcraft::{
    document::{ConnectorKind, Document, FORMAT_VERSION, Port, ShapeKind},
    pdf::{self, PageFormat},
    stroke_style::{self, ArrowHead, LinePattern},
    svg,
};

fn linked() -> (Document, u64, u64, u64) {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [200., 300.]);
    let b = d.add_node(ShapeKind::Rectangle, [1000., 300.]);
    let edge = d.connect(a, b, ConnectorKind::Elbow).unwrap();
    (d, a, b, edge)
}

#[test]
fn all_styles_survive_save_clipboard_insertion_and_vector_exports() {
    let temp = tempfile::tempdir().unwrap();
    for pattern in LinePattern::ALL {
        for head in ArrowHead::ALL {
            let (mut d, a, b, id) = linked();
            let e = &mut d.connectors[0];
            e.pattern = pattern;
            e.arrow_start = true;
            e.start_head = head;
            e.end_head = head;
            e.width = 2.;
            let path = temp.path().join("styles.draw");
            d.save(&path).unwrap();
            assert_eq!(Document::load(&path).unwrap(), d);
            let text = svg::export(&d).unwrap();
            match pattern {
                LinePattern::Solid => assert!(!text.contains("stroke-dasharray")),
                LinePattern::Dashed => assert!(text.contains("stroke-dasharray=\"12 8\"")),
                LinePattern::Dotted => assert!(text.contains("stroke-dasharray=\"0 8\"")),
            }
            assert_eq!(
                text.matches("<polygon").count(),
                if head == ArrowHead::Open { 0 } else { 2 }
            );
            let bytes = pdf::export(&d, PageFormat::Drawing).unwrap();
            assert!(bytes.starts_with(b"%PDF-"));
            let selection = d.selection(&[a, b]);
            d.paste(&selection, [24., 24.]).unwrap();
            assert_eq!(d.connectors[1].pattern, pattern);
            assert_eq!(d.connectors[1].end_head, head);
            d.insert_shape_on_connector(id, ShapeKind::Diamond, [600., 300.])
                .unwrap();
            for e in &d.connectors {
                assert_eq!(e.pattern, pattern);
                assert_eq!(e.start_head, head);
                assert_eq!(e.end_head, head);
            }
        }
    }
}

#[test]
fn version_five_styles_default_without_touching_the_original() {
    let (d, _, _, _) = linked();
    let mut value = serde_json::to_value(&d).unwrap();
    value["format_version"] = 5.into();
    for key in ["pattern", "start_head", "end_head"] {
        value["connectors"][0].as_object_mut().unwrap().remove(key);
    }
    let bytes = serde_json::to_vec(&value).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("v5.draw");
    std::fs::write(&path, &bytes).unwrap();
    let migrated = Document::load(&path).unwrap();
    assert_eq!(migrated.format_version, FORMAT_VERSION);
    assert_eq!(migrated.connectors[0].pattern, LinePattern::Solid);
    assert_eq!(migrated.connectors[0].end_head, ArrowHead::Triangle);
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn reconnect_is_atomic_preserves_style_and_rejects_duplicates_or_self_links() {
    let (mut d, a, b, id) = linked();
    let c = d.add_node(ShapeKind::Diamond, [600., 600.]);
    d.connectors[0].pattern = LinePattern::Dotted;
    d.connectors[0].end_head = ArrowHead::Circle;
    d.set_automatic_route(id, false).unwrap();
    let style = d.connectors[0].clone();
    d.reconnect_connector(id, false, c, Port::Top).unwrap();
    assert_eq!(d.connectors[0].to, c);
    assert_eq!(d.connectors[0].to_port, Port::Top);
    assert_eq!(d.connectors[0].pattern, style.pattern);
    assert_eq!(d.connectors[0].end_head, style.end_head);
    assert_eq!(d.connectors[0].waypoints, style.waypoints);
    let before = d.clone();
    for invalid in [a, 9999] {
        assert!(
            d.reconnect_connector(id, false, invalid, Port::Auto)
                .is_err()
        );
        assert_eq!(d, before);
    }
    d.connect(a, b, ConnectorKind::Straight).unwrap();
    let before = d.clone();
    assert!(d.reconnect_connector(id, false, b, Port::Auto).is_err());
    assert_eq!(d, before);
    d.reconnect_connector(id, true, b, Port::Right).unwrap();
    assert_eq!((d.connectors[0].from, d.connectors[0].to), (b, c));
}

#[test]
fn arrow_geometry_handles_vertical_reversed_and_zero_length_routes() {
    for head in ArrowHead::ALL {
        assert!(stroke_style::marker(&[[0., 0.], [0., 0.]], head, 10.).is_none());
        for tip in [[100., 0.], [-100., 0.], [0., 100.], [0., -100.]] {
            let marker = stroke_style::marker(&[[0., 0.], tip, tip], head, 10.).unwrap();
            for p in marker.points {
                assert!(p.iter().all(|v| v.is_finite()));
                assert!((p[0] - tip[0]).hypot(p[1] - tip[1]) <= 14.01);
                assert!((p[0] - tip[0]) * tip[0] + (p[1] - tip[1]) * tip[1] <= 0.01);
            }
        }
    }
}
