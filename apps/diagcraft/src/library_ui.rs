use super::*;

impl DrawApp {
    pub(super) fn library_sidebar(&mut self, ctx: &egui::Context) {
        let compact = !self.show_library;
        egui::SidePanel::left("library_rail")
            .exact_width(panels_ui::RAIL_WIDTH)
            .resizable(false)
            .frame(panels_ui::frame(false, false))
            .show(ctx, |ui| {
                let label = if compact {
                    "Afficher la bibliothèque"
                } else {
                    "Réduire la bibliothèque"
                };
                let r = panels_ui::panel_toggle(ui, self.show_library, false, label);
                self.focus_ui.anchor(ui, focus_ui::Region::Library, &r);
                if r.clicked() {
                    self.show_library = !self.show_library;
                }
                ui.add_space(10.);
                egui::ScrollArea::vertical()
                    .id_salt("library_rail_scroll")
                    .show(ui, |ui| {
                        for (panel, label) in [
                            (LibraryPanel::Models, "Modèles"),
                            (LibraryPanel::Symbols, "Symboles"),
                            (LibraryPanel::Text, "Texte"),
                            (LibraryPanel::Charts, "Graphiques"),
                            (LibraryPanel::Projects, "Travaux"),
                        ] {
                            let active = self.show_library && self.library_panel == panel;
                            let response =
                                ui.add_sized([36., 40.], egui::Button::new("").selected(active));
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    t(label),
                                )
                            });
                            let r = Rect::from_center_size(response.rect.center(), vec2(23., 23.));
                            let p = ui.painter();
                            let color = if active { ACCENT } else { INK };
                            let stroke = Stroke::new(1.7, color);
                            match panel {
                                LibraryPanel::Models => {
                                    p.rect_stroke(r, 1, stroke, StrokeKind::Inside);
                                    p.vline(r.left() + 6., r.y_range(), stroke);
                                    p.hline(r.x_range(), r.center().y, stroke);
                                }
                                LibraryPanel::Symbols => {
                                    p.rect_stroke(
                                        r.translate(vec2(3., -3.)),
                                        3,
                                        stroke,
                                        StrokeKind::Inside,
                                    );
                                    p.rect(
                                        r.translate(vec2(-3., 3.)),
                                        3,
                                        Color32::WHITE,
                                        stroke,
                                        StrokeKind::Inside,
                                    );
                                }
                                LibraryPanel::Text => {
                                    p.text(
                                        r.center(),
                                        Align2::CENTER_CENTER,
                                        t("T"),
                                        FontId::proportional(25.),
                                        color,
                                    );
                                }
                                LibraryPanel::Projects => {
                                    p.rect_stroke(r, 2, stroke, StrokeKind::Inside);
                                    p.hline(r.x_range(), r.top() + 6., stroke);
                                    p.vline(r.center().x, (r.top() + 6.)..=r.bottom(), stroke);
                                }
                                LibraryPanel::Charts => {
                                    p.line_segment([r.left_top(), r.left_bottom()], stroke);
                                    p.line_segment([r.left_bottom(), r.right_bottom()], stroke);
                                    p.add(Shape::line(
                                        vec![
                                            r.left_center(),
                                            r.center_top(),
                                            r.center(),
                                            r.right_top(),
                                        ],
                                        stroke,
                                    ));
                                }
                            }
                            if response.on_hover_text(t(label)).clicked() {
                                self.library_panel = panel;
                                if panel == LibraryPanel::Projects {
                                    self.open_gallery();
                                } else {
                                    self.show_library = true;
                                }
                            }
                            ui.add_space(6.);
                        }
                    });
            });
        if !self.show_library {
            return;
        }
        let max_width = (ctx.available_rect().width()
            - panels_ui::MIN_CANVAS
            - if self.show_inspector {
                260.
            } else {
                panels_ui::RAIL_WIDTH
            })
        .clamp(240., 400.);
        panels_ui::prepare_width(ctx, "symbol_library", self.library_width, 240., max_width);
        let panel = egui::SidePanel::left("symbol_library")
            .default_width(self.library_width)
            .width_range(240. ..=max_width)
            .resizable(true)
            .frame(panels_ui::frame(false, true))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 6.;
                let _ = panels_ui::heading(ui, match self.library_panel {
                    LibraryPanel::Symbols => "Symboles",
                    LibraryPanel::Models => "Modèles",
                    LibraryPanel::Text => "Texte",
                    LibraryPanel::Charts => "Graphiques",
                    LibraryPanel::Projects => "Mes travaux",
                }, None);
                ui.add(egui::TextEdit::singleline(&mut self.symbol_search)
                    .hint_text(t("Rechercher…")).desired_width(f32::INFINITY).char_limit(120));
                ui.add_space(8.);
                let library_footer = self.library_panel == LibraryPanel::Symbols;
                let body_height = (ui.available_height() - if library_footer { 46. } else { 0. }).max(0.);
                egui::ScrollArea::vertical()
                    .id_salt("library_scroll")
                    .max_height(body_height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.library_panel {
                        LibraryPanel::Projects => {
                            if ui.button(t("Ouvrir mes travaux enregistrés")).clicked(){self.open_gallery();}
                            ui.add_space(12.);
                            ui.label(t("Retrouvez vos dessins et leurs dernières versions enregistrées."));
                            ui.add_space(12.);
                            ui.label(RichText::new(t("Enregistrez un dessin avec Ctrl/⌘ S pour l’ajouter à la galerie.")).color(MUTED));
                        }
                        LibraryPanel::Symbols => self.symbol_library(ui),
                        LibraryPanel::Charts => self.chart_library(ui),
                        LibraryPanel::Models => {
                            if ui.button(t("Ouvrir la galerie de modèles")).clicked() {
                                self.show_templates = true;
                            }
                            ui.add_space(10.);
                            let items: Vec<_> = self
                                .builtin_templates
                                .iter()
                                .filter(|t| matches_search(&t.title, &self.symbol_search))
                                .cloned()
                                .collect();
                            for t in items {
                                if template_card(ui, &t, ui.available_width()).clicked() {
                                    self.open_template(&t);
                                }
                            }
                        }
                        LibraryPanel::Text => {
                            for (name, size, bold) in [
                                ("Titre", 36., true),
                                ("Sous-titre", 24., true),
                                ("Corps de texte", 18., false),
                            ] {
                                if ui
                                    .add_sized(
                                        [ui.available_width(), 52.],
                                        egui::Button::new(
                                            RichText::new(t(name)).size(size * 0.7).strong(),
                                        ),
                                    )
                                    .clicked()
                                {
                                    let mut d = Document::default();
                                    let id = d.add_node(ShapeKind::Text, [0., 0.]);
                                    let mut n = d.node(id).unwrap().clone();
                                    n.text = t(name);
                                    n.font_size = size;
                                    n.text_style.bold = bold;
                                    n.text_style.wrap = true;
                                    n.size = [300., 100.];
                                    self.choose_symbol(n);
                                }
                            }
                            ui.label(
                                RichText::new(t("Choisissez un style puis cliquez sur la page."))
                                    .color(MUTED),
                            );
                        }
                    });
                if library_footer {
                    ui.separator();
                    if ui.add_sized([ui.available_width(), 30.], egui::Button::new(t("+ Bibliothèques de symboles…"))).clicked() {
                        self.open_symbol_manager();
                    }
                }
            });
        panels_ui::remember_width(
            ctx,
            "symbol_library",
            panel.response.rect,
            &mut self.library_width,
        );
    }
    fn symbol_library(&mut self, ui: &mut egui::Ui) {
        let searching = !self.symbol_search.trim().is_empty();
        if self.library.visible("personal")
            || (searching
                && self
                    .library
                    .symbols
                    .iter()
                    .any(|n| matches_search(&n.text, &self.symbol_search)))
        {
            egui::CollapsingHeader::new(t("Ma bibliothèque"))
                .id_salt(("personal", searching))
                .open(searching.then_some(true))
                .default_open(true)
                .show(ui, |ui| {
                    let selected = self
                        .selected
                        .first()
                        .and_then(|id| self.doc.node(*id))
                        .cloned();
                    if ui
                        .add_enabled(
                            selected.is_some(),
                            egui::Button::new(t("+ Ajouter la forme sélectionnée")),
                        )
                        .clicked()
                        && let Some(node) = selected
                    {
                        let mut next = self.library.clone();
                        match next
                            .add_symbol(&node)
                            .and_then(|_| self.persist_library(&next))
                        {
                            Ok(()) => {
                                self.library = next;
                                self.status = "Symbole ajouté à votre bibliothèque".into();
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                    let items = self.library.symbols.clone();
                    ui.horizontal_wrapped(|ui| {
                        for n in items {
                            if matches_search(&n.text, &self.symbol_search) {
                                let label = if n.text.trim().is_empty() {
                                    t(n.kind.label())
                                } else {
                                    format!("{} · {}", n.text.trim(), t(n.kind.label()))
                                };
                                let response = named_symbol_tile(ui, n.kind, false, &label)
                                    .on_hover_text(&n.text);
                                if response.clicked() {
                                    self.choose_symbol(n);
                                }
                            }
                        }
                    });
                });
        }
        if self.library.visible_libraries.is_empty() && self.symbol_search.trim().is_empty() {
            ui.label(t(
                "Aucune bibliothèque affichée. Choisissez-en dans le gestionnaire.",
            ));
        }
        if searching {
            ui.label(
                RichText::new(t("Recherche dans toutes les bibliothèques"))
                    .size(12.)
                    .color(INK),
            );
        }
        let mut matches = 0;
        for library in symbols::LIBRARIES {
            if !searching && !self.library.visible(library.id) {
                continue;
            }
            if searching
                && !library.kinds.iter().any(|kind| {
                    matches_search(
                        &format!("{} {}", t(kind.label()), kind.label()),
                        &self.symbol_search,
                    )
                })
            {
                continue;
            }
            matches += 1;
            let title = library.title;
            let kinds = library.kinds;
            egui::CollapsingHeader::new(t(title))
                .id_salt((title, searching))
                .open(searching.then_some(true))
                .default_open(true)
                .show(ui, |ui| {
                    egui::Grid::new(title)
                        .num_columns(4)
                        .spacing(vec2(5., 6.))
                        .show(ui, |ui| {
                            let mut index = 0;
                            for &kind in kinds {
                                if !matches_search(
                                    &format!("{} {}", t(kind.label()), kind.label()),
                                    &self.symbol_search,
                                ) {
                                    continue;
                                }
                                if symbol_tile(ui, kind, self.tool == Tool::Shape(kind)).clicked() {
                                    let mut preset = symbols::preset(kind);
                                    preset.text = t(&preset.text);
                                    self.choose_symbol(preset);
                                }
                                index += 1;
                                if index % 4 == 0 {
                                    ui.end_row();
                                }
                            }
                        });
                });
        }
        if searching
            && matches == 0
            && !self
                .library
                .symbols
                .iter()
                .any(|n| matches_search(&n.text, &self.symbol_search))
        {
            ui.label(t("Aucun symbole trouvé. Essayez un autre mot."));
        }
    }
    pub(super) fn choose_symbol(&mut self, n: Node) {
        self.set_tool(Tool::Shape(n.kind));
        self.pending_symbol = Some(n);
    }
    fn chart_library(&mut self, ui: &mut egui::Ui) {
        for category in [
            "Histogramme",
            "Graphique à barres",
            "Graphique linéaire",
            "Graphique de zone",
        ] {
            ui.add_space(10.);
            ui.label(RichText::new(t(category)).strong());
            let card_width = ((ui.available_width() - 8.) / 2.).clamp(90., 160.);
            egui::Grid::new(category)
                .num_columns(2)
                .spacing(vec2(8., 10.))
                .show(ui, |ui| {
                    let mut index = 0;
                    for kind in ChartKind::ALL {
                        if kind.category() != category
                            || !matches_search(
                                &format!("{} {}", t(kind.label()), kind.label()),
                                &self.symbol_search,
                            )
                        {
                            continue;
                        }
                        ui.vertical(|ui| {
                            let (r, response) =
                                ui.allocate_exact_size(vec2(card_width, 78.), Sense::click());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(
                                    egui::WidgetType::Button,
                                    ui.is_enabled(),
                                    t(kind.label()),
                                )
                            });
                            ui.painter().rect(
                                r,
                                7,
                                Color32::WHITE,
                                Stroke::new(1., if response.hovered() { ACCENT } else { BORDER }),
                                StrokeKind::Inside,
                            );
                            let mut c = Chart::sample(1, kind, [0., 0.]);
                            c.title.clear();
                            c.legend = false;
                            paint_chart(ui.painter(), &c, r.shrink(6.));
                            ui.add(
                                egui::Label::new(RichText::new(t(kind.label())).size(11.)).wrap(),
                            );
                            focus_ui::outline(ui, &response);
                            reveal_on_focus(ui, &response);
                            if response.on_hover_text(t(kind.label())).clicked() {
                                self.insert_chart(kind);
                            }
                        });
                        index += 1;
                        if index % 2 == 0 {
                            ui.end_row();
                        }
                    }
                });
        }
        ui.add_space(12.);
        ui.label(
            RichText::new(t("Modifiez les valeurs et les séries dans Propriétés."))
                .size(12.)
                .color(MUTED),
        );
    }
    pub(super) fn insert_chart(&mut self, kind: ChartKind) {
        if self.doc.object_count() >= 10_000 {
            self.error = Some("Limite de 10 000 objets atteinte.".into());
            return;
        }
        let center = if self.canvas.is_positive() {
            self.world(self.canvas.center())
        } else {
            self.doc.page_size.map(|v| v / 2.)
        };
        let id = self.doc.add_chart(kind, center);
        self.selected = vec![id];
        self.set_tool(Tool::Select);
        self.show_inspector = true;
        self.commit();
    }
    pub(super) fn chart_inspector(&mut self, ui: &mut egui::Ui, id: u64) {
        let c = self.doc.charts.iter_mut().find(|c| c.id == id).unwrap();
        ui.heading(t("Graphique"));
        ui.add_space(10.);
        ui.add(
            egui::TextEdit::singleline(&mut c.title)
                .desired_width(f32::INFINITY)
                .char_limit(1000),
        );
        egui::ComboBox::from_id_salt("chart_type")
            .selected_text(t(c.kind.label()))
            .show_ui(ui, |ui| {
                for k in ChartKind::ALL {
                    ui.selectable_value(&mut c.kind, k, t(k.label()));
                }
            });
        ui.checkbox(&mut c.legend, t("Afficher la légende"));
        ui.add_space(10.);
        egui::Grid::new("chart_geometry")
            .num_columns(2)
            .show(ui, |ui| {
                for (label, v, min, max) in [
                    ("X", &mut c.position[0], 0., 100_000.),
                    ("L", &mut c.size[0], 120., 20_000.),
                ] {
                    ui.add(
                        egui::DragValue::new(v)
                            .clamp_existing_to_range(false)
                            .prefix(format!("{label} "))
                            .range(min..=max),
                    );
                }
                ui.end_row();
                for (label, v, min, max) in [
                    ("Y", &mut c.position[1], 0., 100_000.),
                    ("H", &mut c.size[1], 120., 20_000.),
                ] {
                    ui.add(
                        egui::DragValue::new(v)
                            .clamp_existing_to_range(false)
                            .prefix(format!("{label} "))
                            .range(min..=max),
                    );
                }
            });
        ui.add_space(16.);
        field_label(ui, "DONNÉES");
        ui.label(
            RichText::new(t("Valeurs positives · 24 catégories, 8 séries maximum"))
                .size(11.)
                .color(MUTED),
        );
        for i in 0..c.categories.len() {
            ui.push_id(i, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut c.categories[i])
                        .char_limit(120)
                        .desired_width(185.),
                );
                ui.horizontal_wrapped(|ui| {
                    for s in &mut c.series {
                        ui.add(
                            egui::DragValue::new(&mut s.values[i])
                                .speed(1.)
                                .range(0.0..=1_000_000.0),
                        )
                        .on_hover_text(&s.name);
                    }
                });
            });
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(c.categories.len() < 24, egui::Button::new(t("+ Catégorie")))
                .clicked()
            {
                c.categories.push(format!("C{}", c.categories.len() + 1));
                for s in &mut c.series {
                    s.values.push(0.);
                }
            }
            if ui
                .add_enabled(c.categories.len() > 1, egui::Button::new("−"))
                .on_hover_text(t("Supprimer la dernière catégorie"))
                .clicked()
            {
                c.categories.pop();
                for s in &mut c.series {
                    s.values.pop();
                }
            }
        });
        ui.add_space(12.);
        field_label(ui, "SÉRIES");
        for (i, s) in c.series.iter_mut().enumerate() {
            ui.push_id(("series", i), |ui| {
                ui.horizontal(|ui| {
                    ui.color_edit_button_srgb(&mut s.color);
                    ui.add(
                        egui::TextEdit::singleline(&mut s.name)
                            .char_limit(120)
                            .desired_width(140.),
                    );
                });
            });
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(c.series.len() < 8, egui::Button::new(t("+ Série")))
                .clicked()
            {
                c.series.push(Series {
                    name: i18n::render("Série {0}", &[("0", format!("{}", c.series.len() + 1))]),
                    values: vec![0.; c.categories.len()],
                    color: [123, 118, 242],
                });
            }
            if ui
                .add_enabled(c.series.len() > 1, egui::Button::new("−"))
                .on_hover_text(t("Supprimer la dernière série"))
                .clicked()
            {
                c.series.pop();
            }
        });
    }
    pub(super) fn select_for_drag(&mut self, id: u64, start: Point, shift: bool) {
        let group = self.doc.expand_groups(&[id]);
        if shift {
            if self.selected.contains(&id) {
                self.selected.retain(|v| !group.contains(v));
            } else {
                self.selected.extend(group);
            }
        } else if !self.selected.contains(&id) {
            self.selected = group;
        }
        self.normalize_selection();
        if !self.selected.contains(&id) || !self.selection_editable() {
            self.drag = None;
            if self.selected.contains(&id) {
                self.status = "La sélection est verrouillée".into();
            }
            return;
        }
        self.begin_edit_drag();
        self.drag = Some(Drag::Move {
            edges: self
                .doc
                .connectors
                .iter()
                .filter(|e| self.selected.contains(&e.id))
                .cloned()
                .collect(),
            start,
            original: self
                .doc
                .nodes
                .iter()
                .filter(|n| self.selected.contains(&n.id))
                .map(|n| (n.id, n.position))
                .collect(),
            paths: self
                .doc
                .drawings
                .iter()
                .filter(|d| self.selected.contains(&d.id))
                .map(|d| (d.id, d.start, d.end))
                .collect(),
            charts: self
                .doc
                .charts
                .iter()
                .filter(|c| self.selected.contains(&c.id))
                .map(|c| (c.id, c.position))
                .collect(),
        });
    }
    pub(super) fn persist_library(&self, next: &Library) -> Result<(), String> {
        if let Some(path) = &self.library_path {
            next.save(path)
        } else {
            next.validate()
        }
    }
    pub(super) fn open_template(&mut self, t: &Template) {
        self.open_tab(t.document.clone(), None);
        self.show_templates = false;
        self.fit_next = true;
        self.selected.clear();
        let mut next = self.library.clone();
        next.remember(&t.id);
        match self.persist_library(&next) {
            Ok(()) => self.library = next,
            Err(e) => self.error = Some(e),
        }
        self.status = i18n::render("Modèle ouvert : {0}", &[("0", t.title.to_string())]);
    }
    pub(super) fn library_dialogs(&mut self, ctx: &egui::Context) {
        if self.show_templates {
            egui::Modal::new("template_gallery".into()).show(ctx, |ui| {
                ui.set_width((ctx.content_rect().width() - 100.).min(1120.));
                ui.horizontal(|ui| {
                    ui.heading(t("Insérer des modèles"));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if ui.button(t("Fermer")).clicked() {
                            self.show_templates = false;
                        }
                    });
                });
                ui.add_space(12.);
                ui.add(
                    egui::TextEdit::singleline(&mut self.template_search)
                        .hint_text(t("Rechercher des diagrammes…"))
                        .char_limit(150)
                        .desired_width(600.),
                );
                ui.add_space(14.);
                let mut chosen = None;
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(175.);
                        for name in [
                            "Recommandé",
                            "Récent",
                            "Modèles personnels",
                            "Général",
                            "Ingénierie",
                            "IT",
                            "Entreprise",
                            "Éducation",
                        ] {
                            ui.selectable_value(&mut self.template_filter, name.into(), t(name));
                            ui.add_space(6.);
                        }
                        ui.separator();
                        if ui
                            .button(t("Enregistrer ce document\ncomme modèle"))
                            .clicked()
                        {
                            self.template_name = self.doc.title.clone();
                            self.save_template_open = true;
                            self.show_templates = false;
                        }
                    });
                    ui.separator();
                    let mut items = self.builtin_templates.clone();
                    items.extend(self.library.templates.iter().map(|t| Template {
                        id: format!("personal:{}", t.name),
                        title: t.name.clone(),
                        category: t.category.clone(),
                        document: t.document.clone(),
                    }));
                    items.retain(|t| {
                        matches_search(&t.title, &self.template_search)
                            && match self.template_filter.as_str() {
                                "Recommandé" => !t.id.starts_with("personal:"),
                                "Récent" => self.library.recent.contains(&t.id),
                                "Modèles personnels" => t.id.starts_with("personal:"),
                                category => t.category == category,
                            }
                    });
                    if self.template_filter == "Récent" {
                        items
                            .sort_by_key(|t| self.library.recent.iter().position(|id| id == &t.id));
                    }
                    let card_width = (((ctx.content_rect().width() - 100.).min(1120.) - 240.) / 3.)
                        .clamp(150., 280.);
                    egui::ScrollArea::vertical()
                        .auto_shrink([false, true])
                        .max_height((ctx.content_rect().height() - 240.).max(200.))
                        .show(ui, |ui| {
                            if items.is_empty() {
                                ui.label(t("Aucun modèle dans cette catégorie."));
                            }
                            egui::Grid::new("template_cards")
                                .num_columns(3)
                                .spacing(vec2(14., 14.))
                                .show(ui, |ui| {
                                    let width = card_width;
                                    for (i, t) in items.iter().enumerate() {
                                        if template_card(ui, t, width).clicked() {
                                            chosen = Some(t.clone());
                                        }
                                        if i % 3 == 2 {
                                            ui.end_row();
                                        }
                                    }
                                });
                        });
                });
                if let Some(t) = chosen {
                    self.open_template(&t);
                }
            });
        }
        if self.save_template_open {
            egui::Modal::new("save_template".into()).show(ctx, |ui| {
                ui.set_width(420.);
                ui.heading(t("Enregistrer un modèle personnel"));
                ui.add_space(12.);
                ui.label(t("Nom"));
                ui.add(
                    egui::TextEdit::singleline(&mut self.template_name)
                        .char_limit(100)
                        .desired_width(f32::INFINITY),
                );
                egui::ComboBox::from_id_salt("template_category")
                    .selected_text(&self.template_category)
                    .show_ui(ui, |ui| {
                        for name in ["Général", "Ingénierie", "IT", "Entreprise", "Éducation"] {
                            ui.selectable_value(&mut self.template_category, name.into(), t(name));
                        }
                    });
                ui.add_space(16.);
                ui.horizontal(|ui| {
                    if ui.button(t("Annuler")).clicked() {
                        self.save_template_open = false;
                    }
                    if ui.button(t("Enregistrer le modèle")).clicked() {
                        let mut next = self.library.clone();
                        match next
                            .add_template(&self.template_name, &self.template_category, &self.doc)
                            .and_then(|_| self.persist_library(&next))
                        {
                            Ok(()) => {
                                self.library = next;
                                self.save_template_open = false;
                                self.status = "Modèle enregistré dans votre bibliothèque".into();
                            }
                            Err(e) => self.error = Some(e),
                        }
                    }
                });
            });
        }
    }
}

