use diagcraft::{
    document::{ConnectorKind, Document, ShapeKind},
    mermaid::{self, MAX_SOURCE_BYTES},
    stroke_style::{ArrowHead, LinePattern},
};
fn read(source: &str) -> Document {
    mermaid::import(source).unwrap().document
}
#[test]
fn classic_shapes_unicode_entities_and_linebreaks_become_editable_objects() {
    let d = read(
        "flowchart LR\nA[\"Début #34;与#34;<br/>suite\"] --> B(Arrondi)\nB ==> C([Fin])\nC -.-> D{Décision}\nD --- E((Cercle))\nE --> F{{Hexagone}}\nF --> G[[Processus]]\nG --> H[(Données)]",
    );
    assert_eq!(
        d.nodes.iter().map(|n| n.kind).collect::<Vec<_>>(),
        vec![
            ShapeKind::Rectangle,
            ShapeKind::Rounded,
            ShapeKind::Terminator,
            ShapeKind::Diamond,
            ShapeKind::Ellipse,
            ShapeKind::Hexagon,
            ShapeKind::PredefinedProcess,
            ShapeKind::Database
        ]
    );
    assert_eq!(d.nodes[0].text, "Début \"与\"\nsuite");
    assert_eq!(d.connectors[1].width, 4.);
    assert_eq!(d.connectors[2].pattern, LinePattern::Dotted);
    assert!(!d.connectors[3].arrow);
    assert!(
        d.nodes
            .windows(2)
            .all(|p| p[0].position[0] < p[1].position[0])
    );
    d.validate().unwrap();
}
#[test]
fn quoted_statement_separators_comments_chains_and_last_declaration_are_preserved() {
    let d = read(
        "%% comment\ngraph TB; A[\"A; %% not comment\"] -->|\"yes; #124; no\"| B --> C; %% trailing\nB{\"New #91;text#93;\"}; D[\"a\nb\"]",
    );
    assert_eq!(d.nodes.len(), 4);
    assert_eq!(d.connectors.len(), 2);
    assert_eq!(d.nodes[0].text, "A; %% not comment");
    assert_eq!(d.nodes[1].text, "New [text]");
    assert_eq!(d.nodes[1].kind, ShapeKind::Diamond);
    assert_eq!(d.nodes[3].text, "a\nb");
    assert_eq!(d.connectors[0].label, "yes; | no");
    assert_eq!(d.nodes[2].text, "C");
}
#[test]
fn every_direction_is_deterministic_and_cycles_and_parallel_edges_are_bounded() {
    for dir in ["TB", "TD", "BT", "LR", "RL"] {
        let source = format!("flowchart {dir}\nA --> B\nB --> C\nD\nA --> B");
        let d = read(&source);
        assert_eq!(d, read(&source));
        assert_eq!(d.connectors.len(), 3);
        let (a, b) = (d.nodes[0].position, d.nodes[2].position);
        match dir {
            "TB" | "TD" => assert!(a[1] < b[1]),
            "BT" => assert!(a[1] > b[1]),
            "LR" => assert!(a[0] < b[0]),
            _ => assert!(a[0] > b[0]),
        }
    }
    let d = read("flowchart TB\nA --> B --> C --> A\nC --> D\nE --> F");
    assert_eq!(d.nodes.len(), 6);
    d.validate().unwrap();
    for (i, n) in d.nodes.iter().enumerate() {
        for m in d.nodes.iter().skip(i + 1) {
            assert!(
                n.position[0] + n.size[0] <= m.position[0]
                    || m.position[0] + m.size[0] <= n.position[0]
                    || n.position[1] + n.size[1] <= m.position[1]
                    || m.position[1] + m.size[1] <= n.position[1]
            );
        }
    }
}
#[test]
fn unsupported_constructs_and_malformed_labels_are_rejected_with_locations_and_hints() {
    for source in [
        "sequenceDiagram\nA->>B: hello",
        "flowchart TB\nsubgraph G\nA\nend",
        "flowchart TB\nclassDef red fill:red",
        "flowchart TB\nA --> A",
        "flowchart TB\nclick A callback",
        "%%{init: {}}%%\nflowchart TB\nA",
        "flowchart TB\nA@{shape:rect}",
        "flowchart TB\nA --> B & C",
        "flowchart TB\nA[\"<script>x</script>\"]",
        "flowchart TB\nA[\"`bold`\"]",
        "flowchart TB\nA[\"missing]",
        "flowchart TB\nA[missing",
        "flowchart TB\nA[\"#55296;\"]",
        "flowchart TB\nA <--> B",
        "flowchart TB\nA -- label --> B",
        "flowchart TB\nA::class",
        "---\ntitle: x\n---\nflowchart TB\nA",
    ] {
        let errors = mermaid::import(source).unwrap_err();
        assert_eq!(errors.len(), 1, "{source}");
        let e = &errors[0];
        assert_eq!(e.severity, "error");
        assert!(e.line.is_some());
        assert!(e.column.is_some());
        assert!(!e.code.is_empty() && !e.message.is_empty() && !e.hint.is_empty());
    }
    assert_eq!(
        mermaid::import("flowchart TB\nA\nB --> B").unwrap_err()[0].line,
        Some(3)
    );
}
#[test]
fn imports_enforce_source_node_link_and_label_budgets_before_building() {
    assert_eq!(
        mermaid::import(&"A".repeat(MAX_SOURCE_BYTES + 1)).unwrap_err()[0].code,
        "source_limit"
    );
    assert_eq!(
        mermaid::import(&format!(
            "flowchart TB\n{}",
            (0..201).map(|i| format!("n{i}\n")).collect::<String>()
        ))
        .unwrap_err()[0]
            .code,
        "node_limit"
    );
    assert_eq!(
        mermaid::import(&format!("flowchart TB\n{}", "A --> B\n".repeat(401))).unwrap_err()[0].code,
        "edge_limit"
    );
    assert_eq!(
        mermaid::import(&format!("flowchart TB\nA[\"{}\"]", "x".repeat(4097))).unwrap_err()[0].code,
        "label_limit"
    );
    let d = read(&format!(
        "flowchart TB\n{}",
        (0..199)
            .map(|i| format!("n{i} --> n{}\n", i + 1))
            .collect::<String>()
    ));
    assert_eq!(d.nodes.len(), 200);
    d.validate().unwrap();
}
#[test]
fn export_roundtrip_preserves_supported_graph_labels_shapes_and_original_document() {
    let mut d = read(
        "flowchart TB\nA[\"Quotes #34; #91; #93; #123; #125; #124; #38; #35; #96; #59; #92;\"] -->|\"文字<br/>two #34;\"| B{\"Question?\"}\nB -.-> C([Fin])\nC ==> D[(Storage)]\nD --- E((Circle))\nF[[Routine]] --> G{{Prepare}}\nG --> H(Rounded)",
    );
    let before = d.clone();
    let out = mermaid::export(&d, 0).unwrap();
    assert_eq!(d, before);
    assert!(out.source.starts_with("flowchart TB\n"));
    assert_eq!(out.diagnostics[0].code, "layout_changed");
    let back = read(&out.source);
    assert_eq!(
        d.nodes
            .iter()
            .map(|n| (&n.text, n.kind))
            .collect::<Vec<_>>(),
        back.nodes
            .iter()
            .map(|n| (&n.text, n.kind))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        d.connectors
            .iter()
            .map(|e| (&e.label, e.arrow, e.width, e.pattern))
            .collect::<Vec<_>>(),
        back.connectors
            .iter()
            .map(|e| (&e.label, e.arrow, e.width, e.pattern))
            .collect::<Vec<_>>()
    );
    d.add_page(false).unwrap();
    let id = d.add_node(ShapeKind::Rectangle, [100., 100.]);
    d.node_mut(id).unwrap().text = "Page two".into();
    assert!(mermaid::export(&d, 1).unwrap().source.contains("Page two"));
    assert!(!mermaid::export(&d, 0).unwrap().source.contains("Page two"));
    assert!(mermaid::export(&d, 2).is_err());
}
#[test]
fn export_reports_omissions_conversions_and_hidden_objects_without_mutating_source() {
    let mut d = read("flowchart TB\nA[One] --> B[Two]");
    d.nodes[0].kind = ShapeKind::Star;
    let image = diagcraft::images::ImageAsset::from_rgba(2, 2, &[255; 16]).unwrap();
    let image_id = d.add_image(image, [500., 500.]).unwrap();
    d.node_mut(image_id).unwrap().text = "Illustration".into();
    d.connectors[0].arrow_start = true;
    d.connectors[0].end_head = ArrowHead::Circle;
    let layer = d.create_layer("Hidden".into()).unwrap();
    let hidden = d.add_node(ShapeKind::Rectangle, [20., 20.]);
    d.node_mut(hidden).unwrap().text = "SECRET_HIDDEN".into();
    d.move_objects_to_layer(&[hidden], layer).unwrap();
    d.update_layer(layer, None, Some(false), None).unwrap();
    // An attached link with a free endpoint must be reported rather than guessed.
    let edge = d
        .connect(d.nodes[0].id, image_id, ConnectorKind::Elbow)
        .unwrap();
    let edge = d.connectors.iter_mut().find(|e| e.id == edge).unwrap();
    edge.to = 0;
    edge.free_end = Some([900., 900.]);
    let before = d.clone();
    let out = mermaid::export(&d, 0).unwrap();
    assert_eq!(d, before);
    assert!(!out.source.contains("SECRET_HIDDEN"));
    let codes: Vec<_> = out.diagnostics.iter().map(|n| n.code.as_str()).collect();
    for code in [
        "organization_flattened",
        "shape_converted",
        "image_omitted",
        "arrow_converted",
        "connector_omitted",
    ] {
        assert!(codes.contains(&code), "{code}");
    }
    assert!(
        out.diagnostics
            .iter()
            .filter(|n| n.code == "shape_converted")
            .all(|n| n.page == Some(0) && n.id.is_some())
    );
    read(&out.source).validate().unwrap();
}

