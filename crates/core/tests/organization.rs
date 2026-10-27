use diagcraft::{
    agent,
    chart::ChartKind,
    document::{ConnectorKind, Document, DrawingKind, FORMAT_VERSION, ShapeKind},
    history::History,
    svg,
};
use serde_json::{Value, json};

fn mixed_page() -> (Document, [u64; 5]) {
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rounded, [250., 200.]);
    let b = doc.add_node(ShapeKind::Rectangle, [650., 200.]);
    doc.node_mut(a).unwrap().text = "SOURCE-ONLY".into();
    doc.node_mut(b).unwrap().text = "DESTINATION-ONLY".into();
    let edge = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
    doc.connectors[0].label = "CONNECTED-LINK".into();
    let path = doc.add_drawing(DrawingKind::Bezier, [100., 500.], [350., 580.]);
    let chart = doc.add_chart(ChartKind::Columns, [850., 500.]);
    doc.charts[0].title = "VISIBLE-CHART".into();
    doc.validate().unwrap();
    (doc, [a, b, edge, path, chart])
}

fn patch(doc: &Document, operations: Value) -> agent::Patch {
    agent::parse_patch(
        &json!({"api_version":1,"expected_revision":agent::revision(doc),"operations":operations})
            .to_string(),
    )
    .unwrap()
}

#[test]
fn legacy_files_keep_empty_metadata_and_organization_changes_affect_revision_and_history() {
    let (mut doc, ids) = mixed_page();
    let plain = serde_json::to_value(&doc).unwrap();
    assert!(plain.get("organization").is_none());
    let roundtrip: Document = serde_json::from_value(plain).unwrap();
    assert_eq!(roundtrip, doc);
    assert_eq!(roundtrip.format_version, FORMAT_VERSION);
    let original = doc.clone();
    let old_revision = agent::revision(&doc);
    let mut history = History::new(&doc);
    doc.group_objects(&ids).unwrap();
    history.record(&doc);
    assert_ne!(agent::revision(&doc), old_revision);
    assert_ne!(doc, original);
    let grouped = doc.clone();
    assert_eq!(history.undo().unwrap(), original);
    assert_eq!(history.redo().unwrap(), grouped);
}

#[test]
fn groups_layers_and_locks_survive_canonical_multipage_roundtrip_and_page_reordering() {
    let (mut doc, ids) = mixed_page();
    let layer = doc.create_layer("Premier calque".into()).unwrap();
    doc.move_objects_to_layer(&ids, layer).unwrap();
    doc.group_objects(&ids).unwrap();
    doc.set_objects_locked(&[ids[0]], true).unwrap();
    let first = doc.page(0).unwrap();
    doc.add_page(true).unwrap();
    doc.update_layer(layer, Some("Deuxième calque".into()), Some(false), None)
        .unwrap();
    let second = doc.page(1).unwrap();
    assert_ne!(first.organization, second.organization);
    let canonical = serde_json::to_vec(&doc).unwrap();
    let revision = agent::revision(&doc);
    doc.switch_page(0).unwrap();
    assert_eq!(serde_json::to_vec(&doc).unwrap(), canonical);
    assert_eq!(agent::revision(&doc), revision);
    assert_eq!(doc.page(0).unwrap(), first);
    assert_eq!(doc.page(1).unwrap(), second);
    let mut restored: Document = serde_json::from_slice(&canonical).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored, doc);
    restored.move_page(1, 0).unwrap();
    assert_eq!(restored.page(0).unwrap(), second);
    assert_eq!(restored.page(1).unwrap(), first);
    assert_eq!(restored.nodes[0].id, ids[0]);
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("organization.draw");
    restored.save(&file).unwrap();
    assert_eq!(Document::load(&file).unwrap(), restored);
}

