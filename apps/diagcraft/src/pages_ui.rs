use super::library_ui::paint_document_preview;
use super::*;
use diagcraft::pages::{self, PageLayout, Unit};

pub(super) struct PageDraft {
    index: usize,
    name: String,
    size: Point,
    unit: Unit,
    auto: bool,
    error: Option<String>,
}
enum PageAction {
    Switch(usize),
    Add(bool),
    Settings,
    Remove,
    Move(usize),
    Fit,
}
impl DrawApp {
    pub(super) fn switch_page(&mut self, index: usize) {
        self.commit();
        if self.doc.switch_page(index).is_ok() {
            self.selected.clear();
            self.drag = None;
            self.set_tool(Tool::Select);
            self.context_open = false;
            self.fit_next = true;
            self.status = i18n::render(
                "{0} · page {1} sur {2}",
                &[
                    ("0", self.doc.page_name.to_string()),
                    ("1", format!("{}", index + 1)),
                    ("2", format!("{}", self.doc.page_count())),
                ],
            );
        }
    }
    pub(super) fn open_page_settings(&mut self) {
        let unit = self.doc.page_layout.unit;
        self.page_draft = Some(PageDraft {
            index: self.doc.page_index,
            name: self.doc.page_name.clone(),
            size: self.doc.page_size.map(|v| unit.from_px(v)),
            unit,
            auto: self.doc.page_layout.auto_size,
            error: None,
        });
        self.show_pages = false;
    }
    fn page_action(&mut self, action: PageAction) {
        match action {
            PageAction::Switch(index) => {
                self.switch_page(index);
                self.show_pages = false;
            }
            PageAction::Settings => self.open_page_settings(),
            PageAction::Fit => {
                if let Err(e) = self.doc.fit_page_to_drawing() {
                    self.error = Some(e);
                }
                self.fit_next = true;
                self.commit();
            }
            PageAction::Add(duplicate) => {
                self.commit();
                if let Err(e) = self.doc.add_page(duplicate) {
                    self.error = Some(e);
                }
                self.selected.clear();
                self.set_tool(Tool::Select);
                self.fit_next = true;
                self.commit();
            }
            PageAction::Remove => {
                self.commit();
                if let Err(e) = self.doc.remove_page(self.doc.page_index) {
                    self.error = Some(e);
                }
                self.selected.clear();
                self.fit_next = true;
                self.commit();
            }
            PageAction::Move(to) => {
                self.commit();
                if let Err(e) = self.doc.move_page(self.doc.page_index, to) {
                    self.error = Some(e);
                }
                self.commit();
            }
        }
    }
    pub(super) fn add_blank_page(&mut self) {
        self.page_action(PageAction::Add(false));
    }
    pub(super) fn pages_bar(&mut self, ctx: &egui::Context) {
        let mut action = None;
        egui::TopBottomPanel::bottom("pages_bar")
            .exact_height(42.)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(6)
                    .stroke(Stroke::new(1., BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let pages = ui.button(i18n::render(
                        "Pages ({0})",
                        &[("0", self.doc.page_count().to_string())],
                    ));
                    self.focus_ui.anchor(ui, focus_ui::Region::Pages, &pages);
                    if pages.clicked() {
                        self.show_pages = true;
                    }
                    if panels_ui::icon_button(ui, "+", "Ajouter une page").clicked() {
                        action = Some(PageAction::Add(false));
                    }
                    let names = self.doc.page_names();
                    egui::ScrollArea::horizontal()
                        .id_salt("page_tabs")
                        .max_width((ui.available_width() - 160.).max(100.))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (i, name) in names.iter().enumerate() {
                                    let label = format!(
                                        "{} · {}",
                                        i + 1,
                                        name.chars().take(22).collect::<String>()
                                    );
                                    let r = ui
                                        .selectable_label(i == self.doc.page_index, label)
                                        .on_hover_text(*name);
                                    if r.clicked() {
                                        action = Some(PageAction::Switch(i));
                                    }
                                }
                            });
                        });
                    if ui.button(t("Taille de page…")).clicked() {
                        action = Some(PageAction::Settings);
                    }
                });
            });
        if let Some(a) = action {
            self.page_action(a);
        }
    }
    pub(super) fn pages_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_pages {
            let mut action = None;
            let modal = egui::Modal::new("page_manager".into()).show(ctx, |ui| {
                let width = (ctx.content_rect().width() - 80.).min(1050.);
                ui.set_width(width);
                ui.horizontal(|ui| {
                    ui.heading(t("Pages du document"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(t("Fermer")).clicked() {
                            self.show_pages = false;
                        }
                    });
                });
                ui.label(t(
                    "Chaque page conserve ses formes, ses dimensions et son fond.",
                ));
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            self.doc.page_count() < 100,
                            egui::Button::new(t("Ajouter une page")),
                        )
                        .clicked()
                    {
                        action = Some(PageAction::Add(false));
                    }
                    if ui
                        .add_enabled(
                            self.doc.page_count() < 100,
                            egui::Button::new(t("Dupliquer la page")),
                        )
                        .clicked()
                    {
                        action = Some(PageAction::Add(true));
                    }
                    if ui.button(t("Nom et dimensions…")).clicked() {
                        action = Some(PageAction::Settings);
                    }
                    if ui
                        .add_enabled(
                            self.doc.page_index > 0,
                            egui::Button::new(t("Déplacer avant")),
                        )
                        .clicked()
                    {
                        action = Some(PageAction::Move(self.doc.page_index - 1));
                    }
                    if ui
                        .add_enabled(
                            self.doc.page_index + 1 < self.doc.page_count(),
                            egui::Button::new(t("Déplacer après")),
                        )
                        .clicked()
                    {
                        action = Some(PageAction::Move(self.doc.page_index + 1));
                    }
                    if ui
                        .add_enabled(
                            self.doc.page_count() > 1,
                            egui::Button::new(t("Supprimer la page")),
                        )
                        .on_hover_text(t("Suppression annulable avec ⌘/Ctrl + Z"))
                        .clicked()
                    {
                        action = Some(PageAction::Remove);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .max_height((ctx.content_rect().height() - 240.).max(200.))
                    .show(ui, |ui| {
                        let card = (width - 44.) / 3.;
                        egui::Grid::new("page_cards")
                            .num_columns(3)
                            .spacing(vec2(12., 12.))
                            .show(ui, |ui| {
                                for i in 0..self.doc.page_count() {
                                    let doc = self.doc.page_document(i).unwrap();
                                    let selected = i == self.doc.page_index;
                                    egui::Frame::new()
                                        .fill(if selected {
                                            Color32::from_rgb(232, 243, 255)
                                        } else {
                                            SURFACE
                                        })
                                        .stroke(Stroke::new(
                                            if selected { 1.5 } else { 1. },
                                            if selected { ACCENT } else { BORDER },
                                        ))
                                        .corner_radius(8)
                                        .inner_margin(10)
                                        .show(ui, |ui| {
                                            ui.vertical(|ui| {
                                                ui.set_width(card - 20.);
                                                let (r, response) = ui.allocate_exact_size(
                                                    vec2(card - 20., 155.),
                                                    Sense::click(),
                                                );
                                                if ui.is_rect_visible(r) {
                                                    paint_document_preview(ui.painter(), &doc, r);
                                                }
                                                if response.clicked() {
                                                    action = Some(PageAction::Switch(i));
                                                }
                                                ui.label(
                                                    RichText::new(format!(
                                                        "{} · {}",
                                                        i + 1,
                                                        doc.page_name
                                                            .chars()
                                                            .take(32)
                                                            .collect::<String>()
                                                    ))
                                                    .strong(),
                                                );
                                                let unit = doc.page_layout.unit;
                                                ui.label(format!(
                                                    "{:.1} × {:.1} {}",
                                                    unit.from_px(doc.page_size[0]),
                                                    unit.from_px(doc.page_size[1]),
                                                    unit.label()
                                                ));
                                                if ui
                                                    .button(t(if selected {
                                                        "Page active"
                                                    } else {
                                                        "Ouvrir"
                                                    }))
                                                    .clicked()
                                                {
                                                    action = Some(PageAction::Switch(i));
                                                }
                                            });
                                        });
                                    if i % 3 == 2 {
                                        ui.end_row();
                                    }
                                }
                            });
                    });
            });
            if modal.should_close() {
                self.show_pages = false;
            }
            if let Some(a) = action {
                self.page_action(a);
            }
        }
        if let Some(draft) = &mut self.page_draft {
            let mut apply = false;
            let mut cancel = false;
            let mut fit = false;
            let modal=egui::Modal::new("page_dimensions".into()).show(ctx,|ui| {
                ui.set_width(500.);ui.heading(t("Nom et dimensions de la page"));ui.add_space(8.);
                ui.label(t("Nom"));ui.add(egui::TextEdit::singleline(&mut draft.name).char_limit(200).desired_width(f32::INFINITY));
                egui::ComboBox::from_id_salt("paper_presets").selected_text(t("Formats de page")).width(300.).show_ui(ui,|ui| {
                    for p in pages::presets(){if ui.selectable_label(false,t(p.label)).clicked(){draft.size=p.size.map(|v|draft.unit.from_px(v));}}
                });
                let old=draft.unit;
                ui.horizontal(|ui| {ui.label(t("Unité"));egui::ComboBox::from_id_salt("paper_unit").selected_text(t(draft.unit.label())).show_ui(ui,|ui| {for unit in Unit::ALL {ui.selectable_value(&mut draft.unit,unit,t(unit.label()));}});});
                if old!=draft.unit {draft.size=draft.size.map(|v|draft.unit.from_px(old.to_px(v)));}
                ui.horizontal(|ui| {for (i,label) in [(0,"Largeur"),(1,"Hauteur")] {ui.label(t(label));ui.add(egui::DragValue::new(&mut draft.size[i]).speed(if draft.unit==Unit::Pixels {1.}else{0.1}).range(draft.unit.from_px(100.)..=draft.unit.from_px(100_000.)).suffix(format!(" {}",draft.unit.label())));}});
                ui.horizontal(|ui| {
                    if ui.selectable_label(draft.size[0]<=draft.size[1],t("Portrait")).clicked() && draft.size[0]>draft.size[1]{draft.size.swap(0,1);}
                    if ui.selectable_label(draft.size[0]>=draft.size[1],t("Paysage")).clicked() && draft.size[0]<draft.size[1]{draft.size.swap(0,1);}
                });
                ui.checkbox(&mut draft.auto,t("Extension automatique vers la droite et le bas"));
                ui.label(RichText::new(t("Les objets gardent leur taille et leur position. Une page plus petite peut rogner le dessin dans les exports.")).small().color(INFO_TEXT));
                if let Some(e)=&draft.error {ui.colored_label(ERROR_TEXT,e);}
                ui.separator();ui.horizontal(|ui| {
                    if ui.button(t("Ajuster au dessin")).clicked(){fit=true;}
                    if ui.button(t("Annuler")).clicked(){cancel=true;}
                    if ui.button(t("Appliquer")).clicked(){apply=true;}
                });
            });
            cancel |= modal.should_close();
            if apply {
                let index = draft.index;
                let name = draft.name.clone();
                let size = draft.size.map(|v| draft.unit.to_px(v));
                let layout = PageLayout {
                    unit: draft.unit,
                    auto_size: draft.auto,
                };
                if index != self.doc.page_index {
                    draft.error = Some("La page active a changé.".into());
                } else {
                    match self.doc.resize_page(name, size, layout) {
                        Ok(()) => {
                            self.page_draft = None;
                            self.fit_next = true;
                            self.commit();
                        }
                        Err(e) => draft.error = Some(e),
                    }
                }
            } else if fit {
                self.page_draft = None;
                self.page_action(PageAction::Fit);
            } else if cancel {
                self.page_draft = None;
            }
        }
    }
}
