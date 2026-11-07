//! Local `.drawio` (draw.io XML) import with explicit page choice, preview,
//! diagnostics and single-undo page insertion. The file is local data: no
//! network access, no script or HTML execution, and no external resource is
//! loaded. The supported subset is documented in docs/drawio.md.
use super::*;
use diagcraft::{agent, drawio};
use std::io::Read;
const MAX_SOURCE: usize = 512 * 1024;
const SAMPLE: &str = r#"<mxfile host="app" type="device">
  <diagram id="p1" name="Page 1">
    <mxGraphModel dx="0" dy="0" grid="1" gridSize="10" page="1">
      <root>
        <mxCell id="0"/>
        <mxCell id="1" parent="0"/>
        <mxCell id="a" value="Début" vertex="1" parent="1" style="shape=rounded;fillColor=#dae8fc;strokeColor=#6c8ebf;">
          <mxGeometry x="40" y="40" width="120" height="60" as="geometry"/>
        </mxCell>
        <mxCell id="b" value="Décision" vertex="1" parent="1" style="rhombus;fillColor=#d5e8d4;">
          <mxGeometry x="260" y="160" width="140" height="90" as="geometry"/>
        </mxCell>
        <mxCell id="e1" edge="1" parent="1" source="a" target="b">
          <mxGeometry relative="1" as="geometry"/>
        </mxCell>
      </root>
    </mxGraphModel>
  </diagram>
</mxfile>"#;

