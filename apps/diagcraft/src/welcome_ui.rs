//! Translate only freshly generated built-in content, never an existing drawing.
use super::*;

fn document() -> Document {
    let mut doc = Document::demo();
    doc.title = t("De l’idée au diagramme");
    for node in &mut doc.nodes {
        node.text = t(&node.text);
    }
    for link in &mut doc.connectors {
        if !link.label.is_empty() {
            link.label = t(&link.label);
        }
    }
    doc
}

pub(super) fn catalog() -> Vec<Template> {
    let mut items = templates::catalog();
    // IDs and categories remain stable for recents, filtering and personal templates.
    for item in &mut items {
        if item.id == "process" {
            item.document = document();
            item.title = item.document.title.clone();
        }
    }
    items
}

impl DrawApp {
    pub(crate) fn initialize_welcome(&mut self) {
        self.builtin_templates = catalog();
        if self.path.is_none() && self.doc == Document::demo() {
            self.replace(document(), None);
            self.stash_tab();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_welcome_is_localized_valid_and_clean_in_all_languages() {
        for language in Language::ALL {
            let ctx = egui::Context::default();
            let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
            app.set_language(Some(language));
            app.initialize_welcome();
            let expected = document();
            assert_eq!(app.doc, expected);
            assert_eq!(app.saved, expected);
            assert_eq!(app.tabs[app.active_tab].doc, expected);
            assert!(!app.history.can_undo());
            assert!(!app.history.can_redo());
            app.doc.validate().unwrap();
            let mut geometry = app.doc.clone();
            let original = Document::demo();
            geometry.title = original.title.clone();
            for (node, source) in geometry.nodes.iter_mut().zip(&original.nodes) {
                assert_eq!(node.text, t(&source.text));
                if language != Language::French {
                    assert_ne!(node.text, source.text);
                }
                node.text = source.text.clone();
            }
            for (link, source) in geometry.connectors.iter_mut().zip(&original.connectors) {
                assert_eq!(link.label, t(&source.label));
                if language != Language::French && !source.label.is_empty() {
                    assert_ne!(
                        link.label,
                        source.label,
                        "{}: connector label must be localized",
                        language.code()
                    );
                }
                link.label = source.label.clone();
            }
            assert_eq!(geometry, original);
            let item = app
                .builtin_templates
                .iter()
                .find(|t| t.id == "process")
                .unwrap();
            assert_eq!(item.document, expected);
            assert_eq!(item.title, expected.title);
            // Later language changes update the catalog, never the active drawing.
            app.set_language(Some(Language::English));
            assert_eq!(app.doc, expected);
        }
        i18n::activate(Language::French);
    }

    #[test]
    fn loaded_drawings_and_personal_templates_are_never_translated() {
        let ctx = egui::Context::default();
        let doc = Document::demo();
        let path = Some(PathBuf::from("example.draw"));
        let mut app = DrawApp::from_context(&ctx, doc.clone(), path.clone(), None);
        app.set_language(Some(Language::Chinese));
        app.initialize_welcome();
        assert_eq!(app.doc, doc);
        assert_eq!(app.saved, doc);
        assert_eq!(app.path, path);
        let personal = Template {
            id: "personal".into(),
            title: doc.title.clone(),
            category: "Général".into(),
            document: doc.clone(),
        };
        app.open_template(&personal);
        assert_eq!(app.doc, doc);
        // Even an unsaved edited demo is not a new welcome drawing.
        app.doc.nodes[0].text = "My text 中文".into();
        let edited = app.doc.clone();
        app.initialize_welcome();
        assert_eq!(app.doc, edited);
        i18n::activate(Language::French);
    }
}
