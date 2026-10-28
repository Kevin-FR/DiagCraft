use diagcraft::{
    agent, agent_io,
    document::{Document, ShapeKind},
};
use serde_json::{Value, json};
use std::io::Cursor;
fn patch(d: &Document, operations: Value) -> agent::Patch {
    agent::parse_patch(
        &json!({"api_version":1,"expected_revision":agent::revision(d),"operations":operations})
            .to_string(),
    )
    .unwrap()
}
fn add(alias: &str) -> Value {
    json!({"op":"add_node","page":0,"alias":alias,"kind":"rounded","position":[80,100],"size":[220,84],"text":"Example"})
}
fn mcp(dir: &std::path::Path, patch: Value) -> Value {
    let messages = [
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"draw_apply","arguments":{"file":"source.draw","patch":patch,"output":"result.draw","dry_run":false}}}),
    ];
    let input = messages
        .iter()
        .map(|v| v.to_string() + "\n")
        .collect::<String>();
    let mut out = vec![];
    agent_io::serve(dir, Cursor::new(input), &mut out).unwrap();
    serde_json::from_str::<Value>(String::from_utf8(out).unwrap().lines().last().unwrap()).unwrap()
        ["result"]
        .clone()
}
#[test]
fn context_identifies_the_operation_page_alias_and_resolved_id_without_mutation() {
    let d = Document::default();
    let before = agent::revision(&d);
    let p = patch(
        &d,
        json!([add("new"),{"op":"update_node","page":0,"target":"new","changes":{"size":[-1,84]}}]),
    );
    let e = agent::preview_detailed(&d, &p).unwrap_err();
    let v = e.to_value();
    let x = &v["diagnostic"];
    assert_eq!(x["code"], "invalid_geometry");
    assert_eq!(x["operation"], 1);
    assert_eq!(x["operation_name"], "update_node");
    assert_eq!(x["page"], 0);
    assert_eq!(x["target"], "new");
    assert!(x["id"].as_u64().is_some_and(|id| id > 0));
    assert_eq!(x["severity"], "error");
    assert!(!x["hint"].as_str().unwrap().is_empty());
    assert!(e.to_string().contains("Opération 2"));
    assert_eq!(agent::revision(&d), before);
    assert!(d.nodes.is_empty());
}
#[test]
fn missing_pages_nodes_and_aliases_have_distinct_actionable_codes() {
    let d = Document::default();
    for (op, code, target) in [
        (
            json!({"op":"update_node","page":8,"target":9,"changes":{"text":"A"}}),
            "page_not_found",
            json!(9),
        ),
        (
            json!({"op":"update_node","page":0,"target":9,"changes":{"text":"A"}}),
            "object_not_found",
            json!(9),
        ),
        (
            json!({"op":"update_node","page":0,"target":"missing","changes":{"text":"A"}}),
            "alias_not_found",
            json!("missing"),
        ),
    ] {
        let e = agent::preview_detailed(&d, &patch(&d, json!([op]))).unwrap_err();
        assert_eq!(e.diagnostic.code, code);
        assert_eq!(e.diagnostic.target, Some(target));
        assert_eq!(e.diagnostic.operation, Some(0));
        assert!(!e.diagnostic.hint.is_empty());
    }
}
#[test]
fn protected_objects_are_identified_but_explicit_unlock_then_edit_is_still_atomic() {
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Rectangle, [200., 200.]);
    d.set_objects_locked(&[id], true).unwrap();
    let before = d.clone();
    let e = agent::preview_detailed(
        &d,
        &patch(
            &d,
            json!([{"op":"update_node","page":0,"target":id,"changes":{"text":"X"}}]),
        ),
    )
    .unwrap_err();
    assert_eq!(e.diagnostic.code, "object_locked");
    assert_eq!(e.diagnostic.id, Some(id));
    assert_eq!(d, before);
    let p = patch(
        &d,
        json!([{"op":"set_objects_locked","page":0,"targets":[id],"locked":false},{"op":"update_node","page":0,"target":id,"changes":{"text":"X"}}]),
    );
    assert_eq!(
        agent::preview_detailed(&d, &p)
            .unwrap()
            .document
            .node(id)
            .unwrap()
            .text,
        "X"
    );
    assert_eq!(d, before);
}
#[test]
fn syntax_schema_and_patch_envelope_errors_preserve_source_locations() {
    let e = agent::parse_patch_detailed("{\n  \"api_version\": 1,\n BAD\n}").unwrap_err();
    assert_eq!(e.diagnostic.code, "patch_json");
    assert_eq!(e.diagnostic.line, Some(3));
    assert!(e.diagnostic.column.unwrap() > 0);
    let d = Document::default();
    let source=serde_json::to_string_pretty(&json!({"api_version":1,"expected_revision":agent::revision(&d),"operations":[add("a"),{"op":"update_node","page":0,"target":"a","changes":{"postion":[30,40]}}]})).unwrap();
    let e = agent::parse_patch_detailed(&source).unwrap_err();
    assert_eq!(e.diagnostic.code, "patch_schema");
    assert_eq!(e.diagnostic.operation, Some(1));
    assert_eq!(e.diagnostic.page, Some(0));
    assert_eq!(e.diagnostic.target, Some(json!("a")));
    assert!(e.diagnostic.line.unwrap() > 3);
    let line = source.lines().nth(e.diagnostic.line.unwrap() - 1).unwrap();
    assert_eq!(line.trim(), "{");
    let e = agent::parse_patch_detailed(
        "{\"api_version\":1,\"expected_revision\":\"x\",\"operations\":[],\"extra\":0}",
    )
    .unwrap_err();
    assert_eq!(e.diagnostic.code, "patch_schema");
    assert!(e.diagnostic.line.is_some());
    assert!(e.diagnostic.operation.is_none());
}
#[test]
fn revisions_api_and_operation_limits_fail_before_operation_execution() {
    let d = Document::default();
    let p = patch(&d, json!([add("a")]));
    let mut invalid = p.clone();
    invalid.expected_revision = "stale".into();
    assert_eq!(
        agent::preview_detailed(&d, &invalid)
            .unwrap_err()
            .diagnostic
            .code,
        "revision_conflict"
    );
    invalid = p.clone();
    invalid.api_version = 2;
    assert_eq!(
        agent::preview_detailed(&d, &invalid)
            .unwrap_err()
            .diagnostic
            .code,
        "unsupported_api"
    );
    invalid = p;
    invalid.operations.clear();
    assert_eq!(
        agent::preview_detailed(&d, &invalid)
            .unwrap_err()
            .diagnostic
            .code,
        "operation_limit"
    );
    assert!(d.nodes.is_empty());
}
#[test]
fn cli_and_mcp_return_matching_structured_patch_failures_and_create_no_output() {
    let temp = tempfile::tempdir().unwrap();
    let d = Document::default();
    let input = temp.path().join("source.draw");
    d.save(&input).unwrap();
    let source = std::fs::read(&input).unwrap();
    let p = patch(
        &d,
        json!([add("new"),{"op":"update_node","page":0,"target":"new","changes":{"size":[-1,84]}}]),
    );
    let pf = temp.path().join("patch.json");
    std::fs::write(&pf, serde_json::to_vec_pretty(&p).unwrap()).unwrap();
    let args = vec![
        "apply".into(),
        input.display().to_string(),
        pf.display().to_string(),
        temp.path().join("result.draw").display().to_string(),
    ];
    let cli = agent_io::cli_detailed(&args).unwrap_err().to_value();
    let reply = mcp(temp.path(), serde_json::to_value(&p).unwrap());
    assert_eq!(reply["isError"], true);
    assert_eq!(reply["structuredContent"], cli);
    assert_eq!(reply["content"][0]["text"], cli["error"]);
    assert!(!temp.path().join("result.draw").exists());
    assert_eq!(std::fs::read(&input).unwrap(), source);
}
#[test]
fn invalid_values_do_not_echo_embedded_image_payloads_or_whole_operations() {
    let secret = "iVBORw0KGgoPRIVATE_PIXELS".repeat(1000);
    let d = Document::default();
    let source=json!({"api_version":1,"expected_revision":agent::revision(&d),"operations":[{"op":"add_node","page":0,"alias":"new","kind":secret,"position":[0,0],"size":[50,50],"text":"X"}]}).to_string();
    let e = agent::parse_patch_detailed(&source).unwrap_err();
    let encoded = e.to_value().to_string();
    assert!(!encoded.contains("PRIVATE_PIXELS"));
    assert!(encoded.len() < 2000);
    let unknown_alias = patch(
        &d,
        json!([{"op":"update_node","page":0,"target":secret,"changes":{"text":"X"}}]),
    );
    let e = agent::preview_detailed(&d, &unknown_alias).unwrap_err();
    assert_eq!(e.diagnostic.code, "alias_not_found");
    assert!(!e.to_value().to_string().contains("PRIVATE_PIXELS"));
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("bad.draw");
    let mut raw = serde_json::to_value(&d).unwrap();
    let mut with_node = d.clone();
    with_node.add_node(ShapeKind::Rectangle, [100., 100.]);
    raw["nodes"] = serde_json::to_value(with_node.nodes).unwrap();
    raw["nodes"][0]["kind"] = json!(secret);
    std::fs::write(&input, raw.to_string()).unwrap();
    let error =
        agent_io::cli_detailed(&["inspect".into(), input.display().to_string()]).unwrap_err();
    assert!(!error.to_value().to_string().contains("PRIVATE_PIXELS"));
}
#[test]
fn document_warnings_have_severity_context_and_repair_hints() {
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Rectangle, [-100., -100.]);
    let n = d.node_mut(id).unwrap();
    n.position = [-100., -100.];
    let warnings = agent::diagnostics(&d);
    let w = warnings
        .iter()
        .find(|w| w["code"] == "outside_page")
        .unwrap();
    assert_eq!(w["severity"], "warning");
    assert_eq!(w["page"], 0);
    assert_eq!(w["id"], id);
    assert!(!w["hint"].as_str().unwrap().is_empty());
}
