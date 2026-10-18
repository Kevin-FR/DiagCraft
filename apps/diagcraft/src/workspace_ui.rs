use super::*;
use library_ui::{paint_document_preview, symbol_tile};
impl DrawApp {
    pub(super) fn open_symbol_manager(&mut self) {
        self.visible_draft = self.library.visible_libraries.clone();
        self.manager_search.clear();
        self.manager_category = "Toutes".into();
        self.context_open = false;
        self.show_symbol_manager = true;
    }
    pub(super) fn apply_libraries(&mut self) {
        let mut next = self.library.clone();
        next.visible_libraries = self.visible_draft.clone();
        match self.persist_library(&next) {
            Ok(()) => {
                self.library = next;
                self.show_symbol_manager = false;
                self.status = "Bibliothèques mises à jour".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn open_gallery(&mut self) {
        self.gallery_previews = self
            .gallery
            .projects()
            .iter()
            .map(|p| (p.clone(), self.gallery.revision(&p.key, 0).ok()))
            .collect();
        self.show_gallery = true;
        self.gallery_versions = None;
        self.context_open = false;
    }
    pub(super) fn open_versions(&mut self, key: String) {
        let count = self
            .gallery
            .projects()
            .iter()
            .find(|p| p.key == key)
            .map(|p| p.revisions.len())
            .unwrap_or(0);
        self.gallery_revision_previews = (0..count)
            .map(|i| self.gallery.revision(&key, i).ok())
            .collect();
        self.gallery_versions = Some(key);
    }
    pub(super) fn restore_revision(&mut self, key: &str, index: usize) {
        match self.gallery.revision(key, index) {
            Ok(mut doc) => {
                doc.title = format!("Copie — {}", doc.title);
                self.open_tab(doc, None);
                self.show_gallery = false;
                self.status="Version récupérée dans un nouvel onglet. Enregistrez la copie à l’emplacement de votre choix.".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn workspace_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_symbol_manager {
            let modal=egui::Modal::new("symbol_manager".into()).show(ctx,|ui|{
                ui.set_width(940_f32.min(ctx.content_rect().width()-60.));
                ui.heading(t("Ajouter une bibliothèque de symboles"));
                ui.label(RichText::new(t("Choisissez les bibliothèques affichées dans le volet de gauche.")).color(MUTED));
                ui.add_space(8.);ui.separator();
                let body_height=(ctx.content_rect().height()-240.).clamp(200.,560.);
                ui.horizontal_top(|ui|{
                    ui.set_height(body_height);
                    ui.vertical(|ui|{
                        ui.set_width(175.);
                        for category in ["Toutes","Ma bibliothèque","Base","Idées","Logiciel"] {ui.selectable_value(&mut self.manager_category,category.into(),t(category));}
                        ui.add_space(14.);
                        if ui.button(t("Tout afficher")).clicked(){self.visible_draft=std::iter::once("personal".into()).chain(symbols::LIBRARIES.iter().map(|l|l.id.into())).collect();}
                        if ui.button(t("Tout masquer")).clicked(){self.visible_draft.clear();}
                    });
                    ui.separator();
                    ui.vertical(|ui|{
                        ui.set_width(ui.available_width());
                        ui.add(egui::TextEdit::singleline(&mut self.manager_search).hint_text(t("Rechercher une bibliothèque ou un symbole…")).desired_width(f32::INFINITY).char_limit(120));
                        egui::ScrollArea::vertical().id_salt("manager_libraries").max_height(body_height-45.).show(ui,|ui|{
                            if (self.manager_category=="Toutes"||self.manager_category=="Ma bibliothèque") && search(&t("Ma bibliothèque"),&self.manager_search) {
                                library_card(ui,"personal","Ma bibliothèque",&mut self.visible_draft,|ui|{if self.library.symbols.is_empty(){ui.label(t("Ajoutez vos formes depuis la sélection ou son menu contextuel."));}else{ui.horizontal_wrapped(|ui|{for n in &self.library.symbols {let _=symbol_tile(ui,n.kind,false);}});}});
                            }
                            for lib in symbols::LIBRARIES {
                                if self.manager_category!="Toutes" && self.manager_category!=lib.category {continue;}
                                if !search(&t(lib.title),&self.manager_search)&&!lib.kinds.iter().any(|k|search(&t(k.label()),&self.manager_search)){continue;}
                                library_card(ui,lib.id,lib.title,&mut self.visible_draft,|ui|{
                                    ui.horizontal_wrapped(|ui|{for &kind in lib.kinds {let _=symbol_tile(ui,kind,false);}});
                                });
                            }
                        });
                    });
                });
                ui.separator();ui.horizontal(|ui|{
                    ui.label(i18n::render("{0} bibliothèques visibles · {1} formes disponibles", &[("0", format!("{}", self.visible_draft.len())),("1", format!("{}", symbols::all_kinds().len()))]));
                    ui.with_layout(Layout::right_to_left(Align::Center),|ui|{
                        if ui.button(RichText::new(t("Appliquer")).color(ACCENT)).clicked(){self.apply_libraries();}
                        if ui.button(t("Annuler")).clicked(){self.show_symbol_manager=false;}
                    });
                });
            });
            if modal.should_close() {
                self.show_symbol_manager = false;
            }
        }
        if self.show_gallery {
            let modal=egui::Modal::new("saved_projects".into()).show(ctx,|ui|{
                ui.set_width(970_f32.min(ctx.content_rect().width()-60.));
                ui.set_height((ctx.content_rect().height()-160.).clamp(400.,700.));
                ui.horizontal(|ui|{
                    ui.heading(t(if self.gallery_versions.is_some(){"Versions enregistrées"}else{"Mes travaux"}));
                    ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button(t("Fermer")).clicked(){self.show_gallery=false;}});
                });
                ui.label(RichText::new(t("Fichiers .draw éditables · 10 versions par travail · historique conservé sur cet ordinateur")).color(MUTED));
                ui.separator();
                if let Some(key)=self.gallery_versions.clone(){
                    if ui.button(t("← Tous mes travaux")).clicked(){self.gallery_versions=None;}
                    if let Some(project)=self.gallery.projects().iter().find(|p|p.key==key).cloned(){
                        ui.label(RichText::new(&project.title).strong());
                        ui.label(RichText::new(project.path.to_string_lossy()).small().color(MUTED));
                        let mut restore=None;
                        egui::ScrollArea::vertical().id_salt("project_versions").max_height((ctx.content_rect().height()-270.).clamp(180.,540.)).show(ui,|ui|{
                            for (i,r) in project.revisions.iter().enumerate(){
                                egui::Frame::new().fill(SURFACE).corner_radius(8).inner_margin(12).show(ui,|ui|{
                                    ui.horizontal(|ui|{
                                        if let Some(doc)=self.gallery_revision_previews.get(i).and_then(Option::as_ref){let (rect,_)=ui.allocate_exact_size(vec2(200.,130.),Sense::hover());paint_document_preview(ui.painter(),doc,rect);}else{ui.label(t("Aperçu indisponible"));}
                                        ui.vertical(|ui|{
                                            ui.label(RichText::new(t(if i==0 {"Dernière version".into()}else{i18n::render("Version précédente {0}", &[("0", format!("{}", project.revisions.len()-i))])})).strong());
                                            ui.label(date(r.saved_at));ui.label(format!("{:.1} Ko",r.bytes as f32/1024.));
                                            if ui.button(t("Récupérer une copie")).clicked(){restore=Some(i);}
                                        });
                                    });
                                });ui.add_space(10.);
                            }
                        });
                        if let Some(i)=restore{self.restore_revision(&key,i);}
                    }
                }else{
                    ui.add(egui::TextEdit::singleline(&mut self.gallery_search).hint_text(t("Rechercher dans mes travaux…")).char_limit(120).desired_width(f32::INFINITY));
                    if self.gallery_previews.is_empty(){
                        ui.add_space(50.);ui.heading(t("Vos dessins commencent ici"));ui.label(t("Enregistrez un document avec Ctrl/⌘ S. Vous le retrouverez ici avec son aperçu et ses versions."));ui.add_space(50.);
                    }
                    let entries=&self.gallery_previews;let mut open=None;let mut versions=None;
                    egui::ScrollArea::vertical().id_salt("saved_project_grid").max_height((ctx.content_rect().height()-220.).clamp(180.,570.)).show(ui,|ui|{
                        let width=((ui.available_width()-30.)/3.).max(220.);
                        egui::Grid::new("projects_grid").num_columns(3).spacing(vec2(14.,14.)).show(ui,|ui|{
                            let mut shown=0;
                            for (p,doc) in entries {
                                if !search(&p.title,&self.gallery_search)&&!search(&p.path.to_string_lossy(),&self.gallery_search){continue;}
                                egui::Frame::new().fill(SURFACE).corner_radius(10).inner_margin(10).show(ui,|ui|{ui.vertical(|ui|{
                                    ui.set_width(width-20.);
                                    let (rect,response)=ui.allocate_exact_size(vec2(width-20.,155.),Sense::click());
                                    if let Some(doc)=doc {paint_document_preview(ui.painter(),doc,rect);}else{ui.painter().text(rect.center(),Align2::CENTER_CENTER,t("Aperçu indisponible"),FontId::proportional(14.),MUTED);}
                                    if response.double_clicked(){open=Some(p.clone());}
                                    ui.add(egui::Label::new(RichText::new(&p.title).strong()).truncate()).on_hover_text(&p.title);
                                    ui.add(egui::Label::new(RichText::new(p.path.file_name().unwrap_or_default().to_string_lossy()).small().color(MUTED)).truncate()).on_hover_text(p.path.to_string_lossy());
                                    if let Some(r)=p.revisions.first(){ui.label(RichText::new(date(r.saved_at)).small().color(MUTED));}
                                    ui.horizontal(|ui|{
                                        if ui.button(t("Ouvrir")).clicked(){open=Some(p.clone());}
                                        if ui.button(i18n::render("Versions ({0})", &[("0",p.revisions.len().to_string())])).clicked(){versions=Some(p.key.clone());}
                                    });
                                });});
                                shown+=1;if shown%3==0{ui.end_row();}
                            }
                            if shown==0&&!entries.is_empty(){ui.label(t("Aucun travail trouvé."));}
                        });
                    });
                    if let Some(p)=open {
                        if p.path.is_file(){self.request(Pending::Open(p.path));self.show_gallery=false;}else{self.restore_revision(&p.key,0);}
                    }
                    if let Some(key)=versions{self.open_versions(key);}
                }
            });
            if modal.should_close() {
                self.show_gallery = false;
            }
        }
        if self.show_pdf_export {
            let modal = egui::Modal::new("pdf_export".into()).show(ctx, |ui| {
                ui.set_width(440.);
                ui.heading(t("Exporter en PDF"));
                ui.label(t("Export vectoriel avec polices intégrées."));
                ui.add_space(12.);
                egui::ComboBox::from_id_salt("pdf_page_size")
                    .selected_text(t(self.pdf_format.label()))
                    .show_ui(ui, |ui| {
                        for format in pdf::PageFormat::ALL {
                            ui.selectable_value(&mut self.pdf_format, format, t(format.label()));
                        }
                    });
                ui.checkbox(
                    &mut self.pdf_all_pages,
                    i18n::render(
                        "Toutes les pages ({0})",
                        &[("0", format!("{}", self.doc.page_count()))],
                    ),
                );
                ui.label(t("Chaque dessin est ajusté en conservant ses proportions."));
                ui.add_space(14.);
                ui.horizontal(|ui| {
                    if ui.button(t("Annuler")).clicked() {
                        self.show_pdf_export = false;
                    }
                    if ui.button(t("Exporter…")).clicked() {
                        self.export_pdf();
                    }
                });
            });
            if modal.should_close() {
                self.show_pdf_export = false;
            }
        }
    }
    fn export_pdf(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(t("Exporter en PDF"))
            .add_filter(t("Document PDF"), &["pdf"])
            .set_file_name("diagramme.pdf")
            .save_file()
        else {
            return;
        };
        let result = if self.pdf_all_pages {
            pdf::export(&self.doc, self.pdf_format)
        } else {
            pdf::export_current(&self.doc, self.pdf_format)
        };
        match result.and_then(|b| document::atomic_write(&path, &b)) {
            Ok(()) => {
                self.status = i18n::render(
                    "PDF exporté : {0}",
                    &[(
                        "0",
                        format!("{}", path.file_name().unwrap_or_default().to_string_lossy()),
                    )],
                );
                self.show_pdf_export = false;
            }
            Err(e) => {
                self.error = Some(i18n::render(
                    "Impossible d’exporter le PDF : {0}",
                    &[("0", e.to_string())],
                ))
            }
        }
    }
}
fn search(s: &str, q: &str) -> bool {
    s.to_lowercase().contains(&q.trim().to_lowercase())
}
fn library_card(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    draft: &mut Vec<String>,
    contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::new()
        .stroke(Stroke::new(1., BORDER))
        .corner_radius(10)
        .inner_margin(14)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let mut checked = draft.iter().any(|s| s == id);
            if ui
                .checkbox(&mut checked, RichText::new(t(title)).strong())
                .changed()
            {
                draft.retain(|s| s != id);
                if checked {
                    draft.push(id.into());
                }
            }
            ui.add_space(6.);
            contents(ui);
        });
    ui.add_space(12.);
}
// Civil date conversion avoids locale or OS dependencies for saved timestamps.
fn date(seconds: u64) -> String {
    let days = (seconds / 86400) as i64;
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    format!(
        "{d:02}/{m:02}/{y} · {:02}:{:02} UTC",
        seconds % 86400 / 3600,
        seconds % 3600 / 60
    )
}