#[test]
fn mixed_groups_expand_move_to_a_layer_and_ungroup_without_changing_content() {
    let (mut doc, ids) = mixed_page();
    let before = doc.clone();
    let group = doc.group_objects(&ids).unwrap();
    assert!(group > 0);
    let mut expanded = doc.expand_groups(&[ids[0]]);
    expanded.sort_unstable();
    let mut expected = ids.to_vec();
    expected.sort_unstable();
    assert_eq!(expanded, expected);
    let layer = doc.create_layer("Ensemble".into()).unwrap();
    doc.move_objects_to_layer(&[ids[0]], layer).unwrap();
    assert!(ids.iter().all(|id| doc.object_layer(*id) == layer));
    doc.ungroup_objects(&[ids[0]]).unwrap();
    assert!(doc.organization.groups.is_empty());
    assert_eq!(doc.nodes, before.nodes);
    assert_eq!(doc.connectors, before.connectors);
    assert_eq!(doc.charts, before.charts);
    assert_eq!(doc.drawings, before.drawings);
    doc.validate().unwrap();
}

#[test]
fn malformed_metadata_is_rejected_including_inactive_pages_and_resource_limits() {
    let (mut doc, ids) = mixed_page();
    let layer = doc.create_layer("Calque".into()).unwrap();
    doc.group_objects(&ids[..2]).unwrap();
    let value = serde_json::to_value(&doc).unwrap();
    let group = &value["organization"]["groups"][0];
    let cases = [
        json!({"layers":[{"id":0,"name":"Reserved","visible":true,"locked":false}]}),
        json!({"layers":[{"id":layer,"name":"A"},{"id":layer,"name":"B"}]}),
        json!({"objects":{"999999":{"layer":0,"locked":true}}}),
        json!({"objects":{ids[0].to_string():{"layer":999999}}}),
        json!({"groups":[{"id":0,"members":[ids[0],ids[1]]}]}),
        json!({"groups":[{"id":1,"members":[ids[0]]}]}),
        json!({"groups":[{"id":1,"members":[ids[0],ids[0]]}]}),
        json!({"groups":[{"id":1,"members":[ids[0],999999]}]}),
        json!({"groups":[group,group]}),
        json!({"groups":[{"id":1,"members":[ids[0],ids[1]]},{"id":2,"members":[ids[1],ids[2]]}]}),
        json!({"layers":[{"id":1,"name":"Different"}],"objects":{ids[1].to_string():{"layer":1}},"groups":[{"id":1,"members":[ids[0],ids[1]]}]}),
        json!({"layers":[{"id":1,"name":" "}]}),
        json!({"layers":[{"id":1,"name":"x".repeat(801)}]}),
    ];
    for organization in cases {
        let mut invalid = value.clone();
        invalid["organization"] = organization;
        let invalid: Document = serde_json::from_value(invalid).unwrap();
        assert!(
            invalid.validate().is_err(),
            "accepted malformed organization: {:?}",
            invalid.organization
        );
    }
    let mut too_many = value.clone();
    too_many["organization"]["layers"] = Value::Array(
        (1..=100)
            .map(|id| json!({"id":id,"name":format!("Layer {id}")}))
            .collect(),
    );
    let too_many: Document = serde_json::from_value(too_many).unwrap();
    assert!(too_many.validate().is_err());
    doc.add_page(true).unwrap();
    doc.organization.groups[0].members.push(999999);
    doc.switch_page(0).unwrap();
    assert!(
        doc.validate().is_err(),
        "inactive pages must also validate metadata"
    );
}

#[test]
fn failed_organization_operations_are_atomic_and_do_not_dangle_after_delete() {
    let (mut doc, ids) = mixed_page();
    let layer = doc.create_layer("Other".into()).unwrap();
    doc.group_objects(&ids[..2]).unwrap();
    let before = doc.clone();
    assert!(doc.group_objects(&[ids[0], 999999]).is_err());
    assert_eq!(doc, before);
    assert!(doc.move_objects_to_layer(&ids, 999999).is_err());
    assert_eq!(doc, before);
    assert!(doc.set_objects_locked(&[ids[0], 999999], true).is_err());
    assert_eq!(doc, before);
    assert!(
        doc.update_layer(layer, Some(" ".into()), None, None)
            .is_err()
    );
    assert_eq!(doc, before);
    assert!(doc.remove_layer(0).is_err());
    assert_eq!(doc, before);
    doc.remove(&[ids[0]]);
    doc.validate().unwrap();
    assert!(doc.organization.groups.iter().all(|group| {
        group.members.len() >= 2 && group.members.iter().all(|id| doc.object_exists(*id))
    }));
    assert!(
        doc.organization
            .objects
            .keys()
            .all(|id| doc.object_exists(*id))
    );
}

