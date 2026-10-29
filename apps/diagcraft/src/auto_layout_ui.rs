//! Preview state is transient and never edits a drawing until Apply.
use super::*;
use diagcraft::auto_layout::{self, Direction, Spacing};

pub(super) struct Panel {
    source: Document,
    tab: usize,
    direction: Direction,
    spacing: Spacing,
    selection_only: bool,
    selection: Vec<u64>,
    preview: Result<auto_layout::Preview, String>,
}
impl Panel {
    fn refresh(&mut self) {
        self.preview = auto_layout::preview_with_spacing(
            &self.source,
            self.selection_only.then_some(self.selection.as_slice()),
            self.direction,
            self.spacing,
        );
    }
    fn current(&self, app: &DrawApp) -> bool {
        self.tab == app.active_tab
            && self.source.page_index == app.doc.page_index
            && self.source == app.doc
    }
}
impl DrawApp {
    pub(super) fn open_auto_layout(&mut self) {
        self.commit();
        let mut panel = Panel {
            source: self.doc.clone(),
            tab: self.active_tab,
            direction: Direction::Horizontal,
            spacing: Spacing::default(),
            selection_only: !self.selected.is_empty(),
            selection: self.selected.clone(),
            preview: Err(String::new()),
        };
        panel.refresh();
        self.auto_layout_ui = Some(panel);
    }
    fn apply_auto_layout(&mut self, panel: &Panel) -> bool {
        if !panel.current(self) {
            return false;
        }
        let Ok(preview) = &panel.preview else {
            return false;
        };
        self.doc = preview.document.clone();
        self.commit();
        self.fit_next = true;
        true
    }
    pub(super) fn auto_layout_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut panel) = self.auto_layout_ui.take() else {
            return;
        };
        let mut close = false;
        let modal = egui::Modal::new("auto_layout_preview".into()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 100.).min(760.));
            ui.heading(t("Mise en page automatique…"));
            ui.horizontal_wrapped(|ui| {
                let before = (panel.direction, panel.selection_only);
                ui.selectable_value(&mut panel.direction, Direction::Horizontal, t("Horizontal"));
                ui.selectable_value(&mut panel.direction, Direction::Vertical, t("Vertical"));
                ui.checkbox(&mut panel.selection_only, t("Sélection uniquement"));
                if before != (panel.direction, panel.selection_only) { panel.refresh(); }
            });
            let previous = panel.spacing;
            ui.horizontal_wrapped(|ui| {
                ui.label(t("Espacement"));
                for (label, gap) in [("Compact", 30.), ("Normal", 60.), ("Aéré", 120.)] {
                    if ui.selectable_label(panel.spacing == Spacing { levels: gap, blocks: gap }, t(label)).clicked() {
                        panel.spacing = Spacing { levels: gap, blocks: gap };
                    }
                }
            });
            ui.horizontal_wrapped(|ui| {
                let levels = ui.label(t("Entre les niveaux"));
                ui.add(egui::DragValue::new(&mut panel.spacing.levels).range(20. ..=300.).speed(1.).suffix(" px")).labelled_by(levels.id);
                let blocks = ui.label(t("Entre les blocs"));
                ui.add(egui::DragValue::new(&mut panel.spacing.blocks).range(20. ..=300.).speed(1.).suffix(" px")).labelled_by(blocks.id);
            });
            if panel.spacing != previous { panel.refresh(); }
            ui.label(t("Les groupes, textes et blocs protégés restent en place. Les points manuels sont conservés."));
            if !panel.current(self) {
                ui.colored_label(theme_ui::ERROR_TEXT, t("Le document a changé. Actualisez l’aperçu avant de continuer."));
                if ui.button(t("Actualiser")).clicked() {
                    panel.source = self.doc.clone(); panel.tab = self.active_tab;
                    panel.selection = self.selected.clone(); panel.refresh();
                }
            }
            match &panel.preview {
                Ok(preview) => {
                    ui.label(i18n::render("{0} blocs déplacés · {1} blocs conservés", &[("0", preview.moved.to_string()), ("1", preview.fixed.to_string())]));
                    ui.label(i18n::render("Croisements entre niveaux voisins : {0} → {1}", &[("0", preview.crossings_before.to_string()), ("1", preview.crossings_after.to_string())])).on_hover_text(t("Comparaison avec le placement automatique initial. Les routes finales peuvent différer."));
                    ui.label(t("Aperçu des positions · liaisons simplifiées"));
                    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), (ctx.content_rect().height() - 425.).clamp(120., 280.)), Sense::hover());
                    paint_preview(ui.painter(), &preview.document, rect);
                    ui.label(t("La page peut s’agrandir. Vérifiez l’aperçu avant d’appliquer."));
                }
                Err(error) => { ui.colored_label(theme_ui::ERROR_TEXT, t(error)); }
            }
            ui.horizontal(|ui| {
                if ui.button(t("Annuler")).clicked() { close = true; }
                let enabled = panel.current(self) && panel.preview.as_ref().is_ok_and(|p| p.moved > 0);
                if ui.add_enabled(enabled, egui::Button::new(t("Appliquer"))).clicked() {
                    close = self.apply_auto_layout(&panel);
                }
            });
        });
        if !close && !modal.should_close() {
            self.auto_layout_ui = Some(panel);
        }
    }
}
fn paint_preview(p: &egui::Painter, doc: &Document, rect: Rect) {
    p.rect_filled(rect, 4, Color32::WHITE);
    let painter = p.with_clip_rect(rect);
    let scale =
        ((rect.width() - 16.) / doc.page_size[0]).min((rect.height() - 16.) / doc.page_size[1]);
    let origin = rect.center() - vec2(doc.page_size[0], doc.page_size[1]) * scale / 2.;
    let map = |point: Point| origin + vec2(point[0], point[1]) * scale;
    let view = doc.organization_view();
    let nodes: std::collections::BTreeMap<_, _> = doc.nodes.iter().map(|n| (n.id, n)).collect();
    // Position-only overview: straight centre lines avoid routing work each frame.
    for edge in &doc.connectors {
        if !view.is_object_visible(edge.id) {
            continue;
        }
        let start = nodes
            .get(&edge.from)
            .map(|n| n.center())
            .or(edge.free_start);
        let end = nodes.get(&edge.to).map(|n| n.center()).or(edge.free_end);
        if let (Some(a), Some(b)) = (start, end) {
            painter.line_segment([map(a), map(b)], Stroke::new(1., BORDER));
        }
    }
    for node in &doc.nodes {
        if !view.is_object_visible(node.id) {
            continue;
        }
        let r = Rect::from_min_size(map(node.position), vec2(node.size[0], node.size[1]) * scale);
        painter.rect(
            r,
            2,
            rgb(node.fill),
            Stroke::new(1., rgb(node.stroke)),
            StrokeKind::Inside,
        );
    }
    for chart in &doc.charts {
        if view.is_object_visible(chart.id) {
            let r = Rect::from_min_size(
                map(chart.position),
                vec2(chart.size[0], chart.size[1]) * scale,
            );
            painter.rect(r, 2, SURFACE, Stroke::new(1., BORDER), StrokeKind::Inside);
        }
    }
    painter.rect_stroke(
        Rect::from_min_size(origin, vec2(doc.page_size[0], doc.page_size[1]) * scale),
        0,
        Stroke::new(1., BORDER),
        StrokeKind::Inside,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preview_is_read_only_apply_is_one_undo_and_stale_state_is_rejected() {
        let ctx = egui::Context::default();
        let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
        let before = app.doc.clone();
        app.open_auto_layout();
        assert!(app.command_dialog_blocked());
        assert_eq!(app.doc, before);
        let panel = app.auto_layout_ui.take().unwrap();
        assert!(panel.preview.is_ok());
        app.doc.title = "Changed".into();
        assert!(!app.apply_auto_layout(&panel));
        app.doc = before.clone();
        app.doc.add_page(false).unwrap();
        assert!(!app.apply_auto_layout(&panel));
        app.doc = before.clone();
        assert!(app.apply_auto_layout(&panel));
        assert_ne!(app.doc, before);
        app.undo();
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
    #[test]
    fn seven_language_modal_fits_short_windows_without_editing_the_drawing() {
        for language in Language::ALL {
            let ctx = egui::Context::default();
            let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
            app.set_language(Some(language));
            let before = app.doc.clone();
            app.open_auto_layout();
            for frame in 0..3 {
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 680.))),
                        ..Default::default()
                    },
                    |ctx| app.auto_layout_dialog(ctx),
                );
                let mut buttons = 0;
                for shape in output.shapes {
                    if let egui::Shape::Text(text) = shape.shape
                        && [t("Appliquer"), t("Annuler")].contains(&text.galley.text().to_owned())
                    {
                        buttons += 1;
                        assert!(text.pos.y >= 0. && text.pos.y + text.galley.size().y <= 680.);
                    }
                }
                if frame == 2 {
                    assert_eq!(buttons, 2);
                }
            }
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
            // Cancel by dropping transient preview, with no document edit.
            app.auto_layout_ui = None;
            assert!(!app.command_dialog_blocked());
        }
        i18n::activate(Language::French);
    }
    #[test]
    fn preset_clicks_refresh_only_the_preview_and_apply_as_one_edit() {
        for language in Language::ALL {
            let ctx = egui::Context::default();
            let mut app = DrawApp::from_context(&ctx, Document::demo(), None, None);
            app.set_language(Some(language));
            app.open_auto_layout();
            let before = app.doc.clone();
            let render = |app: &mut DrawApp, events| {
                ctx.run(
                    egui::RawInput {
                        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 680.))),
                        focused: true,
                        events,
                        ..Default::default()
                    },
                    |ctx| app.auto_layout_dialog(ctx),
                )
            };
            for (label, gap) in [("Compact", 30.), ("Normal", 60.), ("Aéré", 120.)] {
                render(&mut app, vec![]);
                let output = render(&mut app, vec![]);
                let point = output
                    .shapes
                    .iter()
                    .find_map(|shape| {
                        if let egui::Shape::Text(text) = &shape.shape
                            && text.galley.text() == t(label)
                        {
                            Some(text.pos + text.galley.size() / 2.)
                        } else {
                            None
                        }
                    })
                    .expect("preset must be visible in the short window");
                for pressed in [true, false] {
                    render(
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
                let panel = app.auto_layout_ui.as_ref().unwrap();
                assert_eq!(
                    panel.spacing,
                    Spacing {
                        levels: gap,
                        blocks: gap
                    }
                );
                let expected = auto_layout::preview_with_spacing(
                    &before,
                    None,
                    Direction::Horizontal,
                    panel.spacing,
                )
                .unwrap();
                assert_eq!(panel.preview.as_ref().unwrap().document, expected.document);
                assert_eq!(app.doc, before);
                assert!(!app.history.can_undo());
            }
            let mut panel = app.auto_layout_ui.take().unwrap();
            panel.spacing = Spacing {
                levels: 85.,
                blocks: 35.,
            };
            panel.refresh();
            let expected = panel.preview.as_ref().unwrap().document.clone();
            assert!(app.apply_auto_layout(&panel));
            assert_eq!(app.doc, expected);
            app.undo();
            assert_eq!(app.doc, before);
            assert!(!app.history.can_undo());
        }
        i18n::activate(Language::French);
    }
}
