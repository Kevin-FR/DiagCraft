use diagcraft::{agent, document::*, pdf, svg, text_layout};
use serde_json::json;

fn node(text: &str) -> Node {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    let mut node = doc.node(id).unwrap().clone();
    node.text = text.into();
    node
}

#[test]
fn font_metrics_distinguish_wide_narrow_and_chinese_glyphs() {
    let narrow = node("iiiiiiii").text_layout().width;
    let wide = node("WWWWWWWW").text_layout().width;
    assert!(wide > narrow * 2.5);
    let chinese = node("中文绘图");
    assert!(chinese.text_layout().width >= chinese.font_size * 4.);
    let mut mono_i = node("iiiiiiii");
    let mut mono_w = node("WWWWWWWW");
    mono_i.text_style.monospace = true;
    mono_w.text_style.monospace = true;
    assert!((mono_i.text_layout().width - mono_w.text_layout().width).abs() < 0.01);
    let mut bold = node("Bold letters");
    let before = bold.text_layout().width;
    bold.text_style.bold = true;
    assert!(bold.text_layout().width > before);
}

#[test]
fn wrapping_preserves_spaces_paragraphs_tabs_and_combining_clusters() {
    let mut n = node("  Cafe\u{301}  中文\n\nA\tB\r\nlast  ");
    n.size[0] = 70.;
    n.text_style.wrap = true;
    let text = n.text_layout();
    assert_eq!(
        text.text().replace('\n', ""),
        n.text.replace(['\n', '\r'], "").replace('\t', "    ")
    );
    assert!(text.lines.iter().all(|line| !line.starts_with('\u{301}')));
    assert!(text.text().contains("e\u{301}"));
    assert!(text.text().contains("\n\n"));
    assert_eq!(n.text, "  Cafe\u{301}  中文\n\nA\tB\r\nlast  ");
}

#[test]
fn wide_letters_wrap_earlier_than_narrow_letters_of_the_same_count() {
    let mut narrow = node("iiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiiii");
    let mut wide = node("WWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWWW");
    narrow.text_style.wrap = true;
    wide.text_style.wrap = true;
    assert!(wide.text_layout().lines.len() > narrow.text_layout().lines.len());
}

#[test]
fn fit_is_idempotent_and_keeps_text_position_style_and_connection_ids() {
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Diamond, [300., 200.]);
    let b = doc.add_node(ShapeKind::Rectangle, [600., 200.]);
    let edge = doc.connect(a, b, ConnectorKind::Straight).unwrap();
    let n = doc.node_mut(a).unwrap();
    n.text = "A long decision in English, français, español and 中文".into();
    n.text_style.wrap = true;
    let before = n.clone();
    assert_eq!(doc.fit_nodes_to_text(&[a]).unwrap(), 1);
    let n = doc.node(a).unwrap();
    assert_eq!(n.position, before.position);
    assert_eq!(n.size[0], before.size[0]);
    assert_eq!(n.text, before.text);
    assert_eq!(n.text_style, before.text_style);
    assert_eq!(doc.connectors[0].id, edge);
    assert_eq!(doc.connectors[0].from, a);
    let area = text_layout::area(n).1;
    let text = n.text_layout();
    assert!(text.width <= area[0] && text.height <= area[1]);
    assert_eq!(doc.fit_nodes_to_text(&[a]).unwrap(), 0);
    let route = doc.route(&doc.connectors[0]);
    assert_eq!(
        route[0],
        doc.node(a)
            .unwrap()
            .boundary_towards(doc.node(b).unwrap().center())
    );
    doc.validate().unwrap();
}

#[test]
fn unwrapped_fit_resizes_both_dimensions_and_small_width_fits_a_whole_grapheme() {
    let mut n = node("A much longer line than the initial block width\nSecond line");
    let width = n.size[0];
    n.fit_to_text().unwrap();
    assert!(n.size[0] > width);
    assert!(n.text_layout().width <= text_layout::area(&n).1[0]);
    let mut chinese = node("中文");
    chinese.size = [24., 24.];
    chinese.font_size = 64.;
    chinese.text_style.wrap = true;
    chinese.fit_to_text().unwrap();
    assert!(chinese.text_layout().width <= text_layout::area(&chinese).1[0]);
}

