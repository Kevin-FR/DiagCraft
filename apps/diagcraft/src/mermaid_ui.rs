//! Local Mermaid exchange with explicit preview, loss review and revision checks.
use super::*;
use diagcraft::{agent, mermaid};
use std::io::{Read, Write};
const MAX_SOURCE: usize = 256 * 1024;
const SAMPLE: &str = "flowchart LR\n  Idea([Une idée]) --> Plan[Créer un parcours]\n  Plan --> Review{Prêt à partager ?}\n  Review -->|Oui| End([Partager])\n  Review -.->|À peaufiner| Plan\n";

pub(super) struct Panel {
    exporting: bool,
    source: String,
    preview: Option<Preview>,
    diagnostics: Vec<mermaid::Notice>,
    error: Option<String>,
    acknowledged: bool,
    base_revision: String,
}
struct Preview {
    document: Document,
    base_revision: String,
}
impl Panel {
    fn can_export(&self) -> bool {
        self.exporting
            && self.error.is_none()
            && !self.source.is_empty()
            && (self.diagnostics.is_empty() || self.acknowledged)
    }
    fn check_current(&self, doc: &Document) -> Result<(), String> {
        if self.base_revision != agent::revision(doc) {
            return Err(t(
                "Le document a changé. Actualisez l’aperçu avant de continuer.",
            ));
        }
        Ok(())
    }
}
fn read_source(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_SOURCE as u64 {
        return Err(t("Source Mermaid trop volumineuse : maximum 256 Kio."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SOURCE {
        return Err(t("Source Mermaid trop volumineuse : maximum 256 Kio."));
    }
    String::from_utf8(bytes).map_err(|_| t("Le fichier Mermaid doit être en UTF-8."))
}
fn save_source(path: &std::path::Path, source: &str) -> Result<(), String> {
    let exists =
        || t("La sortie existe déjà. Choisissez un nouveau nom pour préserver les fichiers.");
    if path.symlink_metadata().is_ok() {
        return Err(exists());
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."));
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    file.write_all(source.as_bytes())
        .map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist_noclobber(path).map_err(|e| {
        if e.error.kind() == std::io::ErrorKind::AlreadyExists {
            exists()
        } else {
            e.error.to_string()
        }
    })?;
    Ok(())
}
fn append_page(doc: &Document, imported: &Document) -> Result<Document, String> {
    doc.validate()?;
    imported.validate()?;
    let mut next = doc.clone();
    next.add_page(false)?;
    next.page_name = t("Import Mermaid");
    next.page_size = imported.page_size;
    next.page_style = imported.page_style.clone();
    next.page_layout = imported.page_layout.clone();
    next.nodes = imported.nodes.clone();
    next.connectors = imported.connectors.clone();
    next.drawings = imported.drawings.clone();
    next.charts = imported.charts.clone();
    next.organization = imported.organization.clone();
    next.validate()?;
    Ok(next)
}
fn preview_import(panel: &mut Panel, doc: &Document) {
    panel.preview = None;
    panel.error = None;
    panel.acknowledged = false;
    match mermaid::import(&panel.source) {
        Ok(imported) => {
            panel.diagnostics = imported.diagnostics;
            match append_page(doc, &imported.document) {
                Ok(document) => {
                    panel.base_revision = agent::revision(doc);
                    panel.preview = Some(Preview {
                        document,
                        base_revision: panel.base_revision.clone(),
                    });
                }
                Err(error) => panel.error = Some(error),
            }
        }
        Err(errors) => panel.diagnostics = errors,
    }
}
fn notice_text(notice: &mermaid::Notice, exporting: bool) -> (String, String) {
    if notice.code != "layout_changed" {
        return (notice.message.clone(), notice.hint.clone());
    }
    if exporting {
        (
            t(
                "L’export Mermaid ne conserve pas les positions, tailles, parcours, ports, styles ou l’apparence des pages.",
            ),
            t(
                "Conservez le fichier .draw comme source visuelle modifiable et vérifiez l’aperçu Mermaid avant d’enregistrer.",
            ),
        )
    } else {
        (
            t("DiagCraft crée un placement modifiable à partir de la structure du flowchart."),
            t(
                "Vérifiez les positions et les connecteurs avant l’import ; le placement visuel Mermaid n’est pas conservé.",
            ),
        )
    }
}
fn notices(ui: &mut egui::Ui, diagnostics: &[mermaid::Notice], exporting: bool) {
    for notice in diagnostics {
        let color = if notice.severity == "error" {
            Color32::from_rgb(180, 50, 40)
        } else {
            Color32::from_rgb(157, 103, 22)
        };
        let (message, hint) = notice_text(notice, exporting);
        ui.colored_label(color, format!("{} · {}", notice.code, message));
        if let Some(line) = notice.line {
            ui.small(i18n::render(
                "Ligne {0} · colonne {1}",
                &[
                    ("0", line.to_string()),
                    ("1", notice.column.unwrap_or(1).to_string()),
                ],
            ));
        }
        if let Some(id) = notice.id {
            ui.small(i18n::render(
                "Page {0} · objet {1}",
                &[
                    ("0", notice.page.unwrap_or(0).to_string()),
                    ("1", id.to_string()),
                ],
            ));
        }
        if !hint.is_empty() {
            ui.small(hint);
        }
        ui.add_space(4.);
    }
}
impl DrawApp {
    pub(super) fn open_mermaid(&mut self, exporting: bool) {
        if self.rich_editor.is_some() || self.drag.is_some() {
            self.status = "Terminez l’édition en cours avant d’ouvrir Mermaid.".into();
            return;
        }
        self.commit();
        self.clear_connection();
        self.context_open = false;
        let mut panel = Panel {
            exporting,
            source: String::new(),
            preview: None,
            diagnostics: vec![],
            error: None,
            acknowledged: false,
            base_revision: agent::revision(&self.doc),
        };
        if exporting {
            match mermaid::export(&self.doc, self.doc.page_index) {
                Ok(export) => {
                    panel.source = export.source;
                    panel.diagnostics = export.diagnostics;
                }
                Err(error) => panel.error = Some(error),
            }
        }
        self.mermaid_ui = Some(panel);
    }
    fn apply_mermaid_preview(&mut self, panel: &Panel) -> Result<(), String> {
        let preview = panel
            .preview
            .as_ref()
            .ok_or_else(|| t("Prévisualisez le Mermaid avant de l’importer."))?;
        if preview.base_revision != agent::revision(&self.doc) {
            return Err(t(
                "Le document a changé. Actualisez l’aperçu avant de continuer.",
            ));
        }
        preview.document.validate()?;
        // The result only inserts a new page; all existing pages stay byte-identical.
        self.doc = preview.document.clone();
        self.selected.clear();
        self.drag = None;
        self.set_tool(Tool::Select);
        self.normalize_selection();
        self.history.record(&self.doc);
        self.property_edit = false;
        self.fit_next = true;
        self.status =
            "Mermaid importé sur une nouvelle page. Une annulation retire l’import.".into();
        Ok(())
    }
    pub(super) fn mermaid_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut panel) = self.mermaid_ui.take() else {
            return;
        };
        let mut close = false;
        let mut apply = false;
        let mut save = false;
        let mut copy = false;
        let modal = egui::Modal::new("mermaid-exchange".into()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 80.).clamp(480., 960.));
            ui.heading(t(if panel.exporting { "Exporter en Mermaid" } else { "Importer du Mermaid" }));
            let height = (ctx.content_rect().height() - 190.).clamp(260., 650.);
            egui::ScrollArea::vertical().id_salt("mermaid-body").max_height(height).show(ui, |ui| {
                ui.label(t("Flowcharts uniquement : textes et liens deviennent des objets modifiables. Le placement Mermaid n’est pas conservé."));
                if panel.exporting {
                    ui.label(i18n::render("Page exportée : {0}", &[("0", self.doc.page_name.clone())]));
                    if ui.button(t("Actualiser l’export")).clicked() {
                        match mermaid::export(&self.doc, self.doc.page_index) {
                            Ok(export) => {
                                panel.source = export.source;
                                panel.diagnostics = export.diagnostics;
                                panel.error = None;
                                panel.acknowledged = false;
                                panel.base_revision = agent::revision(&self.doc);
                            }
                            Err(error) => panel.error = Some(error),
                        }
                    }
                    egui::ScrollArea::vertical().id_salt("mermaid-output").max_height(205.).show(ui, |ui| {
                        ui.add(egui::TextEdit::multiline(&mut panel.source).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(10).interactive(false));
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(t("Charger un fichier Mermaid…")).clicked()
                            && let Some(path) = rfd::FileDialog::new().add_filter("Mermaid", &["mmd", "mermaid", "txt"]).pick_file()
                        {
                            match read_source(&path) {
                                Ok(source) => {
                                    panel.source = source;
                                    panel.preview = None;
                                    panel.diagnostics.clear();
                                    panel.error = None;
                                }
                                Err(error) => {
                                    panel.error = Some(error);
                                    panel.preview = None;
                                    panel.diagnostics.clear();
                                }
                            }
                        }
                        if ui.button(t("Exemple Mermaid")).clicked() {
                            panel.source = SAMPLE.into();
                            panel.preview = None;
                            panel.error = None;
                            panel.diagnostics.clear();
                        }
                        if ui.button(t("Vérifier et prévisualiser")).clicked() {
                            preview_import(&mut panel, &self.doc);
                        }
                    });
                    ui.columns(2, |cols| {
                        egui::ScrollArea::vertical().id_salt("mermaid-source").max_height(270.).show(&mut cols[0], |ui| {
                            if ui.add(egui::TextEdit::multiline(&mut panel.source).font(egui::TextStyle::Monospace).desired_width(f32::INFINITY).desired_rows(14).char_limit(MAX_SOURCE).hint_text("flowchart LR\n  A[Une idée] --> B[La suite]")).changed() {
                                panel.preview = None;
                                panel.diagnostics.clear();
                                panel.error = None;
                            }
                        });
                        let preview_doc = panel.preview.as_ref().map(|p| &p.document);
                        if let Some(doc) = preview_doc {
                            cols[1].label(RichText::new(t("APERÇU DU RÉSULTAT")).strong());
                            let (rect, _) = cols[1].allocate_exact_size(vec2(cols[1].available_width(), 220.), Sense::hover());
                            library_ui::paint_document_preview(cols[1].painter(), &doc.page_document(doc.page_index).unwrap(), rect);
                            cols[1].small(t("L’import ajoute une page ; les pages existantes restent intactes."));
                        } else {
                            cols[1].label(t("Vérifiez le texte pour préparer l’import."));
                        }
                    });
                }
                if !panel.diagnostics.is_empty() {
                    ui.separator();
                    ui.label(RichText::new(t("Rapport de conversion")).strong());
                    notices(ui, &panel.diagnostics, panel.exporting);
                    if panel.exporting {
                        ui.checkbox(&mut panel.acknowledged, t("J’accepte les pertes décrites dans le rapport."));
                    }
                }
                if let Some(error) = &panel.error {
                    ui.colored_label(Color32::from_rgb(180, 50, 40), error);
                }
            });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                close = ui.button(t("Annuler")).clicked();
                if panel.exporting {
                    save = ui.add_enabled(panel.can_export(), egui::Button::new(t("Enregistrer le Mermaid…"))).clicked();
                    copy = ui.add_enabled(panel.can_export(), egui::Button::new(t("Copier le Mermaid"))).clicked();
                } else {
                    apply = ui.add_enabled(panel.preview.is_some(), egui::Button::new(t("Importer sur une nouvelle page"))).clicked();
                }
            });
        });
        close |= modal.should_close();
        if apply {
            match self.apply_mermaid_preview(&panel) {
                Ok(()) => close = true,
                Err(error) => {
                    panel.error = Some(error);
                    panel.preview = None;
                }
            }
        }
        if (copy || save)
            && let Err(error) = panel.check_current(&self.doc)
        {
            panel.error = Some(error);
            panel.acknowledged = false;
            save = false;
            copy = false;
        }
        if copy {
            ctx.copy_text(panel.source.clone());
            self.status = "Mermaid copié.".into();
            close = true;
        }
        if save
            && let Some(path) = rfd::FileDialog::new()
                .add_filter("Mermaid", &["mmd", "mermaid"])
                .set_file_name("diagramme.mmd")
                .save_file()
        {
            match save_source(&path, &panel.source) {
                Ok(()) => {
                    self.status = "Mermaid exporté.".into();
                    close = true;
                }
                Err(error) => panel.error = Some(error),
            }
        }
        if !close {
            self.mermaid_ui = Some(panel);
        }
    }
    pub(super) fn prepare_mermaid_capture(&mut self, exporting: bool) {
        self.show_library = false;
        self.show_inspector = false;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        if exporting && let Ok(imported) = mermaid::import(SAMPLE) {
            self.doc = imported.document;
            self.history = History::new(&self.doc);
        }
        self.open_mermaid(exporting);
        if !exporting && let Some(panel) = &mut self.mermaid_ui {
            panel.source = SAMPLE.into();
            preview_import(panel, &self.doc);
        }
        self.fit_next = true;
    }
}

#[cfg(test)]
#[path = "mermaid_tests.rs"]
mod tests;
