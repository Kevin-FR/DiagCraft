use super::*;
use diagcraft::images::ImageAsset;
fn asset() -> ImageAsset {
    ImageAsset::from_rgba(4, 2, &[[30, 120, 230, 255]; 8].concat()).unwrap()
}
fn setup() -> (egui::Context, DrawApp) {
    let ctx = egui::Context::default();
    let mut app = DrawApp::from_context(&ctx, Document::default(), None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.show_library = false;
    app.show_quick_toolbar = false;
    (ctx, app)
}
fn frame(ctx: &egui::Context, app: &mut DrawApp, events: Vec<egui::Event>) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 760.))),
            events,
            ..Default::default()
        },
        |ctx| {
            let before = app.doc.clone();
            app.inspector(ctx);
            app.canvas(ctx, !app.image_ui.png_open);
            app.finish_frame_edit(ctx, &before);
            app.png_dialog(ctx);
        },
    )
}
#[test]
fn inserting_images_is_one_undo_and_bad_drop_batches_leave_the_document_unchanged() {
    let (_, mut app) = setup();
    let before = app.doc.clone();
    let ids = app
        .insert_image_assets(
            vec![(asset(), "First".into()), (asset(), "Second".into())],
            Some([300., 250.]),
        )
        .unwrap();
    assert_eq!(app.selected, ids);
    assert_eq!(app.doc.nodes.len(), 2);
    app.undo();
    assert_eq!(app.doc, before);
    let png = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 0, 255, 255]));
    let mut data = std::io::Cursor::new(vec![]);
    image::DynamicImage::ImageRgba8(png)
        .write_to(&mut data, image::ImageFormat::Png)
        .unwrap();
    app.drop_images(
        vec![
            egui::DroppedFile {
                name: "good.png".into(),
                bytes: Some(data.into_inner().into()),
                ..Default::default()
            },
            egui::DroppedFile {
                name: "bad.png".into(),
                bytes: Some(vec![0, 1, 2].into()),
                ..Default::default()
            },
        ],
        None,
    );
    assert_eq!(app.doc, before);
    assert!(app.error.is_some());
}
#[test]
fn opening_the_crop_inspector_does_not_round_or_modify_stored_coordinates() {
    let (ctx, mut app) = setup();
    let id = app
        .insert_image_assets(vec![(asset(), "Description".into())], Some([300., 250.]))
        .unwrap()[0];
    app.doc
        .update_image(id, None, Some([0.1, 0.13, 0.71, 0.67]), None)
        .unwrap();
    app.commit();
    let before = app.doc.clone();
    for _ in 0..3 {
        frame(&ctx, &mut app, vec![]);
    }
    assert_eq!(app.doc, before);
}
#[test]
fn image_resize_keeps_aspect_and_undo_restores_the_original() {
    let (ctx, mut app) = setup();
    let id = app
        .insert_image_assets(vec![(asset(), String::new())], Some([300., 250.]))
        .unwrap()[0];
    app.doc.node_mut(id).unwrap().size = [240., 120.];
    app.commit();
    frame(&ctx, &mut app, vec![]);
    let before = app.doc.clone();
    let node = app.doc.node(id).unwrap();
    let corner = app.screen([
        node.position[0] + node.size[0],
        node.position[1] + node.size[1],
    ]);
    for (p, pressed) in [(corner, true), (corner + vec2(60., 10.), false)] {
        frame(
            &ctx,
            &mut app,
            vec![
                egui::Event::PointerMoved(p),
                egui::Event::PointerButton {
                    pos: p,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
    }
    let n = app.doc.node(id).unwrap();
    assert!((n.size[0] / n.size[1] - 2.).abs() < 0.001);
    assert_ne!(app.doc, before);
    app.undo();
    assert_eq!(app.doc, before);
}
#[test]
fn png_dialog_is_view_only_blocks_commands_and_fits_small_screens() {
    let (ctx, mut app) = setup();
    app.insert_image_assets(vec![(asset(), String::new())], Some([300., 250.]))
        .unwrap();
    let before = app.doc.clone();
    app.open_png_export();
    assert!(app.command_dialog_blocked());
    for _ in 0..3 {
        let output = frame(&ctx, &mut app, vec![]);
        for clipped in output.shapes {
            if let egui::Shape::Text(text) = clipped.shape
                && (text.galley.text() == i18n::t("Exporter en PNG")
                    || text.galley.text() == i18n::t("Exporter…"))
            {
                assert!(
                    text.pos.x >= 0.
                        && text.pos.y >= 0.
                        && text.pos.y + text.galley.size().y <= 760.
                );
            }
        }
    }
    assert_eq!(app.doc, before);
    assert!(!app.paste_image());
}
#[test]
fn image_description_is_searchable_and_png_selection_uses_the_current_page() {
    let (_ctx, mut app) = setup();
    let id = app
        .insert_image_assets(vec![(asset(), "Carte de test".into())], Some([300., 250.]))
        .unwrap()[0];
    app.open_object_search();
    app.object_search.query = "carte".into();
    assert!(!app.object_search.matches().is_empty());
    app.object_search.open = false;
    app.doc.add_page(true).unwrap();
    app.selected = vec![id];
    app.open_png_export();
    assert_eq!(app.image_ui.options.page, 1);
    assert_eq!(
        app.doc.node(id).unwrap().image.as_ref().unwrap().alt,
        "Carte de test"
    );
}

#[test]
fn oversized_png_defaults_remain_explicit_and_fit_preserves_their_ratio() {
    let (_, mut app) = setup();
    app.doc.page_size = [100_000., 10_000.];
    app.open_png_export();
    assert!(app.image_ui.options.validate().is_err());
    app.fit_png_size();
    app.image_ui.options.validate().unwrap();
    assert!(
        (app.image_ui.options.width as f32 / app.image_ui.options.height as f32 - 10.).abs() < 0.02
    );
}