pub(super) struct Panel {
    source: Option<String>,
    pages: Vec<String>,
    page: usize,
    preview: Option<Preview>,
    diagnostics: Vec<drawio::Notice>,
    error: Option<String>,
    acknowledged: bool,
    base_revision: String,
}
struct Preview {
    document: Document,
    base_revision: String,
}
impl Panel {
    fn can_import(&self) -> bool {
        self.source.is_some() && !self.pages.is_empty() && self.error.is_none()
    }
    fn page_name(&self) -> String {
        self.pages
            .get(self.page)
            .cloned()
            .unwrap_or_else(|| t("draw.io import"))
    }
}
fn read_source(path: &std::path::Path) -> Result<String, String> {
    let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_SOURCE as u64 {
        return Err(t("Fichier draw.io trop volumineux : maximum 512 Kio."));
    }
    let mut bytes = Vec::new();
    file.take(MAX_SOURCE as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SOURCE {
        return Err(t("Fichier draw.io trop volumineux : maximum 512 Kio."));
    }
    String::from_utf8(bytes).map_err(|_| t("Le fichier draw.io doit être en UTF-8."))
}
fn append_page(doc: &Document, imported: &Document) -> Result<Document, String> {
    doc.validate()?;
    imported.validate()?;
    let mut next = doc.clone();
    next.add_page(false)?;
    next.page_name = imported.page_name.clone();
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
    let Some(source) = &panel.source else {
        return;
    };
    match drawio::import(source, panel.page) {
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
fn notices(ui: &mut egui::Ui, diagnostics: &[drawio::Notice]) {
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
        if let Some(id) = notice.id {
            ui.small(i18n::render(
                "Page {0} · objet {1}",
                &[
                    ("0", notice.page.unwrap_or(0).to_string()),
                    ("1", id.to_string()),
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
    pub(super) fn open_drawio_import(&mut self) {
        if self.rich_editor.is_some() || self.drag.is_some() {
            self.status = "Terminez l’édition en cours avant d’importer un draw.io.".into();
            return;
        }
        self.commit();
        self.clear_connection();
        self.context_open = false;
        self.drawio_ui = Some(Panel {
            source: None,
            pages: vec![],
            page: 0,
            preview: None,
            diagnostics: vec![],
            error: None,
            acknowledged: false,
            base_revision: agent::revision(&self.doc),
        });
    }
    fn apply_drawio_preview(&mut self, panel: &Panel) -> Result<(), String> {
        let preview = panel
            .preview
            .as_ref()
            .ok_or_else(|| t("Prévisualisez le draw.io avant de l’importer."))?;
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
            "draw.io importé sur une nouvelle page. Une annulation retire l’import.".into();
        Ok(())
    }
    pub(super) fn drawio_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut panel) = self.drawio_ui.take() else {
            return;
        };
        let mut close = false;
        let mut apply = false;
        let modal = egui::Modal::new("drawio-import".into()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 80.).clamp(480., 960.));
            ui.heading(t("Importer un draw.io local"));
            let height = (ctx.content_rect().height() - 190.).clamp(260., 650.);
            egui::ScrollArea::vertical()
                .id_salt("drawio-body")
                .max_height(height)
                .show(ui, |ui| {
                    ui.label(t(
                        "Un sous-ensemble de .drawio est lu : fichiers locaux, aucun accès réseau, images externes jamais chargées.",
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(t("Charger un fichier draw.io…")).clicked()
                            && let Some(path) = rfd::FileDialog::new()
                                .add_filter("draw.io", &["drawio", "xml"])
                                .pick_file()
                        {
                            match read_source(&path) {
                                Ok(source) => match drawio::pages(&source) {
                                    Ok(pages) => {
                                        panel.source = Some(source);
                                        panel.pages = pages;
                                        if panel.page >= panel.pages.len() {
                                            panel.page = panel.pages.len() - 1;
                                        }
                                        panel.preview = None;
                                        panel.diagnostics.clear();
                                        panel.error = None;
                                    }
                                    Err(errors) => {
                                        panel.source = Some(source);
                                        panel.pages = vec![];
                                        panel.preview = None;
                                        panel.diagnostics = errors;
                                        panel.error = None;
                                    }
                                },
                                Err(error) => {
                                    panel.error = Some(error);
                                    panel.preview = None;
                                }
                            }
                        }
                        if ui.button(t("Exemple draw.io")).clicked() {
                            match drawio::pages(SAMPLE) {
                                Ok(pages) => {
                                    panel.source = Some(SAMPLE.into());
                                    panel.pages = pages;
                                    panel.page = 0;
                                    panel.preview = None;
                                    panel.diagnostics.clear();
                                    panel.error = None;
                                }
                                Err(errors) => {
                                    panel.source = Some(SAMPLE.into());
                                    panel.pages = vec![];
                                    panel.preview = None;
                                    panel.diagnostics = errors;
                                }
                            }
                        }
                        if ui
                            .add_enabled(panel.can_import(), egui::Button::new(t("Vérifier et prévisualiser")))
                            .clicked()
                        {
                            preview_import(&mut panel, &self.doc);
                        }
                    });
                    if panel.pages.len() > 1 {
                        ui.separator();
                        ui.label(RichText::new(t("Choix de la page")).strong());
                        egui::ComboBox::from_id_salt("drawio-page")
                            .selected_text(i18n::render(
                                "Page {0} — {1}",
                                &[("0", panel.page.to_string()), ("1", panel.page_name())],
                            ))
                            .show_ui(ui, |ui| {
                                for (i, name) in panel.pages.iter().enumerate() {
                                    ui.selectable_value(
                                        &mut panel.page,
                                        i,
                                        format!("{i} — {name}"),
                                    );
                                }
                            });
                        if panel.page < panel.pages.len() {
                            ui.small(t("Seule la page sélectionnée est importée ; les autres sont signalées comme non reprises."));
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
                        if panel
                            .diagnostics
                            .iter()
                            .any(|n| n.severity == "warning")
                        {
                            ui.checkbox(&mut panel.acknowledged, t("J’accepte les pertes décrites dans le rapport."));
                        }
                    }
                    if let Some(error) = &panel.error {
                        ui.colored_label(ERROR_TEXT, error);
                    }
                });
            ui.separator();
            ui.horizontal_wrapped(|ui| {
                close = ui.button(t("Annuler")).clicked();
                let can_import = panel.preview.is_some()
                    && (panel.diagnostics.iter().all(|n| n.severity != "warning") || panel.acknowledged);
                apply = ui
                    .add_enabled(can_import, egui::Button::new(t("Importer sur une nouvelle page")))
                    .clicked();
            });
        });
        close |= modal.should_close();
        if apply {
            match self.apply_drawio_preview(&panel) {
                Ok(()) => close = true,
                Err(error) => {
                    panel.error = Some(error);
                    panel.preview = None;
                }
            }
        }
        if !close {
            self.drawio_ui = Some(panel);
        }
    }
}

#[cfg(test)]
#[path = "drawio_tests.rs"]
mod tests;
