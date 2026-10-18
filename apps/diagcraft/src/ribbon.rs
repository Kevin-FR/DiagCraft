use super::*;

#[derive(Clone, Copy)]
enum Icon {
    Save,
    Undo,
    Redo,
    Export,
    Page,
    Table,
    Timeline,
    Text,
    Shape,
    Ellipse,
    Diamond,
    Grid,
    Fit,
    Arrow,
    Curve,
}

impl DrawApp {
    pub(super) fn ribbon_is_expanded(&self, ctx: &egui::Context) -> bool {
        self.ribbon_expanded.unwrap_or_else(|| {
            ctx.content_rect().height() >= 850. && ctx.content_rect().width() >= 1350.
        })
    }
    fn compact_ribbon(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_centered(|ui| {
            if ui
                .selectable_label(self.tool == Tool::Select, "Sélection · V")
                .clicked()
            {
                self.set_tool(Tool::Select);
            }
            if ui
                .selectable_label(self.tool == Tool::Pan, "Main · H")
                .clicked()
            {
                self.set_tool(Tool::Pan);
            }
            ui.separator();
            self.shape_menu(ui);
            if ui.button("Texte · T").clicked() {
                self.set_tool(Tool::Shape(ShapeKind::Text));
            }
            if ui.button("Relier · C").clicked() {
                self.activate_connect_tool();
            }
            ui.separator();
            if ui.button("Propriétés").clicked() {
                self.show_inspector = !self.show_inspector;
            }
            ui.add_enabled_ui(!self.selected.is_empty(), |ui| {
                self.align_menu(ui);
                if ui.button("Dupliquer").on_hover_text("Ctrl/⌘ D").clicked() {
                    self.duplicate();
                }
            });
            if !self.selected.is_empty() {
                ui.label(
                    RichText::new(format!(
                        "{} sélectionné{}",
                        self.selected.len(),
                        if self.selected.len() == 1 { "" } else { "s" }
                    ))
                    .size(12.)
                    .color(MUTED),
                );
            }
        });
    }
    pub(super) fn paint_rulers(&self, p: &egui::Painter, outer: Rect) {
        let top = Rect::from_min_max(outer.min, pos2(outer.right(), self.canvas.top()));
        let left = Rect::from_min_max(outer.min, pos2(self.canvas.left(), outer.bottom()));
        p.rect_filled(top, 0, Color32::from_rgb(250, 251, 254));
        p.rect_filled(left, 0, Color32::from_rgb(250, 251, 254));
        let unit = self.doc.page_layout.unit;
        let step = [
            1., 2., 5., 10., 20., 50., 100., 200., 500., 1000., 2000., 5000.,
        ]
        .into_iter()
        .find(|v| unit.to_px(*v) * self.zoom >= 55.)
        .unwrap_or(10000.);
        for axis in 0..2 {
            let (min, max, origin) = if axis == 0 {
                (
                    self.canvas.left(),
                    self.canvas.right(),
                    self.screen([0., 0.]).x,
                )
            } else {
                (
                    self.canvas.top(),
                    self.canvas.bottom(),
                    self.screen([0., 0.]).y,
                )
            };
            let mut value = (unit.from_px((min - origin) / self.zoom) / step).floor() * step;
            let painter = p.with_clip_rect(if axis == 0 { top } else { left });
            loop {
                let at = origin + unit.to_px(value) * self.zoom;
                if at > max {
                    break;
                }
                if at >= min {
                    if axis == 0 {
                        painter.line_segment(
                            [pos2(at, top.bottom() - 5.), pos2(at, top.bottom())],
                            Stroke::new(1., MUTED),
                        );
                        painter.text(
                            pos2(at, top.top() + 8.),
                            Align2::CENTER_CENTER,
                            format!("{}", value as i32),
                            FontId::proportional(10.),
                            MUTED,
                        );
                    } else {
                        painter.line_segment(
                            [pos2(left.right() - 5., at), pos2(left.right(), at)],
                            Stroke::new(1., MUTED),
                        );
                        painter.text(
                            pos2(left.right() - 6., at),
                            Align2::RIGHT_CENTER,
                            format!("{}", value as i32),
                            FontId::proportional(8.),
                            MUTED,
                        );
                    }
                }
                value += step;
            }
        }
        p.text(
            outer.min + vec2(4., 5.),
            Align2::LEFT_TOP,
            unit.label(),
            FontId::proportional(9.),
            MUTED,
        );
    }
    pub(super) fn ribbon(&mut self, ctx: &egui::Context) {
        self.stash_tab();
        egui::TopBottomPanel::top("document_tabs")
            .exact_height(38.)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(227, 230, 236))
                    .inner_margin(egui::Margin::symmetric(12, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    if self.logo.is_none()
                        && let Ok(image) = image::load_from_memory(include_bytes!(
                            "../../../assets/brand/diagcraft-mark-64.png"
                        ))
                    {
                        let rgba = image.to_rgba8();
                        let size = [rgba.width() as usize, rgba.height() as usize];
                        self.logo = Some(ctx.load_texture(
                            "diagcraft_logo",
                            egui::ColorImage::from_rgba_unmultiplied(size, &rgba),
                            egui::TextureOptions::LINEAR,
                        ));
                    }
                    if let Some(logo) = &self.logo {
                        ui.image((logo.id(), vec2(30., 30.)));
                    }
                    ui.label(RichText::new("DiagCraft").size(13.).strong().color(ACCENT));
                    if panels_ui::panel_toggle(
                        ui,
                        self.show_library,
                        false,
                        if self.show_library {
                            "Réduire la bibliothèque"
                        } else {
                            "Afficher la bibliothèque"
                        },
                    )
                    .clicked()
                    {
                        self.show_library = !self.show_library;
                    }
                    ui.add_space(6.);
                    let mut switch = None;
                    let mut close = None;
                    egui::ScrollArea::horizontal()
                        .max_width((ui.available_width() - 100.).max(100.))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (i, tab) in self.tabs.iter().enumerate() {
                                    egui::Frame::new()
                                        .fill(if i == self.active_tab {
                                            Color32::from_rgb(245, 246, 251)
                                        } else {
                                            Color32::TRANSPARENT
                                        })
                                        .corner_radius(6)
                                        .inner_margin(egui::Margin::symmetric(10, 1))
                                        .show(ui, |ui| {
                                            ui.horizontal(|ui| {
                                                let label = tab
                                                    .doc
                                                    .title
                                                    .chars()
                                                    .take(28)
                                                    .collect::<String>();
                                                if ui
                                                    .add(
                                                        egui::Button::new(
                                                            RichText::new(label).size(13.),
                                                        )
                                                        .frame(false),
                                                    )
                                                    .clicked()
                                                {
                                                    switch = Some(i);
                                                }
                                                if tab.doc != tab.saved {
                                                    ui.label(
                                                        RichText::new("•")
                                                            .color(Color32::from_rgb(245, 83, 83)),
                                                    );
                                                }
                                                if ui
                                                    .add(egui::Button::new("×").frame(false))
                                                    .on_hover_text("Fermer ce document")
                                                    .clicked()
                                                {
                                                    close = Some(i);
                                                }
                                            });
                                        });
                                }
                            });
                        });
                    if ui
                        .button("+")
                        .on_hover_text("Nouveau document · Ctrl/⌘ N")
                        .clicked()
                    {
                        self.request(Pending::New);
                    }
                    ui.add_space((ui.available_width() - 32.).max(0.));
                    if panels_ui::panel_toggle(
                        ui,
                        self.show_inspector,
                        true,
                        if self.show_inspector {
                            "Masquer les propriétés"
                        } else {
                            "Afficher les propriétés"
                        },
                    )
                    .clicked()
                    {
                        self.show_inspector = !self.show_inspector;
                    }
                    if let Some(i) = switch {
                        self.switch_tab(i);
                    }
                    if let Some(i) = close {
                        self.request(Pending::CloseTab(i));
                    }
                });
            });
        egui::TopBottomPanel::top("ribbon_tabs")
            .exact_height(46.)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(245, 246, 251))
                    .inner_margin(egui::Margin::symmetric(16, 3)),
            )
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.menu_button(RichText::new("Fichier").size(16.), |ui| {
                        if ui.button("Nouveau document").clicked() {
                            ui.close();
                            self.request(Pending::New);
                        }
                        if ui.button("Mes travaux enregistrés…").clicked() {
                            self.open_gallery();
                            ui.close();
                        }
                        if ui.button("Galerie de modèles…").clicked() {
                            self.show_templates = true;
                            ui.close();
                        }
                        if ui.button("Enregistrer comme modèle…").clicked() {
                            self.template_name = self.doc.title.clone();
                            self.save_template_open = true;
                            ui.close();
                        }
                        if ui.button("Ouvrir…").clicked() {
                            ui.close();
                            self.request(Pending::OpenDialog);
                        }
                        if ui.button("Enregistrer").clicked() {
                            ui.close();
                            self.save(false);
                        }
                        if ui.button("Enregistrer sous…").clicked() {
                            ui.close();
                            self.save(true);
                        }
                        if ui.button("Exporter en SVG…").clicked() {
                            ui.close();
                            self.export();
                        }
                        if ui.button("Exporter en PDF…").clicked() {
                            self.show_pdf_export = true;
                            ui.close();
                        }
                        ui.separator();
                        if ui.button("Fermer le document").clicked() {
                            ui.close();
                            self.request(Pending::CloseTab(self.active_tab));
                        }
                    });
                    ui.separator();
                    if quick(
                        ui,
                        Icon::Undo,
                        "Annuler · Ctrl/⌘ Z",
                        self.history.can_undo(),
                    )
                    .clicked()
                    {
                        self.undo();
                    }
                    if quick(
                        ui,
                        Icon::Redo,
                        "Rétablir · Ctrl/⌘ Maj Z",
                        self.history.can_redo(),
                    )
                    .clicked()
                    {
                        self.redo();
                    }
                    if quick(ui, Icon::Save, "Enregistrer · Ctrl/⌘ S", true).clicked() {
                        self.save(false);
                    }
                    if quick(ui, Icon::Export, "Exporter en SVG · Ctrl/⌘ E", true).clicked() {
                        self.export();
                    }
                    ui.separator();
                    ui.add_space(10.);
                    if ui
                        .button(RichText::new("Agents / IA").color(ACCENT))
                        .clicked()
                    {
                        self.open_agent_panel();
                    }
                    if ui
                        .button("Rechercher…")
                        .on_hover_text(format!(
                            "Rechercher une commande · {}",
                            ctx.format_shortcut(&egui::KeyboardShortcut::new(
                                egui::Modifiers::COMMAND,
                                Key::K
                            ))
                        ))
                        .clicked()
                    {
                        self.open_command_palette();
                    }
                    egui::ScrollArea::horizontal()
                        .id_salt("ribbon_tabs_scroll")
                        .max_width((ui.available_width() - 44.).max(140.))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for (tab, label) in [
                                    (RibbonTab::Home, "Accueil"),
                                    (RibbonTab::Insert, "Insérer"),
                                    (RibbonTab::Design, "Conception"),
                                    (RibbonTab::View, "Affichage"),
                                    (RibbonTab::Symbols, "Symboles"),
                                    (RibbonTab::Advanced, "Avancé"),
                                ] {
                                    let selected = self.ribbon_tab == tab;
                                    let response = ui.add(
                                        egui::Button::new(
                                            RichText::new(label)
                                                .size(16.)
                                                .color(if selected { ACCENT } else { INK })
                                                .strong(),
                                        )
                                        .frame(false),
                                    );
                                    if selected {
                                        let r = response.rect;
                                        ui.painter().hline(
                                            r.x_range(),
                                            r.bottom() + 3.,
                                            Stroke::new(2.5, ACCENT),
                                        );
                                    }
                                    if response.clicked() {
                                        self.ribbon_tab = tab;
                                        self.ribbon_expanded = Some(true);
                                    }
                                }
                            });
                        });
                    ui.add_space((ui.available_width() - 32.).max(0.));
                    let expanded = self.ribbon_is_expanded(ctx);
                    if panels_ui::ribbon_toggle(ui, expanded).clicked() {
                        self.ribbon_expanded = Some(!expanded);
                    }
                });
            });
        egui::TopBottomPanel::top("ribbon")
            .exact_height(if self.ribbon_is_expanded(ctx) {
                120.
            } else {
                48.
            })
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(14, 7))
                    .stroke(Stroke::new(1., BORDER)),
            )
            .show(ctx, |ui| {
                egui::ScrollArea::horizontal()
                    .id_salt("ribbon_scroll")
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(8., 5.);
                        if !self.ribbon_is_expanded(ctx) {
                            self.compact_ribbon(ui);
                        } else {
                            ui.horizontal_top(|ui| match self.ribbon_tab {
                                RibbonTab::Home => self.home_ribbon(ui, ctx),
                                RibbonTab::Insert => self.insert_ribbon(ui),
                                RibbonTab::Design => self.design_ribbon(ui),
                                RibbonTab::View => self.view_ribbon(ui),
                                RibbonTab::Symbols => self.symbols_ribbon(ui),
                                RibbonTab::Advanced => self.advanced_ribbon(ui),
                            });
                        }
                    });
            });
    }
    pub(super) fn home_ribbon(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        group(ui, 150., "Presse-papiers", |ui| {
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(!self.selected.is_empty(), egui::Button::new("Couper"))
                    .clicked()
                {
                    self.copy(ctx, true);
                }
                if ui
                    .add_enabled(!self.selected.is_empty(), egui::Button::new("Copier"))
                    .clicked()
                {
                    self.copy(ctx, false);
                }
            });
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(self.clipboard.is_some(), egui::Button::new("Coller"))
                    .clicked()
                    && let Some(fragment) = self.clipboard.clone()
                {
                    self.paste_fragment(fragment);
                }
                if ui
                    .add_enabled(
                        self.selected.iter().any(|id| self.doc.node(*id).is_some()),
                        egui::Button::new("Pinceau"),
                    )
                    .on_hover_text("Copier le style, puis cliquer sur une autre forme")
                    .clicked()
                {
                    self.edge_brush = None;
                    self.format_brush = self
                        .selected
                        .iter()
                        .find_map(|id| self.doc.node(*id))
                        .cloned();
                    self.set_tool(Tool::Select);
                }
            });
        });
        group(ui, 315., "Police et alignement", |ui| {
            let node = self
                .selected
                .iter()
                .find_map(|id| self.doc.node(*id))
                .cloned();
            ui.add_enabled_ui(node.is_some(), |ui| {
                let mut style = node
                    .as_ref()
                    .map(|n| n.text_style.clone())
                    .unwrap_or_default();
                let original = style.clone();
                let mut size = node.as_ref().map(|n| n.font_size).unwrap_or(18.);
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("font")
                        .selected_text(if style.monospace {
                            "Monospace"
                        } else {
                            "Source Sans 3"
                        })
                        .width(146.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut style.monospace, false, "Source Sans 3");
                            ui.selectable_value(&mut style.monospace, true, "Monospace");
                        });
                    if ui
                        .add(egui::DragValue::new(&mut size).range(6.0..=200.0).speed(1.))
                        .changed()
                    {
                        for n in &mut self.doc.nodes {
                            if self.selected.contains(&n.id) {
                                n.font_size = size;
                            }
                        }
                    }
                    if ui.button("A+").clicked() {
                        self.font_step(2.);
                    }
                    if ui.button("A−").clicked() {
                        self.font_step(-2.);
                    }
                });
                ui.horizontal(|ui| {
                    ui.toggle_value(&mut style.bold, RichText::new("B").strong());
                    ui.toggle_value(&mut style.italic, RichText::new("I").italics());
                    ui.toggle_value(&mut style.underline, RichText::new("U").underline());
                    ui.toggle_value(&mut style.strikethrough, RichText::new("S").strikethrough());
                    egui::ComboBox::from_id_salt("text_align")
                        .selected_text(match style.align {
                            TextAlign::Left => "Gauche",
                            TextAlign::Center => "Centré",
                            TextAlign::Right => "Droite",
                        })
                        .width(74.)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut style.align, TextAlign::Left, "Gauche");
                            ui.selectable_value(&mut style.align, TextAlign::Center, "Centré");
                            ui.selectable_value(&mut style.align, TextAlign::Right, "Droite");
                        });
                    let mut color = node.as_ref().map(|n| n.text_color).unwrap_or([35, 51, 65]);
                    if ui.color_edit_button_srgb(&mut color).changed() {
                        for n in &mut self.doc.nodes {
                            if self.selected.contains(&n.id) {
                                n.text_color = color;
                            }
                        }
                    }
                });
                if style != original {
                    self.change_text(|s| *s = style.clone());
                }
            });
        });
        group(ui, 230., "Outils", |ui| {
            ui.horizontal(|ui| {
                ui.menu_button("Outil Pointeur", |ui| {
                    if ui.button("Sélectionner · V").clicked() {
                        self.set_tool(Tool::Select);
                        ui.close();
                    }
                    if ui.button("Déplacer la vue · H").clicked() {
                        self.set_tool(Tool::Pan);
                        ui.close();
                    }
                });
                self.shape_menu(ui);
            });
            ui.horizontal(|ui| {
                if ui.button("T  Texte").clicked() {
                    self.set_tool(Tool::Shape(ShapeKind::Text));
                }
                if ui.selectable_label(self.tool==Tool::Connect,"Relier · C").on_hover_text("Relier deux blocs : glisser ou cliquer. Deux blocs sélectionnés sont reliés immédiatement.").clicked() {
                    if self.tool==Tool::Connect {self.set_tool(Tool::Select);}else{self.activate_connect_tool();}
                }
                ui.menu_button("Tracé",|ui| {
                    ui.label("Tracé des nouveaux liens");
                    ui.selectable_value(&mut self.connector_kind,ConnectorKind::Elbow,"À angle droit");
                    ui.selectable_value(&mut self.connector_kind,ConnectorKind::Straight,"Ligne directe");
                });
            });
        });
        group(ui, 215., "Styles", |ui| {
            ui.horizontal(|ui| {
                for i in 0..5 {
                    if theme_button(ui, i).clicked() {
                        self.apply_theme(i, false);
                    }
                }
            });
            ui.horizontal(|ui| {
                let node = self.selected.iter().find_map(|id| self.doc.node(*id));
                let mut fill = node.map(|n| n.fill).unwrap_or([255, 255, 255]);
                let mut stroke = node.map(|n| n.stroke).unwrap_or([70, 86, 105]);
                ui.label("Fond");
                if ui.color_edit_button_srgb(&mut fill).changed() {
                    for n in &mut self.doc.nodes {
                        if self.selected.contains(&n.id) {
                            n.fill = fill;
                        }
                    }
                }
                ui.label("Ligne");
                if ui.color_edit_button_srgb(&mut stroke).changed() {
                    for n in &mut self.doc.nodes {
                        if self.selected.contains(&n.id) {
                            n.stroke = stroke;
                        }
                    }
                }
            });
        });
        group(ui, 230., "Organiser", |ui| {
            ui.horizontal(|ui| {
                self.align_menu(ui);
                if ui.button("Dupliquer").clicked() {
                    self.duplicate();
                }
            });
            ui.horizontal(|ui| {
                if ui.button("Premier plan").clicked() {
                    self.reorder(true);
                }
                if ui.button("Arrière-plan").clicked() {
                    self.reorder(false);
                }
            });
        });
    }
    pub(super) fn insert_ribbon(&mut self, ui: &mut egui::Ui) {
        group(ui, 165., "Documents", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, Icon::Page, "Nouveau").clicked() {
                    self.request(Pending::New);
                }
                if tile(ui, Icon::Shape, "Modèle").clicked() {
                    self.show_templates = true;
                }
            });
        });
        group(ui, 280., "Illustrations", |ui| {
            ui.horizontal(|ui| {
                if quick(ui, Icon::Table, "Insérer un tableau", true).clicked() {
                    self.insert_table();
                }
                ui.menu_button("Tableau", |ui| {
                    ui.add(
                        egui::DragValue::new(&mut self.table_rows)
                            .range(1..=8)
                            .prefix("Lignes : "),
                    );
                    ui.add(
                        egui::DragValue::new(&mut self.table_columns)
                            .range(1..=6)
                            .prefix("Colonnes : "),
                    );
                    if ui.button("Insérer un tableau éditable").clicked() {
                        self.insert_table();
                        ui.close();
                    }
                });
                if tile(ui, Icon::Timeline, "Chronologie").clicked() {
                    self.insert_timeline();
                }
                if tile(ui, Icon::Text, "Texte").clicked() {
                    self.set_tool(Tool::Shape(ShapeKind::Text));
                }
            });
        });
        group(ui, 300., "Parties du diagramme", |ui| {
            ui.horizontal(|ui| {
                for (kind, label, icon) in [
                    (ShapeKind::Rectangle, "Rectangle", Icon::Shape),
                    (ShapeKind::Ellipse, "Ellipse", Icon::Ellipse),
                    (ShapeKind::Diamond, "Décision", Icon::Diamond),
                ] {
                    if tile(ui, icon, label).clicked() {
                        self.set_tool(Tool::Shape(kind));
                    }
                }
            });
        });
        group(ui, 100., "Données", |ui| {
            if tile(ui, Icon::Timeline, "Graphique").clicked() {
                self.library_panel = LibraryPanel::Charts;
                self.show_library = true;
            }
        });
        group(ui, 180., "Dessin libre", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, Icon::Arrow, "Trait").clicked() {
                    self.set_tool(Tool::Line);
                }
                if tile(ui, Icon::Curve, "Bézier").clicked() {
                    self.set_tool(Tool::Bezier);
                }
            });
        });
    }
    pub(super) fn design_ribbon(&mut self, ui: &mut egui::Ui) {
        group(ui, 270., "Embellir", |ui| {
            ui.horizontal(|ui| {
                for i in 0..5 {
                    if theme_button(ui, i).clicked() {
                        self.apply_theme(i, true);
                    }
                }
            });
            ui.label(
                RichText::new("Appliquer un thème au diagramme")
                    .size(11.)
                    .color(MUTED),
            );
        });
        group(ui, 235., "Couleur de fond", |ui| {
            ui.horizontal(|ui| {
                ui.label("Arrière-plan uni");
                ui.color_edit_button_srgb(&mut self.doc.page_style.background);
            });
            ui.menu_button("Filigrane", |ui| {
                ui.label("Texte du filigrane (vide pour retirer)");
                ui.add(
                    egui::TextEdit::singleline(&mut self.doc.page_style.watermark).char_limit(1000),
                );
            });
        });
        group(ui, 210., "Grille et marges", |ui| {
            ui.horizontal(|ui| {
                if quick(ui, Icon::Grid, "Afficher ou masquer la grille", true).clicked() {
                    self.grid = !self.grid;
                }
                ui.checkbox(&mut self.grid, "Grilles");
                ui.checkbox(&mut self.margins, "Marges");
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.doc.page_style.grid_spacing)
                        .prefix("Pas ")
                        .suffix(" px")
                        .range(5.0..=200.0),
                );
                ui.add(
                    egui::DragValue::new(&mut self.doc.page_style.margin)
                        .prefix("Marge ")
                        .range(0.0..=1000.0),
                );
            });
        });
        group(ui, 285., "Mise en page", |ui| {
            ui.horizontal(|ui| {
                if ui.button("Ajuster au dessin").clicked() {
                    self.fit_drawing();
                }
                if ui.button("Orientation").clicked() {
                    self.doc.page_size.swap(0, 1);
                    self.fit_next = true;
                }
            });
            self.page_menu(ui);
        });
    }
    pub(super) fn view_ribbon(&mut self, ui: &mut egui::Ui) {
        group(ui, 300., "Affichage", |ui| {
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.rulers, "Règles");
                ui.checkbox(&mut self.grid, "Grilles");
                ui.checkbox(&mut self.margins, "Marges");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.page_breaks, "Repères de page");
                ui.checkbox(&mut self.snap, "Magnétisme")
                    .on_hover_text("⌥ pendant un déplacement pour placer librement.");
                ui.menu_button("Grille…", |ui| {
                    ui.add_enabled(
                        self.snap,
                        egui::Checkbox::new(&mut self.smart_guides, "Repères d’alignement"),
                    );
                    ui.selectable_value(
                        &mut self.doc.page_style.grid_style,
                        GridStyle::Dots,
                        "Points",
                    );
                    ui.selectable_value(
                        &mut self.doc.page_style.grid_style,
                        GridStyle::Lines,
                        "Lignes",
                    );
                    ui.checkbox(
                        &mut self.doc.page_style.export_grid,
                        "Inclure la grille dans le SVG",
                    );
                });
            });
        });
        group(ui, 255., "Panneaux", |ui| {
            ui.menu_button("Panneaux et barre contextuelle", |ui| {
                let mut expanded = self.ribbon_is_expanded(ui.ctx());
                if ui.checkbox(&mut expanded, "Ruban développé").changed() {
                    self.ribbon_expanded = Some(expanded);
                }
                if ui.button("Adapter le ruban à la fenêtre").clicked() {
                    self.ribbon_expanded = None;
                    ui.close();
                }
                ui.checkbox(&mut self.show_library, "Bibliothèque de symboles");
                ui.checkbox(&mut self.show_inspector, "Propriétés de la sélection");
                ui.checkbox(
                    &mut self.show_quick_toolbar,
                    "Barre de mise en forme flottante",
                );
            });
        });
        group(ui, 320., "Zoom", |ui| {
            ui.horizontal(|ui| {
                if tile(ui, Icon::Fit, "Page entière").clicked() {
                    self.fit_next = true;
                }
                if tile(ui, Icon::Page, "Largeur").clicked() {
                    self.fit_width();
                }
                if ui
                    .add_enabled(!self.selected.is_empty(), egui::Button::new("Sélection"))
                    .on_hover_text("Maj + F")
                    .clicked()
                {
                    self.fit_selection(ui.ctx());
                }
                ui.menu_button(format!("{} %", (self.zoom * 100.).round()), |ui| {
                    for percent in [25, 50, 75, 100, 150, 200] {
                        if ui.button(format!("{percent} %")).clicked() {
                            self.set_zoom(percent as f32 / 100., self.canvas.center());
                            ui.close();
                        }
                    }
                });
            });
        });
        group(ui, 230., "Paramètres de page", |ui| {
            self.page_menu(ui);
            if ui.button("Ajuster au dessin").clicked() {
                self.fit_drawing();
            }
        });
    }
    pub(super) fn symbols_ribbon(&mut self, ui: &mut egui::Ui) {
        group(ui, 260., "Bibliothèque", |ui| {
            ui.label("Rechercher dans les formes et le document");
            ui.add(
                egui::TextEdit::singleline(&mut self.symbol_search)
                    .hint_text("Rectangle, décision…")
                    .desired_width(235.),
            );
        });
        group(ui, 470., "Formes essentielles", |ui| {
            ui.horizontal(|ui| {
                for kind in ShapeKind::ALL {
                    if tile(
                        ui,
                        match kind {
                            ShapeKind::Text => Icon::Text,
                            ShapeKind::Ellipse => Icon::Ellipse,
                            ShapeKind::Diamond => Icon::Diamond,
                            _ => Icon::Shape,
                        },
                        kind.label(),
                    )
                    .clicked()
                    {
                        self.set_tool(Tool::Shape(kind));
                    }
                }
            });
        });
        group(ui, 160., "Outils", |ui| {
            if ui.button("Afficher la bibliothèque").clicked() {
                self.show_library = true;
            }
            ui.horizontal(|ui| {
                if ui
                    .button("Gérer…")
                    .on_hover_text("Gérer les bibliothèques")
                    .clicked()
                {
                    self.open_symbol_manager();
                }
                if ui
                    .button("Effacer")
                    .on_hover_text("Effacer la recherche")
                    .clicked()
                {
                    self.symbol_search.clear();
                }
            });
        });
    }
    pub(super) fn advanced_ribbon(&mut self, ui: &mut egui::Ui) {
        group(ui, 230., "Organisation", |ui| {
            self.align_menu(ui);
            ui.menu_button("Répartir la sélection", |ui| {
                if ui.button("Horizontalement").clicked() {
                    self.distribute(false);
                    ui.close();
                }
                if ui.button("Verticalement").clicked() {
                    self.distribute(true);
                    ui.close();
                }
            });
        });
        group(ui, 210., "Sélection", |ui| {
            if ui.button("Sélectionner tous les objets").clicked() {
                self.selected = self
                    .doc
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .chain(self.doc.drawings.iter().map(|d| d.id))
                    .chain(self.doc.charts.iter().map(|c| c.id))
                    .chain(self.doc.connectors.iter().map(|c| c.id))
                    .collect();
            }
            if ui.button("Supprimer la sélection").clicked() {
                self.delete();
            }
        });
        group(ui, 275., "Document", |ui| {
            if ui.button("Travailler avec un agent…").clicked() {
                self.open_agent_panel();
            }
            if ui.button("Vérifier le document").clicked() {
                match self.doc.validate() {
                    Ok(()) => {
                        self.status = "Document vérifié : géométrie et références valides.".into()
                    }
                    Err(e) => self.error = Some(e),
                }
            }
            ui.label(
                RichText::new(format!(
                    "Format .draw v{} · {} objets",
                    self.doc.format_version,
                    self.doc.object_count()
                ))
                .size(12.)
                .color(MUTED),
            );
        });
        group(ui, 120., "Aide", |ui| {
            if tile(ui, Icon::Page, "Raccourcis").clicked() {
                self.run_command(commands_ui::Command::Help, ui.ctx());
            }
        });
    }
    pub(super) fn shape_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Forme", |ui| {
            for kind in ShapeKind::ALL {
                if ui.button(kind.label()).clicked() {
                    self.set_tool(Tool::Shape(kind));
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Outil Trait · L").clicked() {
                self.set_tool(Tool::Line);
                ui.close();
            }
            if ui.button("Outil Bézier · B").clicked() {
                self.set_tool(Tool::Bezier);
                ui.close();
            }
        });
    }
    pub(super) fn page_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Taille de page", |ui| {
            ui.label(format!("Page active : {}", self.doc.page_name));
            for p in diagcraft::pages::presets() {
                if ui.button(p.label).clicked() {
                    self.doc.page_size = p.size;
                    self.fit_next = true;
                    ui.close();
                }
            }
            ui.separator();
            if ui.button("Nom, dimensions et unités…").clicked() {
                self.open_page_settings();
                ui.close();
            }
            if ui.button("Gérer les pages…").clicked() {
                self.show_pages = true;
                ui.close();
            }
        });
    }
    pub(super) fn change_text(&mut self, change: impl Fn(&mut TextStyle)) {
        for n in &mut self.doc.nodes {
            if self.selected.contains(&n.id) {
                change(&mut n.text_style);
            }
        }
        self.commit();
    }
    pub(super) fn font_step(&mut self, step: f32) {
        for n in &mut self.doc.nodes {
            if self.selected.contains(&n.id) {
                n.font_size = (n.font_size + step).clamp(6., 200.);
            }
        }
    }
    pub(super) fn copy(&mut self, ctx: &egui::Context, cut: bool) {
        let fragment = self.doc.selection(&self.selected);
        if fragment.object_count() == 0 {
            return;
        }
        if let Ok(text) = serde_json::to_string(&fragment) {
            ctx.copy_text(text);
            self.clipboard = Some(fragment);
            if cut {
                self.delete();
            }
            self.status = if cut {
                "Sélection coupée"
            } else {
                "Sélection copiée"
            }
            .into();
        }
    }
    pub(super) fn paste_fragment(&mut self, fragment: Document) {
        match self.doc.paste(&fragment, [24., 24.]) {
            Ok(ids) => {
                self.selected = ids;
                self.set_tool(Tool::Select);
                self.commit();
                self.status = "Sélection collée avec ses connexions".into();
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn apply_theme(&mut self, index: usize, all: bool) {
        let colors = theme(index);
        for (i, n) in self.doc.nodes.iter_mut().enumerate() {
            if all || self.selected.contains(&n.id) {
                n.fill = colors[i % colors.len()];
                n.stroke = colors[i % colors.len()].map(|v| v.saturating_sub(65));
                n.text_color = [35, 51, 65];
            }
        }
        for c in &mut self.doc.connectors {
            if all {
                c.color = [102, 118, 138];
            }
        }
        self.status = "Style appliqué au diagramme".into();
    }
    pub(super) fn reorder(&mut self, front: bool) {
        let mut selected = vec![];
        self.doc.nodes.retain(|n| {
            if self.selected.contains(&n.id) {
                selected.push(n.clone());
                false
            } else {
                true
            }
        });
        if front {
            self.doc.nodes.extend(selected);
        } else {
            selected.append(&mut self.doc.nodes);
            self.doc.nodes = selected;
        }
    }
    pub(super) fn align_menu(&mut self, ui: &mut egui::Ui) {
        ui.menu_button("Aligner", |ui| {
            self.alignment_controls(ui);
            ui.separator();
            ui.checkbox(&mut self.snap, "Magnétisme").on_hover_text(
                "Grille et alignement pendant le déplacement. ⌥ pour placer librement.",
            );
            ui.add_enabled(
                self.snap,
                egui::Checkbox::new(&mut self.smart_guides, "Repères d’alignement"),
            );
        });
    }
    pub(super) fn fit_width(&mut self) {
        self.zoom = ((self.canvas.width() - 80.) / self.doc.page_size[0]).clamp(0.2, 2.5);
        self.pan = vec2(
            (self.canvas.width() - self.doc.page_size[0] * self.zoom) / 2.,
            40.,
        );
    }
    fn fit_drawing(&mut self) {
        match self.doc.fit_page_to_drawing() {
            Ok(()) => self.fit_next = true,
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn insert_table(&mut self) {
        let rows = self.table_rows.clamp(1, 8);
        let cols = self.table_columns.clamp(1, 6);
        if self.doc.object_count() + (rows * cols) as usize > 10_000 {
            self.error = Some("Limite de 10 000 objets atteinte.".into());
            return;
        }
        let w = ((self.doc.page_size[0] - 80.) / cols as f32).clamp(24., 160.);
        let h = 54.;
        let origin = [
            (self.doc.page_size[0] - w * cols as f32) / 2.,
            (self.doc.page_size[1] - h * rows as f32).max(0.) / 2.,
        ];
        self.selected.clear();
        for row in 0..rows {
            for col in 0..cols {
                let id = self.doc.add_node(ShapeKind::Rectangle, [0., 0.]);
                let n = self.doc.node_mut(id).unwrap();
                n.position = [origin[0] + col as f32 * w, origin[1] + row as f32 * h];
                n.size = [w, h];
                n.fill = if row == 0 {
                    [231, 240, 254]
                } else {
                    [255, 255, 255]
                };
                n.stroke = [161, 179, 202];
                n.text = format!(
                    "{} {}",
                    if row == 0 { "Colonne" } else { "Cellule" },
                    col + 1
                );
                n.font_size = 16.;
                n.text_style.bold = row == 0;
                self.selected.push(id);
            }
        }
        self.set_tool(Tool::Select);
        self.commit();
        self.status = "Tableau inséré : chaque cellule est éditable.".into();
    }
    pub(super) fn insert_timeline(&mut self) {
        if self.doc.object_count() + 7 > 10_000 {
            self.error = Some("Limite de 10 000 objets atteinte.".into());
            return;
        }
        self.selected.clear();
        let step = (self.doc.page_size[0] - 240.).max(100.) / 3.;
        let mut previous = None;
        for i in 0..4 {
            let id = self.doc.add_node(
                ShapeKind::Rounded,
                [120. + i as f32 * step, self.doc.page_size[1] / 2.],
            );
            let n = self.doc.node_mut(id).unwrap();
            n.size = [160., 84.];
            n.position[0] += 30.;
            n.text = format!("Étape {}", i + 1);
            n.fill = theme(0)[i % 3];
            if let Some(from) = previous {
                self.doc.connect(from, id, ConnectorKind::Straight);
            }
            previous = Some(id);
            self.selected.push(id);
        }
        self.set_tool(Tool::Select);
        self.commit();
        self.status = "Chronologie insérée : personnalisez les quatre étapes.".into();
    }
}

fn group(ui: &mut egui::Ui, width: f32, label: &str, content: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(vec2(width, 100.), Layout::top_down(Align::Center), |ui| {
        ui.allocate_ui_with_layout(vec2(width, 76.), Layout::top_down(Align::LEFT), content);
        ui.label(RichText::new(label).size(11.).color(MUTED));
    });
    ui.separator();
}
fn quick(ui: &mut egui::Ui, icon: Icon, label: &str, enabled: bool) -> egui::Response {
    ui.add_enabled_ui(enabled, |ui| {
        let (r, response) = ui.allocate_exact_size(vec2(29., 32.), Sense::click());
        if response.hovered() {
            ui.painter()
                .rect_filled(r, 4, Color32::from_rgb(228, 234, 246));
        }
        paint_icon(
            ui.painter(),
            r.shrink(6.),
            icon,
            if enabled { INK } else { MUTED },
        );
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
        response.on_hover_text(label)
    })
    .inner
}
fn tile(ui: &mut egui::Ui, icon: Icon, label: &str) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(82., 72.), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(r, 5, Color32::from_rgb(238, 242, 250));
    }
    paint_icon(
        ui.painter(),
        Rect::from_center_size(r.center_top() + vec2(0., 23.), vec2(26., 26.)),
        icon,
        INK,
    );
    ui.painter().text(
        r.center_bottom() - vec2(0., 13.),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(13.),
        INK,
    );
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    response
}
fn theme(index: usize) -> [[u8; 3]; 3] {
    match index {
        0 => [[231, 241, 255], [234, 244, 240], [255, 246, 221]],
        1 => [[239, 236, 254], [227, 240, 255], [252, 237, 245]],
        2 => [[227, 246, 249], [235, 246, 230], [255, 244, 220]],
        3 => [[253, 235, 232], [255, 242, 215], [242, 238, 254]],
        _ => [[237, 239, 244], [224, 235, 246], [233, 243, 249]],
    }
}
fn theme_button(ui: &mut egui::Ui, index: usize) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(34., 36.), Sense::click());
    ui.painter().rect(
        r,
        4,
        Color32::WHITE,
        Stroke::new(1., if response.hovered() { ACCENT } else { BORDER }),
        StrokeKind::Inside,
    );
    let colors = theme(index);
    for (i, c) in colors.into_iter().enumerate() {
        ui.painter().rect_filled(
            Rect::from_min_size(r.min + vec2(6., 5. + i as f32 * 9.), vec2(22., 6.)),
            2,
            rgb(c),
        );
    }
    response.on_hover_text(format!("Thème {}", index + 1))
}
fn paint_icon(p: &egui::Painter, r: Rect, icon: Icon, color: Color32) {
    let s = Stroke::new(1.8, color);
    let at = |x: f32, y: f32| pos2(r.left() + x * r.width(), r.top() + y * r.height());
    match icon {
        Icon::Save => {
            p.rect_stroke(r.shrink(1.), 2, s, StrokeKind::Middle);
            p.rect_stroke(
                Rect::from_two_pos(at(0.25, 0.), at(0.75, 0.4)),
                0,
                s,
                StrokeKind::Middle,
            );
            p.rect_stroke(
                Rect::from_two_pos(at(0.2, 0.65), at(0.8, 1.)),
                0,
                s,
                StrokeKind::Middle,
            );
        }
        Icon::Undo | Icon::Redo => {
            let flip = matches!(icon, Icon::Redo);
            let a = |x, y| at(if flip { 1. - x } else { x }, y);
            p.add(Shape::line(
                vec![
                    a(0.15, 0.35),
                    a(0.65, 0.35),
                    a(0.9, 0.55),
                    a(0.85, 0.85),
                    a(0.6, 0.95),
                ],
                s,
            ));
            p.add(Shape::line(
                vec![a(0.4, 0.1), a(0.15, 0.35), a(0.4, 0.6)],
                s,
            ));
        }
        Icon::Export => {
            p.rect_stroke(
                Rect::from_two_pos(at(0.05, 0.35), at(0.75, 0.95)),
                1,
                s,
                StrokeKind::Middle,
            );
            p.line_segment([at(0.4, 0.6), at(0.95, 0.05)], s);
            p.add(Shape::line(
                vec![at(0.6, 0.05), at(0.95, 0.05), at(0.95, 0.4)],
                s,
            ));
        }
        Icon::Page => {
            p.add(Shape::closed_line(
                vec![
                    at(0.2, 0.05),
                    at(0.65, 0.05),
                    at(0.85, 0.25),
                    at(0.85, 0.95),
                    at(0.2, 0.95),
                ],
                s,
            ));
        }
        Icon::Table | Icon::Grid => {
            p.rect_stroke(r.shrink(2.), 1, s, StrokeKind::Middle);
            for a in [0.33, 0.66] {
                p.line_segment([at(a, 0.1), at(a, 0.9)], s);
                p.line_segment([at(0.1, a), at(0.9, a)], s);
            }
        }
        Icon::Timeline => {
            p.line_segment([at(0.1, 0.5), at(0.9, 0.5)], s);
            for x in [0.15, 0.5, 0.85] {
                p.circle_filled(at(x, 0.5), 3., color);
                p.line_segment([at(x, 0.2), at(x, 0.8)], s);
            }
        }
        Icon::Text => {
            p.line_segment([at(0.1, 0.1), at(0.9, 0.1)], s);
            p.line_segment([at(0.5, 0.1), at(0.5, 0.95)], s);
            p.line_segment([at(0.3, 0.95), at(0.7, 0.95)], s);
        }
        Icon::Shape => {
            p.rect_stroke(r.shrink(2.), 2, s, StrokeKind::Middle);
        }
        Icon::Ellipse => {
            p.add(egui::epaint::EllipseShape {
                center: r.center(),
                radius: vec2(r.width() / 2. - 2., r.height() / 2. - 5.),
                fill: Color32::TRANSPARENT,
                stroke: s,
            });
        }
        Icon::Diamond => {
            p.add(Shape::closed_line(
                vec![
                    r.center_top(),
                    r.right_center(),
                    r.center_bottom(),
                    r.left_center(),
                ],
                s,
            ));
        }
        Icon::Fit => {
            for (x, y, sx, sy) in [
                (0., 0., 1., 1.),
                (1., 0., -1., 1.),
                (0., 1., 1., -1.),
                (1., 1., -1., -1.),
            ] {
                p.add(Shape::line(
                    vec![at(x + sx * 0.3, y), at(x, y), at(x, y + sy * 0.3)],
                    s,
                ));
            }
        }
        Icon::Arrow => {
            p.line_segment([at(0.1, 0.85), at(0.9, 0.15)], s);
        }
        Icon::Curve => {
            p.add(Shape::CubicBezier(
                egui::epaint::CubicBezierShape::from_points_stroke(
                    [
                        at(0.05, 0.85),
                        at(0.05, -0.2),
                        at(0.95, 1.2),
                        at(0.95, 0.15),
                    ],
                    false,
                    Color32::TRANSPARENT,
                    s,
                ),
            ));
        }
    }
}