#[test]
fn locked_incident_connector_blocks_node_deletion_without_a_partial_result() {
    let (mut doc, ids) = mixed_page();
    doc.set_objects_locked(&[ids[2]], true).unwrap();
    let before = doc.clone();
    assert!(doc.try_remove(&[ids[0]]).is_err());
    assert_eq!(doc, before);
    doc.remove(&[ids[0]]);
    assert_eq!(doc, before);
    assert!(
        agent::preview(
            &doc,
            &patch(&doc, json!([{"op":"delete","page":0,"target":ids[0]}]))
        )
        .is_err()
    );
    let result = agent::preview(
        &doc,
        &patch(
            &doc,
            json!([
                {"op":"set_objects_locked","page":0,"targets":[ids[2]],"locked":false},
                {"op":"delete","page":0,"target":ids[0]}
            ]),
        ),
    )
    .unwrap()
    .document;
    assert!(!result.object_exists(ids[0]));
    assert!(!result.object_exists(ids[2]));
    assert!(result.object_exists(ids[1]));
    result.validate().unwrap();
}

#[test]
fn explicit_layer_order_controls_svg_stacking_and_removal_preserves_content() {
    let (mut doc, ids) = mixed_page();
    let back = doc.create_layer("Back".into()).unwrap();
    let front = doc.create_layer("Front".into()).unwrap();
    doc.move_objects_to_layer(&[ids[0]], back).unwrap();
    doc.move_objects_to_layer(&[ids[1]], front).unwrap();
    let before_group = doc.clone();
    assert!(doc.group_objects(&ids[..2]).is_err());
    assert_eq!(doc, before_group);
    let first = svg::export(&doc).unwrap();
    assert!(first.find("SOURCE-ONLY").unwrap() < first.find("DESTINATION-ONLY").unwrap());
    doc.move_layer(front, 0).unwrap();
    let second = svg::export(&doc).unwrap();
    assert!(second.find("DESTINATION-ONLY").unwrap() < second.find("SOURCE-ONLY").unwrap());
    let content = (
        doc.nodes.clone(),
        doc.connectors.clone(),
        doc.drawings.clone(),
        doc.charts.clone(),
    );
    doc.remove_layer(back).unwrap();
    assert_eq!(doc.object_layer(ids[0]), 0);
    assert_eq!(
        content,
        (
            doc.nodes.clone(),
            doc.connectors.clone(),
            doc.drawings.clone(),
            doc.charts.clone()
        )
    );
    doc.validate().unwrap();
}

#[test]
fn copy_and_paste_remap_mixed_group_ids_without_unlocking_the_destination() {
    let (mut source, ids) = mixed_page();
    let source_layer = source.create_layer("Source".into()).unwrap();
    source.move_objects_to_layer(&ids, source_layer).unwrap();
    source.group_objects(&ids).unwrap();
    source.set_objects_locked(&[ids[0]], true).unwrap();
    source
        .update_layer(source_layer, None, None, Some(true))
        .unwrap();
    let fragment = source.selection(&[ids[0]]);
    assert_eq!(fragment.object_count(), source.object_count());
    fragment.validate().unwrap();
    let (mut destination, _) = mixed_page();
    let destination_layer = destination.create_layer("Existing".into()).unwrap();
    destination
        .update_layer(destination_layer, None, None, Some(true))
        .unwrap();
    let old_layers = destination.organization.layers.clone();
    let old_ids = destination.object_ids();
    let pasted = destination.paste(&fragment, [30., 40.]).unwrap();
    assert!(pasted.iter().all(|id| !old_ids.contains(id)));
    for layer in old_layers {
        assert!(destination.organization.layers.contains(&layer));
    }
    let mut expanded = destination.expand_groups(&[pasted[0]]);
    expanded.sort_unstable();
    assert_eq!(expanded.len(), ids.len());
    assert!(expanded.iter().all(|id| !destination.is_object_locked(*id)));
    assert!(expanded.iter().all(|id| destination.is_object_visible(*id)));
    destination.validate().unwrap();
}

