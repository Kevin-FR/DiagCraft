use diagcraft::{agent, appearance::*, appearance_svg, document::*, pdf, svg};

fn node() -> Node {
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Rectangle, [100., 50.]);
    let n = d.node_mut(id).unwrap();
    n.position = [0., 0.];
    n.size = [200., 100.];
    n.fill = [255, 255, 255];
    n.stroke_width = 0.;
    n.appearance.line.mode = PaintMode::None;
    n.text.clear();
    n.clone()
}
fn raster(svg: &str) -> resvg::tiny_skia::Pixmap {
    let tree = resvg::usvg::Tree::from_str(svg, &resvg::usvg::Options::default()).unwrap();
    let mut p = resvg::tiny_skia::Pixmap::new(
        tree.size().width().ceil() as u32,
        tree.size().height().ceil() as u32,
    )
    .unwrap();
    resvg::render(
        &tree,
        resvg::tiny_skia::Transform::identity(),
        &mut p.as_mut(),
    );
    p
}
fn pixel(n: &Node, x: f32, y: f32) -> [u8; 4] {
    let (svg, b) = appearance_svg::node_image(n);
    let p = raster(&svg);
    let x = (x - b[0]).floor() as u32;
    let y = (y - b[1]).floor() as u32;
    if x >= p.width() || y >= p.height() {
        return [0; 4];
    }
    let i = (y * p.width() + x) as usize * 4;
    p.data()[i..i + 4].try_into().unwrap()
}
#[test]
fn defaults_preserve_v7_json_and_agent_revision() {
    let d = Document::demo();
    assert!(
        !serde_json::to_string(&d)
            .unwrap()
            .contains("\"appearance\"")
    );
    let d = Document::load(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../examples/dossier-v7.draw"
    )))
    .unwrap();
    let patch: serde_json::Value =
        serde_json::from_str(include_str!("../../../examples/agent.patch.json")).unwrap();
    assert_eq!(agent::revision(&d), patch["expected_revision"]);
}
#[test]
fn appearance_round_trips_on_multiple_pages_and_exports() {
    let mut d = demo();
    d.add_page(false).unwrap();
    let id = d.add_node(ShapeKind::Diamond, [200., 200.]);
    d.node_mut(id).unwrap().appearance.shadow = Shadow::preset(ShadowKind::Inner, 0);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("styles.draw");
    d.save(&path).unwrap();
    let loaded = Document::load(&path).unwrap();
    assert_eq!(loaded, d);
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(value["format_version"], 7);
    assert!(svg::export(&loaded).unwrap().contains("-inner"));
    assert!(
        pdf::export(&loaded, pdf::PageFormat::Drawing)
            .unwrap()
            .starts_with(b"%PDF-")
    );
}
#[test]
fn invalid_styles_are_rejected_even_when_effect_disabled() {
    for set in [
        |a: &mut Appearance| a.fill.transparency = 101.,
        |a: &mut Appearance| a.fill.tint = f32::NAN,
        |a: &mut Appearance| a.fill.spacing = 0.,
        |a: &mut Appearance| a.shadow.blur = f32::INFINITY,
        |a: &mut Appearance| a.shadow.scale = 0.,
        |a: &mut Appearance| a.shadow.offset[0] = 201.,
        |a: &mut Appearance| a.line.end_scale = 4.,
        |a: &mut Appearance| a.line.mode = PaintMode::Texture,
    ] {
        let mut d = demo();
        set(&mut d.nodes[0].appearance);
        assert!(d.validate().is_err());
        assert!(svg::export(&d).is_err());
    }
}
#[test]
fn gradient_angle_radial_tint_and_transparency_affect_pixels() {
    let mut n = node();
    n.fill = [255, 0, 0];
    n.appearance.fill.secondary = [0, 0, 255];
    n.appearance.fill.mode = PaintMode::Linear;
    let left = pixel(&n, 20., 50.);
    let right = pixel(&n, 180., 50.);
    assert!(left[0] > left[2] && right[2] > right[0]);
    n.appearance.fill.angle = 180.;
    let left = pixel(&n, 20., 50.);
    assert!(left[2] > left[0]);
    n.appearance.fill.mode = PaintMode::Radial;
    assert!(pixel(&n, 100., 50.)[0] > 230);
    assert!(pixel(&n, 195., 50.)[2] > 200);
    n.appearance.fill.mode = PaintMode::Solid;
    n.appearance.fill.transparency = 50.;
    assert!((pixel(&n, 100., 50.)[3] as i32 - 128).abs() < 3);
    n.appearance.fill.transparency = 0.;
    n.appearance.fill.tint = 100.;
    assert_eq!(pixel(&n, 100., 50.), [255, 255, 255, 255]);
    n.appearance.fill.mode = PaintMode::None;
    assert_eq!(pixel(&n, 100., 50.)[3], 0);
}
#[test]
fn pattern_and_texture_variants_produce_distinct_visible_pixels() {
    let mut n = node();
    n.fill = [245, 245, 245];
    n.appearance.fill.secondary = [20, 20, 20];
    let mut bodies = std::collections::HashSet::new();
    for pattern in PatternKind::ALL {
        n.appearance.fill.mode = PaintMode::Pattern;
        n.appearance.fill.pattern = *pattern;
        let (s, _) = appearance_svg::node_image(&n);
        let p = raster(&s);
        assert!(p.data().iter().any(|v| *v > 0));
        assert!(bodies.insert(p.data().to_vec()));
    }
    for texture in TextureKind::ALL {
        n.appearance.fill.mode = PaintMode::Texture;
        n.appearance.fill.texture = *texture;
        let (s, _) = appearance_svg::node_image(&n);
        assert!(bodies.insert(raster(&s).data().to_vec()));
    }
    n.appearance.fill.transparency = 50.;
    for (x, y) in [(24., 24.), (25., 25.), (30., 30.)] {
        assert!((pixel(&n, x, y)[3] as i32 - 128).abs() < 3);
    }
}
#[test]
fn shadows_are_shaped_and_outer_inner_perspective_are_distinct() {
    let mut n = node();
    n.appearance.shadow = Shadow {
        kind: ShadowKind::Outer,
        transparency: 0.,
        color: [0, 0, 0],
        offset: [12., 12.],
        blur: 4.,
        ..Default::default()
    };
    assert!(pixel(&n, 207., 60.)[3] > 100);
    assert!(pixel(&n, 100., 50.)[0] > 245);
    n.appearance.shadow.kind = ShadowKind::Inner;
    assert_eq!(pixel(&n, 207., 60.)[3], 0);
    assert!(pixel(&n, 3., 50.)[0] < 80);
    assert!(pixel(&n, 100., 50.)[0] > 245);
    n.kind = ShapeKind::Diamond;
    assert_eq!(pixel(&n, 2., 2.)[3], 0);
    n.kind = ShapeKind::Rectangle;
    n.appearance.shadow.kind = ShadowKind::Perspective;
    assert_eq!(pixel(&n, 207., 50.)[3], 0);
    assert!(pixel(&n, 207., 105.)[3] > 100);
}
#[test]
fn shadow_parameters_change_geometry_color_and_opacity() {
    let mut n = node();
    n.appearance.shadow = Shadow {
        kind: ShadowKind::Outer,
        color: [255, 0, 0],
        offset: [25., 0.],
        blur: 0.,
        transparency: 0.,
        ..Default::default()
    };
    assert_eq!(pixel(&n, 215., 50.), [255, 0, 0, 255]);
    n.appearance.shadow.transparency = 50.;
    assert!((pixel(&n, 215., 50.)[3] as i32 - 128).abs() < 3);
    n.appearance.shadow.scale = 150.;
    assert!(appearance_svg::node_bounds(&n)[2] > 270.);
    let old = appearance_svg::node_bounds(&n);
    n.appearance.shadow.angle = 45.;
    assert_ne!(appearance_svg::node_bounds(&n), old);
    n.appearance.shadow.blur = 20.;
    assert!(appearance_svg::node_bounds(&n)[2] > old[2]);
}
#[test]
fn compound_contour_keeps_fill_visible_in_its_gaps() {
    let mut n = node();
    n.stroke = [0, 0, 255];
    n.stroke_width = 10.;
    n.appearance.line.mode = PaintMode::Solid;
    n.appearance.line.compound = Compound::Double;
    assert_eq!(pixel(&n, 100., 0.)[0], 255);
    assert!(pixel(&n, 100., 3.)[2] > 200 && pixel(&n, 100., 3.)[0] < 20);
    n.appearance.line.compound = Compound::Triple;
    assert!(pixel(&n, 100., 0.)[0] < 100);
}
#[test]
fn horizontal_and_vertical_connectors_support_gradients_and_compound_lines() {
    let mut d = Document::default();
    let id = d
        .connect_endpoints(
            0,
            0,
            Some([20., 40.]),
            Some([220., 40.]),
            ConnectorKind::Straight,
        )
        .unwrap();
    let e = d.connectors.iter_mut().find(|e| e.id == id).unwrap();
    e.width = 10.;
    e.color = [255, 0, 0];
    e.arrow = false;
    e.appearance.mode = PaintMode::Linear;
    e.appearance.secondary = [0, 0, 255];
    for vertical in [false, true] {
        let e = &mut d.connectors[0];
        if vertical {
            e.free_start = Some([40., 20.]);
            e.free_end = Some([40., 220.]);
            e.appearance.angle = 90.;
        }
        let pts = d.route(&d.connectors[0]);
        let body = appearance_svg::connector(&d.connectors[0], &pts, [255; 3]);
        let p = raster(&appearance_svg::wrap(&body, [0., 0., 250., 250.]));
        let at = |x: u32, y: u32| {
            let i = (y * p.width() + x) as usize * 4;
            [p.data()[i], p.data()[i + 2], p.data()[i + 3]]
        };
        let a = if vertical { at(40, 35) } else { at(35, 40) };
        let b = if vertical { at(40, 205) } else { at(205, 40) };
        assert!(a[0] > a[1] && b[1] > b[0] && a[2] > 200);
    }
    d.connectors[0].appearance.compound = Compound::Double;
    assert!(
        svg::export(&d)
            .unwrap()
            .contains("maskUnits=\"userSpaceOnUse\"")
    );
}
#[test]
fn agent_styles_are_atomic_and_available_in_inspection() {
    let d = demo();
    let patch = serde_json::json!({"api_version":1,"expected_revision":agent::revision(&d),"operations":[{"op":"update_node","page":0,"target":1,"changes":{"appearance":{"fill":{"mode":"linear","secondary":[255,0,0]},"shadow":{"kind":"inner"}}}}]});
    let parsed = agent::parse_patch(&patch.to_string()).unwrap();
    let result = agent::preview(&d, &parsed).unwrap();
    assert_eq!(
        result.document.nodes[0].appearance.shadow.kind,
        ShadowKind::Inner
    );
    assert_eq!(
        agent::inspect(&result.document).unwrap()["pages"][0]["nodes"][0]["appearance"]["fill"]["mode"],
        "linear"
    );
    let mut bad = patch;
    bad["operations"][0]["changes"]["appearance"]["shadow"]["blur"] = serde_json::json!(999);
    assert!(agent::preview(&d, &agent::parse_patch(&bad.to_string()).unwrap()).is_err());
    assert_eq!(d, demo());
}
#[test]
fn fitting_and_automatic_growth_include_shadow_extents() {
    let mut d = demo();
    d.nodes.truncate(1);
    let n = &mut d.nodes[0];
    n.appearance.shadow = Shadow {
        kind: ShadowKind::Outer,
        offset: [120., 140.],
        scale: 170.,
        blur: 24.,
        ..Default::default()
    };
    d.fit_page_to_drawing().unwrap();
    let n = &d.nodes[0];
    let b = appearance_svg::node_bounds(n);
    for a in 0..2 {
        assert!(n.position[a] + b[a] >= 0.);
        assert!(n.position[a] + b[a + 2] <= d.page_size[a]);
    }
    d.page_layout.auto_size = true;
    d.nodes[0].position = [1000., 900.];
    d.grow_page_to_content();
    let n = &d.nodes[0];
    let b = appearance_svg::node_bounds(n);
    assert!(d.page_size[0] >= n.position[0] + b[2]);
    assert!(d.page_size[1] >= n.position[1] + b[3]);
}

