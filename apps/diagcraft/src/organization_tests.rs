use super::*;

fn organization_setup() -> (egui::Context, DrawApp, [u64; 5]) {
    let ctx = egui::Context::default();
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rounded, [250., 180.]);
    let b = doc.add_node(ShapeKind::Rectangle, [650., 180.]);
    doc.node_mut(a).unwrap().text = "HIDDEN-SOURCE".into();
    doc.node_mut(b).unwrap().text = "VISIBLE-DESTINATION".into();
    let edge = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    let path = doc.add_drawing(DrawingKind::Line, [200., 500.], [350., 580.]);
    let chart = doc.add_chart(ChartKind::Columns, [850., 500.]);
    let mut app = DrawApp::from_context(&ctx, doc, None, None);
    app.fit_next = false;
    app.zoom = 1.;
    app.pan = Vec2::ZERO;
    app.rulers = false;
    app.show_library = false;
    app.show_quick_toolbar = false;
    (ctx, app, [a, b, edge, path, chart])
}

fn organization_frame(ctx: &egui::Context, app: &mut DrawApp) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280., 800.))),
            ..Default::default()
        },
        |ctx| {
            let before = app.doc.clone();
            app.canvas(ctx, app.rich_editor.is_none());
            app.finish_frame_edit(ctx, &before);
        },
    )
}

#[test]
fn groups_expand_a_single_member_and_move_as_one_undoable_selection() {
    let (_, mut app, ids) = organization_setup();
    let plain = app.doc.clone();
    app.selected = ids.to_vec();
    app.group_selection();
    assert_eq!(app.doc.organization.groups.len(), 1);
    let grouped = app.doc.clone();
    app.selected = vec![ids[0]];
    app.normalize_selection();
    assert_eq!(app.selected.len(), ids.len());
    app.nudge_selection([10., 1.], false);
    let moved = app.doc.clone();
    for id in [ids[0], ids[1]] {
        let old = grouped.node(id).unwrap().position;
        assert_eq!(
            moved.node(id).unwrap().position,
            [old[0] + 10., old[1] + 1.]
        );
    }
    assert_eq!(moved.drawings[0].start, [210., 501.]);
    assert_eq!(
        moved.charts[0].position,
        [
            grouped.charts[0].position[0] + 10.,
            grouped.charts[0].position[1] + 1.
        ]
    );
    app.undo();
    assert_eq!(app.doc, grouped);
    app.undo();
    assert_eq!(app.doc, plain);
    assert!(!app.history.can_undo());
    app.redo();
    app.redo();
    assert_eq!(app.doc, moved);
}

#[test]
fn locking_a_group_blocks_motion_deletion_styles_and_direct_text_editing() {
    let (_, mut app, ids) = organization_setup();
    app.selected = ids.to_vec();
    app.group_selection();
    app.selected = vec![ids[0]];
    app.set_selection_locked(true);
    assert!(ids.iter().all(|id| app.doc.is_object_locked(*id)));
    let locked = app.doc.clone();
    assert!(!app.selection_editable());
    app.nudge_selection([10., 0.], true);
    assert!(app.keyboard_move.is_none());
    app.delete();
    assert_eq!(app.doc, locked);
    app.font_step(2.);
    app.commit();
    assert_eq!(app.doc, locked);
    app.change_text(|style| style.bold = true);
    assert_eq!(app.doc, locked);
    app.apply_theme(2, false);
    app.commit();
    assert_eq!(app.doc, locked);
    app.open_node_text(ids[0]);
    assert!(app.rich_editor.is_none());
    assert_eq!(app.doc, locked);
    app.set_selection_locked(false);
    assert!(app.selection_editable());
    app.nudge_selection([1., 0.], false);
    assert_ne!(app.doc, locked);
}

