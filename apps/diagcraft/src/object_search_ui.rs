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
pub(super) struct Entry {
    pub id: u64,
    pub kind: Kind,
    pub label: String,
    pub detail: String,
    search: String,
}
#[derive(Default)]
pub(super) struct Search {
    pub open: bool,
    pub query: String,
    pub kind: Kind,
    pub selected: usize,
    pub entries: Vec<Entry>,
    pub rect: Option<Rect>,
    pub close_id: Option<egui::Id>,
    focus: bool,
    page: usize,
    tab: usize,
    revision: String,
}
impl Search {
    pub fn matches(&self) -> Vec<usize> {
        let query = commands_ui::normalize(&self.query);
        let words = query.split_whitespace().collect::<Vec<_>>();
        self.entries
            .iter()
            .enumerate()
            .filter_map(|(i, e)| {
                ((self.kind == Kind::All || self.kind == e.kind)
                    && words.iter().all(|word| e.search.contains(word)))
                .then_some(i)
            })
            .collect()
    }
}

fn entry(id: u64, kind: Kind, text: &str, label: &str, position: Point) -> Entry {
    let title = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or(label);
    let mut short = title.chars().take(80).collect::<String>();
    if title.chars().count() > 80 {
        short.push('…');
    }
    Entry {
        id,
        kind,
        label: short,
        detail: format!(
            "{} · #{} · X {:.0} · Y {:.0} px",
            label, id, position[0], position[1]
        ),
        search: commands_ui::normalize(&format!("{text} {label} {} #{id}", t(kind.label()))),
    }
}
fn entries(doc: &Document) -> Vec<Entry> {
    let view = doc.organization_view();
    doc.nodes
        .iter()
        .map(|n| {
            if let Some(image) = &n.image {
                entry(n.id, Kind::Images, &image.alt, &t("Image"), n.position)
            } else {
                entry(n.id, Kind::Nodes, &n.text, &t(n.kind.label()), n.position)
            }
        })
        .chain(doc.connectors.iter().map(|e| {
            entry(
                e.id,
                Kind::Links,
                &e.label,
                &t("Connecteur"),
                doc.route(e).first().copied().unwrap_or([0., 0.]),
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
            )
        }))
        .chain(
            doc.charts
                .iter()
                .map(|c| entry(c.id, Kind::Charts, &c.title, &t(c.kind.label()), c.position)),
        )
        .filter(|e| view.is_object_visible(e.id))
        .collect()
}

#[derive(Clone, Copy)]
enum Action {
    Select(u64, bool),
    Edit(u64),
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
        self.object_search = Search {
            open: true,
            focus: true,
            page: self.doc.page_index,
            tab: self.active_tab,
            entries: entries(&self.doc),
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
        let (id, add) = match action {
            Action::Select(id, add) => (id, add),
            Action::Edit(id) => (id, false),
        };
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
        if matches!(action, Action::Edit(_)) {
            self.open_node_text(id);
        }
    }
    pub(super) fn object_search_dialog(&mut self, ctx: &egui::Context) {
        if !self.object_search.open {
            return;
        }
        if self.object_search.tab != self.active_tab
            || self.object_search.page != self.doc.page_index
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
                            action = Some(Action::Select(e.id, ctx.input(|i| i.modifiers.shift)));
                        }
                    });
                }
            });
            if found.is_empty() {
                ui.label(t("Aucun objet trouvé sur cette page."));
                ui.label(t("Essayez un autre texte, un type ou un identifiant."));
            }
            ui.separator();
            let active = found.get(search.selected).map(|i| &search.entries[*i]);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add_enabled(active.is_some(), egui::Button::new(t("Sélectionner")))
                    .clicked()
                {
                    action = active.map(|e| Action::Select(e.id, false));
                }
                if ui
                    .add_enabled(
                        active.is_some(),
                        egui::Button::new(t("Ajouter à la sélection")),
                    )
                    .clicked()
                {
                    action = active.map(|e| Action::Select(e.id, true));
                }
                let node = active.filter(|e| e.kind == Kind::Nodes);
                if ui
                    .add_enabled(node.is_some(), egui::Button::new(t("Modifier le texte")))
                    .clicked()
                {
                    action = node.map(|e| Action::Edit(e.id));
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
                    action = Some(Action::Select(e.id, add));
                } else if edit && e.kind == Kind::Nodes {
                    action = Some(Action::Edit(e.id));
                }
            }
        });
        self.object_search.rect = Some(modal.response.rect);
        if action.is_some() {
            // Check the snapshot only on activation, avoiding serialization and
            // hashing of a large drawing on every navigation frame.
            let revision = diagcraft::agent::revision(&self.doc);
            if revision != self.object_search.revision {
                self.object_search.entries = entries(&self.doc);
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