fn matches_search(text: &str, search: &str) -> bool {
    commands_ui::normalize(search)
        .split_whitespace()
        .all(|word| commands_ui::normalize(text).contains(word))
}
pub(super) fn symbol_tile(ui: &mut egui::Ui, kind: ShapeKind, active: bool) -> egui::Response {
    named_symbol_tile(ui, kind, active, &t(kind.label()))
}
fn named_symbol_tile(
    ui: &mut egui::Ui,
    kind: ShapeKind,
    active: bool,
    label: &str,
) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(48., 56.), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    ui.painter().rect_filled(
        r,
        8,
        if active {
            Color32::from_rgb(226, 240, 255)
        } else {
            SURFACE
        },
    );
    let mut r = r.shrink2(vec2(7., 12.));
    let aspect = match kind {
        ShapeKind::Actor => 0.6,
        ShapeKind::BraceLeft | ShapeKind::BraceRight => 0.35,
        ShapeKind::Fork => 8.,
        ShapeKind::InitialState | ShapeKind::FinalState => 1.,
        _ => r.width() / r.height(),
    };
    let size = if r.width() / r.height() > aspect {
        vec2(r.height() * aspect, r.height())
    } else {
        vec2(r.width(), r.width() / aspect)
    };
    r = Rect::from_center_size(r.center(), size);
    let stroke = Stroke::new(
        1.2,
        if response.hovered() || active {
            ACCENT
        } else {
            Color32::from_rgb(155, 164, 174)
        },
    );
    if kind == ShapeKind::Text {
        ui.painter().text(
            r.center(),
            Align2::CENTER_CENTER,
            t("T"),
            FontId::proportional(24.),
            stroke.color,
        );
    } else if kind == ShapeKind::Ellipse {
        ui.painter().add(egui::epaint::EllipseShape {
            center: r.center(),
            radius: r.size() / 2.,
            fill: Color32::TRANSPARENT,
            stroke,
        });
    } else if kind == ShapeKind::Rounded {
        ui.painter().rect(
            r,
            if kind == ShapeKind::Ellipse { 20 } else { 6 },
            Color32::TRANSPARENT,
            stroke,
            StrokeKind::Inside,
        );
    } else {
        paint_geometry(ui.painter(), kind, r, Color32::TRANSPARENT, stroke);
    }
    focus_ui::outline(ui, &response);
    reveal_on_focus(ui, &response);
    response.on_hover_text(t(kind.label()))
}
fn template_card(ui: &mut egui::Ui, t: &Template, width: f32) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(width, width * 0.68 + 43.), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), &t.title)
    });
    ui.painter().rect(
        r,
        10,
        SURFACE,
        Stroke::new(1., if response.hovered() { ACCENT } else { BORDER }),
        StrokeKind::Inside,
    );
    let preview = Rect::from_min_size(
        r.min + vec2(10., 10.),
        vec2(width - 20., width * 0.68 - 10.),
    );
    paint_document_preview(ui.painter(), &t.document, preview);
    let title = if t.title.chars().count() > 32 {
        t.title.chars().take(30).collect::<String>() + "…"
    } else {
        t.title.clone()
    };
    ui.painter().text(
        pos2(r.left() + 10., r.bottom() - 22.),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(13.),
        INK,
    );
    focus_ui::outline(ui, &response);
    reveal_on_focus(ui, &response);
    response.on_hover_text(&t.title)
}

