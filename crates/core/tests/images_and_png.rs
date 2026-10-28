use diagcraft::{
    document::{Document, ShapeKind},
    history::History,
    images::{ImageAsset, ImageContent},
    raster::{self, PngOptions},
};
use image::{DynamicImage, ImageFormat};
use std::io::Cursor;
fn asset(color: [u8; 4]) -> ImageAsset {
    ImageAsset::from_rgba(4, 2, &[color; 8].concat()).unwrap()
}
fn doc() -> Document {
    Document {
        page_size: [200., 100.],
        ..Default::default()
    }
}
fn png(d: &Document, transparent: bool, selection: Vec<u64>) -> image::RgbaImage {
    image::load_from_memory(
        &raster::export(
            d,
            &PngOptions {
                width: 200,
                height: 100,
                transparent,
                selection,
                ..Default::default()
            },
        )
        .unwrap(),
    )
    .unwrap()
    .to_rgba8()
}
#[test]
fn imports_normalize_png_and_jpeg_and_assets_round_trip_without_reencoding() {
    let a = asset([30, 60, 220, 255]);
    let encoded = serde_json::to_string(&a).unwrap();
    let loaded: ImageAsset = serde_json::from_str(&encoded).unwrap();
    assert_eq!(a, loaded);
    assert_eq!(encoded, serde_json::to_string(&loaded).unwrap());
    let mut jpeg = Cursor::new(vec![]);
    DynamicImage::ImageRgb8(image::RgbImage::from_pixel(4, 2, image::Rgb([200, 30, 10])))
        .write_to(&mut jpeg, ImageFormat::Jpeg)
        .unwrap();
    let a = ImageAsset::from_bytes(jpeg.get_ref()).unwrap();
    assert_eq!([a.width(), a.height()], [4, 2]);
    assert!(a.png_base64().starts_with("iVBOR"));
}
#[test]
fn malformed_oversized_and_mismatched_images_are_rejected_before_import() {
    assert!(ImageAsset::from_bytes(b"<svg><script/></svg>").is_err());
    assert!(ImageAsset::from_bytes(&vec![0; diagcraft::images::MAX_INPUT_BYTES + 1]).is_err());
    assert!(ImageAsset::from_rgba(4001, 1000, &[]).is_err());
    assert!(ImageAsset::from_rgba(4, 2, &[0; 3]).is_err());
    assert!(ImageAsset::from_png_base64("not base64").is_err());
    let a = asset([0, 0, 0, 0]);
    let mut raw = serde_json::to_value(a).unwrap();
    raw["width"] = 999.into();
    assert!(serde_json::from_value::<ImageAsset>(raw).is_err());
    let mut bytes = vec![];
    {
        let encoder = png::Encoder::new(&mut bytes, 8193, 1);
        encoder.write_header().unwrap();
    }
    assert!(ImageAsset::from_bytes(&bytes).is_err());
}
#[test]
fn normalized_crop_has_matching_svg_png_and_pdf_geometry_and_keeps_original_pixels() {
    let mut rgba = vec![];
    for _ in 0..2 {
        rgba.extend(
            [
                [255, 0, 0, 255],
                [255, 0, 0, 255],
                [0, 0, 255, 255],
                [0, 0, 255, 255],
            ]
            .concat(),
        );
    }
    let a = ImageAsset::from_rgba(4, 2, &rgba).unwrap();
    let mut d = doc();
    let id = d.add_image(a.clone(), [20., 20.]).unwrap();
    d.node_mut(id).unwrap().size = [80., 40.];
    d.update_image(id, None, Some([0.5, 0., 0.5, 1.]), Some(false))
        .unwrap();
    assert_eq!(d.node(id).unwrap().image.as_ref().unwrap().asset, a);
    let p = png(&d, true, vec![]);
    assert_eq!(p.get_pixel(50, 40).0, [0, 0, 255, 255]);
    assert_eq!(p.get_pixel(5, 5).0, [0, 0, 0, 0]);
    let svg = diagcraft::svg::export(&d).unwrap();
    assert!(svg.contains("data:image/png;base64,"));
    assert!(svg.contains("viewBox=\"2 0 2 2\""));
    let pdf = diagcraft::pdf::export(&d, diagcraft::pdf::PageFormat::Drawing).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/Subtype /Image"));
    // Proportional crops must not reveal cropped-out pixels in the letterbox.
    d.update_image(id, None, None, Some(true)).unwrap();
    let p = png(&d, true, vec![]);
    assert_eq!(p.get_pixel(30, 40).0[3], 0);
    assert_eq!(p.get_pixel(60, 40).0, [0, 0, 255, 255]);
    assert_eq!(p.get_pixel(90, 40).0[3], 0);
    d.update_image(id, None, Some([0., 0., 1., 1.]), None)
        .unwrap();
    assert_eq!(png(&d, true, vec![]).get_pixel(30, 40).0, [255, 0, 0, 255]);
}
#[test]
fn png_transparency_is_straight_alpha_and_dpi_is_stored_in_phys_metadata() {
    let mut d = doc();
    let id = d.add_image(asset([200, 100, 50, 128]), [20., 20.]).unwrap();
    d.node_mut(id).unwrap().size = [80., 40.];
    let p = png(&d, true, vec![]);
    let c = p.get_pixel(40, 40).0;
    assert_eq!(c[3], 128);
    for i in 0..3 {
        assert!(
            (i16::from(c[i]) - i16::from([200u8, 100, 50][i])).abs() <= 1,
            "{c:?}"
        );
    }
    assert_eq!(
        png(&d, false, vec![]).get_pixel(5, 5).0,
        [255, 255, 255, 255]
    );
    let bytes = raster::export(
        &d,
        &PngOptions {
            width: 333,
            height: 222,
            dpi: 150.,
            ..Default::default()
        },
    )
    .unwrap();
    let reader = png::Decoder::new(Cursor::new(bytes)).read_info().unwrap();
    assert_eq!([reader.info().width, reader.info().height], [333, 222]);
    let phys = reader.info().pixel_dims.unwrap();
    assert_eq!(phys.xppu, 5906);
    assert_eq!(phys.yppu, 5906);
    assert_eq!(phys.unit, png::Unit::Meter);
}
#[test]
fn image_shape_masks_and_aspect_ratio_are_applied_by_the_shared_renderer() {
    let mut d = doc();
    let id = d.add_image(asset([0, 0, 255, 255]), [20., 20.]).unwrap();
    let n = d.node_mut(id).unwrap();
    n.size = [80., 40.];
    n.kind = ShapeKind::Diamond;
    let p = png(&d, true, vec![]);
    assert_eq!(p.get_pixel(21, 21).0[3], 0);
    assert_eq!(p.get_pixel(60, 40).0, [0, 0, 255, 255]);
    let n = d.node_mut(id).unwrap();
    n.kind = ShapeKind::Rectangle;
    n.size = [80., 80.];
    let p = png(&d, true, vec![]);
    assert_eq!(p.get_pixel(60, 25).0[3], 0);
    assert_eq!(p.get_pixel(60, 50).0, [0, 0, 255, 255]);
}
#[test]
fn selection_export_excludes_overlapping_unselected_objects_and_keeps_full_document_routes() {
    let mut d = doc();
    let id = d.add_image(asset([0, 0, 255, 255]), [20., 20.]).unwrap();
    d.node_mut(id).unwrap().size = [80., 40.];
    let other = d.add_node(ShapeKind::Rectangle, [60., 40.]);
    let n = d.node_mut(other).unwrap();
    n.position = [20., 20.];
    n.size = [80., 40.];
    n.text.clear();
    n.fill = [255, 0, 0];
    assert_eq!(png(&d, true, vec![]).get_pixel(60, 40).0, [255, 0, 0, 255]);
    let selected = png(&d, true, vec![id]);
    assert_eq!(selected.get_pixel(100, 50).0, [0, 0, 255, 255]);
    let before = d.clone();
    raster::export(
        &d,
        &PngOptions {
            selection: vec![id],
            ..Default::default()
        },
    )
    .unwrap();
    assert_eq!(d, before);
}
#[test]
fn images_are_portable_on_multiple_pages_clipboard_undo_and_saved_documents() {
    let mut d = doc();
    let id = d.add_image(asset([20, 200, 30, 255]), [20., 20.]).unwrap();
    let before = d.clone();
    let mut history = History::new(&d);
    d.update_image(id, None, Some([0.25, 0., 0.5, 1.]), None)
        .unwrap();
    history.record(&d);
    assert_eq!(history.undo().unwrap(), before);
    let fragment = d.selection(&[id]);
    let pasted = d.paste(&fragment, [30., 30.]).unwrap();
    assert_eq!(d.node(pasted[0]).unwrap().image, d.node(id).unwrap().image);
    d.add_page(true).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("portable.draw");
    d.save(&path).unwrap();
    let loaded = Document::load(&path).unwrap();
    assert_eq!(loaded, d);
    assert_eq!(
        serde_json::to_vec(&loaded).unwrap(),
        serde_json::to_vec(&d).unwrap()
    );
    let pdf = diagcraft::pdf::export(&loaded, diagcraft::pdf::PageFormat::Drawing).unwrap();
    assert!(String::from_utf8_lossy(&pdf).contains("/Count 2"));
}
#[test]
fn hidden_and_locked_images_follow_existing_organization_rules() {
    let mut d = doc();
    let id = d.add_image(asset([0, 0, 255, 255]), [20., 20.]).unwrap();
    d.set_objects_locked(&[id], true).unwrap();
    let before = d.clone();
    assert!(
        d.update_image(id, None, Some([0.5, 0., 0.5, 1.]), None)
            .is_err()
    );
    assert_eq!(d, before);
    let layer = d.create_layer("Hidden".into()).unwrap();
    d.set_objects_locked(&[id], false).unwrap();
    d.move_objects_to_layer(&[id], layer).unwrap();
    d.update_layer(layer, None, Some(false), None).unwrap();
    assert!(png(&d, true, vec![]).pixels().all(|p| p.0[3] == 0));
    assert!(
        raster::export(
            &d,
            &PngOptions {
                selection: vec![id],
                ..Default::default()
            }
        )
        .is_err()
    );
}
#[test]
fn image_resource_budget_and_bad_crops_are_atomic() {
    let a = ImageAsset::from_rgba(1000, 1000, &vec![0; 4_000_000]).unwrap();
    let mut d = doc();
    for _ in 0..16 {
        d.add_image(a.clone(), [0., 0.]).unwrap();
    }
    let before = d.clone();
    assert!(d.add_image(a, [0., 0.]).is_err());
    assert_eq!(d, before);
    let id = d.nodes[0].id;
    for crop in [[0., 0., 0., 1.], [0.8, 0., 0.5, 1.], [f32::NAN, 0., 1., 1.]] {
        assert!(d.update_image(id, None, Some(crop), None).is_err());
        assert_eq!(d, before);
    }
}
#[test]
fn agents_import_and_crop_images_atomically_without_dumping_payloads_in_inspection() {
    let d = doc();
    let a = asset([0, 30, 220, 255]);
    let patch = serde_json::json!({"api_version":1,"expected_revision":diagcraft::agent::revision(&d),"operations":[{"op":"add_image","page":0,"alias":"photo","position":[20,20],"size":[80,40],"png_base64":a.png_base64(),"alt":"Original"},{"op":"update_image","page":0,"target":"photo","crop":[0.25,0,0.5,1],"keep_aspect":true}]});
    let patch = diagcraft::agent::parse_patch(&patch.to_string()).unwrap();
    let result = diagcraft::agent::preview(&d, &patch).unwrap();
    let inspection = diagcraft::agent::inspect(&result.document).unwrap();
    let n = &inspection["pages"][0]["nodes"][0];
    assert_eq!(n["object_type"], "image");
    assert!(n["image"]["asset"]["png"].is_null());
    assert_eq!(n["image"]["asset"]["sha256"], a.sha256());
    assert!(d.nodes.is_empty());
    let mut bad = serde_json::to_value(patch).unwrap();
    bad["operations"][1]["crop"] = serde_json::json!([0, 0, 2, 1]);
    assert!(
        diagcraft::agent::preview(
            &d,
            &diagcraft::agent::parse_patch(&bad.to_string()).unwrap()
        )
        .is_err()
    );
    assert!(d.nodes.is_empty());
}
#[test]
fn png_limits_and_missing_scopes_fail_without_modifying_documents() {
    let d = doc();
    for settings in [
        PngOptions {
            width: 0,
            ..Default::default()
        },
        PngOptions {
            width: 8192,
            height: 8192,
            ..Default::default()
        },
        PngOptions {
            dpi: f32::NAN,
            ..Default::default()
        },
        PngOptions {
            page: 100,
            ..Default::default()
        },
        PngOptions {
            selection: vec![99],
            ..Default::default()
        },
    ] {
        assert!(raster::export(&d, &settings).is_err());
    }
    assert!(d.nodes.is_empty());
}
#[test]
fn older_documents_omit_the_optional_image_field_and_keep_their_revision() {
    let d = Document::demo();
    assert!(!serde_json::to_string(&d).unwrap().contains("\"image\""));
    let before = diagcraft::agent::revision(&d);
    let loaded: Document = serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
    assert_eq!(before, diagcraft::agent::revision(&loaded));
    let image = ImageContent::new(asset([0, 0, 0, 0]));
    image.validate().unwrap();
}

