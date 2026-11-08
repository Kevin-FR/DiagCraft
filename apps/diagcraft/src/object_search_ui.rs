//! Find and reveal page objects without editing the document or occupying a panel.
use super::*;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Kind {
    #[default]
    All,
    Nodes,
    Images,
    Links,
    Drawings,
    Charts,
}
impl Kind {
    fn label(self) -> &'static str {
        match self {
            Self::All => "Tous",
            Self::Nodes => "Formes et textes",
            Self::Images => "Images",
            Self::Links => "Connecteurs",
            Self::Drawings => "Traits et courbes",
            Self::Charts => "Graphiques",
        }
    }
}
/// Portée de la recherche d'objets : page active ou toutes les pages.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Scope {
    #[default]
    Active,
    All,
}
pub(super) struct Entry {
    pub id: u64,
    pub kind: Kind,
    pub label: String,
    pub detail: String,
    /// Index de page d'origine (0 pour la page active).
    page: usize,
    /// Étiquettes normalisées (minuscules, sans accents) pour le filtre.
    tags: Vec<String>,
    search: String,
}
#[derive(Default)]
pub(super) struct Search {
    pub open: bool,
    pub query: String,
    pub kind: Kind,
    /// Étiquette filtrante ; `None` = aucune (toutes les étiquettes).
    pub tag: Option<String>,
    pub scope: Scope,
    pub selected: usize,
    pub entries: Vec<Entry>,
    /// Étiquettes distinctes triées, pour le rang de filtres.
    pub tag_list: Vec<String>,
    pub rect: Option<Rect>,
    pub close_id: Option<egui::Id>,
    focus: bool,
    page: usize,
    tab: usize,
    revision: String,
}
const MAX_PAGE_ENTRIES: usize = 300;
impl Search {
    pub fn matches(&self) -> Vec<usize> {
        let query = commands_ui::normalize(&self.query);
        let words = query.split_whitespace().collect::<Vec<_>>();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                let tag_ok = self
                    .tag
                    .as_ref()
                    .is_none_or(|tag| e.tags.iter().any(|t| t == tag));
                ((self.kind == Kind::All || self.kind == e.kind)
                    && tag_ok
                    && words.iter().all(|word| e.search.contains(word)))
                .then_some(i)
            })
            .collect()
    }
}

fn entry(
    id: u64,
    kind: Kind,
    text: &str,
    label: &str,
    position: Point,
    ann: &diagcraft::document::Annotations,
    page: usize,
) -> Entry {
    let title = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(label);
    let mut short = title.chars().take(80).collect::<String>();
    if title.chars().count() > 80 {
        short.push('…');
    }
    let mut detail = format!(
        "{} · #{} · X {:.0} · Y {:.0} px",
        label, id, position[0], position[1]
    );
    if let Some(tag) = ann.tags.first() {
        detail.push_str(&format!(" · {tag}"));
    }
    Entry {
        id,
        kind,
        label: short,
        detail,
        page,
        tags: ann
            .tags
            .iter()
            .map(|tag| commands_ui::normalize(tag))
            .collect(),
        search: commands_ui::normalize(&format!(
            "{text} {label} {} #{id} {}",
            t(kind.label()),
            ann.search_text()
        )),
    }
}
/// Construit une ligne pour un objet d'un document de page donné.
fn object_entries(doc: &Document, page: usize) -> Vec<Entry> {
    let view = doc.organization_view();
    doc.nodes
        .iter()
        .map(|n| {
            if let Some(image) = &n.image {
                entry(
                    n.id,
                    Kind::Images,
                    &image.alt,
                    &t("Image"),
                    n.position,
                    &n.annotations,
                    page,
                )
            } else {
                entry(
                    n.id,
                    Kind::Nodes,
                    &n.text,
                    &t(n.kind.label()),
                    n.position,
                    &n.annotations,
                    page,
                )
            }
        })
        .chain(doc.connectors.iter().map(|e| {
            entry(
                e.id,
                Kind::Links,
                &e.label,
                &t("Connecteur"),
                doc.route(e).first().copied().unwrap_or([0., 0.]),
                &e.annotations,
                page,
            )
        }))
        .chain(doc.drawings.iter().map(|d| {
            entry(
                d.id,
                Kind::Drawings,
                "",
                &t(if d.kind == DrawingKind::Line {
                    "Trait"
                } else {
                    "Courbe de Bézier"
                }),
                d.start,
                &d.annotations,
                page,
            )
        }))
        .chain(doc.charts.iter().map(|c| {
            entry(
                c.id,
                Kind::Charts,
                &c.title,
                &t(c.kind.label()),
                c.position,
                &c.annotations,
                page,
            )
        }))
        .filter(|e| view.is_object_visible(e.id))
        .collect()
}
/// Rassemblent les lignes selon la portée : page active ou toutes les pages
/// (borné à 300 résultats pour rester réactif). Les objets conservent l'index
/// de leur page d'origine pour la navigation.
fn entries(doc: &Document, scope: Scope) -> Vec<Entry> {
    let mut out = if scope == Scope::All {
        let mut all = Vec::new();
        for page in 0..doc.page_count() {
            if let Some(page_doc) = doc.page_document(page) {
                let prefix = i18n::render("Page {0}", &[("0", (page + 1).to_string())]);
                for mut e in object_entries(&page_doc, page) {
                    // Préfixe la page d'origine dans le détail pour la portée
                    // « toutes les pages ».
                    e.detail = format!("{prefix} · {}", e.detail);
                    all.push(e);
                }
            }
        }
        all
    } else {
        object_entries(doc, doc.page_index)
    };
    if out.len() > MAX_PAGE_ENTRIES {
        out.truncate(MAX_PAGE_ENTRIES);
    }
    out
}
/// Étiquettes distinctes normalisées (dans l'ordre d'apparition), bornées à 16.
fn tag_list(entries: &[Entry]) -> Vec<String> {
    let mut seen: Vec<String> = Vec::new();
    for e in entries {
        for tag in &e.tags {
            if !seen.contains(tag) && seen.len() < 16 {
                seen.push(tag.clone());
            }
        }
    }
    seen
}

