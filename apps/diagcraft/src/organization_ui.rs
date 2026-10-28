//! Page organization, shared selection rules and a compact layer manager.
use super::*;

impl DrawApp {
    pub(super) fn assign_new_objects(&mut self, before: &Document) {
        self.normalize_selection();
        if self.drawing_layer == 0 || before.page_index != self.doc.page_index {
            return;
        }
        let previous: std::collections::HashSet<_> = before.object_ids().into_iter().collect();
        let ids = self
            .doc
            .object_ids()
            .into_iter()
            .filter(|id| !previous.contains(id) && self.doc.object_layer(*id) == 0)
            .collect::<Vec<_>>();
        if !ids.is_empty()
            && let Err(error) = self.doc.move_objects_to_layer(&ids, self.drawing_layer)
        {
            self.doc = before.clone();
            self.status = error;
        }
    }

    pub(super) fn selection_editable(&self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let view = self.doc.organization_view();
        self.selected.iter().all(|id| view.can_edit_object(*id))
    }

    pub(super) fn selection_duplicable(&self) -> bool {
        if self.selected.is_empty() {
            return false;
        }
        let view = self.doc.organization_view();
        let locked_layers: std::collections::HashSet<_> = self
            .doc
            .organization
            .layers
            .iter()
            .filter(|l| l.locked)
            .map(|l| l.id)
            .collect();
        let hidden_groups: std::collections::HashSet<_> = self
            .doc
            .organization
            .groups
            .iter()
            .filter(|g| g.members.iter().any(|id| !view.is_object_visible(*id)))
            .map(|g| g.id)
            .collect();
        self.selected.iter().all(|id| {
            view.is_object_visible(*id)
                && !locked_layers.contains(&view.object_layer(*id))
                && view
                    .group_for_object(*id)
                    .is_none_or(|group| !hidden_groups.contains(&group))
        })
    }

    pub(super) fn organized_hit(&self, point: Point) -> Option<u64> {
        let layers = self.doc.layer_ids();
        [
            self.doc.hit_drawing(point, 7. / self.zoom),
            self.hit_connector_cached(point, 7. / self.zoom),
            self.doc.hit_node(point),
            self.doc.hit_chart(point),
        ]
        .into_iter()
        .enumerate()
        .filter_map(|(kind, id)| id.map(|id| (kind, id)))
        .max_by_key(|(kind, id)| {
            (
                layers
                    .iter()
                    .position(|layer| *layer == self.doc.object_layer(*id)),
                *kind,
            )
        })
        .map(|(_, id)| id)
    }

    pub(super) fn normalize_selection(&mut self) {
        let view = self.doc.organization_view();
        let expanded = view.expand_groups(&self.selected);
        let valid: std::collections::HashSet<_> = expanded.iter().copied().collect();
        self.selected.retain(|id| valid.contains(id));
        let mut selected: std::collections::HashSet<_> = self.selected.iter().copied().collect();
        for id in expanded {
            if selected.insert(id) {
                self.selected.push(id);
            }
        }
        self.selected.retain(|id| view.is_object_visible(*id));
        if self.organization_page != self.doc.page_index {
            self.organization_page = self.doc.page_index;
            self.drawing_layer = 0;
            self.layer_names.clear();
            self.show_layers = false;
        }
        if self.drawing_layer != 0
            && !self
                .doc
                .organization
                .layers
                .iter()
                .any(|layer| layer.id == self.drawing_layer && layer.visible && !layer.locked)
        {
            self.drawing_layer = 0;
        }
        if self
            .connect_from
            .is_some_and(|id| id != 0 && !view.is_object_visible(id))
        {
            self.clear_connection();
        }
        if self
            .hover_link_node
            .is_some_and(|id| !view.is_object_visible(id))
        {
            self.hover_link_node = None;
        }
    }

    pub(super) fn reset_organization_view(&mut self) {
        self.organization_page = self.doc.page_index;
        self.drawing_layer = 0;
        self.layer_names.clear();
        self.show_layers = false;
    }

    fn organization_result(&mut self, result: Result<(), String>, message: String) {
        match result {
            Ok(()) => {
                self.normalize_selection();
                self.commit();
                self.status = message;
            }
            Err(error) => self.error = Some(error),
        }
    }

    pub(super) fn group_selection(&mut self) {
        self.normalize_selection();
        if !self.selection_editable() || self.selected.len() < 2 {
            self.status = "Sélectionnez au moins deux objets modifiables du même calque".into();
            return;
        }
        self.commit();
        let result = self.doc.group_objects(&self.selected).map(|_| ());
        self.organization_result(result, t("Objets groupés · Ctrl/⌘ Z pour annuler"));
    }

