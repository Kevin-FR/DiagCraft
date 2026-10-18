use super::*;
use diagcraft::rich_text::{InlineStyle, ResolvedStyle};
use egui::text::{CCursor, CCursorRange, LayoutJob};

pub(super) struct Editor {
    pub draft: Node,
    revision: String,
    page: usize,
    selection: CCursorRange,
    focus: bool,
    undo: Vec<Node>,
    redo: Vec<Node>,
    error: Option<String>,
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

fn editor_job(node: &Node, width: f32) -> LayoutJob {
    let mut job = LayoutJob::default();
    // Keep exactly the input string: TextEdit cursor offsets refer to its characters.
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
            text_format(&style, 1., None, false),
        );
        index = end;
    }
    if job.sections.is_empty() {
        job = LayoutJob::simple_format(
            String::new(),
            text_format(&node.resolved_style_at(0), 1., None, false),
        );
    }
    job.wrap.max_width = width;
    job
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
        ui.fonts_mut(|f| f.layout_job(editor_job(&preview, width)))
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
        ui.colored_label(Color32::DARK_RED, error);
    }
    output
}

impl DrawApp {
    pub(super) fn open_node_text(&mut self, id: u64) {
        self.commit();
        if let Some(draft) = self.doc.node(id).cloned() {
            let count = draft.text.chars().count();
            self.rich_editor = Some(Editor {
                draft,
                revision: diagcraft::agent::revision(&self.doc),
                page: self.doc.page_index,
                selection: CCursorRange::two(CCursor::new(0), CCursor::new(count)),
                focus: true,
                undo: vec![],
                redo: vec![],
                error: None,
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

    pub(super) fn rich_text_dialog(&mut self, ctx: &egui::Context) {
        let Some(editor) = &mut self.rich_editor else {
            return;
        };
        let id = egui::Id::new("rich_text_input");
        let mut apply = false;
        let mut cancel = false;
        let mut style = InlineStyle::default();
        let mut format = false;
        ctx.input_mut(|i| {
            if i.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, Key::Z) {
                editor.undo(true);
            } else if i.consume_key(egui::Modifiers::COMMAND, Key::Z) {
                editor.undo(false);
            }
            apply = i.consume_key(egui::Modifiers::COMMAND, Key::Enter);
            cancel = i.consume_key(egui::Modifiers::NONE, Key::Escape);
            let selection = editor.selection.as_sorted_char_range();
            if !selection.is_empty() {
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
        }
        egui::Modal::new("rich_text_editor".into()).show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 100.).clamp(340., 680.));
            ui.heading(t("Contenu du bloc"));
            ui.label(t(
                "Sélectionnez des mots pour modifier leur mise en forme. Entrée ajoute une ligne.",
            ));
            ui.add_space(8.);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        !editor.undo.is_empty(),
                        egui::Button::new(t("Annuler la modification")),
                    )
                    .on_hover_text(t("Annuler la modification · Ctrl/⌘ Z"))
                    .clicked()
                {
                    editor.undo(false);
                }
                if ui
                    .add_enabled(!editor.redo.is_empty(), egui::Button::new(t("Rétablir")))
                    .on_hover_text(t("Rétablir · Ctrl/⌘ Maj Z"))
                    .clicked()
                {
                    editor.undo(true);
                }
                if ui.button(t("Tout sélectionner")).clicked() {
                    editor.selection = CCursorRange::two(
                        CCursor::new(0),
                        CCursor::new(editor.draft.text.chars().count()),
                    );
                    editor.focus = true;
                }
            });
            let selection = editor.selection.as_sorted_char_range();
            let current = editor.selected_style();
            let mut patch = InlineStyle::default();
            let mut changed = false;
            let mut clear = false;
            ui.add_enabled_ui(!selection.is_empty(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    for (label, tooltip, value, field) in [
                        (
                            RichText::new("B").strong(),
                            "Gras",
                            current.text_style.bold,
                            &mut patch.bold,
                        ),
                        (
                            RichText::new("I").italics(),
                            "Italique",
                            current.text_style.italic,
                            &mut patch.italic,
                        ),
                        (
                            RichText::new("U").underline(),
                            "Souligné",
                            current.text_style.underline,
                            &mut patch.underline,
                        ),
                        (
                            RichText::new("S").strikethrough(),
                            "Barré",
                            current.text_style.strikethrough,
                            &mut patch.strikethrough,
                        ),
                        (
                            RichText::new("Aa").monospace(),
                            "Monospace",
                            current.text_style.monospace,
                            &mut patch.monospace,
                        ),
                    ] {
                        if ui
                            .selectable_label(value, label)
                            .on_hover_text(t(tooltip))
                            .clicked()
                        {
                            *field = Some(!value);
                            changed = true;
                        }
                    }
                    ui.separator();
                    let mut size = current.font_size;
                    if ui
                        .add(
                            egui::DragValue::new(&mut size)
                                .range(6. ..=200.)
                                .speed(1.)
                                .suffix(" px"),
                        )
                        .on_hover_text(t("Taille"))
                        .changed()
                    {
                        patch.font_size = Some(size);
                        changed = true;
                    }
                    let mut color = current.color;
                    if ui
                        .color_edit_button_srgb(&mut color)
                        .on_hover_text(t("Couleur du texte"))
                        .changed()
                    {
                        patch.color = Some(color);
                        changed = true;
                    }
                    clear = ui.button(t("Effacer la mise en forme")).clicked();
                });
            });
            if changed {
                editor.format(Some(&patch));
            }
            if clear {
                editor.format(None);
            }
            if editor.focus || changed || clear {
                let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
                state.cursor.set_char_range(Some(editor.selection));
                state.store(ctx, id);
            }
            ui.add_space(6.);
            let before_edit = editor.draft.clone();
            let output = egui::ScrollArea::vertical()
                .id_salt("rich_text_scroll")
                .max_height(280.)
                .show(ui, |ui| text_input(ui, &mut editor.draft, id, 7))
                .inner;
            if let Some(range) = output.cursor_range {
                editor.selection = range;
            }
            if editor.focus || changed || clear {
                output.response.request_focus();
                editor.focus = false;
            }
            ui.horizontal(|ui| {
                ui.label(t("Alignement"));
                ui.selectable_value(
                    &mut editor.draft.text_style.align,
                    TextAlign::Left,
                    t("Gauche"),
                );
                ui.selectable_value(
                    &mut editor.draft.text_style.align,
                    TextAlign::Center,
                    t("Centrer"),
                );
                ui.selectable_value(
                    &mut editor.draft.text_style.align,
                    TextAlign::Right,
                    t("Droite"),
                );
            });
            ui.checkbox(
                &mut editor.draft.text_style.wrap,
                t("Retour à la ligne automatique"),
            );
            if ui
                .checkbox(
                    &mut editor.draft.text_style.auto_fit,
                    t("Adapter automatiquement au texte"),
                )
                .changed()
                && editor.draft.text_style.auto_fit
            {
                editor.draft.text_style.wrap = true;
            }
            editor.record(before_edit);
            if let Some(error) = &editor.error {
                ui.colored_label(Color32::DARK_RED, error);
            }
            ui.separator();
            ui.horizontal(|ui| {
                cancel |= ui.button(t("Annuler")).clicked();
                apply |= ui
                    .button(RichText::new(t("Appliquer")).strong())
                    .on_hover_text(t("Appliquer · ⌘Entrée / Ctrl+Entrée"))
                    .clicked();
            });
        });
        if cancel {
            self.rich_editor = None;
        } else if apply
            && let Err(error) = self.apply_node_text()
            && let Some(editor) = &mut self.rich_editor
        {
            editor.error = Some(error);
        }
    }
}
