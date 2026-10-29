use super::*;
use diagcraft::rich_text::{InlineStyle, ResolvedStyle};
use egui::text::{CCursor, CCursorRange, LayoutJob};

#[derive(Default)]
struct Finish {
    commit: bool,
    replay: Vec<egui::Event>,
    save: Option<bool>,
    close: bool,
    open: Option<PathBuf>,
}

pub(super) struct Editor {
    pub draft: Node,
    revision: String,
    page: usize,
    selection: CCursorRange,
    focus: bool,
    undo: Vec<Node>,
    redo: Vec<Node>,
    error: Option<String>,
    click: Option<Pos2>,
    reset_scroll: bool,
    finish: Option<Finish>,
    pub(super) node_rect: Rect,
    pub(super) content_rect: Rect,
    pub(super) toolbar_rect: Rect,
}
impl Editor {
    fn record(&mut self, before: Node) {
        if before != self.draft {
            if self.undo.len() >= 64 {
                self.undo.remove(0);
            }
            self.undo.push(before);
            self.redo.clear();
        }
    }
    fn undo(&mut self, redo: bool) {
        let stack = if redo { &mut self.redo } else { &mut self.undo };
        if let Some(previous) = stack.pop() {
            let current = std::mem::replace(&mut self.draft, previous);
            if redo {
                self.undo.push(current);
            } else {
                self.redo.push(current);
            }
            let count = self.draft.text.chars().count();
            self.selection.primary.index = self.selection.primary.index.min(count);
            self.selection.secondary.index = self.selection.secondary.index.min(count);
            self.focus = true;
        }
    }
    fn selected_style(&self) -> ResolvedStyle {
        let range = self.selection.as_sorted_char_range();
        let mut result = self.draft.resolved_style_at(range.start);
        for i in range.skip(1) {
            let other = self.draft.resolved_style_at(i).text_style;
            result.text_style.bold &= other.bold;
            result.text_style.italic &= other.italic;
            result.text_style.underline &= other.underline;
            result.text_style.strikethrough &= other.strikethrough;
            result.text_style.monospace &= other.monospace;
        }
        result
    }
    fn format(&mut self, style: Option<&InlineStyle>) {
        let before = self.draft.clone();
        if let Err(e) = self
            .draft
            .format_text(self.selection.as_sorted_char_range(), style)
        {
            self.error = Some(e);
        }
        self.record(before);
    }
}

fn text_format(
    style: &ResolvedStyle,
    zoom: f32,
    line_height: Option<f32>,
    bold_pass: bool,
) -> egui::TextFormat {
    let family = if style.text_style.monospace {
        "DiagCraftMono"
    } else {
        font_name(&style.text_style)
    };
    let color = if bold_pass && !(style.text_style.monospace && style.text_style.bold) {
        Color32::TRANSPARENT
    } else {
        rgb(style.color)
    };
    egui::TextFormat {
        font_id: FontId::new(
            style.font_size * zoom,
            egui::FontFamily::Name(family.into()),
        ),
        line_height: line_height.map(|h| h * zoom),
        color,
        italics: style.text_style.monospace && style.text_style.italic,
        underline: if style.text_style.underline && !bold_pass {
            Stroke::new(zoom.max(1.), color)
        } else {
            Stroke::NONE
        },
        strikethrough: if style.text_style.strikethrough && !bold_pass {
            Stroke::new(zoom.max(1.), color)
        } else {
            Stroke::NONE
        },
        ..Default::default()
    }
}

pub(super) fn document_job(node: &Node, zoom: f32, bold_pass: bool) -> LayoutJob {
    let text = node.text_layout();
    let mut job = LayoutJob::default();
    for (i, line) in text.runs.iter().enumerate() {
        if i > 0 {
            job.append(
                "\n",
                0.,
                text_format(
                    &node.resolved_style_at(0),
                    zoom,
                    Some(text.line_height),
                    bold_pass,
                ),
            );
        }
        for run in line {
            job.append(
                &run.text,
                0.,
                text_format(&run.style, zoom, Some(text.line_height), bold_pass),
            );
        }
    }
    if job.sections.is_empty() {
        job = LayoutJob::simple_format(
            String::new(),
            text_format(
                &node.resolved_style_at(0),
                zoom,
                Some(text.line_height),
                bold_pass,
            ),
        );
    }
    job.halign = match node.text_style.align {
        TextAlign::Left => Align::LEFT,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::RIGHT,
    };
    job
}