    pub(super) fn ungroup_selection(&mut self) {
        if !self.selection_editable() {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        self.commit();
        let result = self.doc.ungroup_objects(&self.selected);
        self.organization_result(
            result,
            t("Groupe dissocié · les objets restent sélectionnés"),
        );
    }

    pub(super) fn set_selection_locked(&mut self, locked: bool) {
        self.normalize_selection();
        if self.selected.is_empty() {
            return;
        }
        if !locked
            && self.selected.iter().any(|id| {
                self.doc
                    .organization
                    .layers
                    .iter()
                    .any(|layer| layer.id == self.doc.object_layer(*id) && layer.locked)
            })
        {
            self.open_layers();
            self.status = t("Déverrouillez le calque pour modifier ses objets");
            return;
        }
        self.commit();
        let result = self.doc.set_objects_locked(&self.selected, locked);
        self.organization_result(
            result,
            t(if locked {
                "Sélection verrouillée"
            } else {
                "Sélection déverrouillée"
            }),
        );
    }

    pub(super) fn set_layer_visible(&mut self, id: u64, visible: bool) {
        self.commit();
        let result = self.doc.update_layer(id, None, Some(visible), None);
        self.organization_result(result, t("Visibilité du calque mise à jour"));
    }

    pub(super) fn open_layers(&mut self) {
        self.commit();
        self.clear_connection();
        self.context_open = false;
        self.layer_names = self
            .doc
            .organization
            .layers
            .iter()
            .map(|l| (l.id, l.name.clone()))
            .collect();
        self.show_layers = true;
    }

    pub(super) fn organization_menu(&mut self, ui: &mut egui::Ui) {
        use commands_ui::Command;
        ui.menu_button(t("Organiser"), |ui| {
            for command in [
                Command::Group,
                Command::Ungroup,
                Command::Lock,
                Command::Unlock,
            ] {
                self.command_button(ui, command);
            }
            ui.separator();
            self.command_button(ui, Command::Layers);
        });
    }

    pub(super) fn organization_inspector(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            if !self.selected.is_empty() {
                let view = self.doc.organization_view();
                let locked = self.selected.iter().any(|id| view.is_object_locked(*id));
                if ui
                    .button(t(if locked {
                        "Déverrouiller"
                    } else {
                        "Verrouiller"
                    }))
                    .clicked()
                {
                    self.set_selection_locked(!locked);
                }
                self.organization_menu(ui);
            } else if ui.button(t("Gérer les calques…")).clicked() {
                self.open_layers();
            }
        });
        if !self.selected.is_empty() && !self.selection_editable() {
            ui.label(
                RichText::new(t("La sélection est verrouillée"))
                    .small()
                    .color(MUTED),
            );
        }
        ui.add_space(6.);
    }

    pub(super) fn layers_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_layers {
            return;
        }
        let layers = self.doc.organization.layers.clone();
        let mut counts = std::collections::BTreeMap::<u64, usize>::new();
        for id in self.doc.object_ids() {
            *counts.entry(self.doc.object_layer(id)).or_default() += 1;
        }
        let modal = egui::Modal::new("organization_layers".into()).show(ctx, |ui| {
            ui.set_width(620_f32.min(ctx.content_rect().width() - 48.));
            ui.horizontal(|ui| {
                ui.heading(t("Calques"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.button(t("Fermer")).clicked() { self.show_layers = false; }
                });
            });
            ui.label(t("Les calques du haut sont dessinés au premier plan. Les calques masqués ne sont pas exportés."));
            ui.add_space(8.);
            egui::ScrollArea::vertical().id_salt("organization_layer_list")
                .max_height((ctx.content_rect().height() - 260.).clamp(160., 440.)).show(ui, |ui| {
                for (index, layer) in layers.iter().enumerate().rev() {
                    ui.push_id(layer.id, |ui| {
                        egui::Frame::new().stroke(Stroke::new(1., BORDER)).corner_radius(6).inner_margin(10).show(ui, |ui| {
                            ui.horizontal(|ui| {
                                let name = self.layer_names.entry(layer.id).or_insert_with(|| layer.name.clone());
                                let response = ui.add(egui::TextEdit::singleline(name).desired_width(220.).char_limit(120));
                                if (response.lost_focus() || (response.has_focus() && ui.input(|i| i.key_pressed(Key::Enter)))) && *name != layer.name {
                                    let name = name.clone();
                                    let result = self.doc.update_layer(layer.id, Some(name), None, None);
                                    self.organization_result(result, t("Nom du calque mis à jour"));
                                }
                                let count = counts.get(&layer.id).copied().unwrap_or(0);
                                ui.label(if count == 1 { t("1 objet") } else { i18n::render("{0} objets", &[("0", count.to_string())]) });
                                for (label, target, enabled, hint) in [
                                    ("↑", index + 1, index + 1 < layers.len(), "Monter le calque"),
                                    ("↓", index.saturating_sub(1), index > 0, "Descendre le calque"),
                                ] {
                                    if ui.add_enabled(enabled, egui::Button::new(label)).on_hover_text(t(hint)).clicked() {
                                        let result = self.doc.move_layer(layer.id, target);
                                        self.organization_result(result, t("Ordre des calques mis à jour"));
                                    }
                                }
                            });
                            ui.horizontal_wrapped(|ui| {
                                let mut visible = layer.visible;
                                if ui.checkbox(&mut visible, t("Visible")).changed() { self.set_layer_visible(layer.id, visible); }
                                let mut locked = layer.locked;
                                if ui.checkbox(&mut locked, t("Verrouillé")).changed() {
                                    let result = self.doc.update_layer(layer.id, None, None, Some(locked));
                                    self.organization_result(result, t("Verrouillage du calque mis à jour"));
                                }
                                if ui.add_enabled(layer.visible && !layer.locked, egui::Button::new(t("Dessiner ici")).selected(self.drawing_layer == layer.id)).clicked() { self.drawing_layer = layer.id; }
                                if ui.add_enabled(self.selection_editable() && layer.visible && !layer.locked, egui::Button::new(t("Placer la sélection ici"))).clicked() {
                                    let result = self.doc.move_objects_to_layer(&self.selected, layer.id);
                                    self.organization_result(result, t("Sélection déplacée sur le calque"));
                                }
                                if ui.add_enabled(!layer.locked, egui::Button::new(t("Retirer le calque"))).on_hover_text(t("Les objets sont replacés sur le calque de base.")).clicked() {
                                    let result = self.doc.remove_layer(layer.id);
                                    self.organization_result(result, t("Calque retiré · les objets sont conservés"));
                                }
                            });
                        });
                        ui.add_space(8.);
                    });
                }
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.strong(t("Calque de base"));
                    if ui.selectable_label(self.drawing_layer == 0, t("Dessiner ici")).clicked() { self.drawing_layer = 0; }
                    if ui.add_enabled(self.selection_editable(), egui::Button::new(t("Placer la sélection ici"))).clicked() {
                        let result = self.doc.move_objects_to_layer(&self.selected, 0);
                        self.organization_result(result, t("Sélection déplacée sur le calque"));
                    }
                });
                ui.label(RichText::new(t("Le calque de base reste visible et déverrouillé. Déplacez les objets sur un autre calque pour les masquer ensemble.")).small().color(MUTED));
            });
            ui.separator();
            if ui.button(t("Ajouter un calque")).clicked() {
                let name = i18n::render("Calque {0}", &[("0", (layers.len() + 1).to_string())]);
                match self.doc.create_layer(name.clone()) {
                    Ok(id) => {
                        self.layer_names.insert(id, name);
                        self.drawing_layer = id;
                        self.commit();
                        self.status = "Calque ajouté".into();
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        });
        if modal.should_close() {
            self.show_layers = false;
        }
    }

    pub(super) fn paint_group_selection(&self, painter: &egui::Painter) {
        if self.selected.is_empty()
            || self.rich_editor.is_some()
            || matches!(self.drag, Some(Drag::Marquee { .. }))
        {
            return;
        }
        let grouped = self
            .doc
            .organization
            .groups
            .iter()
            .any(|group| group.members.iter().all(|id| self.selected.contains(id)));
        let locked = !self.selection_editable();
        if !grouped && !locked {
            return;
        }
        let Some(rect) = self.selection_rect() else {
            return;
        };
        let color = if locked { MUTED } else { ACCENT };
        let rect = rect.expand(5.);
        painter.rect_stroke(rect, 3, Stroke::new(1.5, color), StrokeKind::Inside);
        let label = t(if locked {
            "Sélection verrouillée"
        } else {
            "Objets groupés"
        });
        let galley = painter.layout_no_wrap(label, FontId::proportional(12.), color);
        let at = pos2(
            rect.left().max(self.canvas.left() + 6.),
            (rect.top() - 24.).max(self.canvas.top() + 6.),
        );
        painter.rect_filled(
            Rect::from_min_size(at, galley.size() + vec2(10., 6.)),
            3,
            Color32::WHITE,
        );
        painter.galley(at + vec2(5., 3.), galley, color);
    }

    pub(super) fn prepare_organization_capture(&mut self) {
        self.prepare_alignment_capture(false);
        self.doc.title = "Organiser les idées".into();
        let ids = self
            .doc
            .nodes
            .iter()
            .take(2)
            .map(|n| n.id)
            .collect::<Vec<_>>();
        let layer = self.doc.create_layer("Étapes du projet".into()).unwrap();
        self.doc.move_objects_to_layer(&ids, layer).unwrap();
        self.doc.group_objects(&ids).unwrap();
        let notes = self.doc.create_layer("Annotations".into()).unwrap();
        let charts = self.doc.charts.iter().map(|c| c.id).collect::<Vec<_>>();
        if !charts.is_empty() {
            self.doc.move_objects_to_layer(&charts, notes).unwrap();
        }
        self.doc
            .update_layer(notes, None, None, Some(true))
            .unwrap();
        self.selected = ids;
        self.history = History::new(&self.doc);
        self.open_layers();
    }
}