#[test]
fn fitting_selection_is_atomic_and_never_clamps_oversized_text_silently() {
    let mut doc = Document::default();
    let a = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    let b = doc.add_node(ShapeKind::Rectangle, [500., 200.]);
    doc.node_mut(a).unwrap().text = "short".into();
    doc.node_mut(b).unwrap().text = "\n".repeat(65_000);
    let before = doc.clone();
    assert!(doc.fit_nodes_to_text(&[a, b]).is_err());
    assert_eq!(doc, before);
    assert!(doc.node_mut(b).unwrap().fit_to_text().is_err());
    assert_eq!(doc, before);
}

#[test]
fn opt_in_round_trips_on_multiple_pages_and_default_styles_keep_legacy_serialization() {
    let default = serde_json::to_value(TextStyle::default()).unwrap();
    assert!(default.get("auto_fit").is_none());
    let legacy: TextStyle = serde_json::from_value(default).unwrap();
    assert!(!legacy.auto_fit);
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rounded, [200., 200.]);
    let n = doc.node_mut(id).unwrap();
    n.text_style.auto_fit = true;
    n.text_style.wrap = true;
    doc.fit_automatic_text().unwrap();
    doc.add_page(true).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.draw");
    doc.save(&path).unwrap();
    assert_eq!(Document::load(&path).unwrap(), doc);
    assert_eq!(doc.format_version, 7);
    assert!(doc.page_document(0).unwrap().nodes[0].text_style.auto_fit);
    assert!(doc.page_document(1).unwrap().nodes[0].text_style.auto_fit);
}

#[test]
fn agent_changes_fit_before_reporting_and_do_not_hide_invalid_geometry() {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    doc.node_mut(id).unwrap().text_style.auto_fit = true;
    doc.node_mut(id).unwrap().text_style.wrap = true;
    let patch = |changes| {
        agent::parse_patch(&json!({"api_version":1,"expected_revision":agent::revision(&doc),"operations":[{"op":"update_node","page":0,"target":id,"changes":changes}]}).to_string()).unwrap()
    };
    let preview = agent::preview(
        &doc,
        &patch(json!({"text":"Long text 中文 français ".repeat(20)})),
    )
    .unwrap();
    let n = preview.document.node(id).unwrap();
    assert_eq!(n.size[0], doc.node(id).unwrap().size[0]);
    assert!(n.size[1] > doc.node(id).unwrap().size[1]);
    assert_eq!(
        preview.report["revision"],
        agent::revision(&preview.document)
    );
    let context = agent::inspect(&preview.document).unwrap();
    assert_eq!(
        context["pages"][0]["nodes"][0]["text_layout"]["overflow"],
        false
    );
    assert!(agent::preview(&doc, &patch(json!({"size":[0,84]}))).is_err());
    assert!(agent::preview(&doc, &patch(json!({"font_size":201}))).is_err());
    assert!(agent::parse_patch(&json!({"api_version":1,"expected_revision":agent::revision(&doc),"operations":[{"op":"update_node","page":0,"target":id,"changes":{"text_style":{"auto_fit":true,"wrap":true}}}]}).to_string()).is_ok());
}

#[test]
fn svg_pdf_share_explicit_lines_and_bundled_font_families() {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    let n = doc.node_mut(id).unwrap();
    n.text = "English français Deutsch español italiano português 中文".into();
    n.text_style.wrap = true;
    n.text_style.monospace = true;
    n.fit_to_text().unwrap();
    let lines = n.text_layout().lines;
    let exported = svg::export(&doc).unwrap();
    assert_eq!(exported.matches("<tspan ").count(), lines.len());
    for line in &lines {
        assert!(exported.contains(&format!(">{line}</tspan>")));
    }
    assert!(exported.contains("'Hack','Noto Sans SC',monospace"));
    assert!(exported.contains("xml:space=\"preserve\""));
    assert!(
        pdf::export(&doc, pdf::PageFormat::Drawing)
            .unwrap()
            .starts_with(b"%PDF-")
    );
}
