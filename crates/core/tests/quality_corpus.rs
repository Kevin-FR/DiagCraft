#[path = "../qa/corpus.rs"]
mod corpus;
use diagcraft::{agent, document::Document, mermaid};
use serde_json::json;

#[test]
fn large_original_corpora_are_portable_and_page_navigation_keeps_their_revision() {
    let temp = tempfile::tempdir().unwrap();
    for (name, mut doc, nodes, links, pages) in [
        ("grid", corpus::grid(40, 25), 1000, 975, 1),
        ("pages", corpus::multipage(), 1000, 950, 5),
        ("images", corpus::images(), 3, 0, 1),
    ] {
        doc.validate().unwrap();
        assert_eq!(doc.page_count(), pages);
        assert_eq!(
            (0..pages)
                .map(|i| doc.page(i).unwrap().nodes.len())
                .sum::<usize>(),
            nodes
        );
        assert_eq!(
            (0..pages)
                .map(|i| doc.page(i).unwrap().connectors.len())
                .sum::<usize>(),
            links
        );
        let path = temp.path().join(format!("{name}.draw"));
        doc.save(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let revision = agent::revision(&doc);
        for i in 0..pages {
            doc.switch_page(i).unwrap();
            assert_eq!(agent::revision(&doc), revision);
        }
        assert_eq!(agent::revision(&Document::load(&path).unwrap()), revision);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
fn mutations(source: &[u8]) -> Vec<Vec<u8>> {
    let mut out = vec![
        vec![],
        b"null".to_vec(),
        b"[]".to_vec(),
        b"{\"nested\":".to_vec(),
    ];
    for cut in [
        1,
        source.len() / 3,
        source.len() / 2,
        source.len().saturating_sub(1),
    ] {
        out.push(source[..cut].to_vec());
    }
    let mut seed = 0xD1A6_C4AF_u64;
    for _ in 0..64 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let at = seed as usize % source.len();
        let mut bytes = source.to_vec();
        bytes[at] ^= 1 << ((seed >> 32) % 8);
        out.push(bytes);
    }
    out
}
#[test]
fn corrupted_document_and_patch_corpus_is_rejected_or_remains_valid_without_panics_or_partial_writes()
 {
    let temp = tempfile::tempdir().unwrap();
    let d = corpus::grid(2, 2);
    let revision = agent::revision(&d);
    let bytes = serde_json::to_vec(&d).unwrap();
    let path = temp.path().join("corrupted.draw");
    for bytes in mutations(&bytes) {
        std::fs::write(&path, &bytes).unwrap();
        let result =
            std::panic::catch_unwind(|| Document::load(&path)).expect("document load panicked");
        if let Ok(loaded) = result {
            loaded.validate().unwrap();
            let canonical = serde_json::to_vec(&loaded).unwrap();
            let again: Document = serde_json::from_slice(&canonical).unwrap();
            assert_eq!(loaded, again);
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
    let patch=json!({"api_version":1,"expected_revision":revision,"operations":[{"op":"set_title","title":"Must stay atomic"},{"op":"update_node","page":0,"target":d.nodes[0].id,"changes":{"text":"Corpus 中文"}}]}).to_string();
    for bytes in mutations(patch.as_bytes()) {
        let text = String::from_utf8_lossy(&bytes);
        let result = std::panic::catch_unwind(|| {
            agent::parse_patch_detailed(&text).and_then(|p| agent::preview_detailed(&d, &p))
        })
        .expect("patch panicked");
        if let Ok(preview) = result {
            preview.document.validate().unwrap();
        }
        assert_eq!(agent::revision(&d), revision);
    }
}
#[test]
fn mutated_mermaid_never_panics_and_never_returns_an_invalid_document() {
    let source = b"flowchart LR\nA[Start] --> B{Ready?}\nB -->|Yes| C([Finish])\nB -.->|No| A\n";
    for bytes in mutations(source) {
        let text = String::from_utf8_lossy(&bytes);
        let result =
            std::panic::catch_unwind(|| mermaid::import(&text)).expect("Mermaid parser panicked");
        if let Ok(imported) = result {
            imported.document.validate().unwrap();
        }
    }
}
