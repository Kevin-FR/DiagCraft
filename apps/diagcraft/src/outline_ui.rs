//! Page-local navigator. All state is transient; never mutates organization.
use super::*;
use std::collections::{BTreeMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum TreeKey {
    Layer(u64),
    Group(u64),
    Object(u64),
}
struct Row {
    key: TreeKey,
    depth: usize,
    label: String,
    detail: String,
    search: String,
    ancestors: Vec<TreeKey>,
}
#[derive(Default)]
pub(super) struct State {
    query: String,
    collapsed: HashSet<TreeKey>,
    context: Option<(usize, usize)>,
    focused: Option<(TreeKey, egui::Id)>,
    pending_focus: Option<TreeKey>,
}
fn short(text: &str) -> String {
    text.chars()
        .take(80)
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_owned()
}
fn rows(doc: &Document) -> Vec<Row> {
    let view = doc.organization_view();
    let mut buckets: BTreeMap<u64, BTreeMap<Option<u64>, Vec<Row>>> = BTreeMap::new();
    let objects = doc
        .nodes
        .iter()
        .map(|n| {
            (
                n.id,
                if let Some(image) = &n.image {
                    short(&image.alt)
                } else {
                    short(&n.text)
                },
                if n.image.is_some() {
                    t("Image")
                } else {
                    t(n.kind.label())
                },
            )
        })
        .chain(
            doc.connectors
                .iter()
                .map(|c| (c.id, short(&c.label), t("Connecteur"))),
        )
        .chain(doc.drawings.iter().map(|d| {
            (
                d.id,
                String::new(),
                t(if d.kind == DrawingKind::Line {
                    "Trait"
                } else {
                    "Courbe de Bézier"
                }),
            )
        }))
        .chain(
            doc.charts
                .iter()
                .map(|c| (c.id, short(&c.title), t(c.kind.label()))),
        );
    for (id, mut label, kind) in objects {
        if label.is_empty() {
            label = kind.clone();
        }
        let mut detail = format!("{kind} · #{id}");
        let visible = view.is_object_visible(id);
        if !visible {
            detail.push_str(&format!(" · {}", t("Masqué")));
        }
        if view.is_object_locked(id) {
            detail.push_str(&format!(" · {}", t("Verrouillé")));
        }
        let layer = view.object_layer(id);
        let group = view.group_for_object(id);
        let mut ancestors = vec![TreeKey::Layer(layer)];
        if let Some(group) = group {
            ancestors.push(TreeKey::Group(group));
        }
        buckets
            .entry(layer)
            .or_default()
            .entry(group)
            .or_default()
            .push(Row {
                key: TreeKey::Object(id),
                depth: if group.is_some() { 2 } else { 1 },
                search: commands_ui::normalize(&format!("{label} {detail}")),
                label,
                detail,
                ancestors,
            });
    }
    let mut out = vec![];
    for layer in doc.layer_ids().into_iter().rev() {
        let info = doc.organization.layers.iter().find(|l| l.id == layer);
        let name = info.map_or_else(|| t("Calque de base"), |l| short(&l.name));
        let mut detail = t("Calque");
        if info.is_some_and(|l| !l.visible) {
            detail.push_str(&format!(" · {}", t("Masqué")));
        }
        if info.is_some_and(|l| l.locked) {
            detail.push_str(&format!(" · {}", t("Verrouillé")));
        }
        out.push(Row {
            key: TreeKey::Layer(layer),
            depth: 0,
            search: commands_ui::normalize(&format!("{name} {detail}")),
            label: name,
            detail,
            ancestors: vec![],
        });
        if let Some(mut groups) = buckets.remove(&layer) {
            let ungrouped = groups.remove(&None);
            for (group, objects) in groups {
                if let Some(id) = group {
                    let label = format!("{} #{id}", t("Groupe"));
                    out.push(Row {
                        key: TreeKey::Group(id),
                        depth: 1,
                        search: commands_ui::normalize(&label),
                        label,
                        detail: format!("{} {}", objects.len(), t("Objets")),
                        ancestors: vec![TreeKey::Layer(layer)],
                    });
                }
                out.extend(objects);
            }
            if let Some(objects) = ungrouped {
                out.extend(objects);
            }
        }
    }
    out
}
fn shown_rows(rows: &[Row], state: &State) -> Vec<usize> {
    let words = commands_ui::normalize(&state.query)
        .split_whitespace()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if words.is_empty() {
        return rows
            .iter()
            .enumerate()
            .filter_map(|(i, r)| {
                (!r.ancestors.iter().any(|a| state.collapsed.contains(a))).then_some(i)
            })
            .collect();
    }
    let matching: HashSet<_> = rows
        .iter()
        .filter(|r| words.iter().all(|w| r.search.contains(w)))
        .map(|r| r.key)
        .collect();
    let parents: HashSet<_> = rows
        .iter()
        .filter(|r| matching.contains(&r.key))
        .flat_map(|r| r.ancestors.iter().copied())
        .collect();
    rows.iter()
        .enumerate()
        .filter_map(|(i, r)| {
            (matching.contains(&r.key)
                || parents.contains(&r.key)
                || r.ancestors.iter().any(|a| matching.contains(a)))
            .then_some(i)
        })
        .collect()
}
impl DrawApp {
    fn outline_choose(&mut self, id: u64, additive: bool, ctx: &egui::Context) {
        if !self.doc.organization_view().is_object_visible(id) {
            self.status = t("Affichez le calque pour sélectionner cet objet.");
            return;
        }
        let mut next = if additive {
            self.selected.clone()
        } else {
            vec![]
        };
        let mut seen: HashSet<_> = next.iter().copied().collect();
        for candidate in self.doc.expand_groups(&[id]) {
            if seen.insert(candidate) {
                next.push(candidate);
            }
        }
        self.set_tool(Tool::Select);
        self.selected = next;
        self.normalize_selection();
        self.reveal_selection(ctx);
    }
    pub(super) fn outline_panel(&mut self, ui: &mut egui::Ui) {
        let context = (self.active_tab, self.doc.page_index);
        if self.outline_ui.context != Some(context) {
            self.outline_ui.collapsed.clear();
            self.outline_ui.focused = None;
            self.outline_ui.pending_focus = None;
            self.outline_ui.context = Some(context);
        }
        let mut page = None;
        egui::ComboBox::from_id_salt("outline_pages")
            .selected_text(short(&self.doc.page_name))
            .truncate()
            .width(ui.available_width())
            .show_ui(ui, |ui| {
                for (i, name) in self.doc.page_names().into_iter().enumerate() {
                    if ui
                        .selectable_label(
                            i == self.doc.page_index,
                            format!("{} · {}", i + 1, short(name)),
                        )
                        .clicked()
                    {
                        page = Some(i);
                    }
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut self.outline_ui.query)
                .hint_text(t("Texte, type ou #identifiant…"))
                .char_limit(120)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal_wrapped(|ui| {
            if ui.small_button(t("Tout développer")).clicked() {
                self.outline_ui.collapsed.clear();
            }
            if ui.small_button(t("Calques…")).clicked() {
                self.show_layers = true;
            }
        });
        ui.label(
            RichText::new(t("Cliquez pour sélectionner · Maj : ajouter"))
                .small()
                .color(INFO_TEXT),
        );
        let rows = rows(&self.doc);
        let mut shown = shown_rows(&rows, &self.outline_ui);
        if let Some((key, id)) = self.outline_ui.focused
            && ui.memory(|m| m.focused()) == Some(id)
            && let Some(index) = shown.iter().position(|i| rows[*i].key == key)
        {
            let up = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp));
            let down = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown));
            let left = ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowLeft));
            let right =
                ui.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::ArrowRight));
            if up || down || left || right {
                ui.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
            }
            if up || down {
                let next = if up {
                    index.saturating_sub(1)
                } else {
                    (index + 1).min(shown.len() - 1)
                };
                self.outline_ui.pending_focus = Some(rows[shown[next]].key);
            }
            if left {
                if matches!(key, TreeKey::Object(_)) || self.outline_ui.collapsed.contains(&key) {
                    self.outline_ui.pending_focus = rows[shown[index]].ancestors.last().copied();
                } else {
                    self.outline_ui.collapsed.insert(key);
                }
            }
            if right && !matches!(key, TreeKey::Object(_)) {
                self.outline_ui.collapsed.remove(&key);
            }
            shown = shown_rows(&rows, &self.outline_ui);
        }
        let mut chosen = None;
        if shown.is_empty() {
            ui.label(t("Aucun objet trouvé sur cette page."));
        }
        let pending_index = self
            .outline_ui
            .pending_focus
            .and_then(|key| shown.iter().position(|i| rows[*i].key == key));
        let scroll = egui::ScrollArea::vertical()
            .id_salt("outline_rows")
            .auto_shrink([false, false]);
        let scroll = if let Some(index) = pending_index {
            scroll.vertical_scroll_offset(index as f32 * (44. + ui.spacing().item_spacing.y))
        } else {
            scroll
        };
        scroll.show_rows(ui, 44., shown.len(), |ui, range| {
            for index in range {
                let row = &rows[shown[index]];
                ui.push_id((self.active_tab, self.doc.page_index, row.key), |ui| {
                    let selected =
                        matches!(row.key,TreeKey::Object(id) if self.selected.contains(&id));
                    let (rect, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), 44.), Sense::hover());
                    let response = ui.interact(rect, ui.make_persistent_id("row"), Sense::click());
                    let branch = !matches!(row.key, TreeKey::Object(_));
                    let mut name = format!("{} · {}", row.label, row.detail);
                    if branch {
                        name.push_str(&format!(
                            " · {}",
                            t(
                                if self.outline_ui.collapsed.contains(&row.key)
                                    && self.outline_ui.query.trim().is_empty()
                                {
                                    "Replié"
                                } else {
                                    "Développé"
                                }
                            )
                        ));
                    }
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            if branch {
                                egui::WidgetType::Button
                            } else {
                                egui::WidgetType::SelectableLabel
                            },
                            ui.is_enabled(),
                            selected,
                            &name,
                        )
                    });
                    let visuals = ui.style().interact_selectable(&response, selected);
                    ui.painter().rect(
                        response.rect,
                        4,
                        visuals.weak_bg_fill,
                        visuals.bg_stroke,
                        StrokeKind::Inside,
                    );

                    let mut rect = response.rect.shrink2(vec2(6., 3.));
                    rect.min.x += row.depth as f32 * 10.;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                    child.set_clip_rect(rect.intersect(ui.clip_rect()));
                    child.spacing_mut().item_spacing.y = 0.;
                    let prefix = if branch {
                        if self.outline_ui.collapsed.contains(&row.key)
                            && self.outline_ui.query.trim().is_empty()
                        {
                            "▸ "
                        } else {
                            "▾ "
                        }
                    } else {
                        ""
                    };
                    child.add(
                        egui::Label::new(format!("{}{}", prefix, row.label))
                            .truncate()
                            .selectable(false),
                    );
                    child.add(
                        egui::Label::new(RichText::new(&row.detail).small().color(INFO_TEXT))
                            .truncate()
                            .selectable(false),
                    );

                    focus_ui::outline(ui, &response);
                    if self.outline_ui.pending_focus == Some(row.key) {
                        response.request_focus();
                        self.outline_ui.pending_focus = None;
                        self.outline_ui.focused = Some((row.key, response.id));
                    }
                    if response.has_focus() {
                        self.outline_ui.focused = Some((row.key, response.id));
                    }
                    if response.gained_focus() {
                        response.scroll_to_me(None);
                    }
                    if response.clicked() {
                        response.request_focus();
                        self.outline_ui.focused = Some((row.key, response.id));
                    }
                    if response.on_hover_text(name).clicked() {
                        match row.key {
                            TreeKey::Object(id) => {
                                chosen = Some((id, ui.input(|i| i.modifiers.shift)))
                            }
                            TreeKey::Layer(_) | TreeKey::Group(_) => {
                                if !self.outline_ui.collapsed.remove(&row.key) {
                                    self.outline_ui.collapsed.insert(row.key);
                                }
                            }
                        }
                    }
                });
            }
        });
        if let Some(page) = page {
            self.switch_page(page);
        } else if let Some((id, add)) = chosen {
            self.outline_choose(id, add, ui.ctx());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (egui::Context, DrawApp, u64, u64, u64) {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let a = doc.add_node(ShapeKind::Rectangle, [300., 300.]);
        doc.node_mut(a).unwrap().text = "Équipe 中文".into();
        let b = doc.add_node(ShapeKind::Diamond, [700., 300.]);
        let edge = doc.connect(a, b, ConnectorKind::Elbow).unwrap();
        doc.group_objects(&[a, b]).unwrap();
        let hidden = doc.add_node(ShapeKind::Ellipse, [300., 700.]);
        let layer = doc.create_layer("Private".into()).unwrap();
        doc.move_objects_to_layer(&[hidden], layer).unwrap();
        doc.update_layer(layer, None, Some(false), None).unwrap();
        let app = DrawApp::from_context(&ctx, doc, None, None);
        (ctx, app, a, edge, hidden)
    }
    fn render(
        ctx: &egui::Context,
        app: &mut DrawApp,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1200., 680.))),
                events,
                focused: true,
                ..Default::default()
            },
            |ctx| {
                egui::SidePanel::left("outline_test")
                    .exact_width(256.)
                    .show(ctx, |ui| app.outline_panel(ui));
            },
        )
    }
    #[test]
    fn hierarchy_lists_each_object_once_and_search_reveals_folded_ancestors() {
        let (_, app, a, edge, hidden) = fixture();
        let rows = rows(&app.doc);
        let objects: HashSet<_> = rows
            .iter()
            .filter_map(|r| {
                if let TreeKey::Object(id) = r.key {
                    Some(id)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(objects.len(), app.doc.object_count());
        assert!(objects.contains(&edge));
        let row = rows.iter().find(|r| r.key == TreeKey::Object(a)).unwrap();
        assert_eq!(row.ancestors.len(), 2);
        assert!(
            rows.iter()
                .find(|r| r.key == TreeKey::Object(hidden))
                .unwrap()
                .detail
                .contains(&t("Masqué"))
        );
        let mut state = State::default();
        state.collapsed.extend(row.ancestors.iter().copied());
        assert!(
            !shown_rows(&rows, &state)
                .iter()
                .any(|i| rows[*i].key == row.key)
        );
        state.query = "equipe 中文".into();
        let found = shown_rows(&rows, &state);
        assert!(found.iter().any(|i| rows[*i].key == row.key));
        for ancestor in &row.ancestors {
            assert!(found.iter().any(|i| rows[*i].key == *ancestor));
        }
        state.query = format!("#{hidden}");
        assert!(
            shown_rows(&rows, &state)
                .iter()
                .any(|i| rows[*i].key == TreeKey::Object(hidden))
        );
        state.query = "Nothing matching this".into();
        assert!(shown_rows(&rows, &state).is_empty());
    }
    #[test]
    fn selecting_groups_locked_and_hidden_objects_preserves_drawing_and_redo() {
        let (ctx, mut app, a, _, hidden) = fixture();
        let single = app.doc.add_node(ShapeKind::Rectangle, [900., 700.]);
        app.doc.set_objects_locked(&[single], true).unwrap();
        app.history = History::new(&app.doc);
        let before = app.doc.clone();
        app.doc.title = "Redo branch".into();
        app.commit();
        app.undo();
        assert!(app.history.can_redo());
        app.outline_choose(a, false, &ctx);
        assert_eq!(app.selected.len(), 3);
        app.outline_choose(single, true, &ctx);
        assert!(app.selected.contains(&single));
        assert_eq!(app.selected.len(), 4);
        app.outline_choose(single, true, &ctx);
        assert_eq!(app.selected.len(), 4);
        let selected = app.selected.clone();
        app.outline_choose(hidden, false, &ctx);
        assert_eq!(app.selected, selected);
        assert_eq!(app.doc, before);
        assert!(app.history.can_redo());
        assert!(!app.history.can_undo());
    }
    #[test]
    fn virtual_rows_can_be_reached_and_selected_with_arrow_keys() {
        let ctx = egui::Context::default();
        let mut doc = Document::default();
        let ids: Vec<_> = (0..2000)
            .map(|i| {
                let id = doc.add_node(ShapeKind::Rectangle, [300., 300.]);
                doc.node_mut(id).unwrap().text = format!("Object {i}");
                id
            })
            .collect();
        let mut app = DrawApp::from_context(&ctx, doc.clone(), None, None);
        for _ in 0..3 {
            render(&ctx, &mut app, vec![]);
        }
        app.outline_ui.pending_focus = Some(TreeKey::Object(ids[1999]));
        for _ in 0..2 {
            render(&ctx, &mut app, vec![]);
        }
        let (key, id) = app.outline_ui.focused.unwrap();
        assert_eq!(key, TreeKey::Object(ids[1999]));
        assert_eq!(ctx.memory(|m| m.focused()), Some(id));
        let event = |key, pressed| egui::Event::Key {
            key,
            physical_key: None,
            pressed,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        render(&ctx, &mut app, vec![event(egui::Key::ArrowUp, true)]);
        render(&ctx, &mut app, vec![event(egui::Key::ArrowUp, false)]);
        assert_eq!(
            app.outline_ui.focused.unwrap().0,
            TreeKey::Object(ids[1998])
        );
        render(&ctx, &mut app, vec![event(egui::Key::Enter, true)]);
        render(&ctx, &mut app, vec![event(egui::Key::Enter, false)]);
        assert_eq!(app.selected, vec![ids[1998]]);
        assert_eq!(app.doc, doc);
        assert!(!app.history.can_undo());
    }
    #[test]
    fn seven_language_rows_expose_names_and_page_changes_refresh_the_same_id() {
        for language in Language::ALL {
            let (ctx, mut app, a, _, _) = fixture();
            ctx.enable_accesskit();
            app.set_language(Some(language));
            let before = app.doc.clone();
            let mut output = render(&ctx, &mut app, vec![]);
            for _ in 0..2 {
                output = render(&ctx, &mut app, vec![]);
            }
            let tree = output.platform_output.accesskit_update.unwrap();
            assert!(tree.nodes.iter().any(|(_, n)| {
                n.label()
                    .is_some_and(|s| s.contains("Équipe 中文") && s.contains(&format!("#{a}")))
            }));
            assert_eq!(app.doc, before);
            app.doc.add_page(false).unwrap();
            let new = app.doc.add_node(ShapeKind::Rectangle, [300., 300.]);
            app.doc.node_mut(new).unwrap().text = "New page object".into();
            render(&ctx, &mut app, vec![]);
            let rows = rows(&app.doc);
            assert!(
                rows.iter()
                    .any(|r| r.key == TreeKey::Object(new) && r.label == "New page object")
            );
            assert!(!rows.iter().any(|r| r.label == "Équipe 中文"));
        }
        i18n::activate(Language::French);
    }
    #[test]
    fn outline_queries_are_not_saved_in_preferences() {
        let (_, mut app, _, _, _) = fixture();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("interface.json");
        app.interface_path = Some(path.clone());
        app.library_panel = LibraryPanel::Outline;
        app.outline_ui.query = "Private query".into();
        app.save_interface().unwrap();
        let saved = std::fs::read_to_string(path).unwrap();
        assert!(!saved.contains("Private query"));
        assert!(!saved.contains("Outline"));
        assert!(saved.contains("Symbols"));
    }
    #[test]
    fn branches_collapse_and_expand_with_keyboard_without_edits() {
        let (ctx, mut app, a, _, _) = fixture();
        let before = app.doc.clone();
        let group = app.doc.group_for_object(a).unwrap().id;
        for _ in 0..3 {
            render(&ctx, &mut app, vec![]);
        }
        app.outline_ui.pending_focus = Some(TreeKey::Group(group));
        for _ in 0..2 {
            render(&ctx, &mut app, vec![]);
        }
        for (key, collapsed) in [(egui::Key::ArrowLeft, true), (egui::Key::ArrowRight, false)] {
            for pressed in [true, false] {
                render(
                    &ctx,
                    &mut app,
                    vec![egui::Event::Key {
                        key,
                        physical_key: None,
                        pressed,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                );
            }
            assert_eq!(
                app.outline_ui.collapsed.contains(&TreeKey::Group(group)),
                collapsed
            );
        }
        assert_eq!(app.doc, before);
        assert!(!app.history.can_undo());
    }
}