#[test]
fn layer_visibility_clears_hidden_selection_connections_and_drawing_target() {
    let (ctx, mut app, ids) = organization_setup();
    let layer = app.doc.create_layer("Reference".into()).unwrap();
    app.doc.move_objects_to_layer(&[ids[0]], layer).unwrap();
    app.commit();
    let visible = app.doc.clone();
    app.selected = vec![ids[0], ids[2]];
    app.connect_from = Some(ids[0]);
    app.hover_link_node = Some(ids[0]);
    app.drawing_layer = layer;
    app.set_layer_visible(layer, false);
    assert!(app.selected.is_empty());
    assert!(app.connect_from.is_none());
    assert!(app.hover_link_node.is_none());
    assert_eq!(app.drawing_layer, 0);
    assert!(
        app.doc
            .hit_node(app.doc.node(ids[0]).unwrap().center())
            .is_none()
    );
    let hits = app.marquee_hits([0., 0.], app.doc.page_size);
    assert!(!hits.contains(&ids[0]));
    assert!(!hits.contains(&ids[2]));
    assert!(hits.contains(&ids[1]));
    let output = organization_frame(&ctx, &mut app);
    assert!(!output.shapes.iter().any(|shape| matches!(&shape.shape,
        Shape::Text(text) if text.galley.job.text.contains("HIDDEN-SOURCE"))));
    app.open_object_search();
    assert!(app.object_search.open);
    assert!(
        app.object_search
            .entries
            .iter()
            .all(|entry| app.doc.is_object_visible(entry.id))
    );
    app.object_search = Default::default();
    app.undo();
    assert_eq!(app.doc, visible);
    assert!(app.doc.is_object_visible(ids[0]));
    assert!(app.doc.is_object_visible(ids[2]));
}

#[test]
fn a_group_with_a_hidden_attached_link_cannot_be_partially_edited() {
    let (_, mut app, ids) = organization_setup();
    app.doc.group_objects(&[ids[0], ids[2]]).unwrap();
    let layer = app.doc.create_layer("Endpoint".into()).unwrap();
    app.doc.move_objects_to_layer(&[ids[1]], layer).unwrap();
    app.commit();
    app.set_layer_visible(layer, false);
    app.selected = vec![ids[0]];
    app.normalize_selection();
    assert!(app.doc.is_object_visible(ids[0]));
    assert!(!app.doc.is_object_visible(ids[2]));
    assert!(!app.selection_editable());
    let before = app.doc.clone();
    app.nudge_selection([10., 0.], true);
    assert!(app.keyboard_move.is_none());
    app.open_node_text(ids[0]);
    assert!(app.rich_editor.is_none());
    assert_eq!(app.doc, before);
}

#[test]
fn commit_rejects_accidental_raw_writes_to_locked_content_without_an_undo_step() {
    let (_, mut app, ids) = organization_setup();
    app.selected = vec![ids[0]];
    app.set_selection_locked(true);
    let locked = app.doc.clone();
    app.doc.node_mut(ids[0]).unwrap().text = "Unprotected write".into();
    app.commit();
    assert_eq!(app.doc, locked);
    app.undo();
    assert!(!app.doc.is_object_locked(ids[0]));
    assert!(!app.history.can_undo());
}

#[test]
fn stale_auto_fit_on_a_protected_import_does_not_discard_an_unrelated_edit() {
    let (_, mut app, ids) = organization_setup();
    let node = app.doc.node_mut(ids[0]).unwrap();
    node.text = "A long imported title that needs more than this tiny frame".into();
    node.size = [24., 24.];
    node.text_style.auto_fit = true;
    app.doc.set_objects_locked(&[ids[0]], true).unwrap();
    // Simulate a valid imported document, whose old auto-fit size is retained.
    app.history = History::new(&app.doc);
    app.saved = app.doc.clone();
    let protected = app.doc.node(ids[0]).unwrap().clone();
    app.doc.node_mut(ids[1]).unwrap().text = "Editable content".into();
    app.commit();
    assert_eq!(app.doc.node(ids[0]).unwrap(), &protected);
    assert_eq!(app.doc.node(ids[1]).unwrap().text, "Editable content");
    assert!(app.history.can_undo());
}

#[test]
fn drawing_layer_and_layer_name_drafts_do_not_leak_between_pages() {
    let (_, mut app, ids) = organization_setup();
    let layer = app.doc.create_layer("First".into()).unwrap();
    app.doc.move_objects_to_layer(&[ids[0]], layer).unwrap();
    app.doc.add_page(true).unwrap();
    app.doc
        .update_layer(layer, Some("Second".into()), None, None)
        .unwrap();
    app.doc.switch_page(0).unwrap();
    app.history = History::new(&app.doc);
    app.saved = app.doc.clone();
    app.normalize_selection();
    app.drawing_layer = layer;
    app.layer_names
        .insert(layer, "Unsaved first-page name".into());
    app.switch_page(1);
    app.normalize_selection();
    assert_eq!(app.drawing_layer, 0);
    assert!(!app.layer_names.contains_key(&layer));
    assert_eq!(app.doc.organization.layers[0].name, "Second");
}

