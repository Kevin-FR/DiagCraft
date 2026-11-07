//! Local CSV → diagram import with explicit column choice, preview, diagnostics
//! and single-undo page insertion. Column values are treated as data, never as
//! formulas, HTML or instructions; no network access occurs.
use super::*;
use diagcraft::{agent, csv};
use std::io::Read;
const MAX_SOURCE: usize = 256 * 1024;
const SAMPLE: &str = "id,nom,equipe,responsable\nA1,Alice,Design,Chef\nB2,Bob,Design,Chef\nC3,Carol,Dev,Chef\nChef,Cheffe d'equipe,Equipe,\n";

pub(super) struct Panel {
    source: Option<String>,
    headers: Vec<String>,
    label: usize,
    identity: Option<usize>,
    connect_multi: Vec<bool>,
    preview: Option<Preview>,
    diagnostics: Vec<csv::Notice>,
    error: Option<String>,
    base_revision: String,
}
struct Preview {
    document: Document,
    base_revision: String,
}
impl Panel {
    fn can_preview(&self) -> bool {
        self.source.is_some() && !self.headers.is_empty() && self.error.is_none()
    }
    fn build_options(&self) -> csv::CsvOptions {
        csv::CsvOptions {
            label: self.headers.get(self.label).cloned(),
            identity: self.identity.and_then(|i| self.headers.get(i).cloned()),
            connect: self
                .connect_multi
                .iter()
                .enumerate()
                .filter(|(_, b)| **b)
                .filter_map(|(i, _)| self.headers.get(i).cloned())
                .collect(),
        }
    }
}
fn read_source(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_SOURCE as u64 {
        return Err(t("Fichier CSV trop volumineux : maximum 256 Kio."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SOURCE {
        return Err(t("Fichier CSV trop volumineux : maximum 256 Kio."));
    }
    String::from_utf8(bytes).map_err(|_| t("Le fichier CSV doit être en UTF-8."))
}
fn append_page(doc: &Document, imported: &Document) -> Result<Document, String> {
    doc.validate()?;
    imported.validate()?;
    let mut next = doc.clone();
    next.add_page(false)?;
    next.page_name = t("Import CSV");
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
    let Some(source) = &panel.source else {
        return;
    };
    let options = panel.build_options();
    match csv::import(source, &options) {
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
fn notices(ui: &mut egui::Ui, diagnostics: &[csv::Notice]) {
    for notice in diagnostics {
        let color = if notice.severity == "error" {
            ERROR_TEXT
        } else {
            WARNING_TEXT
        };
        ui.colored_label(color, format!("{} · {}", notice.code, notice.message));
        if let Some(line) = notice.line {
            ui.small(i18n::render(
                "Ligne {0} · colonne {1}",
                &[
                    ("0", line.to_string()),
                    ("1", notice.column.unwrap_or(1).to_string()),
                ],
            ));
        }
        if !notice.hint.is_empty() {
            ui.small(&notice.hint);
        }
        ui.add_space(4.);
    }
}
impl DrawApp {
    pub(super) fn open_csv_import(&mut self) {
        if self.rich_editor.is_some() || self.drag.is_some() {
            self.status = "Terminez l’édition en cours avant d’importer un CSV.".into();
            return;
        }
        self.commit();
        self.clear_connection();
        self.context_open = false;
        self.csv_ui = Some(Panel {
            source: None,
            headers: vec![],
            label: 0,
            identity: None,
            connect_multi: vec![],
            preview: None,
            diagnostics: vec![],
            error: None,
            base_revision: agent::revision(&self.doc),
        });
    }
    fn apply_csv_preview(&mut self, panel: &Panel) -> Result<(), String> {
        let preview = panel
            .preview
            .as_ref()
            .ok_or_else(|| t("Prévisualisez le CSV avant de l’importer."))?;
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
        self.status = "CSV importé sur une nouvelle page. Une annulation retire l’import.".into();
        Ok(())
    }
    pub(super) fn csv_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut panel) = self.csv_ui.take() else {
            return;
        };
        let mut close = false;
        let mut apply = false;
        let modal = egui::Modal::new("csv-import".into()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 80.).clamp(480., 960.));
            ui.heading(t("Importer un CSV local"));
            let height = (ctx.content_rect().height() - 190.).clamp(260., 650.);
            egui::ScrollArea::vertical()
                .id_salt("csv-body")
                .max_height(height)
                .show(ui, |ui| {
                    ui.label(t(
                        "Chaque ligne devient une forme ; une colonne d’identité relie les formes par valeurs.",
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(t("Charger un fichier CSV…")).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("CSV", &["csv", "txt"])
                                .pick_file()
                        {
                            match read_source(&path) {
                                Ok(source) => match csv::headers(&source) {
                                    Ok(headers) => {
                                        let connect_multi = vec![false; headers.len()];
                                        panel.source = Some(source);
                                        panel.headers = headers;
                                        panel.connect_multi = connect_multi;
                                        panel.preview = None;
                                        panel.diagnostics.clear();
                                        panel.error = None;
                                    }
                                    Err(errors) => {
                                        panel.source = Some(source);
                                        panel.diagnostics = errors;
                                        panel.preview = None;
                                        panel.error = None;
                                    }
                                },
                                Err(error) => {
                                    panel.error = Some(error);
                                    panel.preview = None;
                                    panel.diagnostics.clear();
                                }
                            }
                        }
                        if ui.button(t("Exemple CSV")).clicked()
                            && let Ok(headers) = csv::headers(SAMPLE)
                        {
                            panel.source = Some(SAMPLE.into());
                            panel.headers = headers;
                            panel.connect_multi = vec![false; panel.headers.len()];
                            panel.preview = None;
                            panel.error = None;
                            panel.diagnostics.clear();
                        }
                        if ui.add_enabled(panel.can_preview(), egui::Button::new(t("Vérifier et prévisualiser"))).clicked()
                        {
                            preview_import(&mut panel, &self.doc);
                        }
                    });
                    if !panel.headers.is_empty() {
                        ui.separator();
                        ui.label(RichText::new(t("Choix des colonnes")).strong());
                        egui::ComboBox::from_id_salt("csv-label")
                            .selected_text(i18n::render(
                                "Libellé : {0}",
                                &[(
                                    "0",
                                    panel
                                        .headers
                                        .get(panel.label)
                                        .cloned()
                                        .unwrap_or_default(),
                                )],
                            ))
                            .show_ui(ui, |ui| {
                                for (i, h) in panel.headers.iter().enumerate() {
                                    ui.selectable_value(&mut panel.label, i, h.clone());
                                }
                            });
                        egui::ComboBox::from_id_salt("csv-identity")
                            .selected_text(i18n::render(
                                "Identité : {0}",
                                &[(
                                    "0",
                                    panel
                                        .identity
                                        .and_then(|i| panel.headers.get(i))
                                        .cloned()
                                        .unwrap_or_else(|| t("Aucune")),
                                )],
                            ))
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut panel.identity, None, t("Aucune (libellé utilisé)"));
                                for (i, h) in panel.headers.iter().enumerate() {
                                    ui.selectable_value(&mut panel.identity, Some(i), h.clone());
                                }
                            });
                        ui.label(t("Colonnes de relation (valeurs séparées par |) :"));
                        for (i, h) in panel.headers.iter().enumerate() {
                            ui.checkbox(&mut panel.connect_multi[i], h.clone());
                        }
                    }
                    let preview_doc = panel.preview.as_ref().map(|p| &p.document);
                    if let Some(doc) = preview_doc {
                        ui.separator();
                        ui.label(RichText::new(t("APERÇU DU RÉSULTAT")).strong());
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 200.),
                            Sense::hover(),
                        );
                        library_ui::paint_document_preview(
                            ui.painter(),
                            &doc.page_document(doc.page_index).unwrap(),
                            rect,
                        );
                        ui.small(t("L’import ajoute une page ; les pages existantes restent intactes."));
                    }
                    if !panel.diagnostics.is_empty() {
                        ui.separator();
                        ui.label(RichText::new(t("Rapport de conversion")).strong());
                        notices(ui, &panel.diagnostics);
                    } else if panel.source.is_some() {
                        ui.label(t("Configurez les colonnes puis prévisualisez le résultat."));
                    }
                    if let Some(error) = &panel.error {
                        ui.colored_label(ERROR_TEXT, error);
                    }
                });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                close = ui.button(t("Annuler")).clicked();
                apply = ui
                    .add_enabled(panel.preview.is_some(), egui::Button::new(t("Importer sur une nouvelle page")))
                    .clicked();
            });
        });
        close |= modal.should_close();
        if apply {
            match self.apply_csv_preview(&panel) {
                Ok(()) => close = true,
                Err(error) => {
                    panel.error = Some(error);
                    panel.preview = None;
                }
            }
        }
        if !close {
            self.csv_ui = Some(panel);
        }
    }
}

#[cfg(test)]
#[path = "csv_tests.rs"]
mod tests;