fn reveal_on_focus(ui: &egui::Ui, response: &egui::Response) {
    // Tab traversal and accessibility Focus requests can reach clipped cards.
    // A resize can also clip the focused card without changing its identity.
    // Remember only its clip size, so pointer scrolling remains unrestricted.
    if !response.has_focus() {
        return;
    }
    let clip = ui.clip_rect();
    let resized = ui.ctx().data_mut(|data| {
        let key = egui::Id::new("library_focused_clip_size");
        let previous = data.get_temp::<(egui::Id, Vec2)>(key);
        data.insert_temp(key, (response.id, clip.size()));
        previous.is_some_and(|(id, size)| {
            id == response.id && (size - clip.size()).abs().max_elem() > 0.5
        })
    });
    if response.gained_focus() || (resized && !clip.contains_rect(response.rect)) {
        response.scroll_to_me(None);
    }
}
pub(super) fn paint_document_preview(p: &egui::Painter, d: &Document, r: Rect) {
    p.rect_filled(r, 5, SURFACE);
    let scale = (r.width() / d.page_size[0]).min(r.height() / d.page_size[1]);
    let origin = r.center() - vec2(d.page_size[0], d.page_size[1]) * scale / 2.;
    let map = |point: Point| origin + vec2(point[0], point[1]) * scale;
    let page = Rect::from_min_size(origin, vec2(d.page_size[0], d.page_size[1]) * scale);
    p.rect(
        page,
        0,
        rgb(d.page_style.background),
        Stroke::new(0.6, BORDER),
        StrokeKind::Inside,
    );
    let p = p.with_clip_rect(page.intersect(r));
    for edge in &d.connectors {
        let points: Vec<_> = d.route(edge).into_iter().map(map).collect();
        paint_pattern(
            &p,
            &points,
            Stroke::new((edge.width * scale).max(0.5), rgb(edge.color)),
            edge.pattern,
        );
        if edge.arrow {
            paint_head(
                &p,
                &points,
                rgb(edge.color),
                3.,
                0.7,
                edge.end_head,
                rgb(d.page_style.background),
            );
        }
        if edge.arrow_start {
            let reverse = points.iter().rev().copied().collect::<Vec<_>>();
            paint_head(
                &p,
                &reverse,
                rgb(edge.color),
                3.,
                0.7,
                edge.start_head,
                rgb(d.page_style.background),
            );
        }
    }
    for drawing in &d.drawings {
        let points = drawing.points().into_iter().map(map).collect::<Vec<_>>();
        p.add(Shape::line(
            points.clone(),
            Stroke::new((drawing.width * scale).max(0.5), rgb(drawing.color)),
        ));
        if drawing.arrow {
            paint_arrow(&p, &points, rgb(drawing.color), 3.);
        }
    }
    for n in &d.nodes {
        let rect = Rect::from_min_size(map(n.position), vec2(n.size[0], n.size[1]) * scale);
        if n.kind != ShapeKind::Text {
            paint_geometry(
                &p,
                n.kind,
                rect,
                rgb(n.fill),
                Stroke::new(0.7, rgb(n.stroke)),
            );
        }
        if n.font_size * scale >= 3. {
            p.text(
                rect.center(),
                Align2::CENTER_CENTER,
                n.display_text(),
                FontId::proportional((n.font_size * scale).max(3.)),
                rgb(n.text_color),
            );
        }
    }
    for c in &d.charts {
        paint_chart(
            &p,
            c,
            Rect::from_min_size(map(c.position), vec2(c.size[0], c.size[1]) * scale),
        );
    }
}