#[test]
fn large_custom_pages_bound_shadow_rasters_and_keep_small_shadows_exportable() {
    let mut d = Document {
        page_size: [100_000., 100_000.],
        ..Default::default()
    };
    let a = d.add_node(ShapeKind::Rectangle, [500., 500.]);
    let b = d.add_node(ShapeKind::Rounded, [30_000., 30_000.]);
    let n = d.node_mut(a).unwrap();
    n.position = [100., 100.];
    n.size = [20_000., 18_000.];
    n.text.clear();
    n.appearance.shadow = Shadow {
        kind: ShadowKind::Outer,
        scale: 300.,
        blur: 64.,
        angle: 45.,
        ..Default::default()
    };
    let n = d.node_mut(b).unwrap();
    n.size = [24., 24.];
    n.text.clear();
    n.appearance.shadow = Shadow {
        kind: ShadowKind::Outer,
        scale: 10.,
        blur: 0.,
        ..Default::default()
    };
    let bytes = pdf::export(&d, pdf::PageFormat::Drawing).unwrap();
    assert!(bytes.starts_with(b"%PDF-"));
    assert!(bytes.len() < 8_000_000);
}
#[test]
fn explicit_zero_corner_radius_and_inner_shadow_respect_shape_alpha() {
    let mut n = node();
    n.kind = ShapeKind::Rounded;
    assert_eq!(pixel(&n, 1., 1.)[3], 0);
    n.appearance.line.corner_radius = Some(0.);
    assert_eq!(pixel(&n, 1., 1.)[3], 255);
    n.appearance.fill.mode = PaintMode::None;
    n.appearance.shadow = Shadow::preset(ShadowKind::Inner, 0);
    assert_eq!(pixel(&n, 100., 50.)[3], 0);
}
#[test]
fn styled_curves_and_arrow_sizes_render_save_and_are_agent_editable() {
    let mut d = Document::default();
    let id = d.add_drawing(DrawingKind::Bezier, [80., 120.], [600., 280.]);
    let patch = serde_json::json!({"api_version":1,"expected_revision":agent::revision(&d),"operations":[{"op":"update_drawing","page":0,"target":id,"start":[80,120],"end":[600,280],"bend":90,"appearance":{"mode":"linear","secondary":[255,100,0],"cap":"square","compound":"double","end_scale":2}}]});
    let result = agent::preview(&d, &agent::parse_patch(&patch.to_string()).unwrap())
        .unwrap()
        .document;
    assert_eq!(result.drawings[0].appearance.end_scale, 2.);
    let s = svg::export(&result).unwrap();
    assert!(s.contains("linearGradient") && s.contains("stroke-linecap=\"square\""));
    assert!(pdf::export(&result, pdf::PageFormat::Drawing).is_ok());
    let mut e = appearance_svg::drawing_edge(&result.drawings[0]);
    e.arrow = true;
    let small = appearance_svg::connector(&e, &result.drawings[0].points(), [255; 3]);
    e.appearance.end_scale = 3.;
    let large = appearance_svg::connector(&e, &result.drawings[0].points(), [255; 3]);
    assert_ne!(
        raster(&appearance_svg::wrap(&small, [0., 0., 700., 400.])).data(),
        raster(&appearance_svg::wrap(&large, [0., 0., 700., 400.])).data()
    );
}

#[test]
fn dotted_lines_remain_visible_with_flat_round_and_square_caps() {
    let mut d = Document::default();
    d.connect_endpoints(
        0,
        0,
        Some([20., 40.]),
        Some([220., 40.]),
        ConnectorKind::Straight,
    )
    .unwrap();
    let e = &mut d.connectors[0];
    e.pattern = diagcraft::stroke_style::LinePattern::Dotted;
    e.width = 6.;
    e.arrow = false;
    for cap in Cap::ALL {
        d.connectors[0].appearance.cap = *cap;
        let p = raster(&appearance_svg::wrap(
            &appearance_svg::connector(&d.connectors[0], &d.route(&d.connectors[0]), [255; 3]),
            [0., 0., 250., 80.],
        ));
        assert!(p.data().chunks_exact(4).filter(|v| v[3] > 100).count() > 100);
    }
}