fn editor_job(node: &Node, width: f32, zoom: f32, inline: bool) -> LayoutJob {
    let mut job = LayoutJob::default();
    // Keep exactly the input string: TextEdit cursor offsets refer to its characters.
    let line_height = inline.then(|| node.text_layout().line_height);
    let mut index = 0;
    let chars: Vec<_> = node.text.char_indices().collect();
    while index < chars.len() {
        let style = node.resolved_style_at(index);
        let mut end = index + 1;
        while end < chars.len() && node.resolved_style_at(end) == style {
            end += 1;
        }
        let byte_end = chars.get(end).map(|c| c.0).unwrap_or(node.text.len());
        job.append(
            &node.text[chars[index].0..byte_end],
            0.,
            text_format(&style, zoom, line_height, false),
        );
        index = end;
    }
    if job.sections.is_empty() {
        job = LayoutJob::simple_format(
            String::new(),
            text_format(&node.resolved_style_at(0), zoom, line_height, false),
        );
    }
    job.wrap.max_width = if inline && !node.text_style.wrap {
        f32::INFINITY
    } else {
        width
    };
    if inline {
        job.halign = text_align(node.text_style.align);
    }
    job
}

fn text_align(align: TextAlign) -> Align {
    match align {
        TextAlign::Left => Align::LEFT,
        TextAlign::Center => Align::Center,
        TextAlign::Right => Align::RIGHT,
    }
}

// TextEdit expects nonnegative galley coordinates. Normalize the public row
// positions and bounds together so centered/right-aligned rows retain accurate
// caret, selection and IME coordinates, without changing the source string.
fn inline_galley(
    ui: &egui::Ui,
    node: &Node,
    width: f32,
    zoom: f32,
) -> std::sync::Arc<egui::Galley> {
    let mut galley = ui.fonts_mut(|f| f.layout_job(editor_job(node, width, zoom, true)));
    let shift = -galley.rect.min.x;
    if shift != 0. {
        let g = std::sync::Arc::make_mut(&mut galley);
        for row in &mut g.rows {
            row.pos.x += shift;
        }
        g.rect = g.rect.translate(vec2(shift, 0.));
        g.mesh_bounds = g.mesh_bounds.translate(vec2(shift, 0.));
    }
    galley
}

pub(super) fn text_input(
    ui: &mut egui::Ui,
    node: &mut Node,
    id: egui::Id,
    rows: usize,
) -> egui::text_edit::TextEditOutput {
    let original = node.clone();
    let mut buffer = node.text.clone();
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
        let mut preview = original.clone();
        if preview.set_text(text.as_str().to_owned()).is_err() {
            preview.text = text.as_str().to_owned();
            preview.text_runs.clear();
        }
        ui.fonts_mut(|f| f.layout_job(editor_job(&preview, width, 1., false)))
    };
    let output = egui::TextEdit::multiline(&mut buffer)
        .id(id)
        .desired_width(f32::INFINITY)
        .desired_rows(rows)
        .char_limit(65_536)
        .layouter(&mut layouter)
        .show(ui);
    if output.response.changed()
        && let Err(error) = node.set_text(buffer)
    {
        ui.colored_label(ERROR_TEXT, error);
    }
    output
}

impl DrawApp {
    pub(super) fn open_node_text(&mut self, id: u64) {
        self.open_node_text_at(id, None);
    }

