use diagcraft::{
    agent, agent_io,
    document::{Document, ShapeKind},
};
use serde_json::{Value, json};
use std::io::Cursor;
fn patch(d: &Document, ops: Value) -> agent::Patch {
    agent::parse_patch(
        &json!({"api_version":1,"expected_revision":agent::revision(d),"operations":ops})
            .to_string(),
    )
    .unwrap()
}
fn add(alias: &str, page: usize, x: f32) -> Value {
    json!({"op":"add_node","page":page,"alias":alias,"kind":"rounded","position":[x,100],"size":[220,84],"text":"Une idée"})
}
#[test]
fn aliases_connect_exact_top_left_positions_and_a_free_end() {
    let d = Document::default();
    let before = d.clone();
    let p = patch(
        &d,
        json!([add("bloc",0,80.),{"op":"add_connector","page":0,"alias":"lien","start":{"node":"bloc","port":"bottom"},"end":{"point":[500,400]},"style":{"label":"Suite","pattern":"dashed"}}]),
    );
    let out = agent::preview(&d, &p).unwrap();
    assert_eq!(d, before);
    assert_eq!(out.document.nodes[0].position, [80., 100.]);
    assert_eq!(out.document.connectors[0].from, out.document.nodes[0].id);
    assert_eq!(out.document.connectors[0].free_end, Some([500., 400.]));
    assert_eq!(
        out.report["aliases"]["bloc"],
        json!([0, out.document.nodes[0].id])
    );
}
#[test]
fn failed_second_operation_rolls_back_everything() {
    let d = Document::default();
    let before = agent::revision(&d);
    let p = patch(
        &d,
        json!([add("bloc",0,80.),{"op":"update_node","page":0,"target":"bloc","changes":{"size":[2,84]}}]),
    );
    assert!(agent::preview(&d, &p).unwrap_err().contains("Opération 2"));
    assert_eq!(agent::revision(&d), before);
    assert!(d.nodes.is_empty());
}
#[test]
fn rejects_stale_revision_and_unknown_nested_fields() {
    let mut d = Document::default();
    let p = patch(&d, json!([add("a", 0, 80.)]));
    d.title = "Modifié".into();
    assert!(agent::preview(&d, &p).unwrap_err().contains("Révision"));
    for changes in [
        json!({"postion":[20,30]}),
        json!({"text_style":{"bould":true}}),
    ] {
        assert!(agent::parse_patch(&json!({"api_version":1,"expected_revision":agent::revision(&d),"operations":[{"op":"update_node","page":0,"target":1,"changes":changes}]}).to_string()).is_err());
    }
}
#[test]
fn page_navigation_does_not_change_revision_or_canonical_inspection() {
    let mut d = diagcraft::templates::multipage();
    let r = agent::revision(&d);
    let original = agent::inspect(&d).unwrap();
    d.switch_page(2).unwrap();
    assert_eq!(r, agent::revision(&d));
    assert_eq!(original, agent::inspect(&d).unwrap());
    assert_eq!(original["pages"][2]["index"], 2);
    assert_eq!(original["coordinates"]["position"], "top_left");
}
#[test]
fn page_local_ids_and_aliases_survive_page_reorder() {
    let d = Document::default();
    let p = patch(
        &d,
        json!([add("first",0,80.),{"op":"add_page","name":"Annexe","size":[800,600]},add("second",1,200.),{"op":"move_page","from":1,"to":0},{"op":"update_node","page":0,"target":"second","changes":{"text":"Annexe modifiée"}},{"op":"update_node","page":1,"target":"first","changes":{"text":"Original modifié"}}]),
    );
    let out = agent::preview(&d, &p).unwrap();
    assert_eq!(
        out.document.page(0).unwrap().nodes[0].text,
        "Annexe modifiée"
    );
    assert_eq!(
        out.document.page(1).unwrap().nodes[0].text,
        "Original modifié"
    );
    assert_eq!(
        out.document.page(0).unwrap().nodes[0].id,
        out.document.page(1).unwrap().nodes[0].id
    );
}
#[test]
fn cannot_address_deleted_alias_even_when_id_reused() {
    let d = Document::default();
    let p = patch(
        &d,
        json!([add("old",0,80.),{"op":"delete","page":0,"target":"old"},add("new",0,80.),{"op":"update_node","page":0,"target":"old","changes":{"text":"wrong"}}]),
    );
    assert!(agent::preview(&d, &p).is_err());
}
#[test]
fn endpoints_are_exclusive_and_cannot_reference_another_page_alias() {
    let d = Document::default();
    for endpoint in [
        json!({"node":"n","point":[20,30]}),
        json!({"point":[20,30],"port":"left"}),
        json!({}),
    ] {
        let p = patch(
            &d,
            json!([add("n",0,80.),{"op":"add_connector","page":0,"alias":"e","start":endpoint,"end":{"point":[500,300]}}]),
        );
        assert!(agent::preview(&d, &p).is_err());
    }
    let p = patch(
        &d,
        json!([add("n",0,80.),{"op":"add_page","name":"P2","size":[800,600]},{"op":"update_node","page":1,"target":"n","changes":{"text":"no"}}]),
    );
    assert!(agent::preview(&d, &p).is_err());
}
#[test]
fn deletes_incident_connectors_and_all_page_content() {
    let mut d = Document::default();
    let a = d.add_node(ShapeKind::Rectangle, [200., 200.]);
    let b = d.add_node(ShapeKind::Rectangle, [600., 200.]);
    d.connect(a, b, diagcraft::document::ConnectorKind::Elbow)
        .unwrap();
    let p = patch(&d, json!([{"op":"delete","page":0,"target":a}]));
    let out = agent::preview(&d, &p).unwrap();
    assert_eq!(out.document.nodes.len(), 1);
    assert!(out.document.connectors.is_empty());
    assert!(agent::preview(&d, &patch(&d, json!([{"op":"delete_page","page":0}]))).is_err());
}
#[test]
fn supports_graphics_curves_and_reattachment_with_styles() {
    let d = Document::default();
    let p = patch(
        &d,
        json!([add("n",0,80.),{"op":"add_chart","page":0,"alias":"c","kind":"line","position":[400,300],"changes":{"title":"CA","categories":["A","B"],"series":[{"name":"Total","values":[10,20],"color":[0,100,255]}]}},{"op":"add_drawing","page":0,"alias":"d","kind":"bezier","start":[100,400],"end":[300,500]},{"op":"update_drawing","page":0,"target":"d","start":[110,410],"end":[310,510],"bend":30},{"op":"add_connector","page":0,"alias":"e","start":{"point":[50,50]},"end":{"point":[500,80]}},{"op":"update_connector","page":0,"target":"e","changes":{"start":{"node":"n","port":"top"},"color":[10,20,30],"width":3,"end_head":"circle"}}]),
    );
    let out = agent::preview(&d, &p).unwrap();
    assert_eq!(out.document.charts[0].title, "CA");
    assert_eq!(out.document.charts[0].position, [400., 300.]);
    assert_eq!(out.document.drawings[0].bend, 30.);
    assert!(out.document.connectors[0].free_start.is_none());
    assert_eq!(out.document.connectors[0].from, out.document.nodes[0].id);
}
#[test]
fn diagnostics_include_free_connectors_and_fit_page_removes_warning() {
    let d = Document::default();
    let p = patch(
        &d,
        json!([{"op":"add_connector","page":0,"alias":"e","start":{"point":[-50,30]},"end":{"point":[1500,1000]}}]),
    );
    let out = agent::preview(&d, &p).unwrap();
    assert_eq!(out.report["diagnostics"][0]["code"], "outside_page");
    let fixed = agent::preview(
        &out.document,
        &patch(&out.document, json!([{"op":"fit_page","page":0}])),
    )
    .unwrap();
    assert!(agent::diagnostics(&fixed.document).is_empty());
}
#[test]
fn cli_dry_run_is_json_and_save_never_overwrites() {
    let t = tempfile::tempdir().unwrap();
    let file = t.path().join("a.draw");
    let pf = t.path().join("patch.json");
    let out = t.path().join("b.draw");
    let d = Document::default();
    d.save(&file).unwrap();
    std::fs::write(
        &pf,
        serde_json::to_vec(&patch(&d, json!([add("n", 0, 80.)]))).unwrap(),
    )
    .unwrap();
    let mut args = vec![
        "apply".into(),
        file.display().to_string(),
        pf.display().to_string(),
        out.display().to_string(),
        "--dry-run".into(),
    ];
    assert_eq!(agent_io::cli(&args).unwrap()["dry_run"], true);
    assert!(!out.exists());
    args.pop();
    assert_eq!(agent_io::cli(&args).unwrap()["dry_run"], false);
    assert_eq!(Document::load(&out).unwrap().nodes.len(), 1);
    assert!(agent_io::cli(&args).is_err());
    args[3] = args[1].clone();
    assert!(agent_io::cli(&args).is_err());
    assert_eq!(Document::load(&file).unwrap(), d);
}
fn mcp(dir: &std::path::Path, messages: Vec<Value>) -> Vec<Value> {
    let mut input = String::new();
    for m in messages {
        input.push_str(&m.to_string());
        input.push('\n');
    }
    let mut output = Vec::new();
    agent_io::serve(dir, Cursor::new(input), &mut output).unwrap();
    String::from_utf8(output)
        .unwrap()
        .lines()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect()
}
fn init() -> Value {
    json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"test","version":"1"}}})
}
fn ready() -> Value {
    json!({"jsonrpc":"2.0","method":"notifications/initialized"})
}
fn call(name: &str, args: Value) -> Value {
    json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":name,"arguments":args}})
}
#[test]
fn mcp_initializes_lists_tools_and_keeps_stdout_ndjson() {
    let t = tempfile::tempdir().unwrap();
    let out = mcp(
        t.path(),
        vec![
            json!({"jsonrpc":"2.0","id":0,"method":"tools/list"}),
            init(),
            ready(),
            json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
            call("draw_catalog", json!({})),
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}),
        ],
    );
    assert_eq!(out.len(), 4);
    assert!(out[0].get("error").is_some());
    assert_eq!(out[1]["result"]["protocolVersion"], "2025-11-25");
    assert_eq!(out[2]["result"]["tools"].as_array().unwrap().len(), 4);
    assert_eq!(out[3]["result"]["isError"], false);
    assert!(out[2]["result"]["tools"][3]["inputSchema"]["$defs"].is_object());
}
#[test]
fn mcp_confines_paths_and_symlinks_to_root_and_returns_tool_errors() {
    let t = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    Document::default()
        .save(&outside.path().join("a.draw"))
        .unwrap();
    let requests = vec![
        init(),
        ready(),
        call("draw_inspect", json!({"file":"../a.draw"})),
        call(
            "draw_inspect",
            json!({"file":outside.path().join("a.draw")}),
        ),
        call("draw_inspect", json!({"file":"x","typo":true})),
        call("unknown", json!({})),
    ];
    #[cfg(unix)]
    let requests = {
        let mut requests = requests;
        std::os::unix::fs::symlink(outside.path(), t.path().join("escape")).unwrap();
        requests.push(call("draw_inspect", json!({"file":"escape/a.draw"})));
        requests
    };
    let out = mcp(t.path(), requests);
    for v in out.iter().skip(1) {
        assert!(v["result"]["isError"] == true || v.get("error").is_some());
    }
}
#[test]
fn mcp_apply_is_dry_by_default_and_requires_new_output() {
    let t = tempfile::tempdir().unwrap();
    let d = Document::default();
    d.save(&t.path().join("a.draw")).unwrap();
    let p = serde_json::to_value(patch(&d, json!([add("n", 0, 80.)]))).unwrap();
    let out = mcp(
        t.path(),
        vec![
            init(),
            ready(),
            call(
                "draw_apply",
                json!({"file":"a.draw","patch":p,"output":"dry.draw"}),
            ),
            call(
                "draw_apply",
                json!({"file":"a.draw","patch":p,"dry_run":false}),
            ),
            call(
                "draw_apply",
                json!({"file":"a.draw","patch":p,"output":"a.draw","dry_run":false}),
            ),
            call(
                "draw_apply",
                json!({"file":"a.draw","patch":p,"output":"new.draw","dry_run":false}),
            ),
        ],
    );
    assert_eq!(out[1]["result"]["structuredContent"]["dry_run"], true);
    assert_eq!(out[2]["result"]["isError"], true);
    assert_eq!(out[3]["result"]["isError"], true);
    assert_eq!(out[4]["result"]["isError"], false);
    assert!(!t.path().join("dry.draw").exists());
    assert_eq!(
        Document::load(&t.path().join("new.draw"))
            .unwrap()
            .nodes
            .len(),
        1
    );
}
#[test]
fn patch_and_protocol_limits_are_enforced() {
    let d = Document::default();
    let p = patch(&d, json!([]));
    assert!(agent::preview(&d, &p).is_err());
    let p = patch(&d, json!(vec![json!({"op":"set_title","title":"x"}); 501]));
    assert!(agent::preview(&d, &p).is_err());
    let mut p = patch(&d, json!([add("n", 0, 80.)]));
    p.api_version = 99;
    assert!(agent::preview(&d, &p).is_err());
    let t = tempfile::tempdir().unwrap();
    let mut output = Vec::new();
    agent_io::serve(t.path(), Cursor::new("{bad}\n"), &mut output).unwrap();
    let error: Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(error["error"]["code"], -32700);
}