#[test]
fn hidden_nodes_hide_attached_connectors_without_losing_data_or_exporting_them() {
    let (mut doc, ids) = mixed_page();
    let before = doc.clone();
    let layer = doc.create_layer("Hidden source".into()).unwrap();
    doc.move_objects_to_layer(&[ids[0]], layer).unwrap();
    doc.update_layer(layer, None, Some(false), None).unwrap();
    assert!(!doc.is_object_visible(ids[0]));
    assert!(!doc.is_object_visible(ids[2]));
    assert!(doc.is_object_visible(ids[1]));
    assert_eq!(doc.nodes, before.nodes);
    assert_eq!(doc.connectors, before.connectors);
    let svg = svg::export(&doc).unwrap();
    assert!(!svg.contains("SOURCE-ONLY"));
    assert!(!svg.contains("CONNECTED-LINK"));
    assert!(svg.contains("DESTINATION-ONLY"));
    assert!(svg.contains("VISIBLE-CHART"));
    doc.update_layer(layer, None, Some(true), None).unwrap();
    assert!(doc.is_object_visible(ids[0]));
    assert!(doc.is_object_visible(ids[2]));
    let svg = svg::export(&doc).unwrap();
    assert!(svg.contains("SOURCE-ONLY"));
    assert!(svg.contains("CONNECTED-LINK"));
}

#[test]
fn lock_guard_protects_all_stored_content_but_allows_derived_routes_to_follow_nodes() {
    let (mut doc, ids) = mixed_page();
    doc.set_objects_locked(&[ids[2]], true).unwrap();
    let mut moved = doc.clone();
    moved.node_mut(ids[0]).unwrap().position[0] += 40.;
    assert_ne!(
        doc.route(&doc.connectors[0]),
        moved.route(&moved.connectors[0])
    );
    doc.check_locked_changes(&moved).unwrap();
    let mut changed_edge = doc.clone();
    changed_edge.connectors[0].label = "Changed".into();
    assert!(doc.check_locked_changes(&changed_edge).is_err());
    let mut deleted = doc.clone();
    deleted.nodes.retain(|node| node.id != ids[0]);
    deleted.connectors.clear();
    assert!(doc.check_locked_changes(&deleted).is_err());
    for id in [ids[0], ids[3], ids[4]] {
        let mut locked = doc.clone();
        locked.set_objects_locked(&[id], true).unwrap();
        let mut changed = locked.clone();
        if id == ids[0] {
            changed.node_mut(id).unwrap().text = "Changed".into();
        }
        if id == ids[3] {
            changed.drawings[0].start[0] += 5.;
        }
        if id == ids[4] {
            changed.charts[0].title = "Changed".into();
        }
        assert!(locked.check_locked_changes(&changed).is_err());
        let mut unlocked = locked.clone();
        unlocked.set_objects_locked(&[id], false).unwrap();
        locked.check_locked_changes(&unlocked).unwrap();
    }
}

#[test]
fn agent_lock_operations_are_atomic_and_explicit_unlock_can_precede_an_edit() {
    let (mut doc, ids) = mixed_page();
    doc.group_objects(&ids[..2]).unwrap();
    let locked = agent::preview(
        &doc,
        &patch(
            &doc,
            json!([
                {"op":"set_objects_locked","page":0,"targets":[ids[0]],"locked":true}
            ]),
        ),
    )
    .unwrap()
    .document;
    assert!(locked.is_object_locked(ids[0]));
    assert!(locked.is_object_locked(ids[1]));
    let before = locked.clone();
    for operations in [
        json!([{"op":"update_node","page":0,"target":ids[0],"changes":{"text":"No"}}]),
        json!([{"op":"set_title","title":"Must roll back"},{"op":"delete","page":0,"target":ids[0]}]),
        json!([{"op":"fit_page","page":0}]),
    ] {
        assert!(agent::preview(&locked, &patch(&locked, operations)).is_err());
        assert_eq!(locked, before);
    }
    let edited = agent::preview(&locked, &patch(&locked, json!([
        {"op":"set_objects_locked","page":0,"targets":[ids[0]],"locked":false},
        {"op":"update_node","page":0,"target":ids[0],"changes":{"text":"Explicitly unlocked"}}
    ]))).unwrap().document;
    assert_eq!(edited.node(ids[0]).unwrap().text, "Explicitly unlocked");
    assert!(!edited.is_object_locked(ids[0]));
    assert!(!edited.is_object_locked(ids[1]));
}

