use diagcraft::{
    document::{Document, FORMAT_VERSION, ShapeKind},
    gallery::Gallery,
    geometry,
    library::Library,
    pdf::{self, PageFormat},
    svg, symbols, templates,
};
use std::path::Path;
fn area(p: &[[f32; 2]]) -> f32 {
    p.iter()
        .zip(p.iter().cycle().skip(1))
        .take(p.len())
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum::<f32>()
        .abs()
        / 2.
}
#[test]
fn all_catalog_symbols_render_hit_and_round_trip() {
    for k in symbols::all_kinds() {
        let g = geometry::shape(k);
        let triangles = geometry::triangulate(&g.outline);
        let sum = triangles
            .iter()
            .map(|t| area(&[g.outline[t[0]], g.outline[t[1]], g.outline[t[2]]]))
            .sum::<f32>();
        assert!(
            (sum - area(&g.outline)).abs() < 0.0001,
            "{k:?}: {sum} vs {}",
            area(&g.outline)
        );
        assert!(geometry::contains(&g.outline, [0.5, 0.5]), "{k:?} center");
        let mut n = symbols::preset(k);
        n.id = 1;
        n.position = [100., 100.];
        assert!(n.contains(n.center()), "{k:?}");
        let doc = Document {
            nodes: vec![n],
            ..Default::default()
        };
        doc.validate().unwrap();
        let clone: Document = serde_json::from_slice(&serde_json::to_vec(&doc).unwrap()).unwrap();
        assert_eq!(clone, doc);
        assert!(!svg::export(&doc).unwrap().contains("NaN"));
    }
    symbols::demo().validate().unwrap();
}
#[test]
fn hidden_libraries_survive_restart_and_leave_documents_intact() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("library.json");
    let mut library = Library::default();
    let doc = symbols::demo();
    library.visible_libraries = vec!["uml".into()];
    library.add_template("Mes formes", "UML", &doc).unwrap();
    library.save(&path).unwrap();
    let read = Library::load(&path).unwrap();
    assert!(read.visible("uml"));
    assert!(!read.visible("flowchart"));
    assert_eq!(read.templates[0].document, doc);
    library.visible_libraries.clear();
    library.save(&path).unwrap();
    assert!(Library::load(&path).unwrap().visible_libraries.is_empty());
}
#[test]
fn legacy_preferences_and_documents_migrate() {
    let library: Library = serde_json::from_str("{\"symbols\":[],\"templates\":[]}").unwrap();
    assert!(library.visible("flowchart"));
    for version in 1..FORMAT_VERSION {
        let mut doc = Document::demo();
        doc.format_version = version;
        doc.upgrade_legacy();
        assert_eq!(doc.format_version, FORMAT_VERSION);
        doc.validate().unwrap();
    }
}
#[test]
fn unknown_or_duplicate_library_identifiers_are_rejected() {
    let mut lib = Library {
        visible_libraries: vec!["../bad".into()],
        ..Default::default()
    };
    assert!(lib.validate().is_err());
    lib.visible_libraries = vec!["uml".into(), "uml".into()];
    assert!(lib.validate().is_err());
}
#[test]
fn gallery_preserves_original_deduplicates_and_restores_after_restart() {
    let temp = tempfile::tempdir().unwrap();
    let file = temp.path().join("work.draw");
    let root = temp.path().join("gallery");
    let original = templates::support();
    original.save(&file).unwrap();
    let mut gallery = Gallery::open(root.clone()).unwrap();
    gallery.preserve_existing(&file).unwrap();
    let mut changed = original.clone();
    changed.title = "Modifié".into();
    changed.save(&file).unwrap();
    gallery.record(&file, &changed).unwrap();
    gallery.record(&file, &changed).unwrap();
    let key = gallery.projects()[0].key.clone();
    assert_eq!(gallery.projects()[0].revisions.len(), 2);
    drop(gallery);
    let gallery = Gallery::open(root).unwrap();
    assert_eq!(gallery.revision(&key, 1).unwrap(), original);
    assert_eq!(gallery.revision(&key, 0).unwrap(), changed);
    std::fs::remove_file(&file).unwrap();
    assert_eq!(gallery.revision(&key, 0).unwrap(), changed);
}
#[test]
fn gallery_retains_ten_versions_and_only_removes_owned_files() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("gallery");
    std::fs::create_dir(&root).unwrap();
    let unrelated = root.join("leave-me.draw");
    std::fs::write(&unrelated, b"keep").unwrap();
    let mut gallery = Gallery::open(root.clone()).unwrap();
    for i in 0..15 {
        let doc = Document {
            title: format!("Version {i}"),
            ..Default::default()
        };
        gallery.record(&temp.path().join("a.draw"), &doc).unwrap();
    }
    let p = &gallery.projects()[0];
    assert_eq!(p.revisions.len(), 10);
    assert_eq!(gallery.revision(&p.key, 9).unwrap().title, "Version 5");
    assert_eq!(std::fs::read(&unrelated).unwrap(), b"keep");
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 12);
    for i in 0..42 {
        gallery
            .record(
                &temp.path().join(format!("project-{i}.draw")),
                &Document::default(),
            )
            .unwrap();
    }
    assert_eq!(gallery.projects().len(), 40);
    assert!(unrelated.exists());
}
#[test]
fn gallery_index_is_atomic_and_changed_snapshots_are_detected() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("gallery");
    let mut gallery = Gallery::open(root.clone()).unwrap();
    gallery
        .record(Path::new("same.draw"), &Document::demo())
        .unwrap();
    let p = gallery.projects()[0].clone();
    let name = format!("{}-{}.draw", p.key, p.revisions[0].hash);
    std::fs::write(root.join(name), b"{}").unwrap();
    assert!(gallery.revision(&p.key, 0).unwrap_err().contains("altérée"));
    let index = root.join("index-v1.json");
    std::fs::remove_file(&index).unwrap();
    std::fs::create_dir(&index).unwrap();
    let doc = Document {
        title: "Other".into(),
        ..Default::default()
    };
    assert!(gallery.record(Path::new("other.draw"), &doc).is_err());
    assert_eq!(gallery.projects().len(), 1);
}
#[test]
fn gallery_rejects_unsafe_index_filenames() {
    let temp = tempfile::tempdir().unwrap();
    let bad = serde_json::json!({"projects":[{"key":"../../other","title":"Bad","path":temp.path().join("a.draw"),"revisions":[{"hash":"../secret","saved_at":0,"bytes":10}]}]});
    std::fs::write(
        temp.path().join("index-v1.json"),
        serde_json::to_vec(&bad).unwrap(),
    )
    .unwrap();
    assert!(Gallery::open(temp.path().into()).is_err());
    let mut disabled = Gallery::unavailable("Index invalide".into());
    assert!(
        disabled
            .record(Path::new("keep.draw"), &Document::demo())
            .is_err()
    );
    assert!(disabled.projects().is_empty());
}
#[test]
fn pdf_is_vector_embeds_fonts_and_rejects_invalid_documents() {
    let mut d = Document::demo();
    let id = d.add_node(ShapeKind::Text, [800., 700.]);
    let n = d.node_mut(id).unwrap();
    n.text = "Cœur, État et résolution".into();
    n.text_style.bold = true;
    let bytes = pdf::export(&d, PageFormat::A4Landscape).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(bytes.starts_with(b"%PDF-1."));
    assert!(text.contains("/FontFile"));
    assert!(!text.contains("/Subtype /Image"));
    assert!(text.contains("/ToUnicode"));
    d.page_size[0] = f32::NAN;
    assert!(pdf::export(&d, PageFormat::Drawing).is_err());
}