    pub(super) fn open_node_text_at(&mut self, id: u64, click: Option<Pos2>) {
        if self.rich_editor.is_some() {
            return;
        }
        if !self.doc.can_edit_object(id) {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        self.commit();
        if let Some(draft) = self.doc.node(id).cloned() {
            let count = draft.text.chars().count();
            let node_rect = self.node_screen_rect(&draft);
            self.rich_editor = Some(Editor {
                draft,
                revision: diagcraft::agent::revision(&self.doc),
                page: self.doc.page_index,
                selection: CCursorRange::two(CCursor::new(0), CCursor::new(count)),
                focus: true,
                undo: vec![],
                redo: vec![],
                error: None,
                click,
                reset_scroll: true,
                finish: None,
                node_rect,
                content_rect: Rect::NOTHING,
                toolbar_rect: Rect::NOTHING,
            });
            self.context_open = false;
            self.focus_text = None;
            self.clear_connection();
        }
    }

    pub(super) fn apply_node_text(&mut self) -> Result<(), String> {
        let Some(editor) = self.rich_editor.as_ref() else {
            return Ok(());
        };
        if editor.page != self.doc.page_index
            || editor.revision != diagcraft::agent::revision(&self.doc)
        {
            return Err("Le document a changé. Fermez l’éditeur puis rouvrez le bloc.".into());
        }
        if !self.doc.can_edit_object(editor.draft.id) {
            return Err(t("La sélection est verrouillée"));
        }
        let mut draft = editor.draft.clone();
        draft.validate_text_runs()?;
        if draft.text_style.auto_fit {
            draft.fit_to_text()?;
        }
        let mut result = self.doc.clone();
        let id = draft.id;
        *result.node_mut(id).ok_or("Bloc introuvable.")? = draft;
        result.validate()?;
        self.doc = result;
        self.rich_editor = None;
        self.commit();
        self.status = "Contenu du bloc mis à jour".into();
        Ok(())
    }

    pub(super) fn finish_inline_text(&mut self, ctx: &egui::Context, cancel: bool) -> bool {
        if self.rich_editor.is_none() {
            return true;
        }
        if cancel {
            self.rich_editor = None;
        } else if let Err(error) = self.apply_node_text() {
            if let Some(editor) = &mut self.rich_editor {
                editor.error = Some(error);
            }
            return false;
        }
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("rich_text_input")));
        true
    }

    pub(super) fn inline_outside_press(&mut self, ctx: &egui::Context) {
        let Some(editor) = &self.rich_editor else {
            return;
        };
        if ctx.input(|i| i.key_pressed(Key::Escape) && i.modifiers.is_none()) {
            return;
        }
        if egui::Popup::is_any_open(ctx) {
            return;
        }
        let press = ctx.input(|i| {
            i.pointer
                .any_pressed()
                .then(|| i.pointer.interact_pos())
                .flatten()
        });
        let outside = press
            .is_some_and(|p| !editor.node_rect.contains(p) && !editor.toolbar_rect.contains(p));
        if press.is_some()
            && ctx.input(|i| {
                i.events.iter().any(|e| {
                    matches!(
                        e,
                        egui::Event::Text(_)
                            | egui::Event::Paste(_)
                            | egui::Event::Cut
                            | egui::Event::Ime(_)
                            | egui::Event::Key { pressed: true, .. }
                    )
                })
            })
        {
            let replay = ctx.input(|i| {
                i.events
                    .iter()
                    .filter(|e| {
                        matches!(
                            e,
                            egui::Event::PointerMoved(_)
                                | egui::Event::PointerButton { .. }
                                | egui::Event::PointerGone
                        )
                    })
                    .cloned()
                    .collect()
            });
            let finish = self
                .rich_editor
                .as_mut()
                .unwrap()
                .finish
                .get_or_insert_with(Default::default);
            finish.commit |= outside;
            finish.replay = replay;
            ctx.input_mut(|i| {
                i.pointer = Default::default();
                i.events.retain(|e| {
                    !matches!(
                        e,
                        egui::Event::PointerMoved(_)
                            | egui::Event::PointerButton { .. }
                            | egui::Event::PointerGone
                    )
                });
            });
        } else if outside && !self.finish_inline_text(ctx, false) {
            // An invalid draft must not trigger a document action behind the editor.
            ctx.input_mut(|i| {
                i.pointer = Default::default();
                i.events
                    .retain(|event| !matches!(event, egui::Event::PointerButton { .. }));
            });
        }
    }

    pub(super) fn inline_text_shortcuts(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.rich_editor else {
            return;
        };
        let mut apply = false;
        let mut cancel = false;
        let mut style = InlineStyle::default();
        let mut format = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::Z)
                || i.consume_key(egui::Modifiers::COMMAND, Key::Y)
            {
                editor.undo(true);
            } else if i.consume_key(egui::Modifiers::COMMAND, Key::Z) {
                editor.undo(false);
            }
            apply = i.consume_key(egui::Modifiers::COMMAND, Key::Enter);
            cancel = i.consume_key(egui::Modifiers::NONE, Key::Escape);
            if !editor.selection.as_sorted_char_range().is_empty() {
                let current = editor.selected_style();
                if i.consume_key(egui::Modifiers::COMMAND, Key::B) {
                    style.bold = Some(!current.text_style.bold);
                    format = true;
                }
                if i.consume_key(egui::Modifiers::COMMAND, Key::I) {
                    style.italic = Some(!current.text_style.italic);
                    format = true;
                }
                if i.consume_key(egui::Modifiers::COMMAND, Key::U) {
                    style.underline = Some(!current.text_style.underline);
                    format = true;
                }
            }
        });
        if format {
            editor.format(Some(&style));
            editor.focus = true;
        }
        if cancel {
            self.finish_inline_text(ctx, true);
        } else if apply {
            self.rich_editor
                .as_mut()
                .unwrap()
                .finish
                .get_or_insert_with(Default::default)
                .commit = true;
        }
        if let Some(editor) = &mut self.rich_editor
            && ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S))
        {
            let save_as = ctx.input_mut(|i| {
                let shifted = i.modifiers.shift;
                i.consume_key(
                    if shifted {
                        egui::Modifiers::COMMAND | egui::Modifiers::SHIFT
                    } else {
                        egui::Modifiers::COMMAND
                    },
                    Key::S,
                );
                shifted
            });
            let finish = editor.finish.get_or_insert_with(Default::default);
            finish.commit = true;
            finish.save = Some(save_as);
        }
    }

    pub(super) fn inline_preview_document(&self) -> Option<Document> {
        let editor = self.rich_editor.as_ref()?;
        if editor.page != self.doc.page_index {
            return None;
        }
        let mut doc = self.doc.clone();
        let node = doc.node_mut(editor.draft.id)?;
        *node = editor.draft.clone();
        if node.text_style.auto_fit {
            let _ = node.fit_to_text();
        }
        Some(doc)
    }

    pub(super) fn inline_text(&mut self, ui: &mut egui::Ui) {
        let Some(editor) = &self.rich_editor else {
            return;
        };
        let mut preview = editor.draft.clone();
        if preview.text_style.auto_fit {
            let _ = preview.fit_to_text();
        }
        let node_rect = self.node_screen_rect(&preview);
        let (origin, size) = diagcraft::text_layout::area(&preview);
        let content_rect =
            Rect::from_min_size(self.screen(origin), vec2(size[0], size[1]) * self.zoom);
        let zoom = self.zoom;
        let canvas = self.canvas;
        let editor = self.rich_editor.as_mut().unwrap();
        editor.node_rect = node_rect;
        editor.content_rect = content_rect;
        let id = egui::Id::new("rich_text_input");
        let align = text_align(preview.text_style.align);
        if ui.input(|i| {
            i.pointer.primary_pressed()
                && i.pointer
                    .interact_pos()
                    .is_some_and(|p| node_rect.contains(p) && !content_rect.contains(p))
        }) {
            editor.click = ui.input(|i| i.pointer.interact_pos());
            editor.focus = true;
        }
        let focus_requested = editor.focus;
        if editor.focus {
            let galley = inline_galley(ui, &preview, content_rect.width().max(24.), zoom);
            if let Some(click) = editor.click.take() {
                let text_pos = Align2([align, Align::Center])
                    .align_size_within_rect(galley.size(), content_rect)
                    .min;
                editor.selection = CCursorRange::one(galley.cursor_from_pos(click - text_pos));
            }
            let mut state = if editor.reset_scroll {
                Default::default()
            } else {
                egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default()
            };
            state.cursor.set_char_range(Some(editor.selection));
            state.store(ui.ctx(), id);
            ui.memory_mut(|m| m.request_focus(id));
            editor.focus = false;
        }
        let before = editor.draft.clone();
        let mut buffer = editor.draft.text.clone();
        let original = editor.draft.clone();
        let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, width: f32| {
            let mut preview = original.clone();
            if preview.set_text(text.as_str().to_owned()).is_err() {
                preview.text = text.as_str().to_owned();
                preview.text_runs.clear();
            }
            inline_galley(ui, &preview, width, zoom)
        };
        ui.scope_builder(egui::UiBuilder::new().max_rect(content_rect), |ui| {
            ui.set_clip_rect(content_rect.intersect(canvas).expand(1.));
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            let scroll = egui::ScrollArea::both()
                .id_salt(("inline_text_scroll", editor.draft.id))
                .max_width(content_rect.width().max(1.))
                .max_height(content_rect.height().max(1.))
                .min_scrolled_width(1.)
                .min_scrolled_height(1.)
                .auto_shrink([false, false])
                .scroll_source(egui::scroll_area::ScrollSource {
                    drag: false,
                    ..Default::default()
                });
            let scroll = if editor.reset_scroll {
                scroll.scroll_offset(Vec2::ZERO)
            } else {
                scroll
            };
            scroll.show(ui, |ui| {
                let output = egui::TextEdit::multiline(&mut buffer)
                    .id(id)
                    .frame(false)
                    .margin(0.)
                    .font(FontId::proportional(preview.font_size * zoom))
                    .desired_width(content_rect.width().max(24.))
                    .desired_rows(1)
                    .min_size(content_rect.size())
                    .horizontal_align(align)
                    .vertical_align(Align::Center)
                    .char_limit(65_536)
                    .layouter(&mut layouter)
                    .show(ui);
                if let Some(range) = output.cursor_range {
                    editor.selection = range;
                } else if focus_requested {
                    // A dismissed modal can own input for one final frame.
                    // Retry focus and replay unhandled typing so entering text
                    // immediately after a search never drops its first letters.
                    editor.focus = true;
                    self.replay_input.extend(ui.input(|i| {
                        i.events
                            .iter()
                            .filter(|event| {
                                matches!(
                                    event,
                                    egui::Event::Text(_)
                                        | egui::Event::Paste(_)
                                        | egui::Event::Cut
                                        | egui::Event::Ime(_)
                                        | egui::Event::Key { .. }
                                )
                            })
                            .cloned()
                            .collect::<Vec<_>>()
                    }));
                    ui.ctx().request_repaint();
                }
                if output.response.changed() {
                    match editor.draft.set_text(buffer.clone()) {
                        Ok(()) => editor.error = None,
                        Err(error) => editor.error = Some(error),
                    }
                }
            });
        });
        editor.reset_scroll = false;
        editor.record(before);
    }

    fn finalize_inline_frame(&mut self, ctx: &egui::Context) {
        if self.rich_editor.as_ref().is_some_and(|editor| editor.focus) {
            ctx.request_repaint();
            return;
        }
        let finish = self
            .rich_editor
            .as_mut()
            .and_then(|editor| editor.finish.take());
        if let Some(finish) = finish
            && (!finish.commit || self.finish_inline_text(ctx, false))
        {
            if let Some(save_as) = finish.save {
                self.save(save_as);
            }
            if finish.close {
                self.request(Pending::Close);
            } else if let Some(path) = finish.open {
                self.request(Pending::Open(path));
            }
            self.replay_input.extend(finish.replay);
            ctx.request_repaint();
        }
    }

    pub(super) fn defer_inline_close(&mut self) {
        if let Some(editor) = &mut self.rich_editor {
            let finish = editor.finish.get_or_insert_with(Default::default);
            finish.commit = true;
            finish.close = true;
        }
    }

    pub(super) fn inline_open(&mut self, path: PathBuf) {
        if let Some(editor) = &mut self.rich_editor {
            let finish = editor.finish.get_or_insert_with(Default::default);
            finish.commit = true;
            finish.open = Some(path);
        } else {
            self.request(Pending::Open(path));
        }
    }

    pub(super) fn inline_text_toolbar(&mut self, ctx: &egui::Context) {
        let Some(editor) = &self.rich_editor else {
            return;
        };
        let node = editor.node_rect;
        let canvas = self.canvas.shrink(8.);
        let width = canvas.width().clamp(240., 640.);
        let last_height = editor.toolbar_rect.height().max(64.);
        let x = node.center().x - width / 2.;
        let y = if node.top() - last_height - 8. >= canvas.top() {
            node.top() - last_height - 8.
        } else {
            node.bottom() + 8.
        };
        let pos = pos2(
            x.clamp(canvas.left(), (canvas.right() - width).max(canvas.left())),
            y.clamp(
                canvas.top(),
                (canvas.bottom() - last_height).max(canvas.top()),
            ),
        );
        let mut apply = false;
        let mut cancel = false;
        let editor = self.rich_editor.as_mut().unwrap();
        let area = egui::Area::new("inline_text_toolbar".into()).order(egui::Order::Foreground)
            .fixed_pos(pos).constrain_to(canvas).show(ctx, |ui| {
                egui::Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.,BORDER)).corner_radius(8).inner_margin(8).show(ui, |ui| {
                    ui.set_width(width-16.);
                    ui.horizontal_wrapped(|ui| {
                        let current = editor.selected_style();
                        let mut patch = InlineStyle::default();
                        let mut changed = false;
                        ui.add_enabled_ui(!editor.selection.as_sorted_char_range().is_empty(), |ui| {
                            ui.horizontal(|ui| {
                                for (label,tooltip,value,field) in [
                                    (RichText::new("B").strong(),"Gras",current.text_style.bold,&mut patch.bold),
                                    (RichText::new("I").italics(),"Italique",current.text_style.italic,&mut patch.italic),
                                    (RichText::new("U").underline(),"Souligné",current.text_style.underline,&mut patch.underline),
                                ] { let response = ui.selectable_label(value,label).on_hover_text(t(tooltip));
                                    if response.clicked() { *field=Some(!value);changed=true;editor.focus=true; } }
                                let mut size = current.font_size;
                                if ui.add(egui::DragValue::new(&mut size).range(6. ..=200.).suffix(" px")).on_hover_text(t("Taille")).changed() {patch.font_size=Some(size);changed=true;}
                                let mut color = current.color;
                                if ui.color_edit_button_srgb(&mut color).on_hover_text(t("Couleur du texte")).changed() {patch.color=Some(color);changed=true;}
                            });
                        });
                        if changed {editor.format(Some(&patch));}
                        ui.menu_button(t("Plus"), |ui| {
                            if ui.add_enabled(!editor.undo.is_empty(),egui::Button::new(t("Annuler la modification"))).on_hover_text(t("Annuler la modification · Ctrl/⌘ Z")).clicked() {editor.undo(false);ui.close();}
                            if ui.add_enabled(!editor.redo.is_empty(),egui::Button::new(t("Rétablir"))).clicked() {editor.undo(true);ui.close();}
                            if ui.button(t("Tout sélectionner")).clicked() {editor.selection=CCursorRange::two(CCursor::new(0),CCursor::new(editor.draft.text.chars().count()));editor.focus=true;ui.close();}
                            ui.separator();
                            let mut selected = editor.selected_style();
                            let mut patch = InlineStyle::default();
                            let mut changed = false;
                            ui.add_enabled_ui(!editor.selection.as_sorted_char_range().is_empty(), |ui| {
                                for (label,value,field) in [("Barré",&mut selected.text_style.strikethrough,&mut patch.strikethrough),("Monospace",&mut selected.text_style.monospace,&mut patch.monospace)] {
                                    if ui.checkbox(value,t(label)).changed() {*field=Some(*value);changed=true;}
                                }
                                if ui.button(t("Effacer la mise en forme")).clicked() {editor.format(None);editor.focus=true;ui.close();}
                            });
                            if changed {editor.format(Some(&patch));editor.focus=true;}
                            let before = editor.draft.clone();
                            ui.separator();
                            ui.horizontal(|ui| {
                                ui.label(t("Alignement"));
                                for (label,align) in [("Gauche",TextAlign::Left),("Centrer",TextAlign::Center),("Droite",TextAlign::Right)] {ui.selectable_value(&mut editor.draft.text_style.align,align,t(label));}
                            });
                            ui.checkbox(&mut editor.draft.text_style.wrap,t("Retour à la ligne automatique"));
                            if ui.checkbox(&mut editor.draft.text_style.auto_fit,t("Adapter automatiquement au texte")).changed() && editor.draft.text_style.auto_fit {editor.draft.text_style.wrap=true;}
                            editor.record(before);
                        });
                        ui.separator();
                        cancel = ui.button(t("Annuler")).on_hover_text(t("Échap : annuler la saisie")).clicked();
                        apply = ui.button(RichText::new(t("Terminer")).strong()).on_hover_text(t("Cliquer en dehors du bloc ou ⌘Entrée / Ctrl+Entrée pour terminer.")).clicked();
                    });
                    ui.label(RichText::new(t("Écrire dans le bloc · Entrée : nouvelle ligne")).small().color(INFO_TEXT));
                    if let Some(error)=&editor.error {ui.colored_label(ERROR_TEXT,error);}
                });
            });
        editor.toolbar_rect = area.response.rect;
        if cancel || apply {
            self.finish_inline_text(ctx, cancel);
        } else {
            self.finalize_inline_frame(ctx);
        }
    }

    pub(super) fn commit_inline_if_needed(&mut self) -> bool {
        if self.rich_editor.is_none() {
            return true;
        }
        match self.apply_node_text() {
            Ok(()) => true,
            Err(error) => {
                if let Some(editor) = &mut self.rich_editor {
                    editor.error = Some(error);
                }
                false
            }
        }
    }
}
