use diagcraft::{
    document::{Document, ShapeKind},
    images::ImageAsset,
    pdf,
    raster::{self, PngOptions},
    rich_text::InlineStyle,
    svg, text_layout,
};

fn block(kind: ShapeKind, wrap: bool) -> (Document, u64) {
    let mut doc = Document {
        page_size: [400., 300.],
        ..Default::default()
    };
    let id = doc.add_node(kind, [180., 140.]);
    let n = doc.node_mut(id).unwrap();
    n.position = [100., 100.];
    n.size = [180., 80.];
    n.text = "BEGIN English français 中文 very long words WWWWWWWWWWWWWWWWWWWWWWWWWWW END".into();
    n.text_style.wrap = wrap;
    n.text_color = [210, 10, 20];
    (doc, id)
}

fn png(doc: &Document) -> image::RgbaImage {
    image::load_from_memory(
        &raster::export(
            doc,
            &PngOptions {
                width: 400,
                height: 300,
                transparent: true,
                ..Default::default()
            },
        )
        .unwrap(),
    )
    .unwrap()
    .to_rgba8()
}

#[test]
fn block_text_is_clipped_to_its_content_area_in_svg_and_transparent_png() {
    for kind in [ShapeKind::Rectangle, ShapeKind::Diamond, ShapeKind::Ellipse] {
        for wrap in [false, true] {
            let (mut doc, id) = block(kind, wrap);
            doc.node_mut(id).unwrap().font_size = 24.;
            let node = doc.node(id).unwrap();
            let (origin, size) = text_layout::area(node);
            let before = doc.clone();
            let exported = svg::export(&doc).unwrap();
            assert!(exported.contains(&format!(
                "<clipPath id=\"diagcraft-text-{id}\" clipPathUnits=\"userSpaceOnUse\"><rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>",
                origin[0], origin[1], size[0], size[1]
            )));
            assert!(exported.contains(&format!(
                "<g clip-path=\"url(#diagcraft-text-{id})\">\n<text"
            )));
            let image = png(&doc);
            let mut red_pixels = 0;
            for (x, y, color) in image.enumerate_pixels() {
                if color[3] > 0 && color[0] > 160 && color[1] < 90 && color[2] < 90 {
                    red_pixels += 1;
                    assert!(
                        (x as f32) >= origin[0].floor()
                            && (x as f32) < (origin[0] + size[0]).ceil()
                    );
                    assert!(
                        (y as f32) >= origin[1].floor()
                            && (y as f32) < (origin[1] + size[1]).ceil()
                    );
                }
                // Shape strokes can extend a pixel beyond their geometry.
                if !(98..=282).contains(&x) || !(98..=182).contains(&y) {
                    assert_eq!(
                        color[3], 0,
                        "unexpected painted pixel {x},{y} for {kind:?}, wrap {wrap}"
                    );
                }
            }
            assert!(red_pixels > 0, "no visible text for {kind:?}, wrap {wrap}");
            assert_eq!(doc, before);
        }
    }
}

#[test]
fn vertical_overflow_starts_at_the_top_and_fitting_text_stays_centered() {
    let (mut doc, id) = block(ShapeKind::Rectangle, true);
    doc.node_mut(id).unwrap().text = "BEGIN\nMIDDLE\nEND\nFINAL".into();
    let n = doc.node(id).unwrap();
    let layout = n.text_layout();
    let (origin, size) = text_layout::area(n);
    assert!(layout.height > size[1]);
    let baseline = origin[1] + (layout.line_height + layout.ascent + layout.descent) / 2.;
    let exported = svg::export(&doc).unwrap();
    assert!(exported.contains(&format!("y=\"{baseline}\">BEGIN</tspan>")));
    assert!(exported.contains(">FINAL</tspan>"));
    assert!(
        png(&doc)
            .enumerate_pixels()
            .any(|(_, y, p)| y < 135 && p[0] > 160 && p[1] < 90)
    );
    doc.node_mut(id).unwrap().text = "Fits".into();
    let n = doc.node(id).unwrap();
    let baseline = n.text_layout().first_baseline(n.center()[1]);
    assert!(
        svg::export(&doc)
            .unwrap()
            .contains(&format!("y=\"{baseline}\">Fits</tspan>"))
    );
}

#[test]
fn clipping_preserves_rich_runs_original_document_and_image_geometry() {
    let (mut doc, id) = block(ShapeKind::Rounded, true);
    doc.node_mut(id)
        .unwrap()
        .format_text(
            0..5,
            Some(&InlineStyle {
                bold: Some(true),
                italic: Some(true),
                font_size: Some(40.),
                color: Some([210, 10, 20]),
                underline: Some(true),
                ..Default::default()
            }),
        )
        .unwrap();
    let before = doc.clone();
    let exported = svg::export(&doc).unwrap();
    assert!(exported.contains("font-size=\"40\" fill=\"#d20a14\" font-weight=\"700\" font-style=\"italic\" text-decoration=\"underline\">BEGIN</tspan>"));
    assert!(exported.contains(" END</tspan>"));
    assert!(
        pdf::export(&doc, pdf::PageFormat::Drawing)
            .unwrap()
            .starts_with(b"%PDF-")
    );
    assert_eq!(doc, before);
    // An image has its own shape clip; the text clip must not include its body.
    let image = doc
        .add_image(
            ImageAsset::from_rgba(1, 1, &[10, 20, 30, 255]).unwrap(),
            [300., 220.],
        )
        .unwrap();
    doc.node_mut(image).unwrap().text = "An image caption long enough to overflow".into();
    let exported = svg::export(&doc).unwrap();
    let image_at = exported.find("<image ").unwrap();
    let caption_clip_at = exported
        .find(&format!("<clipPath id=\"diagcraft-text-{image}\""))
        .unwrap();
    assert!(image_at < caption_clip_at);
}

#[test]
fn standalone_text_preserves_unclipped_centered_overflow() {
    let (doc, id) = block(ShapeKind::Text, false);
    let n = doc.node(id).unwrap();
    let baseline = n.text_layout().first_baseline(n.center()[1]);
    let exported = svg::export(&doc).unwrap();
    assert!(!exported.contains("diagcraft-text-"));
    assert!(exported.contains(&format!("y=\"{baseline}\">BEGIN")));
    assert!(
        png(&doc)
            .enumerate_pixels()
            .any(|(x, _, p)| x < 98 && p[3] > 0)
    );
}