pub(super) fn paint_geometry(
    p: &egui::Painter,
    kind: ShapeKind,
    r: Rect,
    fill: Color32,
    stroke: Stroke,
) {
    if kind == ShapeKind::Ellipse {
        p.add(egui::epaint::EllipseShape {
            center: r.center(),
            radius: r.size() / 2.,
            fill,
            stroke,
        });
        return;
    }
    if kind == ShapeKind::Rounded {
        p.rect(
            r,
            ((r.height() * 0.18).min(20.)) as u8,
            fill,
            stroke,
            StrokeKind::Middle,
        );
        return;
    }
    let g = geometry::shape(kind);
    let map = |[x, y]: Point| r.min + vec2(x * r.width(), y * r.height());
    paint_polygon(
        p,
        &g.outline.iter().copied().map(map).collect::<Vec<_>>(),
        fill,
    );
    p.add(Shape::closed_line(
        g.outline.into_iter().map(map).collect(),
        stroke,
    ));
    for fill in g.fills {
        paint_polygon(
            p,
            &fill.into_iter().map(map).collect::<Vec<_>>(),
            stroke.color,
        );
    }
    for detail in g.details {
        p.add(Shape::line(detail.into_iter().map(map).collect(), stroke));
    }
}
fn paint_polygon(p: &egui::Painter, points: &[Pos2], fill: Color32) {
    if fill == Color32::TRANSPARENT {
        return;
    }
    let coords: Vec<_> = points.iter().map(|p| [p.x, p.y]).collect();
    let mut mesh = egui::Mesh::default();
    for &pt in points {
        mesh.colored_vertex(pt, fill);
    }
    for [a, b, c] in geometry::triangulate(&coords) {
        mesh.add_triangle(a as u32, b as u32, c as u32);
    }
    p.add(Shape::mesh(mesh));
}
pub(super) fn paint_chart(p: &egui::Painter, c: &Chart, r: Rect) {
    let scale = (r.width() / c.size[0]).min(r.height() / c.size[1]);
    let map = |v: Point| r.min + vec2(v[0] * scale, v[1] * scale);
    let p = p.with_clip_rect(r);
    for primitive in c.primitives() {
        match primitive {
            Primitive::Rect {
                position,
                size,
                fill,
            } => {
                p.rect_filled(
                    Rect::from_min_size(map(position), vec2(size[0], size[1]) * scale),
                    0,
                    rgb(fill),
                );
            }
            Primitive::Path {
                points,
                fill,
                stroke,
                width,
                closed,
            } => {
                let points: Vec<_> = points.into_iter().map(map).collect();
                if let Some(fill) = fill {
                    paint_polygon(&p, &points, rgb(fill));
                }
                if let Some(stroke) = stroke {
                    p.add(if closed {
                        Shape::closed_line(points, Stroke::new(width * scale, rgb(stroke)))
                    } else {
                        Shape::line(points, Stroke::new(width * scale, rgb(stroke)))
                    });
                }
            }
            Primitive::Circle {
                position,
                radius,
                color,
            } => {
                p.circle_filled(map(position), radius * scale, rgb(color));
            }
            Primitive::Text {
                position,
                text,
                size,
                color,
                align,
            } => {
                p.text(
                    map(position),
                    match align {
                        TextAlign::Center => Align2::CENTER_CENTER,
                        TextAlign::Left => Align2::LEFT_CENTER,
                        TextAlign::Right => Align2::RIGHT_CENTER,
                    },
                    text,
                    FontId::proportional(size * scale),
                    rgb(color),
                );
            }
        }
    }
}
pub(super) fn paint_selection(p: &egui::Painter, r: Rect, handles: bool) {
    p.extend(Shape::dashed_line(
        &[
            r.left_top(),
            r.right_top(),
            r.right_bottom(),
            r.left_bottom(),
            r.left_top(),
        ],
        Stroke::new(1.3, ACCENT),
        4.,
        3.,
    ));
    if handles {
        for (at, _) in resize_handles(r) {
            p.rect(
                Rect::from_center_size(at, vec2(7., 7.)),
                1,
                Color32::WHITE,
                Stroke::new(1.4, ACCENT),
                StrokeKind::Middle,
            );
        }
    }
}