#[test]
fn agent_layer_actions_can_hide_content_but_cannot_silently_edit_hidden_objects() {
    let (doc, ids) = mixed_page();
    let added = agent::preview(
        &doc,
        &patch(
            &doc,
            json!([
                {"op":"add_layer","page":0,"alias":"reference","name":"Reference"}
            ]),
        ),
    )
    .unwrap();
    let layer = added.document.organization.layers[0].id;
    let hidden = agent::preview(
        &added.document,
        &patch(
            &added.document,
            json!([
                {"op":"move_objects_to_layer","page":0,"targets":[ids[0]],"layer":layer},
                {"op":"update_layer","page":0,"layer":layer,"visible":false}
            ]),
        ),
    )
    .unwrap()
    .document;
    let before = hidden.clone();
    assert!(agent::preview(&hidden, &patch(&hidden, json!([
        {"op":"update_node","page":0,"target":ids[0],"changes":{"text":"Must stay unchanged"}}
    ]))).is_err());
    assert_eq!(hidden, before);
    let restored = agent::preview(
        &hidden,
        &patch(
            &hidden,
            json!([
                {"op":"update_layer","page":0,"layer":layer,"visible":true},
                {"op":"update_node","page":0,"target":ids[0],"changes":{"text":"Visible again"}}
            ]),
        ),
    )
    .unwrap()
    .document;
    assert!(restored.is_object_visible(ids[0]));
    assert_eq!(restored.node(ids[0]).unwrap().text, "Visible again");
}

#[test]
fn layer_capacity_rejects_creation_and_paste_atomically() {
    let (mut destination, _) = mixed_page();
    for index in 1..100 {
        destination.create_layer(format!("Layer {index}")).unwrap();
    }
    destination.validate().unwrap();
    let before = destination.clone();
    assert!(destination.create_layer("Over limit".into()).is_err());
    assert_eq!(destination, before);
    let (mut source, ids) = mixed_page();
    let layer = source.create_layer("Imported layer".into()).unwrap();
    source.move_objects_to_layer(&ids[..2], layer).unwrap();
    let fragment = source.selection(&ids[..2]);
    assert!(destination.paste(&fragment, [0., 0.]).is_err());
    assert_eq!(destination, before);
}