#[test]
fn chinese_and_latin_text_use_embedded_fonts_without_changing_the_document() {
    let mut d = Document::default();
    let id = d.add_node(ShapeKind::Text, [500., 400.]);
    d.node_mut(id).unwrap().text = "中文图表 Grüße Español Português".into();
    d.node_mut(id).unwrap().size = [800., 80.];
    let before = d.clone();
    let source = svg::export(&d).unwrap();
    assert!(source.contains("中文图表 Grüße Español Português"));
    assert!(source.contains("Noto Sans SC"));
    let bytes = pdf::export(&d, PageFormat::Drawing).unwrap();
    let text = String::from_utf8_lossy(&bytes);
    assert!(
        text.contains("NotoSansSC"),
        "Chinese fallback font must be embedded"
    );
    assert!(text.contains("/ToUnicode"));
    assert!(!text.contains("/Subtype /Image"));
    assert_eq!(d, before);
}
#[test]
fn paper_formats_preserve_page_orientation() {
    let d = Document::default();
    let [w, h] = PageFormat::A4Landscape.size_px(&d);
    assert!(w > h);
    assert!((w / h - 2_f32.sqrt()).abs() < 0.0001);
    let [w, h] = PageFormat::A4Portrait.size_px(&d);
    assert!(h > w);
    assert!(PageFormat::parse("unknown").is_err());
}
#[test]
fn pdf_exports_chart_data_manual_routes_and_monospace_without_rasterizing() {
    let mut d = templates::charts();
    let id = d.add_node(ShapeKind::UmlClass, [400., 700.]);
    let mut n = symbols::preset(ShapeKind::UmlClass);
    n.id = id;
    n.position = [280., 620.];
    *d.node_mut(id).unwrap() = n;
    let b = d.add_node(ShapeKind::Note, [800., 700.]);
    let edge = d
        .connect(id, b, diagcraft::document::ConnectorKind::Elbow)
        .unwrap();
    d.connectors
        .iter_mut()
        .find(|c| c.id == edge)
        .unwrap()
        .arrow_start = true;
    d.set_automatic_route(edge, false).unwrap();
    let pdf = pdf::export(&d, PageFormat::Drawing).unwrap();
    let text = String::from_utf8_lossy(&pdf);
    assert!(text.contains("Hack"));
    assert!(!text.contains("/Subtype /Image"));
    assert!(pdf.len() > 4000);
}
