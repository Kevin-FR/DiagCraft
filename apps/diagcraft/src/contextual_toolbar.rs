use super::*;
use context_menu::{Action, shape_picker};

pub(super) fn arrow_selector(
    ui: &mut egui::Ui,
    salt: impl std::hash::Hash,
    enabled: &mut bool,
    head: &mut ArrowHead,
) {
    egui::ComboBox::from_id_salt(salt)
        .width(96.)
        .selected_text(t(if *enabled { head.label() } else { "Aucune" }))
        .show_ui(ui, |ui| {
            if ui.selectable_label(!*enabled, t("Aucune")).clicked() {
                *enabled = false;
            }
            for candidate in ArrowHead::ALL {
                if ui
                    .selectable_label(*enabled && *head == candidate, t(candidate.label()))
                    .clicked()
                {
                    *enabled = true;
                    *head = candidate;
                }
            }
        });
}
pub(super) fn edge_style_controls(ui: &mut egui::Ui, e: &mut document::Connector) {
    egui::ComboBox::from_id_salt(("edge_pattern", e.id))
        .selected_text(t(e.pattern.label()))
        .show_ui(ui, |ui| {
            for pattern in LinePattern::ALL {
                ui.selectable_value(&mut e.pattern, pattern, t(pattern.label()));
            }
        });
    ui.horizontal(|ui| {
        ui.label(t("Départ"));
        arrow_selector(
            ui,
            ("start_head", e.id),
            &mut e.arrow_start,
            &mut e.start_head,
        );
    });
    ui.horizontal(|ui| {
        ui.label(t("Arrivée"));
        arrow_selector(ui, ("end_head", e.id), &mut e.arrow, &mut e.end_head);
    });
}
fn group(ui: &mut egui::Ui, width: f32, label: &str, body: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.set_width(width);
        body(ui);
        ui.label(RichText::new(t(label)).small().color(MUTED));
    });
    ui.separator();
}
impl DrawApp {
    pub(super) fn quick_toolbar(&mut self, ctx: &egui::Context) {
        if !self.show_quick_toolbar
            || !self.selection_editable()
            || self.selected.len() != 1
            || self.drag.is_some()
            || self.tool == Tool::Connect
        {
            return;
        }
        let Some(selected) = self.selection_rect() else {
            return;
        };
        let id = self.selected[0];
        let width = (self.canvas.width() - 40.).clamp(240., 1000.);
        let height = 126.;
        let min = self.canvas.min + vec2(10., 10.);
        let max = pos2(
            (self.canvas.right() - width - 30.).max(min.x),
            (self.canvas.bottom() - height - 10.).max(min.y),
        );
        let automatic = pos2(
            selected.left().clamp(min.x, max.x),
            (selected.top() - height - 12.).clamp(min.y, max.y),
        );
        let stored = self.toolbar_position.unwrap_or(automatic);
        let pos = if self.toolbar_pinned {
            pos2(stored.x.clamp(min.x, max.x), stored.y.clamp(min.y, max.y))
        } else {
            automatic
        };
        let mut action = None;
        egui::Area::new("selection_toolbar".into())
            .default_size(vec2(width, height))
            .order(egui::Order::Foreground)
            .fixed_pos(pos)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .stroke(Stroke::new(1., BORDER))
                    .corner_radius(9)
                    .shadow(ctx.style().visuals.window_shadow)
                    .inner_margin(10)
                    .show(ui, |ui| {
                        ui.set_width(width);
                        ui.spacing_mut().item_spacing = vec2(6., 4.);
                        ui.horizontal(|ui| {
                            let (r, grip) = ui.allocate_exact_size(vec2(20., 18.), Sense::drag());
                            for y in [r.center().y - 3., r.center().y + 3.] {
                                for x in [r.center().x - 4., r.center().x, r.center().x + 4.] {
                                    ui.painter().circle_filled(pos2(x, y), 1., MUTED);
                                }
                            }
                            if grip.on_hover_text(t("Déplacer la barre")).dragged() {
                                self.toolbar_position =
                                    Some(pos + ctx.input(|i| i.pointer.delta()));
                                self.toolbar_pinned = true;
                            }
                            ui.label(
                                RichText::new(t(
                                    if self.doc.connectors.iter().any(|e| e.id == id) {
                                        "Connecteur"
                                    } else if self.doc.node(id).is_some() {
                                        "Forme"
                                    } else {
                                        "Mise en forme"
                                    },
                                ))
                                .strong()
                                .size(12.)
                                .color(MUTED),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if ui
                                    .small_button("×")
                                    .on_hover_text(t("Masquer la barre · Affichage → Panneaux"))
                                    .clicked()
                                {
                                    self.show_quick_toolbar = false;
                                }
                                if ui
                                    .selectable_label(
                                        self.toolbar_pinned,
                                        t(if self.toolbar_pinned {
                                            "Détacher"
                                        } else {
                                            "Épingler"
                                        }),
                                    )
                                    .on_hover_text(t(
                                        "Conserver la position de la barre lors des sélections",
                                    ))
                                    .clicked()
                                {
                                    self.toolbar_pinned = !self.toolbar_pinned;
                                    self.toolbar_position = self.toolbar_pinned.then_some(pos);
                                }
                            });
                        });
                        egui::ScrollArea::horizontal()
                            .id_salt("toolbar_controls")
                            .max_height(82.)
                            .show(ui, |ui| {
                                ui.horizontal_top(|ui| {
                                    ui.set_height(71.);
                                    if let Some(n) = self.doc.node_mut(id) {
                                        group(ui, 206., "Police et alignement", |ui| {
                                            ui.horizontal(|ui| {
                                                egui::ComboBox::from_id_salt(("toolbar_font", id))
                                                    .width(125.)
                                                    .selected_text(t(if n.text_style.monospace {
                                                        "Monospace"
                                                    } else {
                                                        "Source Sans 3"
                                                    }))
                                                    .show_ui(ui, |ui| {
                                                        ui.selectable_value(
                                                            &mut n.text_style.monospace,
                                                            false,
                                                            "Source Sans 3",
                                                        );
                                                        ui.selectable_value(
                                                            &mut n.text_style.monospace,
                                                            true,
                                                            "Monospace",
                                                        );
                                                    });
                                                ui.add(
                                                    egui::DragValue::new(&mut n.font_size)
                                                        .range(6.0..=200.0)
                                                        .speed(1.),
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                for (label, value) in [
                                                    ("B", &mut n.text_style.bold),
                                                    ("I", &mut n.text_style.italic),
                                                    ("U", &mut n.text_style.underline),
                                                ] {
                                                    if ui
                                                        .selectable_label(*value, t(label))
                                                        .clicked()
                                                    {
                                                        *value = !*value;
                                                    }
                                                }
                                                egui::ComboBox::from_id_salt(("toolbar_align", id))
                                                    .width(78.)
                                                    .selected_text(t(match n.text_style.align {
                                                        TextAlign::Left => "Gauche",
                                                        TextAlign::Center => "Centré",
                                                        TextAlign::Right => "Droite",
                                                    }))
                                                    .show_ui(ui, |ui| {
                                                        for (v, label) in [
                                                            (TextAlign::Left, "Gauche"),
                                                            (TextAlign::Center, "Centré"),
                                                            (TextAlign::Right, "Droite"),
                                                        ] {
                                                            ui.selectable_value(
                                                                &mut n.text_style.align,
                                                                v,
                                                                t(label),
                                                            );
                                                        }
                                                    });
                                            });
                                        });
                                        group(ui, 178., "Couleurs", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(t("Fond"));
                                                ui.color_edit_button_srgb(&mut n.fill);
                                                ui.label(t("Trait"));
                                                ui.color_edit_button_srgb(&mut n.stroke);
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(t("Texte"));
                                                ui.color_edit_button_srgb(&mut n.text_color);
                                                ui.add(
                                                    egui::DragValue::new(&mut n.stroke_width)
                                                        .range(0.0..=20.0)
                                                        .suffix(" px")
                                                        .speed(0.1),
                                                );
                                            });
                                        });
                                        group(ui, 130., "Styles", |ui| {
                                            ui.menu_button(t("Styles de forme"), |ui| {
                                                for (i, color) in PALETTE.iter().enumerate() {
                                                    if ui
                                                        .button(
                                                            RichText::new(format!(
                                                                "Style {}",
                                                                i + 1
                                                            ))
                                                            .background_color(rgb(*color)),
                                                        )
                                                        .clicked()
                                                    {
                                                        action = Some(Action::NodeStyle(i));
                                                        ui.close();
                                                    }
                                                }
                                            });
                                            if ui.button(t("Reproduire le style")).clicked() {
                                                action = Some(Action::Brush);
                                            }
                                        });
                                        group(ui, 220., "Organiser", |ui| {
                                            ui.horizontal(|ui| {
                                                if ui.button(t("Premier plan")).clicked() {
                                                    action = Some(Action::Front);
                                                }
                                                if ui.button(t("Arrière-plan")).clicked() {
                                                    action = Some(Action::Back);
                                                }
                                            });
                                            ui.horizontal(|ui| {
                                                ui.menu_button(t("Remplacer"), |ui| {
                                                    if let Some(kind) = shape_picker(ui) {
                                                        action = Some(Action::Replace(kind));
                                                        ui.close();
                                                    }
                                                });
                                                if ui.button(t("Texte")).clicked() {
                                                    action = Some(Action::Text);
                                                }
                                            });
                                        });
                                        if ui.button(t("Paramètres")).clicked() {
                                            action = Some(Action::Properties);
                                        }
                                    } else if let Some(e) =
                                        self.doc.connectors.iter_mut().find(|e| e.id == id)
                                    {
                                        group(ui, 176., "Flèches", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.label(t("Départ"));
                                                arrow_selector(
                                                    ui,
                                                    ("toolbar_start", id),
                                                    &mut e.arrow_start,
                                                    &mut e.start_head,
                                                );
                                            });
                                            ui.horizontal(|ui| {
                                                ui.label(t("Arrivée"));
                                                arrow_selector(
                                                    ui,
                                                    ("toolbar_end", id),
                                                    &mut e.arrow,
                                                    &mut e.end_head,
                                                );
                                            });
                                        });
                                        if ui
                                            .button(t("Inverser"))
                                            .on_hover_text(t("Inverser les flèches"))
                                            .clicked()
                                        {
                                            action = Some(Action::SwapArrows);
                                        }
                                        group(ui, 125., "Type de connecteur", |ui| {
                                            let old = e.kind;
                                            egui::ComboBox::from_id_salt(("toolbar_route", id))
                                                .width(115.)
                                                .selected_text(t(
                                                    if e.kind == ConnectorKind::Elbow {
                                                        "Angle droit"
                                                    } else {
                                                        "Direct"
                                                    },
                                                ))
                                                .show_ui(ui, |ui| {
                                                    ui.selectable_value(
                                                        &mut e.kind,
                                                        ConnectorKind::Elbow,
                                                        t("Angle droit"),
                                                    );
                                                    ui.selectable_value(
                                                        &mut e.kind,
                                                        ConnectorKind::Straight,
                                                        t("Direct"),
                                                    );
                                                });
                                            if old != e.kind && e.kind == ConnectorKind::Straight {
                                                e.waypoints.clear();
                                            }
                                            if ui
                                                .add_enabled(
                                                    e.kind == ConnectorKind::Elbow,
                                                    egui::Button::new(t(
                                                        if e.waypoints.is_empty() {
                                                            "Automatique"
                                                        } else {
                                                            "Manuel"
                                                        },
                                                    )),
                                                )
                                                .clicked()
                                            {
                                                action = Some(Action::Automatic(
                                                    !e.waypoints.is_empty(),
                                                ));
                                            }
                                        });
                                        group(ui, 154., "Trait", |ui| {
                                            ui.horizontal(|ui| {
                                                ui.color_edit_button_srgb(&mut e.color);
                                                ui.add(
                                                    egui::DragValue::new(&mut e.width)
                                                        .range(0.5..=20.0)
                                                        .speed(0.1)
                                                        .suffix(" px"),
                                                );
                                            });
                                            egui::ComboBox::from_id_salt(("toolbar_pattern", id))
                                                .width(130.)
                                                .selected_text(t(e.pattern.label()))
                                                .show_ui(ui, |ui| {
                                                    for pattern in LinePattern::ALL {
                                                        ui.selectable_value(
                                                            &mut e.pattern,
                                                            pattern,
                                                            t(pattern.label()),
                                                        );
                                                    }
                                                });
                                        });
                                        group(ui, 140., "Mise en forme", |ui| {
                                            if ui.button(t("Texte du lien")).clicked() {
                                                action = Some(Action::Text);
                                            }
                                            if ui.button(t("Reproduire le style")).clicked() {
                                                action = Some(Action::Brush);
                                            }
                                        });
                                        group(ui, 172., "Formes", |ui| {
                                            ui.menu_button(t("Insérer une forme"), |ui| {
                                                if let Some(kind) = shape_picker(ui) {
                                                    action = Some(Action::Insert(kind));
                                                    ui.close();
                                                }
                                            });
                                            if ui.button(t("Paramètres")).clicked() {
                                                action = Some(Action::Properties);
                                            }
                                        });
                                    } else if let Some(c) =
                                        self.doc.charts.iter_mut().find(|c| c.id == id)
                                    {
                                        egui::ComboBox::from_id_salt(("toolbar_chart", id))
                                            .selected_text(t(c.kind.label()))
                                            .show_ui(ui, |ui| {
                                                for kind in ChartKind::ALL {
                                                    ui.selectable_value(
                                                        &mut c.kind,
                                                        kind,
                                                        t(kind.label()),
                                                    );
                                                }
                                            });
                                        ui.checkbox(&mut c.legend, t("Légende"));
                                        if ui.button(t("Paramètres")).clicked() {
                                            action = Some(Action::Properties);
                                        }
                                    } else if let Some(d) =
                                        self.doc.drawings.iter_mut().find(|d| d.id == id)
                                    {
                                        ui.color_edit_button_srgb(&mut d.color);
                                        ui.add(
                                            egui::DragValue::new(&mut d.width)
                                                .range(0.5..=20.)
                                                .speed(0.1)
                                                .suffix(" px"),
                                        );
                                        ui.checkbox(&mut d.arrow, t("Flèche"));
                                        if ui.button(t("Paramètres")).clicked() {
                                            action = Some(Action::Properties);
                                        }
                                    }
                                });
                            });
                    });
            });
        if let Some(action) = action {
            if matches!(action, Action::Insert(_))
                && let Some(e) = self.doc.connectors.iter().find(|e| e.id == id)
            {
                self.context_position = self.doc.edge_label_point(e);
            }
            self.perform_action(action, ctx);
        }
    }
}
