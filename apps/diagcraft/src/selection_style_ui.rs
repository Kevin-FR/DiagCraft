//! Shared appearance edits preserve the unedited differences in a selection.
use super::*;
use diagcraft::appearance::{Fill, Line, Shadow};

macro_rules! changed_fields {
    ($target:expr, $before:expr, $after:expr, $replace:expr; $($field:ident),+ $(,)?) => {
        $(if $replace || $before.$field != $after.$field {
            $target.$field = $after.$field;
        })+
    };
}

fn fill(target: &mut Fill, before: &Fill, after: &Fill, replace: bool) {
    changed_fields!(target, before, after, replace;
        mode, secondary, tint, transparency, angle, pattern, texture, spacing);
}
fn line(target: &mut Line, before: &Line, after: &Line, replace: bool) {
    changed_fields!(target, before, after, replace;
        mode, secondary, tint, transparency, angle, pattern, cap, join, compound,
        corner_radius, start_scale, end_scale);
}
fn shadow(target: &mut Shadow, before: &Shadow, after: &Shadow, replace: bool) {
    changed_fields!(target, before, after, replace;
        kind, color, transparency, scale, blur, angle);
    for i in 0..2 {
        if replace || before.offset[i] != after.offset[i] {
            target.offset[i] = after.offset[i];
        }
    }
}

impl DrawApp {
    fn selection_style_sample(&self, id: u64) -> Option<Node> {
        if let Some(n) = self.doc.node(id).filter(|n| n.kind != ShapeKind::Text) {
            return Some(n.clone());
        }
        if self.style_tab != appearance_ui::StyleTab::Line {
            return None;
        }
        let mut sample = symbols::preset(ShapeKind::Rectangle);
        if let Some(e) = self.doc.connectors.iter().find(|e| e.id == id) {
            sample.stroke = e.color;
            sample.stroke_width = e.width;
            sample.appearance.line = e.appearance.clone();
            sample.appearance.line.pattern = e.pattern;
            return Some(sample);
        }
        if let Some(d) = self.doc.drawings.iter().find(|d| d.id == id) {
            sample.stroke = d.color;
            sample.stroke_width = d.width;
            sample.appearance.line = d.appearance.clone();
            return Some(sample);
        }
        None
    }

    fn apply_selection_style(
        &mut self,
        before: &Node,
        after: &Node,
        replace_effect: bool,
        uniform: bool,
    ) {
        if !self.selection_editable() {
            return;
        }
        let replace = replace_effect || uniform;
        for n in &mut self.doc.nodes {
            if !self.selected.contains(&n.id) || n.kind == ShapeKind::Text {
                continue;
            }
            match self.style_tab {
                appearance_ui::StyleTab::Fill => {
                    fill(
                        &mut n.appearance.fill,
                        &before.appearance.fill,
                        &after.appearance.fill,
                        replace,
                    );
                    changed_fields!(n, before, after, uniform; fill);
                }
                appearance_ui::StyleTab::Line => {
                    line(
                        &mut n.appearance.line,
                        &before.appearance.line,
                        &after.appearance.line,
                        replace,
                    );
                    changed_fields!(n, before, after, uniform; stroke, stroke_width);
                }
                appearance_ui::StyleTab::Shadow => {
                    shadow(
                        &mut n.appearance.shadow,
                        &before.appearance.shadow,
                        &after.appearance.shadow,
                        replace,
                    );
                }
            }
        }
        if self.style_tab != appearance_ui::StyleTab::Line {
            return;
        }
        for e in &mut self.doc.connectors {
            if self.selected.contains(&e.id) {
                // Connector dashes live in the top-level pattern, unlike drawings.
                let unused_pattern = e.appearance.pattern;
                line(
                    &mut e.appearance,
                    &before.appearance.line,
                    &after.appearance.line,
                    replace,
                );
                e.appearance.pattern = unused_pattern;
                if uniform || before.stroke != after.stroke {
                    e.color = after.stroke;
                }
                if uniform || before.stroke_width != after.stroke_width {
                    e.width = after.stroke_width.max(0.5);
                }
                if replace || before.appearance.line.pattern != after.appearance.line.pattern {
                    e.pattern = after.appearance.line.pattern;
                }
            }
        }
        for d in &mut self.doc.drawings {
            if self.selected.contains(&d.id) {
                line(
                    &mut d.appearance,
                    &before.appearance.line,
                    &after.appearance.line,
                    replace,
                );
                if uniform || before.stroke != after.stroke {
                    d.color = after.stroke;
                }
                if uniform || before.stroke_width != after.stroke_width {
                    d.width = after.stroke_width.max(0.5);
                }
            }
        }
    }

