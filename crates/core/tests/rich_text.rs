use diagcraft::{
    agent,
    document::*,
    pdf,
    rich_text::{InlineStyle, MAX_RUNS, TextRun},
    svg, text_layout,
};
use serde_json::json;

fn example() -> (Document, u64) {
    let mut doc = Document::default();
    let id = doc.add_node(ShapeKind::Rounded, [350., 250.]);
    let n = doc.node_mut(id).unwrap();
    n.text = "test Le BLOC\n\nSuperbe 中文 e\u{301} 👩‍💻".into();
    n.size = [440., 160.];
    (doc, id)
}
fn range(text: &str, word: &str) -> std::ops::Range<usize> {
    let byte = text.find(word).unwrap();
    let start = text[..byte].chars().count();
    start..start + word.chars().count()
}
#[test]
fn mixed_styles_overlay_and_reset_only_the_selected_words() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    n.format_text(
        range(&n.text, "Le"),
        Some(&InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    n.format_text(
        range(&n.text, "Superbe"),
        Some(&InlineStyle {
            italic: Some(true),
            underline: Some(true),
            font_size: Some(28.),
            color: Some([150, 20, 60]),
            ..Default::default()
        }),
    )
    .unwrap();
    let be = range(&n.text, "be");
    n.format_text(
        be.clone(),
        Some(&InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    assert!(!n.resolved_style_at(0).text_style.bold);
    assert!(n.resolved_style_at(5).text_style.bold);
    let style = n.resolved_style_at(be.start);
    assert!(style.text_style.bold && style.text_style.italic && style.text_style.underline);
    assert_eq!(style.font_size, 28.);
    assert_eq!(style.color, [150, 20, 60]);
    n.format_text(be.clone(), None).unwrap();
    assert_eq!(n.resolved_style_at(be.start).font_size, n.font_size);
    assert!(n.resolved_style_at(be.start - 1).text_style.italic);
    n.validate_text_runs().unwrap();
}
#[test]
fn edits_shift_unicode_ranges_preserve_other_words_and_inherit_at_the_caret() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    n.format_text(
        range(&n.text, "中文"),
        Some(&InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    n.set_text(format!("Préfixe {}", n.text)).unwrap();
    let at = range(&n.text, "中文").start;
    assert_eq!(n.text_runs[0].start, at);
    n.set_text(n.text.replace("中文", "中绘文")).unwrap();
    assert!((at..at + 3).all(|i| n.resolved_style_at(i).text_style.bold));
    n.set_text(n.text.replace("中绘文", "")).unwrap();
    assert!(n.text_runs.is_empty());
    n.set_text("".into()).unwrap();
    assert!(n.text.is_empty());
    let before = n.clone();
    assert!(n.set_text("中".repeat(65_537)).is_err());
    assert_eq!(*n, before);
}
#[test]
fn formatting_never_splits_combining_accents_or_emoji_and_invalid_ranges_are_rejected() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    let accent = range(&n.text, "e\u{301}");
    n.format_text(
        accent.start + 1..accent.end,
        Some(&InlineStyle {
            underline: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(n.text_runs[0].start, accent.start);
    let emoji = range(&n.text, "👩‍💻");
    n.format_text(
        emoji.start + 1..emoji.start + 2,
        Some(&InlineStyle {
            italic: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    assert_eq!(n.text_runs.last().unwrap().end, emoji.end);
    n.validate_text_runs().unwrap();
    for runs in [
        vec![TextRun {
            start: accent.start + 1,
            end: accent.end,
            style: Default::default(),
        }],
        vec![
            TextRun {
                start: 0,
                end: 3,
                style: Default::default(),
            },
            TextRun {
                start: 2,
                end: 5,
                style: Default::default(),
            },
        ],
        vec![TextRun {
            start: 0,
            end: 300,
            style: Default::default(),
        }],
        vec![TextRun {
            start: 0,
            end: 1,
            style: InlineStyle {
                font_size: Some(f32::NAN),
                ..Default::default()
            },
        }],
    ] {
        n.text_runs = runs;
        assert!(n.validate_text_runs().is_err());
    }
}
#[test]
fn inline_font_sizes_drive_wrapping_fitting_and_styled_line_fragments() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    n.text_style.wrap = true;
    n.size = [210., 80.];
    let plain = n.text_layout();
    n.format_text(
        range(&n.text, "Superbe 中文"),
        Some(&InlineStyle {
            font_size: Some(48.),
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    let rich = n.text_layout();
    assert!(rich.height > plain.height);
    assert!(rich.line_height > plain.line_height);
    assert_eq!(rich.lines.len(), rich.runs.len());
    assert_eq!(rich.text().replace('\n', ""), n.text.replace('\n', ""));
    assert!(
        rich.runs
            .iter()
            .flatten()
            .any(|r| r.style.font_size == 48. && r.style.text_style.bold)
    );
    n.fit_to_text().unwrap();
    let fitted = n.text_layout();
    let area = text_layout::area(n).1;
    assert!(fitted.width <= area[0] && fitted.height <= area[1]);
}
#[test]
fn rich_text_round_trips_on_multiple_pages_and_old_documents_keep_plain_text() {
    let (mut doc, id) = example();
    doc.node_mut(id)
        .unwrap()
        .format_text(
            5..7,
            Some(&InlineStyle {
                bold: Some(true),
                ..Default::default()
            }),
        )
        .unwrap();
    let first = doc.nodes[0].clone();
    doc.add_page(false).unwrap();
    let second = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
    assert!(doc.node(second).unwrap().text_runs.is_empty());
    let value = serde_json::to_value(&doc).unwrap();
    assert_eq!(value["format_version"], 7);
    let loaded: Document = serde_json::from_value(value).unwrap();
    loaded.validate().unwrap();
    assert_eq!(loaded.page_document(0).unwrap().nodes[0], first);
    let mut legacy = serde_json::to_value(&first).unwrap();
    legacy.as_object_mut().unwrap().remove("text_runs");
    let plain: Node = serde_json::from_value(legacy).unwrap();
    assert_eq!(plain.text, first.text);
    assert!(
        !serde_json::to_value(plain)
            .unwrap()
            .as_object()
            .unwrap()
            .contains_key("text_runs")
    );
}
#[test]
fn agent_inline_edits_are_strict_revision_checked_and_atomic() {
    let (doc, id) = example();
    let patch = |changes| {
        json!({"api_version":1,"expected_revision":agent::revision(&doc),"operations":[{"op":"set_title","title":"changed"},{"op":"update_node","page":0,"target":id,"changes":changes}]}).to_string()
    };
    let invalid = agent::parse_patch(&patch(
        json!({"text_runs":[{"start":2,"end":1,"style":{"bold":true}}]}),
    ))
    .unwrap();
    let before = doc.clone();
    assert!(agent::preview(&doc, &invalid).is_err());
    assert_eq!(doc, before);
    assert!(
        agent::parse_patch(&patch(
            json!({"text_runs":[{"start":0,"end":2,"style":{"bould":true}}]})
        ))
        .is_err()
    );
    let valid=agent::parse_patch(&patch(json!({"text":"中文\nSuperbe","text_runs":[{"start":0,"end":2,"style":{"bold":true,"font_size":32}}],"text_style":{"wrap":true,"auto_fit":true}}))).unwrap();
    let preview = agent::preview(&doc, &valid).unwrap();
    let inspected = agent::inspect(&preview.document).unwrap();
    assert_eq!(inspected["pages"][0]["nodes"][0]["text_runs"][0]["end"], 2);
    assert_eq!(
        inspected["pages"][0]["nodes"][0]["text_layout"]["overflow"],
        false
    );
    let mut stale = valid;
    stale.expected_revision = "sha256:wrong".into();
    assert!(agent::preview(&doc, &stale).is_err());
}
#[test]
fn svg_and_pdf_keep_inline_styles_blank_lines_and_escaped_user_content() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    n.format_text(
        range(&n.text, "Le"),
        Some(&InlineStyle {
            bold: Some(true),
            ..Default::default()
        }),
    )
    .unwrap();
    n.format_text(
        range(&n.text, "Superbe"),
        Some(&InlineStyle {
            italic: Some(true),
            underline: Some(true),
            color: Some([170, 30, 60]),
            font_size: Some(28.),
            ..Default::default()
        }),
    )
    .unwrap();
    n.set_text(format!("{} <script>&", n.text)).unwrap();
    let before = doc.clone();
    let source = svg::export(&doc).unwrap();
    assert!(source.contains("font-weight=\"700\"") && source.contains("font-style=\"italic\""));
    assert!(
        source.contains("font-size=\"28\"") && source.contains("text-decoration=\"underline\"")
    );
    assert!(source.contains("&lt;script&gt;&amp;") && !source.contains("<script>"));
    let bytes = pdf::export(&doc, pdf::PageFormat::Drawing).unwrap();
    let pdf = String::from_utf8_lossy(&bytes);
    assert!(
        pdf.contains("/ToUnicode")
            && pdf.contains("NotoSansSC")
            && !pdf.contains("/Subtype /Image")
    );
    assert_eq!(doc, before);
}
#[test]
fn inline_run_budget_is_enforced_without_partially_changing_the_node() {
    let (mut doc, id) = example();
    let n = doc.node_mut(id).unwrap();
    n.text = "ab".repeat(MAX_RUNS + 1);
    n.text_runs = (0..MAX_RUNS)
        .map(|i| TextRun {
            start: i * 2,
            end: i * 2 + 1,
            style: InlineStyle {
                bold: Some(true),
                ..Default::default()
            },
        })
        .collect();
    let before = n.clone();
    assert!(
        n.format_text(
            MAX_RUNS * 2..MAX_RUNS * 2 + 1,
            Some(&InlineStyle {
                bold: Some(true),
                ..Default::default()
            })
        )
        .is_err()
    );
    assert_eq!(*n, before);
}