#[test]
fn agent_can_create_layer_and_group_aliases_in_one_batch_and_keep_them_after_page_reorder() {
    let doc = Document::default();
    let output = agent::preview(&doc, &patch(&doc, json!([
        {"op":"add_page","name":"Annexe","size":[800,600]},
        {"op":"add_layer","page":1,"alias":"work","name":"Work"},
        {"op":"add_node","page":1,"alias":"a","kind":"rounded","position":[80,100],"size":[180,80],"text":"First"},
        {"op":"add_node","page":1,"alias":"b","kind":"rectangle","position":[400,100],"size":[180,80],"text":"Second"},
        {"op":"move_page","from":1,"to":0},
        {"op":"move_objects_to_layer","page":0,"targets":["a","b"],"layer":"work"},
        {"op":"group_objects","page":0,"targets":["a","b"]}
    ]))).unwrap();
    let page = output.document.page_document(0).unwrap();
    let layer = page.organization.layers[0].id;
    assert_eq!(output.report["layer_aliases"]["work"], json!([0, layer]));
    assert_eq!(page.organization.groups.len(), 1);
    assert_eq!(page.organization.groups[0].members.len(), 2);
    assert!(
        page.nodes
            .iter()
            .all(|node| page.object_layer(node.id) == layer)
    );
    let inspection = agent::inspect(&output.document).unwrap();
    assert_eq!(
        inspection["pages"][0]["organization"]["groups"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(
        inspection["pages"][1]["organization"]
            .as_object()
            .is_some_and(|value| value.is_empty())
            || inspection["pages"][1].get("organization").is_none()
    );
}

#[test]
fn deleted_or_foreign_page_layer_aliases_cannot_target_a_reused_id() {
    let doc = Document::default();
    for operations in [
        json!([
            {"op":"add_layer","page":0,"alias":"old","name":"Old"},
            {"op":"delete_layer","page":0,"layer":"old"},
            {"op":"add_layer","page":0,"alias":"new","name":"New"},
            {"op":"update_layer","page":0,"layer":"old","visible":false}
        ]),
        json!([
            {"op":"add_layer","page":0,"alias":"work","name":"Work"},
            {"op":"add_page","name":"Annexe","size":[800,600]},
            {"op":"add_node","page":1,"alias":"a","kind":"rounded","position":[80,100],"size":[180,80],"text":"First"},
            {"op":"move_objects_to_layer","page":1,"targets":["a"],"layer":"work"}
        ]),
    ] {
        assert!(agent::preview(&doc, &patch(&doc, operations)).is_err());
        assert!(doc.nodes.is_empty());
        assert!(doc.organization.is_empty());
        assert_eq!(doc.page_count(), 1);
    }
}

#[test]
fn duplicate_keeps_existing_layers_and_unlocks_only_the_remapped_copies() {
    let (mut doc, ids) = mixed_page();
    let layer = doc.create_layer("Existing".into()).unwrap();
    doc.move_objects_to_layer(&ids, layer).unwrap();
    let group = doc.group_objects(&ids).unwrap();
    doc.set_objects_locked(&[ids[0]], true).unwrap();
    let old_layers = doc.organization.layers.clone();
    let copies = doc.duplicate_objects(&[ids[0]], [24., 24.]).unwrap();
    assert_eq!(doc.organization.layers, old_layers);
    assert_eq!(doc.organization.groups.len(), 2);
    assert!(ids.iter().all(|id| doc.is_object_locked(*id)));
    assert_eq!(doc.group_for_object(ids[0]).unwrap().id, group);
    let copied_group = doc.group_for_object(copies[0]).unwrap();
    assert_ne!(copied_group.id, group);
    assert_eq!(copied_group.members.len(), ids.len());
    assert!(copied_group.members.iter().all(|id| {
        !ids.contains(id) && doc.object_layer(*id) == layer && doc.can_edit_object(*id)
    }));
    doc.update_layer(layer, None, None, Some(true)).unwrap();
    let before = doc.clone();
    assert!(doc.duplicate_objects(&[ids[0]], [24., 24.]).is_err());
    assert_eq!(doc, before);
    doc.validate().unwrap();
}

#[test]
fn equal_ids_on_duplicate_pages_do_not_bypass_locked_content_checks() {
    let (mut doc, ids) = mixed_page();
    doc.set_objects_locked(&[ids[0]], true).unwrap();
    doc.add_page(true).unwrap();
    doc.switch_page(0).unwrap();
    let original = doc.clone();
    let mut changed = doc.clone();
    changed.node_mut(ids[0]).unwrap().text = "Unapproved page-zero edit".into();
    assert!(original.check_locked_changes(&changed).is_err());
    let mut reordered = doc.clone();
    reordered.move_page(0, 1).unwrap();
    original.check_locked_changes(&reordered).unwrap();
    let output = agent::preview(&doc, &patch(&doc, json!([
        {"op":"set_objects_locked","page":0,"targets":[ids[0]],"locked":false},
        {"op":"update_node","page":0,"target":ids[0],"changes":{"text":"Only page zero changes"}}
    ]))).unwrap().document;
    assert_eq!(
        output.page_document(0).unwrap().node(ids[0]).unwrap().text,
        "Only page zero changes"
    );
    assert_eq!(
        output.page_document(1).unwrap().node(ids[0]).unwrap().text,
        "SOURCE-ONLY"
    );
    assert!(output.page_document(1).unwrap().is_object_locked(ids[0]));
}