    pub(super) fn selection_inspector(&mut self, ui: &mut egui::Ui) {
        let nodes: Vec<_> = self
            .doc
            .nodes
            .iter()
            .filter(|n| self.selected.contains(&n.id))
            .collect();
        let shapes = nodes.iter().filter(|n| n.kind != ShapeKind::Text).count();
        let edges = self
            .doc
            .connectors
            .iter()
            .filter(|e| self.selected.contains(&e.id))
            .count();
        let drawings = self
            .doc
            .drawings
            .iter()
            .filter(|d| self.selected.contains(&d.id))
            .count();
        let all_corners = shapes > 0
            && edges + drawings == 0
            && nodes
                .iter()
                .filter(|n| n.kind != ShapeKind::Text)
                .all(|n| appearance_ui::node_corners(n).is_some());
        ui.label(
            RichText::new(i18n::render(
                "{0} objets",
                &[("0", format!("{}", self.selected.len()))],
            ))
            .size(20.)
            .strong(),
        );
        ui.add_space(6.);
        self.alignment_controls(ui);
        ui.separator();
        if shapes + edges + drawings > 0 {
            appearance_ui::style_tabs(ui, &mut self.style_tab, shapes > 0);
            let count = if self.style_tab == appearance_ui::StyleTab::Line {
                shapes + edges + drawings
            } else {
                shapes
            };
            let sample = self
                .selected
                .iter()
                .find_map(|id| self.selection_style_sample(*id));
            let varied = sample.as_ref().is_some_and(|first| {
                self.selected
                    .iter()
                    .filter_map(|id| self.selection_style_sample(*id))
                    .any(|n| match self.style_tab {
                        appearance_ui::StyleTab::Fill => {
                            n.fill != first.fill || n.appearance.fill != first.appearance.fill
                        }
                        appearance_ui::StyleTab::Line => {
                            n.stroke != first.stroke
                                || n.stroke_width != first.stroke_width
                                || n.appearance.line != first.appearance.line
                        }
                        appearance_ui::StyleTab::Shadow => {
                            n.appearance.shadow != first.appearance.shadow
                        }
                    })
            });
            ui.label(
                RichText::new(format!("{count} / {} · {}", self.selected.len(), t(if varied { "Styles variés" } else { "Même style" })))
                    .size(12.)
                    .color(INFO_TEXT),
            ).on_hover_text(t("Valeurs du premier objet compatible. Seuls les champs modifiés sont appliqués. Remplissage et ombre : formes. Ligne : formes, liens et tracés. Textes seuls et graphiques conservent leur apparence."));
            if let Some(before) = sample {
                let corners = all_corners
                    .then(|| appearance_ui::node_corners(&before))
                    .flatten();
                let mut after = before.clone();
                let replace = appearance_ui::style_body(
                    ui,
                    &mut after,
                    self.style_tab,
                    &self.styled_cache,
                    false,
                    corners,
                    if edges + drawings > 0 { 0.5 } else { 0. },
                );
                let uniform = ui.button(t("Uniformiser cet onglet")).on_hover_text(t("Reprendre tous les réglages affichés de cet onglet, y compris la couleur et l’épaisseur, pour les objets compatibles.")).clicked();
                self.apply_selection_style(&before, &after, replace, uniform);
            }
        }
        if let Some(before) = self
            .selected
            .iter()
            .find_map(|id| self.doc.node(*id))
            .cloned()
        {
            egui::CollapsingHeader::new(t("Typographie des formes")).show(ui, |ui| {
                let mut after = before.clone();
                ui.label(t("Le contenu de chaque texte est conservé."));
                appearance_ui::color(ui, "Couleur du texte", &mut after.text_color);
                appearance_ui::scalar(ui, "Taille du texte", &mut after.font_size, 8.0..=64.0, " px");
                ui.horizontal(|ui| {
                    ui.checkbox(&mut after.text_style.bold, t("Gras"));
                    ui.checkbox(&mut after.text_style.italic, t("Italique"));
                });
                ui.checkbox(&mut after.text_style.underline, t("Souligné"));
                ui.checkbox(&mut after.text_style.wrap, t("Retour à la ligne automatique"));
                if ui.checkbox(&mut after.text_style.auto_fit, t("Adapter automatiquement au texte")).changed()
                    && after.text_style.auto_fit { after.text_style.wrap = true; }
                let uniform = ui.button(t("Uniformiser ces réglages")).clicked();
                for n in &mut self.doc.nodes {
                    if self.selected.contains(&n.id) {
                        changed_fields!(n, before, after, uniform; text_color, font_size);
                        changed_fields!(n.text_style, before.text_style, after.text_style, uniform; bold, italic, underline, wrap, auto_fit);
                    }
                }
            });
            if ui.button(t("Ajuster les formes au texte")).clicked() {
                self.fit_selection_text();
            }
        }
    }
}
