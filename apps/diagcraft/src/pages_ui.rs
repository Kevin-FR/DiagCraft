use super::library_ui::paint_document_preview;
use super::*;
use diagcraft::pages::{self, PageLayout, Unit};

#[derive(Clone)]
pub(super) struct PageReturn {
    index: usize,
    document: Document,
}

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
    pub(super) fn switch_page(&mut self, index: usize) -> bool {
        if index >= self.doc.page_count() {
            return false;
        }
        let was_editing = self.rich_editor.is_some();
        if !self.commit_inline_if_needed() {
            return false;
        }
        if !was_editing {
            self.commit();
        }
        self.switch_page_committed(index)
    }
    // The caller must have committed the inline editor or document before calling.
    pub(super) fn switch_page_committed(&mut self, index: usize) -> bool {
        let previous = self.doc.page_index;
        if self.doc.switch_page(index).is_ok() {
            if previous != index {
                self.page_return = Some(PageReturn {
                    index: previous,
                    document: self.doc.clone(),
                });
            }
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
            true
        } else {
            false
        }
    }
    fn can_return_to_page(&self) -> bool {
        self.page_return.as_ref().is_some_and(|previous| {
            previous.index < self.doc.page_count()
                && previous.index != self.doc.page_index
                && previous.document == self.doc
        })
    }
    fn return_to_page(&mut self) {
        let was_editing = self.rich_editor.is_some();
        if !self.commit_inline_if_needed() {
            return;
        }
        if self.can_return_to_page() {
            let index = self.page_return.as_ref().unwrap().index;
            if was_editing {
                self.switch_page_committed(index);
            } else {
                self.switch_page(index);
            }
        } else {
            self.status = t("Changez de page pour activer le retour.");
        }
    }
    pub(super) fn open_page_settings(&mut self) {
        if !self.commit_inline_if_needed() {
            self.show_pages = false;
            return;
        }
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
        if let PageAction::Switch(index) = action {
            if self.switch_page(index) || self.rich_editor.is_some() {
                self.show_pages = false;
            }
            return;
        }
        let was_editing = self.rich_editor.is_some();
        if !self.commit_inline_if_needed() {
            self.show_pages = false;
            return;
        }
        match action {
            PageAction::Switch(_) => unreachable!(),
            PageAction::Settings => self.open_page_settings(),
            PageAction::Fit => {
                if let Err(e) = self.doc.fit_page_to_drawing() {
                    self.error = Some(e);
                    return;
                }
                self.fit_next = true;
                self.commit();
            }
            PageAction::Add(duplicate) => {
                if !was_editing {
                    self.commit();
                }
                if let Err(e) = self.doc.add_page(duplicate) {
                    self.error = Some(e);
                    return;
                }
                self.selected.clear();
                self.set_tool(Tool::Select);
                self.fit_next = true;
                self.commit();
            }
            PageAction::Remove => {
                if !was_editing {
                    self.commit();
                }
                if let Err(e) = self.doc.remove_page(self.doc.page_index) {
                    self.error = Some(e);
                    return;
                }
                self.selected.clear();
                self.fit_next = true;
                self.commit();
            }
            PageAction::Move(to) => {
                if !was_editing {
                    self.commit();
                }
                if let Err(e) = self.doc.move_page(self.doc.page_index, to) {
                    self.error = Some(e);
                    return;
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
                    let can_return = self.can_return_to_page();
                    if ui
                        .add_enabled_ui(can_return, |ui| {
                            panels_ui::icon_button(ui, "←", "Retour à la page précédente")
                        })
                        .inner
                        .on_disabled_hover_text(t("Changez de page pour activer le retour."))
                        .clicked()
                    {
                        self.return_to_page();
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn two_pages() -> (egui::Context, DrawApp) {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        doc.add_node(ShapeKind::Rectangle, [300., 300.]);
        doc.add_page(false).unwrap();
        doc.add_node(ShapeKind::Diamond, [300., 300.]);
        doc.switch_page(0).unwrap();
        let app = DrawApp::from_context(&ctx, doc, None, None);
        (ctx, app)
    }

    #[test]
    fn page_switch_commits_inline_text_once_and_preserves_undo() {
        let (_, mut app) = two_pages();
        let id = app.doc.nodes[0].id;
        let original = app.doc.nodes[0].text.clone();
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.text = "Modified before navigation".into();
        assert!(app.switch_page(1));
        assert!(app.rich_editor.is_none());
        assert_eq!(app.doc.page_index, 1);
        assert_eq!(
            app.doc.page(0).unwrap().nodes[0].text,
            "Modified before navigation"
        );
        app.undo();
        assert_eq!(app.doc.page(0).unwrap().nodes[0].text, original);
        assert!(!app.history.can_undo());
        assert!(app.history.can_redo());
    }

    #[test]
    fn nonexistent_page_does_not_commit_or_discard_the_inline_draft() {
        let (_, mut app) = two_pages();
        let id = app.doc.nodes[0].id;
        let before = app.doc.clone();
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.text = "Pending".into();
        assert!(!app.switch_page(99));
        assert_eq!(app.doc, before);
        assert_eq!(app.rich_editor.as_ref().unwrap().draft.text, "Pending");
        assert!(!app.history.can_undo());
    }

    #[test]
    fn unchanged_inline_draft_and_navigation_preserve_redo() {
        let (_, mut app) = two_pages();
        app.doc.title = "Undo branch".into();
        app.commit();
        app.undo();
        assert!(app.history.can_redo());
        let id = app.doc.nodes[0].id;
        app.open_node_text(id);
        assert!(app.switch_page(1));
        assert!(app.rich_editor.is_none());
        assert!(app.history.can_redo());
        assert!(!app.history.can_undo());
    }

    #[test]
    fn page_operations_refuse_invalid_inline_draft_without_changes() {
        for action in [
            PageAction::Switch(1),
            PageAction::Add(false),
            PageAction::Add(true),
            PageAction::Remove,
            PageAction::Move(1),
            PageAction::Settings,
            PageAction::Fit,
        ] {
            let (_, mut app) = two_pages();
            let id = app.doc.nodes[0].id;
            let before = app.doc.clone();
            app.open_node_text(id);
            app.rich_editor.as_mut().unwrap().draft.font_size = 201.;
            app.show_pages = true;
            app.page_action(action);
            assert_eq!(app.doc, before);
            assert_eq!(app.doc.page_index, 0);
            assert!(app.rich_editor.is_some());
            assert!(!app.show_pages);
            assert!(!app.history.can_undo());
            assert!(app.page_draft.is_none());
        }
    }

    #[test]
    fn direct_page_settings_refuse_invalid_inline_draft() {
        let (_, mut app) = two_pages();
        let id = app.doc.nodes[0].id;
        let before = app.doc.clone();
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.font_size = 201.;
        app.open_page_settings();
        assert!(app.page_draft.is_none());
        assert!(app.rich_editor.is_some());
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn adding_a_page_after_inline_edit_has_separate_undo_steps() {
        let (_, mut app) = two_pages();
        let id = app.doc.nodes[0].id;
        let original = app.doc.nodes[0].text.clone();
        app.open_node_text(id);
        app.rich_editor.as_mut().unwrap().draft.text = "Retained edit".into();
        app.page_action(PageAction::Add(false));
        assert_eq!(app.doc.page_count(), 3);
        assert_eq!(app.doc.page(0).unwrap().nodes[0].text, "Retained edit");
        app.undo();
        assert_eq!(app.doc.page_count(), 2);
        assert_eq!(app.doc.page(0).unwrap().nodes[0].text, "Retained edit");
        app.undo();
        assert_eq!(app.doc.page(0).unwrap().nodes[0].text, original);
        assert!(!app.history.can_undo());
    }

    #[test]
    fn refusing_last_page_deletion_preserves_selection_tool_view_and_history() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let id = doc.add_node(ShapeKind::Rectangle, [300., 300.]);
        let mut app = DrawApp::from_context(&ctx, doc.clone(), None, None);
        app.doc.title = "Redo branch".into();
        app.commit();
        app.undo();
        assert!(app.history.can_redo());
        app.selected = vec![id];
        app.tool = Tool::Shape(ShapeKind::Diamond);
        app.fit_next = false;
        app.zoom = 1.25;
        app.pan = vec2(42., 27.);
        app.page_action(PageAction::Remove);
        assert!(app.error.is_some());
        assert_eq!(app.doc, doc);
        assert_eq!(app.selected, vec![id]);
        assert!(matches!(app.tool, Tool::Shape(ShapeKind::Diamond)));
        assert!(!app.fit_next);
        assert_eq!(app.zoom, 1.25);
        assert_eq!(app.pan, vec2(42., 27.));
        assert!(!app.history.can_undo());
        assert!(app.history.can_redo());
    }

    #[test]
    fn refusing_101st_page_preserves_selection_tool_view_and_history() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        for _ in 1..100 {
            doc.add_page(false).unwrap();
        }
        let id = doc.add_node(ShapeKind::Rectangle, [300., 300.]);
        let mut app = DrawApp::from_context(&ctx, doc.clone(), None, None);
        app.doc.title = "Redo branch".into();
        app.commit();
        app.undo();
        assert!(app.history.can_redo());
        app.selected = vec![id];
        app.tool = Tool::Shape(ShapeKind::Diamond);
        app.fit_next = false;
        app.zoom = 1.25;
        app.pan = vec2(42., 27.);
        app.page_action(PageAction::Add(false));
        assert!(app.error.is_some());
        assert_eq!(app.doc, doc);
        assert_eq!(app.selected, vec![id]);
        assert!(matches!(app.tool, Tool::Shape(ShapeKind::Diamond)));
        assert!(!app.fit_next);
        assert_eq!(app.zoom, 1.25);
        assert_eq!(app.pan, vec2(42., 27.));
        assert!(!app.history.can_undo());
        assert!(app.history.can_redo());
    }

    #[test]
    fn return_navigation_is_reversible_and_does_not_edit_the_document() {
        let (_, mut app) = two_pages();
        app.doc.title = "Undo branch".into();
        app.commit();
        app.undo();
        assert!(app.history.can_redo());
        let before = app.doc.clone();
        app.switch_page(1);
        assert!(app.can_return_to_page());
        app.switch_page(1);
        assert!(app.can_return_to_page());
        app.return_to_page();
        assert_eq!(app.doc.page_index, 0);
        assert_eq!(app.doc, before);
        assert!(app.can_return_to_page());
        app.return_to_page();
        assert_eq!(app.doc.page_index, 1);
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
        assert!(app.history.can_redo());
    }

    #[test]
    fn changed_document_does_not_return_to_a_potentially_wrong_page() {
        let (_, mut app) = two_pages();
        app.switch_page(1);
        app.doc.move_page(0, 1).unwrap();
        assert!(!app.can_return_to_page());
        let current = app.doc.page_index;
        app.return_to_page();
        assert_eq!(app.doc.page_index, current);
    }

    #[test]
    fn return_navigation_belongs_to_each_tab() {
        let (_, mut app) = two_pages();
        app.switch_page(1);
        app.open_tab(Document::default(), None);
        assert!(!app.can_return_to_page());
        app.switch_tab(0);
        assert!(app.can_return_to_page());
        app.return_to_page();
        assert_eq!(app.doc.page_index, 0);
        app.switch_tab(1);
        assert!(!app.can_return_to_page());
        app.close_tab(1);
        assert!(app.can_return_to_page());
    }

    #[test]
    fn return_button_can_be_reached_and_activated_from_keyboard() {
        let (ctx, mut app) = two_pages();
        app.switch_page(1);
        let frame = |app: &mut DrawApp, events| {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(600., 680.))),
                    events,
                    focused: true,
                    ..Default::default()
                },
                |ctx| app.pages_bar(ctx),
            )
        };
        frame(&mut app, vec![]);
        let event = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&mut app, vec![event(egui::Key::Tab, true)]);
        frame(&mut app, vec![event(egui::Key::Tab, false)]);
        frame(&mut app, vec![event(egui::Key::Enter, true)]);
        frame(&mut app, vec![event(egui::Key::Enter, false)]);
        assert_eq!(app.doc.page_index, 0);
    }
    #[test]
    fn return_button_is_accessible_in_seven_languages_and_can_be_clicked() {
        for language in Language::ALL {
            let (ctx, mut app) = two_pages();
            ctx.enable_accesskit();
            app.set_language(Some(language));
            app.switch_page(1);
            let frame = |app: &mut DrawApp, events| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 680.))),
                        events,
                        focused: true,
                        ..Default::default()
                    },
                    |ctx| app.pages_bar(ctx),
                )
            };
            frame(&mut app, vec![]);
            let output = frame(&mut app, vec![]);
            let tree = output.platform_output.accesskit_update.unwrap();
            assert!(
                tree.nodes.iter().any(
                    |(_, node)| node.label() == Some(t("Retour à la page précédente").as_str())
                )
            );
            let point = output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::Shape::Text(text) if text.galley.text() == "←" => {
                        Some(text.pos + text.galley.size() / 2.)
                    }
                    _ => None,
                })
                .expect("return button must be visible");
            for pressed in [true, false] {
                frame(
                    &mut app,
                    vec![
                        egui::Event::PointerMoved(point),
                        egui::Event::PointerButton {
                            pos: point,
                            button: egui::PointerButton::Primary,
                            pressed,
                            modifiers: egui::Modifiers::NONE,
                        },
                    ],
                );
            }
            assert_eq!(app.doc.page_index, 0);
        }
        i18n::activate(Language::French);
    }
}
