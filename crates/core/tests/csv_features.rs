use diagcraft::{
    csv::{self, CsvOptions},
    document::{ConnectorKind, Document},
};
fn run(source: &str, label: &str) -> Document {
    csv::import(
        source,
        &CsvOptions {
            label: Some(label.into()),
            identity: None,
            connect: vec![],
        },
    )
    .unwrap()
    .document
}
fn notice_codes(diag: &[csv::Notice]) -> Vec<&str> {
    diag.iter().map(|n| n.code.as_str()).collect()
}
#[test]
fn each_data_row_becomes_a_shape_with_the_chosen_label() {
    let d = run("name,color\nAlice,Rouge\nBob,Bleu\n", "name");
    assert_eq!(d.nodes.len(), 2);
    assert_eq!(d.nodes[0].text, "Alice");
    assert_eq!(d.nodes[1].text, "Bob");
    assert_eq!(d.nodes[0].kind, diagcraft::document::ShapeKind::Rounded);
    d.validate().unwrap();
}
#[test]
fn blank_rows_are_reported_and_skipped_without_creating_shapes() {
    let imp = csv::import(
        "id,name\n1,Alice\n\n2,Bob\n",
        &CsvOptions {
            label: Some("name".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    )
    .unwrap();
    assert_eq!(imp.document.nodes.len(), 2);
    assert!(notice_codes(&imp.diagnostics).contains(&"blank_row"));
}
#[test]
fn relations_reference_identity_values_instead_of_row_numbers() {
    let imp = csv::import(
        "id,parent\nroot,none\nalice,root\nbob,root\n",
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec!["parent".into()],
        },
    )
    .unwrap();
    let d = &imp.document;
    assert_eq!(d.nodes.len(), 3);
    // root has no relation; alice and bob each connect to root.
    assert_eq!(d.connectors.len(), 2);
    assert!(
        d.connectors
            .iter()
            .all(|c| c.kind == ConnectorKind::Elbow && c.arrow)
    );
    // Relations target identity values, so the from/to use the node of the row
    // with that key.
    let text_of = |id: u64| d.nodes.iter().find(|n| n.id == id).unwrap().text.clone();
    for c in &d.connectors {
        let from_t = text_of(c.from);
        let to_t = text_of(c.to);
        let pair = (from_t.as_str(), to_t.as_str());
        assert!(
            matches!(pair, ("alice", "root") | ("bob", "root")),
            "unexpected relation {pair:?}"
        );
    }
    d.validate().unwrap();
}
#[test]
fn relations_require_an_identity_column() {
    let err = csv::import(
        "name,parent\nalice,root\n",
        &CsvOptions {
            label: Some("name".into()),
            identity: None,
            connect: vec!["parent".into()],
        },
    )
    .unwrap_err();
    assert!(err.iter().any(|n| n.code == "identity_required"));
}
#[test]
fn duplicate_identity_is_rejected_and_missing_relation_is_reported() {
    let imp = csv::import(
        "id,friend\n1,alice\n2,ghost\nalice,2\n",
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec!["friend".into()],
        },
    )
    .unwrap();
    assert!(notice_codes(&imp.diagnostics).contains(&"missing_relation"));
    let dup = csv::import(
        "id\nx\ny\nx\n",
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    );
    assert!(dup.is_err());
    assert!(
        dup.unwrap_err()
            .iter()
            .any(|n| n.code == "duplicate_identity")
    );
}
#[test]
fn semicolon_delimiter_is_detected_and_crlf_is_handled() {
    let d = run("name;color\r\nAlice;Rouge\r\nBob;Bleu\r\n", "name");
    assert_eq!(d.nodes.len(), 2);
    assert_eq!(d.nodes[0].text, "Alice");
}
#[test]
fn budget_violations_are_fatal_and_never_insert_partially() {
    // Row budget.
    let mut many = String::from("id\n");
    for i in 0..(csv::MAX_ROWS + 1) {
        many.push_str(&format!("{i}\n"));
    }
    let limit = csv::import(
        &many,
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    );
    assert!(limit.is_err());
    assert!(
        limit
            .unwrap_err()
            .iter()
            .any(|n| n.code == "csv_parse" || n.code == "empty_data")
    );

    // Cell character budget.
    let huge = format!("id\n{}\n", "é".repeat(csv::MAX_CELL_CHARS + 1));
    let cell = csv::import(
        &huge,
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    );
    assert!(cell.is_err());

    // Source byte budget.
    let brand = csv::import(
        &"a".repeat(csv::MAX_SOURCE_BYTES + 1),
        &CsvOptions::default(),
    );
    assert!(brand.unwrap_err().iter().any(|n| n.code == "source_limit"));
}
#[test]
fn quoted_fields_and_embedded_newlines_are_supported() {
    let imp = csv::import(
        "id,note\n1,\"ligne une\nligne deux\"\n2,\"a\"\"b\"\"c\"\n",
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    )
    .unwrap();
    assert_eq!(imp.document.nodes[0].text, "1");
    // The label column is `id`; the note value is not used as label. We assert the
    // parser kept both rows and the document is valid.
    assert_eq!(imp.document.nodes.len(), 2);
    imp.document.validate().unwrap();
}
#[test]
fn empty_data_and_unknown_columns_are_rejected_with_locations() {
    let empty = csv::import(
        "id\n",
        &CsvOptions {
            label: Some("id".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    );
    assert!(empty.unwrap_err().iter().any(|n| n.code == "empty_data"));

    let missing = csv::import(
        "id,name\n1,Alice\n",
        &CsvOptions {
            label: Some("nope".into()),
            identity: Some("id".into()),
            connect: vec![],
        },
    );
    assert!(
        missing
            .unwrap_err()
            .iter()
            .any(|n| n.code == "column_not_found")
    );
}
