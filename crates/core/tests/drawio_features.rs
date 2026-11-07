//! Feature tests for the bounded `.drawio` import subset (see docs/drawio.md).
use diagcraft::drawio::{self, Import};

fn sample() -> &'static str {
    r#"<mxfile host="app" type="device">
  <diagram id="p1" name="Page 1">
    <mxGraphModel dx="0" dy="0" grid="1" gridSize="10" page="1">
      <root>
        <mxCell id="0"/>
        <mxCell id="1" parent="0"/>
        <mxCell id="a" value="Début" vertex="1" parent="1" style="shape=rounded;fillColor=#dae8fc;strokeColor=#6c8ebf;">
          <mxGeometry x="40" y="40" width="120" height="60" as="geometry"/>
        </mxCell>
        <mxCell id="b" value="Choix &amp; test&lt;br&gt;ligne 2" vertex="1" parent="1" style="rhombus;fillColor=#d5e8d4;">
          <mxGeometry x="220" y="150" width="140" height="90" as="geometry"/>
        </mxCell>
        <mxCell id="e1" edge="1" parent="1" source="a" target="b" style="edgeStyle=orthogonalEdgeStyle;">
          <mxGeometry relative="1" as="geometry"/>
        </mxCell>
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>"#
}

#[test]
fn imports_vertices_and_edges() {
    let Import {
        document,
        diagnostics,
    } = drawio::import(sample(), 0).expect("import succeeds");
    assert!(
        diagnostics.is_empty(),
        "unexpected diagnostics: {diagnostics:?}"
    );
    assert_eq!(document.nodes.len(), 2);
    assert_eq!(document.connectors.len(), 1);
    let a = document
        .nodes
        .iter()
        .find(|n| n.text == "Début")
        .expect("node A");
    assert_eq!(a.kind, diagcraft::document::ShapeKind::Rounded);
    assert_eq!(a.position, [40., 40.]);
    assert_eq!(a.size, [120., 60.]);
    // The label contains a decoded `&` and an `&lt;br&gt;` turned into a line break.
    let b = document
        .nodes
        .iter()
        .find(|n| n.text.contains("Choix"))
        .expect("node B");
    assert_eq!(b.kind, diagcraft::document::ShapeKind::Diamond);
    assert!(b.text.starts_with("Choix & test"));
    assert!(b.text.contains('\n'), "br becomes a line break");
    let conn = &document.connectors[0];
    assert_eq!(conn.from, a.id);
    assert_eq!(conn.to, b.id);
}

#[test]
fn lists_pages() {
    let names = drawio::pages(sample()).expect("pages parsed");
    assert_eq!(names, vec!["Page 1"]);
}

#[test]
fn bare_graph_model_is_one_page() {
    let src = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
      <mxCell id="a" value="A" vertex="1" parent="1"><mxGeometry x="0" y="0" width="50" height="30" as="geometry"/></mxCell>
    </root></mxGraphModel>"#;
    assert_eq!(drawio::pages(src).unwrap(), vec!["Diagram"]);
    let Import { document, .. } = drawio::import(src, 0).unwrap();
    assert_eq!(document.nodes.len(), 1);
    assert_eq!(document.nodes[0].text, "A");
}

#[test]
fn out_of_range_page_rejected() {
    let err = drawio::import(sample(), 5).unwrap_err();
    assert!(err.iter().any(|n| n.code == "page_out_of_range"));
}

#[test]
fn multi_page_skips_others_with_warning() {
    let src = r#"<mxfile>
      <diagram id="a" name="Alpha"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
        <mxCell id="x" value="Alpha" vertex="1" parent="1"><mxGeometry x="0" y="0" width="40" height="40" as="geometry"/></mxCell>
      </root></mxGraphModel></diagram>
      <diagram id="b" name="Beta"><mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
        <mxCell id="y" value="Beta" vertex="1" parent="1"><mxGeometry x="0" y="0" width="40" height="40" as="geometry"/></mxCell>
      </root></mxGraphModel></diagram>
    </mxfile>"#;
    let Import {
        document,
        diagnostics,
    } = drawio::import(src, 1).unwrap();
    assert_eq!(document.nodes.len(), 1);
    assert_eq!(document.nodes[0].text, "Beta");
    assert!(diagnostics.iter().any(|n| n.code == "page_skipped"));
}

#[test]
fn external_image_is_dropped_with_warning() {
    let src = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
      <mxCell id="a" value="A" vertex="1" parent="1" style="shape=image;image=https://example.com/x.png">
        <mxGeometry x="0" y="0" width="50" height="30" as="geometry"/></mxCell>
    </root></mxGraphModel>"#;
    let Import {
        document,
        diagnostics,
    } = drawio::import(src, 0).unwrap();
    assert_eq!(document.nodes.len(), 1);
    assert!(document.nodes[0].image.is_none());
    assert!(diagnostics.iter().any(|n| n.code == "image_external"));
}

#[test]
fn dangling_edge_is_skipped_with_warning() {
    let src = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
      <mxCell id="a" value="A" vertex="1" parent="1"><mxGeometry x="0" y="0" width="50" height="30" as="geometry"/></mxCell>
      <mxCell id="e" edge="1" parent="1" source="a" style="endArrow=none;"><mxGeometry relative="1" as="geometry"/></mxCell>
    </root></mxGraphModel>"#;
    let Import {
        document,
        diagnostics,
    } = drawio::import(src, 0).unwrap();
    assert_eq!(document.connectors.len(), 0);
    assert!(diagnostics.iter().any(|n| n.code == "edge_dangling"));
}

#[test]
fn unknown_shape_imported_as_rectangle() {
    let src = r#"<mxGraphModel><root><mxCell id="0"/><mxCell id="1" parent="0"/>
      <mxCell id="a" value="A" vertex="1" parent="1" style="shape=weirdShape">
        <mxGeometry x="0" y="0" width="50" height="30" as="geometry"/></mxCell>
    </root></mxGraphModel>"#;
    let Import { document, .. } = drawio::import(src, 0).unwrap();
    assert_eq!(
        document.nodes[0].kind,
        diagcraft::document::ShapeKind::Rectangle
    );
}

#[test]
fn source_over_limit_is_error() {
    let big = " ".repeat(512 * 1024 + 1);
    let err = drawio::import(&big, 0).unwrap_err();
    assert!(err.iter().any(|n| n.code == "source_limit"));
}

#[test]
fn malformed_xml_is_error() {
    let err = drawio::import("<mxfile><diagram>", 0).unwrap_err();
    assert!(err.iter().any(|n| n.code == "xml_parse"));
}

#[test]
fn non_drawio_is_error() {
    let err = drawio::import("<html><body/></html>", 0).unwrap_err();
    assert!(err.iter().any(|n| n.code == "not_drawio"));
}