#[derive(Clone, Copy)]
enum Action {
    Select { id: u64, add: bool, page: usize },
    Edit { id: u64, page: usize },
}

fn result_row(ui: &mut egui::Ui, e: &Entry, selected: bool, height: f32) -> egui::Response {
    let response = ui.allocate_response(vec2(ui.available_width(), height), Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            format!("{} · {}", e.label, e.detail),
        )
    });
    let visuals = ui.style().interact_selectable(&response, selected);
    ui.painter().rect(
        response.rect,
        6,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        StrokeKind::Inside,
    );
    let mut content =
        ui.new_child(egui::UiBuilder::new().max_rect(response.rect.shrink2(vec2(9., 5.))));
    content.set_clip_rect(response.rect.intersect(ui.clip_rect()));
    content.spacing_mut().item_spacing.y = 2.;
    content.add(egui::Label::new(&e.label).truncate().selectable(false));
    content.add(
        egui::Label::new(RichText::new(&e.detail).small().color(INFO_TEXT))
            .truncate()
            .selectable(false),
    );
    response
}

impl DrawApp {
    pub(super) fn open_object_search(&mut self) {
        if self.command_dialog_blocked()
            || self.drag.is_some()
            || self.show_help
            || self.command_palette.open
        {
            return;
        }
        self.commit();
        self.context_open = false;
        let entries = entries(&self.doc, Scope::Active);
        let tag_list = tag_list(&entries);
        self.object_search = Search {
            open: true,
            focus: true,
            page: self.doc.page_index,
            tab: self.active_tab,
            entries,
            tag_list,
            revision: diagcraft::agent::revision(&self.doc),
            ..Default::default()
        };
    }
    fn close_object_search(&mut self, ctx: &egui::Context) {
        self.object_search = Search::default();
        ctx.memory_mut(|m| {
            if let Some(focus) = m.focused() {
                m.surrender_focus(focus);
            }
        });
    }
    pub(super) fn reveal_selection(&mut self, ctx: &egui::Context) {
        let Some(bounds) = self.selection_rect() else {
            return;
        };
        let viewport = self.canvas.shrink(36.);
        if !viewport.is_positive() {
            return;
        }
        self.fit_next = false;
        if bounds.width() > viewport.width() || bounds.height() > viewport.height() {
            self.fit_selection(ctx);
        } else if !viewport.contains_rect(bounds) {
            self.pan += viewport.center() - bounds.center();
        }
    }
    fn choose_object(&mut self, ctx: &egui::Context, action: Action) {
        let (id, add, page, edit) = match action {
            Action::Select { id, add, page } => (id, add, page, false),
            Action::Edit { id, page } => (id, false, page, true),
        };
        // Une page distante exige une navigation avant la sélection.
        if page != self.doc.page_index && !self.switch_page(page) {
            return;
        }
        let old = std::mem::take(&mut self.selected);
        self.set_tool(Tool::Select);
        self.selected = self.doc.expand_groups(&[id]);
        self.normalize_selection();
        self.reveal_selection(ctx);
        if add {
            self.selected = old;
            if !self.selected.contains(&id) {
                self.selected.push(id);
            }
        }
        self.normalize_selection();
        self.status = i18n::render(
            "Objets sélectionnés : {0} · Maj + F pour zoomer",
            &[("0", self.selected.len().to_string())],
        );
        if edit {
            self.open_node_text(id);
        }
    }
    pub(super) fn object_search_dialog(&mut self, ctx: &egui::Context) {
        if !self.object_search.open {
            return;
        }
        if self.object_search.tab != self.active_tab
            || (self.object_search.scope == Scope::Active
                && self.object_search.page != self.doc.page_index)
        {
            self.close_object_search(ctx);
            return;
        }
        let focused = self.object_search.focus
            || ctx.memory(|m| {
                m.focused()
                    .is_none_or(|id| id == egui::Id::new("object_query"))
            });
        let up = focused && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowUp));
        let down =
            focused && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::ArrowDown));
        if up || down {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        let add = focused && ctx.input_mut(|i| i.consume_key(egui::Modifiers::SHIFT, Key::Enter));
        let select = focused && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::Enter));
        let edit = focused && ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, Key::F2));
        let mut action = None;
        let mut close = false;
        let search = &mut self.object_search;
        let mut new_scope = search.scope;
        let modal = egui::Modal::new("object_search".into()).show(ctx, |ui| {
            ui.set_width(620_f32.min((ctx.content_rect().width() - 48.).max(240.)));
            ui.horizontal(|ui| {
                ui.heading(t("Rechercher un objet"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    let response = ui.button(t("Fermer"));
                    search.close_id = Some(response.id);
                    close = response.clicked();
                });
            });
            ui.label(i18n::render(
                "Page active : {0}",
                &[("0", self.doc.page_name.clone())],
            ));
            let focus_requested = search.focus;
            if search.focus {
                ui.memory_mut(|m| m.request_focus("object_query".into()));
                search.focus = false;
            }
            let output = egui::TextEdit::singleline(&mut search.query)
                .id("object_query".into())
                .hint_text(t("Texte, type ou #identifiant…"))
                .char_limit(120)
                .desired_width(f32::INFINITY)
                .show(ui);
            if focus_requested && output.cursor_range.is_none() {
                search.focus = true;
                self.replay_input.extend(ui.input(|i| {
                    i.events
                        .iter()
                        .filter(|e| {
                            matches!(
                                e,
                                egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Ime(_)
                            )
                        })
                        .cloned()
                        .collect::<Vec<_>>()
                }));
                ctx.request_repaint();
            }
            let query = output.response;
            let mut changed = query.changed();
            new_scope = search.scope;
            ui.horizontal(|ui| {
                ui.label(t("Portée :"));
                egui::ComboBox::from_id_salt("object_search_scope")
                    .selected_text(t(if new_scope == Scope::All {
                        "Toutes les pages"
                    } else {
                        "Page active"
                    }))
                    .width(220.)
                    .show_ui(ui, |ui| {
                        if ui
                            .selectable_value(&mut new_scope, Scope::Active, t("Page active"))
                            .changed()
                        {
                            changed = true;
                        }
                        if ui
                            .selectable_value(&mut new_scope, Scope::All, t("Toutes les pages"))
                            .changed()
                        {
                            changed = true;
                        }
                    });
            });
            ui.horizontal_wrapped(|ui| {
                for kind in [
                    Kind::All,
                    Kind::Nodes,
                    Kind::Images,
                    Kind::Links,
                    Kind::Drawings,
                    Kind::Charts,
                ] {
                    if ui
                        .selectable_value(&mut search.kind, kind, t(kind.label()))
                        .changed()
                    {
                        changed = true;
                        search.focus = true;
                    }
                }
            });
            if !search.tag_list.is_empty() {
                ui.horizontal_wrapped(|ui| {
                    ui.label(t("Étiquette :"));
                    if ui
                        .selectable_label(search.tag.is_none(), t("Toutes"))
                        .clicked()
                    {
                        search.tag = None;
                        changed = true;
                    }
                    for tag in &search.tag_list {
                        let active = search.tag.as_deref() == Some(tag.as_str());
                        if ui.selectable_label(active, tag.as_str()).clicked() {
                            search.tag = if active { None } else { Some(tag.clone()) };
                            changed = true;
                        }
                    }
                });
            }
            let found = search.matches();
            if changed {
                search.selected = 0;
            }
            search.selected = search.selected.min(found.len().saturating_sub(1));
            if !found.is_empty() && (up || down) {
                search.selected =
                    (search.selected + if up { found.len() - 1 } else { 1 }) % found.len();
            }
            ui.label(
                RichText::new(i18n::render(
                    "Objets trouvés : {0}",
                    &[("0", found.len().to_string())],
                ))
                .small()
                .color(INFO_TEXT),
            );
            if new_scope == Scope::All && search.entries.len() == MAX_PAGE_ENTRIES {
                ui.label(t("Seuls les 300 premiers résultats sont affichés."));
            }
            let row_height = 48.;
            let scroll = egui::ScrollArea::vertical()
                .id_salt("object_results")
                .max_height((ctx.content_rect().height() - 390.).clamp(72., 320.));
            let scroll = if up || down || changed {
                scroll.vertical_scroll_offset(
                    search.selected as f32 * (row_height + ui.spacing().item_spacing.y),
                )
            } else {
                scroll
            };
            scroll.show_rows(ui, row_height, found.len(), |ui, range| {
                for index in range {
                    let e = &search.entries[found[index]];
                    ui.push_id(e.id, |ui| {
                        let row = result_row(ui, e, index == search.selected, row_height);
                        if row.clicked() {
                            action = Some(Action::Select {
                                id: e.id,
                                add: ctx.input(|i| i.modifiers.shift),
                                page: e.page,
                            });
                        }
                    });
                }
            });
            if found.is_empty() {
                if new_scope == Scope::All {
                    ui.label(t("Aucun résultat dans les pages."));
                } else {
                    ui.label(t("Aucun objet trouvé sur cette page."));
                }
                ui.label(t("Essayez un autre texte, un type ou un identifiant."));
            }
            ui.separator();
            let active = found.get(search.selected).map(|i| &search.entries[*i]);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(active.is_some(), egui::Button::new(t("Sélectionner")))
                    .clicked()
                {
                    action = active.map(|e| Action::Select {
                        id: e.id,
                        add: false,
                        page: e.page,
                    });
                }
                if ui
                    .add_enabled(
                        active.is_some(),
                        egui::Button::new(t("Ajouter à la sélection")),
                    )
                    .clicked()
                {
                    action = active.map(|e| Action::Select {
                        id: e.id,
                        add: true,
                        page: e.page,
                    });
                }
                let node = active.filter(|e| e.kind == Kind::Nodes);
                if ui
                    .add_enabled(node.is_some(), egui::Button::new(t("Modifier le texte")))
                    .clicked()
                {
                    action = node.map(|e| Action::Edit {
                        id: e.id,
                        page: e.page,
                    });
                }
            });
            ui.label(
                RichText::new(t(
                    "↑ ↓ Choisir · Entrée Sélectionner · Maj+Entrée Ajouter · F2 Texte",
                ))
                .small()
                .color(INFO_TEXT),
            );
            if let Some(e) = active {
                if select || add {
                    action = Some(Action::Select {
                        id: e.id,
                        add,
                        page: e.page,
                    });
                } else if edit && e.kind == Kind::Nodes {
                    action = Some(Action::Edit {
                        id: e.id,
                        page: e.page,
                    });
                }
            }
        });
        self.object_search.rect = Some(modal.response.rect);
        let scope_changed = new_scope != self.object_search.scope;
        self.object_search.scope = new_scope;
        if scope_changed {
            self.object_search.entries = entries(&self.doc, new_scope);
            self.object_search.tag_list = tag_list(&self.object_search.entries);
            if self
                .object_search
                .tag
                .as_ref()
                .is_some_and(|tag| !self.object_search.tag_list.contains(tag))
            {
                self.object_search.tag = None;
            }
            self.object_search.selected = 0;
            self.object_search.focus = true;
        }
        // Check the snapshot only on activation, avoiding serialization and
        // hashing of a large drawing on every navigation frame.
        if action.is_some() {
            let revision = diagcraft::agent::revision(&self.doc);
            if revision != self.object_search.revision {
                self.object_search.entries = entries(&self.doc, new_scope);
                self.object_search.tag_list = tag_list(&self.object_search.entries);
                self.object_search.revision = revision;
                self.object_search.selected = 0;
                self.object_search.focus = true;
                action = None;
            }
        }
        if close || modal.should_close() || action.is_some() {
            self.close_object_search(ctx);
            if let Some(action) = action {
                self.choose_object(ctx, action);
            }
        }
    }

    pub(super) fn place_shape_in_view(&mut self, ctx: &egui::Context) {
        let Tool::Shape(kind) = self.tool else {
            return;
        };
        if self.doc.object_count() >= 10_000 {
            return;
        }
        self.commit();
        let point = self.world(self.canvas.center());
        let id = self.add_node(kind, point);
        let bounds = self.doc.edit_bounds();
        let node = self.doc.node_mut(id).unwrap();
        if let Some(mut custom) = self.pending_symbol.take() {
            custom.id = id;
            custom.position = [
                point[0] - custom.size[0] / 2.,
                point[1] - custom.size[1] / 2.,
            ];
            *node = custom;
        }
        node.text_style.wrap = true;
        for (a, bound) in bounds.into_iter().enumerate() {
            node.position[a] = node.position[a].clamp(0., (bound - node.size[a]).max(0.));
        }
        let initial = node.position;
        let size = node.size;
        let page = Rect::from_min_size(Pos2::ZERO, vec2(bounds[0], bounds[1]));
        let visible = Rect::from_two_pos(
            pos2(
                self.world(self.canvas.left_top())[0],
                self.world(self.canvas.left_top())[1],
            ),
            pos2(
                self.world(self.canvas.right_bottom())[0],
                self.world(self.canvas.right_bottom())[1],
            ),
        )
        .intersect(page);
        let mut candidates = vec![initial];
        for radius in 1..=8 {
            for direction in [
                [1., 0.],
                [0., 1.],
                [-1., 0.],
                [0., -1.],
                [1., 1.],
                [-1., 1.],
                [1., -1.],
                [-1., -1.],
            ] {
                candidates.push([
                    initial[0] + direction[0] * (size[0] + 24.) * radius as f32,
                    initial[1] + direction[1] * (size[1] + 24.) * radius as f32,
                ]);
            }
        }
        // Prefer a nearby empty spot in the visible page, then elsewhere on the
        // page. Never move existing objects just to make room for an insertion.
        let position = [visible, page]
            .into_iter()
            .find_map(|area| {
                candidates.iter().copied().find(|p| {
                    let rect = Rect::from_min_size(pos2(p[0], p[1]), vec2(size[0], size[1]));
                    area.contains_rect(rect)
                        && !self.doc.nodes.iter().filter(|n| n.id != id).any(|n| {
                            Rect::from_min_size(
                                pos2(n.position[0], n.position[1]),
                                vec2(n.size[0], n.size[1]),
                            )
                            .expand(12.)
                            .intersects(rect)
                        })
                        && !self.doc.charts.iter().any(|c| {
                            Rect::from_min_size(
                                pos2(c.position[0], c.position[1]),
                                vec2(c.size[0], c.size[1]),
                            )
                            .expand(12.)
                            .intersects(rect)
                        })
                })
            })
            .unwrap_or(initial);
        self.doc.node_mut(id).unwrap().position = position;
        self.selected = vec![id];
        self.set_tool(Tool::Select);
        self.reveal_selection(ctx);
        self.commit();
        self.open_node_text(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activate() {
        i18n::activate(Language::French);
    }

    #[test]
    fn tag_filter_combines_with_text_and_kind() {
        activate();
        let mut doc = Document::default();
        let urgent = doc.add_node(ShapeKind::Rectangle, [100., 100.]);
        doc.node_mut(urgent).unwrap().text = "Livraison".into();
        doc.node_mut(urgent)
            .unwrap()
            .annotations
            .tags
            .push("urgent".into());
        let du = doc.add_node(ShapeKind::Rectangle, [300., 100.]);
        doc.node_mut(du).unwrap().text = "Correctif".into();
        doc.node_mut(du)
            .unwrap()
            .annotations
            .tags
            .push("urgent".into());
        let backlog = doc.add_node(ShapeKind::Diamond, [500., 100.]);
        doc.node_mut(backlog).unwrap().text = "Réflexion".into();
        doc.node_mut(backlog)
            .unwrap()
            .annotations
            .tags
            .push("backlog".into());

        let entries = super::entries(&doc, Scope::Active);
        let mut search = Search {
            entries,
            scope: Scope::Active,
            ..Default::default()
        };
        // Sans filtre d'étiquette, le texte filtre comme avant.
        search.query = "Livraison".into();
        let hits = search.matches();
        assert_eq!(hits.len(), 1);
        assert_eq!(search.entries[hits[0]].id, urgent);
        // Filtre d'étiquette seul.
        search.query.clear();
        search.tag = Some(commands_ui::normalize("urgent"));
        let hits = search.matches();
        let ids: Vec<_> = hits.iter().map(|&i| search.entries[i].id).collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&urgent) && ids.contains(&du));
        // Filtre combiné étiquette + texte.
        search.query = "correctif".into();
        let hits = search.matches();
        assert_eq!(hits.len(), 1);
        assert_eq!(search.entries[hits[0]].id, du);
        // Désactiver le filtre (None) retrouve tous les objets.
        search.tag = None;
        search.query.clear();
        assert_eq!(search.matches().len(), 3);
    }

    #[test]
    fn tag_list_collects_distinct_normalized_tags() {
        activate();
        let mut doc = Document::default();
        let a = doc.add_node(ShapeKind::Rectangle, [100., 100.]);
        doc.node_mut(a)
            .unwrap()
            .annotations
            .tags
            .push("Urgent".into());
        doc.node_mut(a)
            .unwrap()
            .annotations
            .tags
            .push("Plan".into());
        let b = doc.add_node(ShapeKind::Rectangle, [300., 100.]);
        doc.node_mut(b)
            .unwrap()
            .annotations
            .tags
            .push("URGENT".into());
        let entries = super::entries(&doc, Scope::Active);
        let list = super::tag_list(&entries);
        assert!(list.contains(&"urgent".to_string()));
        assert!(list.contains(&"plan".to_string()));
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn all_pages_scope_finds_objects_on_other_pages() {
        activate();
        let mut doc = Document::default();
        let local = doc.add_node(ShapeKind::Rectangle, [100., 100.]);
        doc.node_mut(local).unwrap().text = "Local".into();
        doc.add_page(false).unwrap();
        let remote = doc.add_node(ShapeKind::Rectangle, [200., 200.]);
        doc.node_mut(remote).unwrap().text = "Cible distante".into();
        doc.switch_page(0).unwrap();

        // Portée active : uniquement la page courante.
        let active = super::entries(&doc, Scope::Active);
        assert!(active.iter().all(|e| e.page == 0));
        assert!(active.iter().all(|e| !e.search.contains("distante")));

        // Portée toutes les pages : l'objet distant est retrouvé, page 1.
        let all = super::entries(&doc, Scope::All);
        let hit = all
            .iter()
            .find(|e| e.page == 1 && e.search.contains("distante"))
            .expect("objet distant trouvé");
        assert_eq!(hit.id, remote);
        assert_eq!(hit.page, 1);
        assert!(hit.detail.starts_with("Page 2 ·"));

        let mut search = Search {
            entries: all,
            scope: Scope::All,
            ..Default::default()
        };
        search.query = "distante".into();
        let hits = search.matches();
        assert_eq!(hits.len(), 1);
        assert_eq!(search.entries[hits[0]].id, remote);
        assert_eq!(search.entries[hits[0]].page, 1);
    }

    #[test]
    fn results_are_bounded_and_report_truncation() {
        activate();
        let mut doc = Document::default();
        for _ in 0..310 {
            doc.add_node(ShapeKind::Rectangle, [100., 100.]);
        }
        let entries = super::entries(&doc, Scope::Active);
        assert_eq!(entries.len(), MAX_PAGE_ENTRIES);
    }
}