#[test]
fn document_image_budget_is_checked_before_decompressing_embedded_payloads() {
    use base64::Engine;
    let mut header = b"\x89PNG\r\n\x1a\n".to_vec();
    header.extend(13u32.to_be_bytes());
    header.extend(b"IHDR");
    header.extend(1000u32.to_be_bytes());
    header.extend(1000u32.to_be_bytes());
    header.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    let raw = serde_json::json!({"png":base64::engine::general_purpose::STANDARD.encode(header),"width":1000,"height":1000});
    let asset: ImageAsset = serde_json::from_value(raw).unwrap();
    let mut d = doc();
    let id = d.add_node(ShapeKind::Rectangle, [0., 0.]);
    d.node_mut(id).unwrap().image = Some(ImageContent::new(asset));
    let template = d.nodes[0].clone();
    d.nodes = (1..=17)
        .map(|id| {
            let mut n = template.clone();
            n.id = id;
            n
        })
        .collect();
    let loaded: Document = serde_json::from_slice(&serde_json::to_vec(&d).unwrap()).unwrap();
    assert!(loaded.validate().unwrap_err().starts_with("Trop d’images"));
    d.nodes.truncate(1);
    assert!(d.validate().unwrap_err().contains("Image illisible"));
}

#[test]
fn selection_of_a_connector_keeps_the_route_around_unselected_obstacles() {
    use diagcraft::document::{ConnectorKind, Port};
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [180., 300.]);
    let b = d.add_node(ShapeKind::Rectangle, [1100., 300.]);
    let wall = d.add_node(ShapeKind::Rectangle, [620., 300.]);
    d.node_mut(wall).unwrap().size = [250., 300.];
    d.node_mut(wall).unwrap().position = [450., 150.];
    let edge = d.connect(a, b, ConnectorKind::Elbow).unwrap();
    d.connectors[0].from_port = Port::Right;
    d.connectors[0].to_port = Port::Left;
    let route = d.route(&d.connectors[0]);
    let bounds = raster::region(&d, &[edge]).unwrap();
    let bytes = raster::export(
        &d,
        &PngOptions {
            width: 1000,
            height: 600,
            transparent: true,
            selection: vec![edge],
            ..Default::default()
        },
    )
    .unwrap();
    let p = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let segment = route
        .windows(2)
        .find(|s| s[0][1] == s[1][1] && s[0][1] < 150. && (s[1][0] - s[0][0]).abs() > 100.)
        .unwrap();
    let mid = [(segment[0][0] + segment[1][0]) / 2., segment[0][1]];
    let x = ((mid[0] - bounds[0]) / bounds[2] * 1000.) as u32;
    let y = ((mid[1] - bounds[1]) / bounds[3] * 600.) as u32;
    assert!(
        (x.saturating_sub(2)..=x + 2)
            .any(|x| (y.saturating_sub(2)..=y + 2).any(|y| p.get_pixel(x, y).0[3] > 50))
    );
}

#[test]
fn retina_inputs_are_downsampled_with_alpha_aware_filtering_without_changing_input() {
    let bytes = vec![128u8; 2500 * 1800 * 4];
    let before = bytes.clone();
    let asset = ImageAsset::from_rgba(2500, 1800, &bytes).unwrap();
    assert!(u64::from(asset.width()) * u64::from(asset.height()) <= diagcraft::images::MAX_PIXELS);
    assert_eq!(bytes, before);
    assert!(asset.width() < 2500);
}