#[test]
fn layer_dialog_keeps_its_close_button_visible_on_small_screens_in_all_languages() {
    for language in Language::ALL {
        for size in [vec2(1200., 760.), vec2(1440., 900.)] {
            let (ctx, mut app, ids) = organization_setup();
            for index in 0..8 {
                app.doc
                    .create_layer(format!("Reference layer {index}"))
                    .unwrap();
            }
            app.selected = vec![ids[0]];
            app.commit();
            i18n::activate(language);
            app.open_layers();
            assert!(app.command_dialog_blocked());
            let before = app.doc.clone();
            let screen = Rect::from_min_size(Pos2::ZERO, size);
            let mut output = None;
            for _ in 0..4 {
                output = Some(ctx.run(
                    egui::RawInput {
                        screen_rect: Some(screen),
                        ..Default::default()
                    },
                    |ctx| {
                        app.canvas(ctx, true);
                        app.layers_dialog(ctx);
                    },
                ));
            }
            let close = t("Fermer");
            let heading = t("Calques");
            let text_rects = output
                .unwrap()
                .shapes
                .into_iter()
                .filter_map(|shape| {
                    if let Shape::Text(text) = shape.shape {
                        (text.galley.job.text == close || text.galley.job.text == heading)
                            .then(|| Rect::from_min_size(text.pos, text.galley.size()))
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>();
            assert!(
                text_rects.len() >= 2,
                "missing modal title or close button: {language:?}"
            );
            assert!(
                text_rects
                    .iter()
                    .all(|rect| screen.expand(1.).contains_rect(*rect)),
                "modal controls outside {size:?}: {language:?}, {text_rects:?}"
            );
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    events: vec![egui::Event::Key {
                        key: Key::Delete,
                        physical_key: Some(Key::Delete),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ctx| {
                    app.shortcuts(ctx);
                    app.canvas(ctx, false);
                    app.layers_dialog(ctx);
                },
            );
            assert_eq!(app.doc, before);
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn validated_agent_unlock_and_edit_is_applied_as_a_single_undo_step() {
    let (_, mut app, ids) = organization_setup();
    app.selected = vec![ids[0]];
    app.set_selection_locked(true);
    let before = app.doc.clone();
    let patch = serde_json::json!({
        "api_version":1,"expected_revision":diagcraft::agent::revision(&app.doc),
        "operations":[
            {"op":"set_objects_locked","page":0,"targets":[ids[0]],"locked":false},
            {"op":"update_node","page":0,"target":ids[0],"changes":{"text":"Approved agent content"}}
        ]
    });
    app.apply_agent_patch(&patch.to_string()).unwrap();
    assert_eq!(app.doc.node(ids[0]).unwrap().text, "Approved agent content");
    assert!(!app.doc.is_object_locked(ids[0]));
    let edited = app.doc.clone();
    app.undo();
    assert_eq!(app.doc, before);
    app.redo();
    assert_eq!(app.doc, edited);
}

#[test]
fn alignment_treats_a_group_as_one_unit_and_preserves_internal_offsets() {
    let (_, mut app, ids) = organization_setup();
    app.doc.group_objects(&ids[..2]).unwrap();
    app.commit();
    app.selected = vec![ids[0], ids[4]];
    app.normalize_selection();
    let before = app.doc.clone();
    app.align_selected(0, 0.);
    let a = app.doc.node(ids[0]).unwrap().position;
    let b = app.doc.node(ids[1]).unwrap().position;
    let old_a = before.node(ids[0]).unwrap().position;
    let old_b = before.node(ids[1]).unwrap().position;
    assert_eq!(
        [b[0] - a[0], b[1] - a[1]],
        [old_b[0] - old_a[0], old_b[1] - old_a[1]]
    );
    assert_eq!(app.doc.charts[0].position[0], a[0].min(b[0]));
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn duplicate_of_a_locked_group_reuses_its_layer_without_unlocking_the_original() {
    let (_, mut app, ids) = organization_setup();
    let layer = app.doc.create_layer("Working layer".into()).unwrap();
    app.doc.move_objects_to_layer(&ids, layer).unwrap();
    app.doc.group_objects(&ids).unwrap();
    app.commit();
    app.selected = vec![ids[0]];
    app.set_selection_locked(true);
    let before = app.doc.clone();
    app.duplicate();
    assert_eq!(app.doc.organization.layers, before.organization.layers);
    assert_eq!(app.doc.organization.groups.len(), 2);
    assert!(ids.iter().all(|id| app.doc.is_object_locked(*id)));
    assert!(app.selected.iter().all(|id| !ids.contains(id)
        && app.doc.object_layer(*id) == layer
        && app.doc.can_edit_object(*id)));
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn distribution_moves_the_middle_group_without_changing_its_internal_geometry() {
    let (_, mut app, ids) = organization_setup();
    for (id, position) in [(ids[0], [500., 100.]), (ids[1], [800., 120.])] {
        let node = app.doc.node_mut(id).unwrap();
        node.position = position;
        node.size = [100., 60.];
    }
    app.doc.charts[0].position = [0., 400.];
    app.doc.charts[0].size = [120., 120.];
    let end = app.doc.add_node(ShapeKind::Rectangle, [1050., 300.]);
    app.doc.node_mut(end).unwrap().position = [1000., 270.];
    app.doc.node_mut(end).unwrap().size = [100., 60.];
    app.doc.group_objects(&ids[..2]).unwrap();
    app.commit();
    app.selected = vec![ids[0], ids[4], end];
    app.normalize_selection();
    let before = app.doc.clone();
    app.distribute(false);
    assert_eq!(app.doc.node(ids[0]).unwrap().position, [360., 100.]);
    assert_eq!(app.doc.node(ids[1]).unwrap().position, [660., 120.]);
    assert_eq!(app.doc.charts[0].position, before.charts[0].position);
    assert_eq!(
        app.doc.node(end).unwrap().position,
        before.node(end).unwrap().position
    );
    app.undo();
    assert_eq!(app.doc, before);
}

#[test]
fn double_clicking_a_chart_does_not_edit_an_occluded_node_in_the_same_group() {
    let (ctx, mut app, ids) = organization_setup();
    app.doc.charts[0].position = [100., 100.];
    app.doc.group_objects(&[ids[0], ids[4]]).unwrap();
    app.commit();
    let before = app.doc.clone();
    let _ = organization_frame(&ctx, &mut app);
    let point = app.screen(app.doc.node(ids[0]).unwrap().center());
    assert_eq!(app.organized_hit(app.world(point)), Some(ids[4]));
    for pressed in [true, false, true, false] {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280., 800.))),
                events: vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed,
                        modifiers: egui::Modifiers::NONE,
                    },
                ],
                ..Default::default()
            },
            |ctx| {
                let before = app.doc.clone();
                app.canvas(ctx, true);
                app.finish_frame_edit(ctx, &before);
            },
        );
    }
    assert!(app.selected.contains(&ids[0]) && app.selected.contains(&ids[4]));
    assert!(
        app.rich_editor.is_none(),
        "the top chart must occlude the node's text editor"
    );
    assert_eq!(app.doc, before);
}

#[test]
fn alignment_buttons_fit_a_narrow_inspector_in_every_language() {
    for language in Language::ALL {
        let (ctx, mut app, ids) = organization_setup();
        app.selected = vec![ids[0], ids[1], ids[4]];
        i18n::activate(language);
        let screen = Rect::from_min_size(Pos2::ZERO, vec2(280., 800.));
        let mut output = None;
        for _ in 0..3 {
            output = Some(ctx.run(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| app.alignment_controls(ui));
                },
            ));
        }
        for shape in output.unwrap().shapes {
            if let Shape::Text(text) = shape.shape {
                let rect = Rect::from_min_size(text.pos, text.galley.size());
                assert!(
                    screen.expand(1.).contains_rect(rect),
                    "{language:?}: {rect:?}"
                );
            }
        }
    }
    i18n::activate(Language::French);
}

#[test]
fn duplicate_on_base_layer_ignores_the_current_drawing_target() {
    let (_, mut app, ids) = organization_setup();
    let layer = app.doc.create_layer("New drawings".into()).unwrap();
    app.doc.group_objects(&ids).unwrap();
    app.commit();
    app.drawing_layer = layer;
    app.selected = vec![ids[0]];
    let before = app.doc.clone();
    app.duplicate();
    assert_ne!(app.doc, before);
    assert_eq!(app.doc.organization.layers, before.organization.layers);
    assert_eq!(app.drawing_layer, layer);
    assert!(
        app.selected
            .iter()
            .all(|id| !ids.contains(id) && app.doc.object_layer(*id) == 0)
    );
    app.undo();
    assert_eq!(app.doc, before);
}
