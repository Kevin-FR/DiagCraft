use super::*;

#[derive(Clone, Copy)]
pub(super) enum Action {
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    Front,
    Back,
    Properties,
    Text,
    FitText,
    SelectAll,
    Fit,
    Export,
    ExportPdf,
    Replace(ShapeKind),
    Linked(ShapeKind, Port),
    Insert(ShapeKind),
    Automatic(bool),
    AddPoint,
    RemovePoint(usize),
    Library,
    Brush,
    SwapArrows,
    Detach(bool),
    NodeStyle(usize),
}

impl DrawApp {
    pub(super) fn open_context(&mut self, screen: Pos2, select_target: bool) {
        self.quick_link_port = None;
        if select_target {
            let point = self.world(screen);
            let hit = self.organized_hit(point);
            match hit {
                Some(id) if !self.selected.contains(&id) => self.selected = vec![id],
                None => self.selected.clear(),
                _ => {}
            }
        }
        self.normalize_selection();
        self.context_position = self.world(screen);
        self.context_screen = screen;
        self.context_open = true;
        if self.drag.is_some() && !matches!(self.drag, Some(Drag::Marquee { .. })) {
            self.commit();
        }
        self.drag = None;
        self.set_tool(Tool::Select);
    }
    pub(super) fn canvas_context(&mut self, ui: &egui::Ui, response: &egui::Response) {
        if response.secondary_clicked()
            && let Some(pos) = response.interact_pointer_pos()
        {
            self.open_context(pos, true);
        }
        let mut open = self.context_open;
        let mut action = None;
        egui::Popup::context_menu(response)
            .id(response.id.with("draw_context"))
            .open_bool(&mut open)
            .at_position(self.context_screen)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show(|ui| {
                ui.set_width(if self.quick_link_port.is_some() {
                    210.
                } else {
                    310.
                });
                ui.spacing_mut().item_spacing = vec2(8., 7.);
                egui::ScrollArea::vertical()
                    .max_height((ui.ctx().content_rect().height() - 40.).max(160.))
                    .min_scrolled_height(if self.quick_link_port.is_some() {
                        0.
                    } else {
                        (ui.ctx().content_rect().height() - 40.).max(160.)
                    })
                    .show(ui, |ui| {
                        action = self.context_contents(ui);
                    });
            });
        self.context_open = open;
        if let Some(action) = action {
            self.perform_action(action, ui.ctx());
        }
    }
    fn context_contents(&mut self, ui: &mut egui::Ui) -> Option<Action> {
        let mut action = None;
        if let Some(port) = self.quick_link_port {
            ui.label(RichText::new(t("Ajouter une forme liée")).strong());
            if let Some(kind) = shape_picker(ui) {
                ui.close();
                return Some(Action::Linked(kind, port));
            }
            return None;
        }
        let id = (self.selected.len() == 1).then(|| self.selected[0]);
        let node = id.and_then(|id| self.doc.node(id)).cloned();
        let edge = id
            .and_then(|id| self.doc.connectors.iter().find(|e| e.id == id))
            .cloned();
        let chart = id
            .and_then(|id| self.doc.charts.iter().find(|c| c.id == id))
            .cloned();
        let path = id
            .and_then(|id| self.doc.drawings.iter().find(|d| d.id == id))
            .cloned();
        ui.label(
            RichText::new(t(if let Some(n) = &node {
                n.kind.label().into()
            } else if edge.is_some() {
                "Connecteur".into()
            } else if chart.is_some() {
                "Graphique".into()
            } else if path.is_some() {
                "Trait / courbe".into()
            } else if self.selected.is_empty() {
                "Page".into()
            } else {
                i18n::render(
                    "{0} objets sélectionnés",
                    &[("0", format!("{}", self.selected.len()))],
                )
            }))
            .strong()
            .size(16.),
        );
        if let Some(id) = id {
            ui.add_enabled_ui(self.selection_editable(), |ui| {
                action = self.quick_formats(ui, id);
            });
        }
        ui.add_enabled_ui(self.selection_editable(), |ui| {
            if node.is_some() {
                ui.separator();
                if menu_item(ui, "Modifier le texte…", None, true) {
                    action = Some(Action::Text);
                }
                if menu_item(ui, "Ajuster la forme au texte", None, true) {
                    action = Some(Action::FitText);
                }
                ui.menu_button(t("Remplacer la forme"), |ui| {
                    if let Some(kind) = shape_picker(ui) {
                        action = Some(Action::Replace(kind));
                        ui.close();
                    }
                });
                ui.menu_button(t("Ajouter une forme liée"), |ui| {
                    for (port, label) in [
                        (Port::Right, "À droite"),
                        (Port::Bottom, "En dessous"),
                        (Port::Left, "À gauche"),
                        (Port::Top, "Au-dessus"),
                    ] {
                        ui.menu_button(t(label), |ui| {
                            if let Some(kind) = shape_picker(ui) {
                                action = Some(Action::Linked(kind, port));
                                ui.close();
                            }
                        });
                    }
                });
                if menu_item(ui, "Ajouter à ma bibliothèque", None, true) {
                    action = Some(Action::Library);
                }
            }
            if node.is_none()
                && self.selected.iter().any(|id| self.doc.node(*id).is_some())
                && menu_item(ui, "Ajuster les formes au texte", None, true)
            {
                action = Some(Action::FitText);
            }
            if let Some(e) = &edge {
                ui.separator();
                if menu_item(ui, "Libérer le départ", None, e.from != 0) {
                    action = Some(Action::Detach(true));
                }
                if menu_item(ui, "Libérer l’arrivée", None, e.to != 0) {
                    action = Some(Action::Detach(false));
                }
                let automatic = e.waypoints.is_empty();
                if menu_item(
                    ui,
                    if automatic {
                        "Passer en routage manuel"
                    } else {
                        "Rétablir le routage automatique"
                    },
                    None,
                    e.kind == ConnectorKind::Elbow,
                ) {
                    action = Some(Action::Automatic(!automatic));
                }
                if menu_item(
                    ui,
                    "Ajouter un point de passage ici",
                    None,
                    e.kind == ConnectorKind::Elbow && e.waypoints.len() < 16,
                ) {
                    action = Some(Action::AddPoint);
                }
                if let Some(index) = e
                    .waypoints
                    .iter()
                    .position(|p| self.screen(*p).distance(self.context_screen) <= 12.)
                    && menu_item(ui, "Supprimer ce point de passage", None, true)
                {
                    action = Some(Action::RemovePoint(index));
                }
                ui.menu_button(t("Insérer une forme sur le connecteur"), |ui| {
                    if let Some(kind) = shape_picker(ui) {
                        action = Some(Action::Insert(kind));
                        ui.close();
                    }
                });
                ui.label(
                    RichText::new(t(if automatic {
                        "Le tracé automatique suit les formes."
                    } else {
                        "Glissez les losanges bleus pour modifier le tracé."
                    }))
                    .size(11.)
                    .color(INFO_TEXT),
                );
            }
        });
        ui.separator();
        self.organization_menu(ui);
        if menu_item(
            ui,
            "Annuler la modification",
            Some("Ctrl/⌘ Z"),
            self.history.can_undo(),
        ) {
            action = Some(Action::Undo);
        }
        if menu_item(
            ui,
            "Rétablir",
            Some("Ctrl/⌘ Maj Z"),
            self.history.can_redo(),
        ) {
            action = Some(Action::Redo);
        }
        let copyable = !self.selected.is_empty();
        for (label, shortcut, enabled, a) in [
            (
                "Couper",
                Some("Ctrl/⌘ X"),
                self.selection_editable(),
                Action::Cut,
            ),
            ("Copier", Some("Ctrl/⌘ C"), copyable, Action::Copy),
            ("Coller ici", Some("Ctrl/⌘ V"), true, Action::Paste),
            (
                "Dupliquer",
                Some("Ctrl/⌘ D"),
                self.selection_duplicable(),
                Action::Duplicate,
            ),
            (
                "Supprimer",
                Some("Suppr"),
                self.selection_editable(),
                Action::Delete,
            ),
        ] {
            if menu_item(ui, label, shortcut, enabled) {
                action = Some(a);
            }
        }
        if !self.selected.is_empty() {
            ui.separator();
            let orderable = self.selection_editable()
                && self.selected.iter().all(|id| self.doc.node(*id).is_some());
            if menu_item(ui, "Mettre au premier plan", None, orderable) {
                action = Some(Action::Front);
            }
            if menu_item(ui, "Mettre à l’arrière-plan", None, orderable) {
                action = Some(Action::Back);
            }
            if menu_item(ui, "Propriétés…", None, true) {
                action = Some(Action::Properties);
            }
        } else {
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(t("Fond de page"));
                ui.color_edit_button_srgb(&mut self.doc.page_style.background);
            });
            ui.checkbox(&mut self.grid, t("Afficher la grille"));
            ui.checkbox(&mut self.snap, t("Magnétisme"));
            if menu_item(ui, "Ajouter un texte ici", None, true) {
                action = Some(Action::Text);
            }
            if menu_item(
                ui,
                "Sélectionner tous les objets",
                Some("Ctrl/⌘ A"),
                self.doc.object_count() > 0,
            ) {
                action = Some(Action::SelectAll);
            }
            if menu_item(ui, "Ajuster la page à la fenêtre", Some("F"), true) {
                action = Some(Action::Fit);
            }
            if menu_item(ui, "Exporter en PDF…", Some("Ctrl/⌘ ⇧ E"), true) {
                action = Some(Action::ExportPdf);
            }
            if menu_item(ui, "Exporter en SVG…", Some("Ctrl/⌘ E"), true) {
                action = Some(Action::Export);
            }
        }
        ui.separator();
        ui.checkbox(
            &mut self.show_quick_toolbar,
            t("Barre de mise en forme flottante"),
        );
        if action.is_some() {
            ui.close();
        }
        action
    }
    fn quick_formats(&mut self, ui: &mut egui::Ui, id: u64) -> Option<Action> {
        let mut action = None;
        if let Some(n) = self.doc.node_mut(id) {
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::DragValue::new(&mut n.font_size)
                        .prefix("Taille ")
                        .speed(1.)
                        .range(6.0..=200.0),
                );
                for (label, value) in [
                    ("B", &mut n.text_style.bold),
                    ("I", &mut n.text_style.italic),
                    ("U", &mut n.text_style.underline),
                ] {
                    if ui.selectable_label(*value, t(label)).clicked() {
                        *value = !*value;
                    }
                }
                egui::ComboBox::from_id_salt(("quick_align", id))
                    .width(80.)
                    .selected_text(t(match n.text_style.align {
                        TextAlign::Left => "Gauche",
                        TextAlign::Center => "Centré",
                        TextAlign::Right => "Droite",
                    }))
                    .show_ui(ui, |ui| {
                        for (v, name) in [
                            (TextAlign::Left, "Gauche"),
                            (TextAlign::Center, "Centré"),
                            (TextAlign::Right, "Droite"),
                        ] {
                            ui.selectable_value(&mut n.text_style.align, v, t(name));
                        }
                    });
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(t("Fond"));
                ui.color_edit_button_srgb(&mut n.fill);
                ui.label(t("Ligne"));
                ui.color_edit_button_srgb(&mut n.stroke);
                ui.label(t("Texte"));
                ui.color_edit_button_srgb(&mut n.text_color);
            });
        } else if let Some(e) = self.doc.connectors.iter_mut().find(|e| e.id == id) {
            contextual_toolbar::edge_style_controls(ui, e);
            ui.horizontal_wrapped(|ui| {
                ui.checkbox(&mut e.arrow_start, t("Flèche départ"));
                ui.checkbox(&mut e.arrow, t("Flèche arrivée"));
                let old = e.kind;
                egui::ComboBox::from_id_salt(("quick_edge_kind", id))
                    .width(95.)
                    .selected_text(t(if e.kind == ConnectorKind::Elbow {
                        "Angle droit"
                    } else {
                        "Direct"
                    }))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut e.kind, ConnectorKind::Elbow, t("Angle droit"));
                        ui.selectable_value(&mut e.kind, ConnectorKind::Straight, t("Direct"));
                    });
                if old != e.kind && e.kind == ConnectorKind::Straight {
                    e.waypoints.clear();
                }
            });
            ui.horizontal_wrapped(|ui| {
                ui.label(t("Trait"));
                ui.color_edit_button_srgb(&mut e.color);
                ui.add(
                    egui::DragValue::new(&mut e.width)
                        .range(0.5..=20.0)
                        .speed(0.1)
                        .suffix(" px"),
                );
                if e.kind == ConnectorKind::Elbow {
                    let automatic = e.waypoints.is_empty();
                    if ui
                        .small_button(t(if automatic {
                            "Routage : automatique"
                        } else {
                            "Routage : manuel"
                        }))
                        .clicked()
                    {
                        action = Some(Action::Automatic(!automatic));
                    }
                }
            });
        } else if let Some(c) = self.doc.charts.iter_mut().find(|c| c.id == id) {
            ui.horizontal_wrapped(|ui| {
                egui::ComboBox::from_id_salt(("quick_chart", id))
                    .selected_text(t(c.kind.label()))
                    .show_ui(ui, |ui| {
                        for kind in ChartKind::ALL {
                            ui.selectable_value(&mut c.kind, kind, t(kind.label()));
                        }
                    });
                ui.checkbox(&mut c.legend, t("Légende"));
            });
        } else if let Some(d) = self.doc.drawings.iter_mut().find(|d| d.id == id) {
            ui.horizontal_wrapped(|ui| {
                ui.label(t("Trait"));
                ui.color_edit_button_srgb(&mut d.color);
                ui.add(
                    egui::DragValue::new(&mut d.width)
                        .range(0.5..=20.)
                        .speed(0.1)
                        .suffix(" px"),
                );
                ui.checkbox(&mut d.arrow, t("Flèche"));
            });
        }
        action
    }
    pub(super) fn perform_action(&mut self, action: Action, ctx: &egui::Context) {
        self.normalize_selection();
        if matches!(
            action,
            Action::Cut
                | Action::Delete
                | Action::Front
                | Action::Back
                | Action::FitText
                | Action::Replace(_)
                | Action::Linked(_, _)
                | Action::Insert(_)
                | Action::Automatic(_)
                | Action::AddPoint
                | Action::RemovePoint(_)
                | Action::SwapArrows
                | Action::Detach(_)
                | Action::NodeStyle(_)
        ) && !self.selection_editable()
        {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        self.context_open = false;
        self.quick_link_port = None;
        let id = self.selected.first().copied();
        match action {
            Action::Brush => {
                self.format_brush = id.and_then(|id| self.doc.node(id)).cloned();
                self.edge_brush = id
                    .and_then(|id| self.doc.connectors.iter().find(|e| e.id == id))
                    .cloned();
                self.status = if self.edge_brush.is_some() {
                    "Cliquez sur un connecteur pour reproduire son style."
                } else {
                    "Cliquez sur une forme pour reproduire son style."
                }
                .into();
            }
            Action::Detach(start) => {
                if let Some(e) = id.and_then(|id| self.doc.connectors.iter().find(|e| e.id == id)) {
                    let id = e.id;
                    let route = self.cached_route(e.id);
                    let point = if start { route.first() } else { route.last() }.copied();
                    if let Some(point) = point
                        && let Err(e) = self.doc.release_connector_end(id, start, point)
                    {
                        self.error = Some(e);
                    }
                }
            }
            Action::SwapArrows => {
                if let Some(e) = self.doc.connectors.iter_mut().find(|e| Some(e.id) == id) {
                    std::mem::swap(&mut e.arrow, &mut e.arrow_start);
                    std::mem::swap(&mut e.end_head, &mut e.start_head);
                }
            }
            Action::NodeStyle(index) => {
                if let Some(n) = id.and_then(|id| self.doc.node_mut(id)) {
                    n.fill = PALETTE[index.min(PALETTE.len() - 1)];
                    n.stroke = [66, 88, 110];
                    n.stroke_width = 1.5;
                }
            }
            Action::Undo => self.undo(),
            Action::Redo => self.redo(),
            Action::Copy => self.copy(ctx, false),
            Action::Cut => self.copy(ctx, true),
            Action::Duplicate => self.duplicate(),
            Action::Delete => self.delete(),
            Action::Paste => {
                if let Some(focus) = ctx.memory(|m| m.focused()) {
                    ctx.memory_mut(|m| m.surrender_focus(focus));
                }
                self.paste_position = Some(self.context_position);
                if self.paste_image() {
                    return;
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            }
            Action::Front => self.reorder(true),
            Action::Back => self.reorder(false),
            Action::Properties => self.show_inspector = true,
            Action::Fit => self.fit_next = true,
            Action::FitText => self.fit_selection_text(),
            Action::Export => self.export(),
            Action::ExportPdf => self.show_pdf_export = true,
            Action::SelectAll => {
                let view = self.doc.organization_view();
                self.selected = self
                    .doc
                    .object_ids()
                    .into_iter()
                    .filter(|id| view.is_object_visible(*id))
                    .collect();
            }
            Action::Text => {
                if !self.selected.is_empty() && !self.selection_editable() {
                    self.status = "La sélection est verrouillée".into();
                    return;
                }
                if let Some(id) = id
                    && (self.doc.node(id).is_some()
                        || self.doc.connectors.iter().any(|e| e.id == id))
                {
                    if self.doc.node(id).is_some() {
                        self.open_node_text(id);
                    } else {
                        self.focus_text = Some(id);
                        self.show_inspector = true;
                    }
                } else if self.selected.is_empty() && self.doc.object_count() < 10_000 {
                    let id = self.add_node(ShapeKind::Text, self.context_position);
                    self.selected = vec![id];
                    self.show_inspector = true;
                    self.focus_text = Some(id);
                }
            }
            Action::Replace(kind) => {
                if let Some(n) = id.and_then(|id| self.doc.node_mut(id)) {
                    n.kind = kind;
                }
            }
            Action::Linked(kind, port) => {
                if let Some(id) = id {
                    match self.doc.add_linked_shape(id, kind, port) {
                        Ok(id) => {
                            self.selected = vec![id];
                            self.fit_next = true;
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            }
            Action::Insert(kind) => {
                if let Some(id) = id {
                    match self
                        .doc
                        .insert_shape_on_connector(id, kind, self.context_position)
                    {
                        Ok(id) => self.selected = vec![id],
                        Err(e) => self.error = Some(e),
                    }
                }
            }
            Action::Automatic(automatic) => {
                if let Some(id) = id {
                    if let Err(e) = self.doc.set_automatic_route(id, automatic) {
                        self.error = Some(e);
                    } else {
                        self.status = if automatic {
                            "Routage automatique rétabli"
                        } else {
                            "Routage manuel : glissez les points de passage bleus"
                        }
                        .into();
                    }
                }
            }
            Action::AddPoint => {
                if let Some(id) = id
                    && let Err(e) = self.doc.add_connector_waypoint(id, self.context_position)
                {
                    self.error = Some(e);
                }
            }
            Action::RemovePoint(index) => {
                if let Some(e) = self.doc.connectors.iter_mut().find(|e| Some(e.id) == id)
                    && index < e.waypoints.len()
                {
                    e.waypoints.remove(index);
                }
            }
            Action::Library => {
                if let Some(n) = id.and_then(|id| self.doc.node(id)).cloned() {
                    let mut next = self.library.clone();
                    match next
                        .add_symbol(&n)
                        .and_then(|_| self.persist_library(&next))
                    {
                        Ok(()) => {
                            self.library = next;
                            self.status = "Symbole ajouté à votre bibliothèque".into();
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            }
        }
        self.commit();
    }
    pub(super) fn selection_rect(&self) -> Option<Rect> {
        let mut rect = Rect::NOTHING;
        for n in &self.doc.nodes {
            if self.selected.contains(&n.id) {
                rect = rect.union(Rect::from_min_size(
                    self.screen(n.position),
                    vec2(n.size[0], n.size[1]) * self.zoom,
                ));
            }
        }
        for c in &self.doc.charts {
            if self.selected.contains(&c.id) {
                rect = rect.union(Rect::from_min_size(
                    self.screen(c.position),
                    vec2(c.size[0], c.size[1]) * self.zoom,
                ));
            }
        }
        for e in &self.doc.connectors {
            if self.selected.contains(&e.id) {
                for p in self.cached_route(e.id) {
                    rect.extend_with(self.screen(p));
                }
            }
        }
        for d in &self.doc.drawings {
            if self.selected.contains(&d.id) {
                for p in d.points() {
                    rect.extend_with(self.screen(p));
                }
            }
        }
        rect.is_finite().then_some(rect)
    }
    pub(super) fn paint_connector_handles(&self, p: &egui::Painter) {
        if self.selected.len() != 1 || !self.selection_editable() {
            return;
        }
        let Some(e) = self
            .doc
            .connectors
            .iter()
            .find(|e| e.id == self.selected[0])
        else {
            return;
        };
        let route = self.cached_route(e.id);
        for (index, point) in route.first().into_iter().chain(route.last()).enumerate() {
            p.circle(
                self.screen(*point),
                4.,
                Color32::WHITE,
                Stroke::new(
                    1.5,
                    if index == 0 {
                        Color32::from_rgb(241, 87, 87)
                    } else {
                        ACCENT
                    },
                ),
            );
        }
        for point in self.segment_handles(e.id) {
            let pt = self.screen(point);
            if !e
                .waypoints
                .iter()
                .any(|p| self.screen(*p).distance(pt) < 10.)
            {
                p.circle(pt, 3.5, Color32::WHITE, Stroke::new(1., ACCENT));
            }
        }
        if e.kind != ConnectorKind::Elbow {
            return;
        }
        for point in &e.waypoints {
            let pt = self.screen(*point);
            p.add(Shape::convex_polygon(
                vec![
                    pt + vec2(0., -5.),
                    pt + vec2(5., 0.),
                    pt + vec2(0., 5.),
                    pt + vec2(-5., 0.),
                ],
                Color32::WHITE,
                Stroke::new(1.5, ACCENT),
            ));
        }
    }
    pub(super) fn paste_text(&mut self, text: &str) {
        let point = self.paste_position.take();
        if text.len() > document::MAX_FILE_BYTES as usize {
            self.error = Some("Presse-papiers trop volumineux.".into());
            return;
        }
        if let Ok(mut fragment) = serde_json::from_str::<Document>(text) {
            fragment.upgrade_legacy();
            if let Err(e) = fragment.validate() {
                self.error = Some(e);
                return;
            }
            if let Some(point) = point {
                let positions = fragment
                    .nodes
                    .iter()
                    .map(|n| n.position)
                    .chain(fragment.charts.iter().map(|c| c.position))
                    .chain(fragment.drawings.iter().flat_map(|d| d.controls()))
                    .chain(fragment.connectors.iter().flat_map(|e| fragment.route(e)));
                let min =
                    positions.fold([f32::INFINITY; 2], |a, b| [a[0].min(b[0]), a[1].min(b[1])]);
                if min.iter().all(|v| v.is_finite()) {
                    match self
                        .doc
                        .paste(&fragment, [point[0] - min[0], point[1] - min[1]])
                    {
                        Ok(ids) => {
                            self.selected = ids;
                            self.commit();
                        }
                        Err(e) => self.error = Some(e),
                    }
                }
            } else {
                self.paste_fragment(fragment);
            }
        } else if text.len() <= 4000 && !text.trim().is_empty() && self.doc.object_count() < 10_000
        {
            let center = point.unwrap_or_else(|| self.world(self.canvas.center()));
            let id = self.add_node(ShapeKind::Text, center);
            self.doc.node_mut(id).unwrap().text = text.into();
            self.selected = vec![id];
            self.commit();
        }
    }
}
fn menu_item(ui: &mut egui::Ui, label: &str, shortcut: Option<&str>, enabled: bool) -> bool {
    let mut button = egui::Button::new(t(label));
    if let Some(text) = shortcut {
        button = button.shortcut_text(text);
    }
    ui.add_enabled(enabled, button).clicked()
}
pub(super) fn shape_picker(ui: &mut egui::Ui) -> Option<ShapeKind> {
    let mut chosen = None;
    egui::Grid::new("context_shapes")
        .num_columns(4)
        .spacing(vec2(6., 6.))
        .show(ui, |ui| {
            for (i, kind) in ShapeKind::FLOWCHART
                .into_iter()
                .chain(ShapeKind::ARROWS)
                .enumerate()
            {
                let (r, response) = ui.allocate_exact_size(vec2(44., 40.), Sense::click());
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        kind.label(),
                    )
                });
                ui.painter().rect_filled(
                    r,
                    5,
                    if response.hovered() {
                        Color32::from_rgb(228, 241, 255)
                    } else {
                        SURFACE
                    },
                );
                paint_geometry(
                    ui.painter(),
                    kind,
                    r.shrink(7.),
                    Color32::WHITE,
                    Stroke::new(1., MUTED),
                );
                if response.on_hover_text(t(kind.label())).clicked() {
                    chosen = Some(kind);
                }
                if i % 4 == 3 {
                    ui.end_row();
                }
            }
        });
    chosen
}