#[test]
fn literal_html_entities_and_line_endings_remain_data_through_export() {
    let mut d =
        read("flowchart TB\nA[\"#60;br/#62; #60;script#62; #38;#35; literal\"] -->|\"x#9;y\"| B");
    assert_eq!(d.nodes[0].text, "<br/> <script> &# literal");
    assert_eq!(d.connectors[0].label, "x\ty");
    let out = mermaid::export(&d, 0).unwrap();
    let back = read(&out.source);
    assert_eq!(back.nodes[0].text, d.nodes[0].text);
    assert_eq!(back.connectors[0].label, "x\ty");
    d.nodes[0].text = "first\r\nsecond\rthird".into();
    let out = mermaid::export(&d, 0).unwrap();
    assert!(
        out.diagnostics
            .iter()
            .any(|n| n.code == "label_normalized" && n.id == Some(d.nodes[0].id))
    );
    assert_eq!(read(&out.source).nodes[0].text, "first\nsecond\nthird");
    d.nodes[0].text.push('\0');
    assert!(
        mermaid::export(&d, 0)
            .unwrap_err()
            .contains("control characters")
    );
    for label in ["#quot;", "&amp;", "&#x26;", "#0;"] {
        assert!(mermaid::import(&format!("flowchart TB\nA[\"{label}\"]")).is_err());
    }
}
#[test]
fn unicode_edge_byte_limit_and_escaped_export_source_limit_are_explicit() {
    let label = format!("{}x", "与".repeat(1333));
    assert_eq!(label.len(), 4000);
    let mut d = read(&format!("flowchart TB\nA -->|\"{label}\"| B"));
    let out = mermaid::export(&d, 0).unwrap();
    assert_eq!(read(&out.source).connectors[0].label, label);
    d.connectors.clear();
    d.nodes.clear();
    for _ in 0..20 {
        let id = d.add_node(ShapeKind::Rectangle, [100., 100.]);
        d.node_mut(id).unwrap().text = "&".repeat(4096);
    }
    assert!(mermaid::export(&d, 0).unwrap_err().contains("256 KiB"));
    let errors =
        mermaid::import(&format!("flowchart TB\nA[\"{}\"]", "<br/>".repeat(4097))).unwrap_err();
    assert_eq!(errors[0].code, "label_limit");
}
#[test]
fn only_visible_charts_and_drawings_are_reported_as_omitted() {
    use diagcraft::{chart::ChartKind, document::DrawingKind};
    let mut d = read("flowchart TB\nA[Visible]");
    let visible_chart = d.add_chart(ChartKind::Columns, [300., 300.]);
    let hidden_chart = d.add_chart(ChartKind::Bars, [600., 300.]);
    let visible_drawing = d.add_drawing(DrawingKind::Line, [0., 0.], [100., 100.]);
    let hidden_drawing = d.add_drawing(DrawingKind::Line, [30., 30.], [200., 200.]);
    let layer = d.create_layer("Hidden".into()).unwrap();
    d.move_objects_to_layer(&[hidden_chart, hidden_drawing], layer)
        .unwrap();
    d.update_layer(layer, None, Some(false), None).unwrap();
    let out = mermaid::export(&d, 0).unwrap();
    let charts: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|n| n.code == "chart_omitted")
        .map(|n| n.id)
        .collect();
    let drawings: Vec<_> = out
        .diagnostics
        .iter()
        .filter(|n| n.code == "drawing_omitted")
        .map(|n| n.id)
        .collect();
    assert_eq!(charts, vec![Some(visible_chart)]);
    assert_eq!(drawings, vec![Some(visible_drawing)]);
}
#[test]
fn parallel_undirected_edges_survive_explicit_redefinitions() {
    let d = read(
        "flowchart TB\nA[Initial] ---|first| B\nA ---|second| B\nA --- B\nA{Latest}\nB[Finish]",
    );
    assert_eq!(d.nodes.len(), 2);
    assert_eq!(d.nodes[0].text, "Latest");
    assert_eq!(d.nodes[0].kind, ShapeKind::Diamond);
    assert_eq!(d.connectors.len(), 3);
    assert!(
        d.connectors
            .iter()
            .all(|e| !e.arrow && e.from == d.nodes[0].id && e.to == d.nodes[1].id)
    );
    let back = read(&mermaid::export(&d, 0).unwrap().source);
    assert_eq!(
        back.connectors
            .iter()
            .map(|e| e.label.as_str())
            .collect::<Vec<_>>(),
        vec!["first", "second", ""]
    );
}
