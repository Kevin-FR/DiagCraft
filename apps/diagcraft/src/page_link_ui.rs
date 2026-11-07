use super::*;

/// Formulaire de création ou d'édition d'un lien de navigation vers une autre
/// page. Reprend le modèle des brouillons modaux (`PageDraft`) : le champ est
/// présent tant que la fenêtre est ouverte.
pub(super) struct LinkDraft {
    /// Objet source du lien (0 = page entière), tel que sélectionné.
    source: u64,
    /// Identifiant du lien à éditer, ou `None` pour en créer un nouveau.
    existing: Option<u64>,
    /// Index de la page cible choisie.
    target_index: usize,
    /// Étiquette facultative affichée à la place du nom de la page cible.
    label: String,
    error: Option<String>,
}

impl DrawApp {
    /// Commande « Créer un lien vers une page » : choisit un objet source (la
    /// sélection, sinon le premier objet visible) et ouvre le formulaire.
    pub(super) fn create_page_link(&mut self) {
        let organization = self.doc.organization_view();
        let source = self
            .selected
            .iter()
            .copied()
            .find(|id| self.doc.node(*id).is_some())
            .or_else(|| {
                self.doc
                    .nodes
                    .iter()
                    .map(|n| n.id)
                    .find(|id| organization.is_object_visible(*id))
            });
        let Some(source) = source else {
            self.status = "Ajoutez d'abord une forme à la page.".into();
            return;
        };
        if self.doc.page_count() < 2 {
            self.status = "Ajoutez d'abord une autre page.".into();
            return;
        }
        self.open_page_link(source);
    }

    /// Ouvre le formulaire de création ou d'édition d'un lien porté par
    /// l'objet source donné. Détecte l'éventuel lien existant.
    pub(super) fn open_page_link(&mut self, source: u64) {
        if !self.commit_inline_if_needed() {
            return;
        }
        let existing = self.doc.link_from(source).map(|l| l.id);
        let mut target_index = 0;
        if let Some(existing) = existing {
            if let Some(l) = self.doc.link(existing) {
                target_index = self.doc.page_index_by_id(l.target_page).unwrap_or_default();
            }
        } else {
            // Premier index de page différent de la page active.
            let names = self.doc.page_names();
            target_index = (0..names.len())
                .find(|&i| i != self.doc.page_index)
                .unwrap_or(self.doc.page_index);
        }
        self.link_draft = Some(LinkDraft {
            source,
            existing,
            target_index,
            label: existing
                .and_then(|id| self.doc.link(id))
                .map(|l| l.label.clone())
                .unwrap_or_default(),
            error: None,
        });
    }

    /// Rend le formulaire modal de lien. Retourne `true` s'il y a encore un
    /// brouillon à l'issue de la frame (fenêtre ouverte).
    fn link_dialog_ui(&mut self, ctx: &egui::Context) -> bool {
        let Some(draft) = self.link_draft.take() else {
            return false;
        };
        let mut draft = draft;
        let mut keep = true;
        let mut changed = false;
        egui::Modal::new("page_link_dialog".into()).show(ctx, |ui| {
            ui.set_width(320.);
            ui.heading(if draft.existing.is_some() {
                t("Modifier le lien")
            } else {
                t("Créer un lien vers une page")
            });
            ui.add_space(6.);
            if let Some(e) = &draft.error {
                ui.colored_label(ERROR_TEXT, e);
                ui.add_space(4.);
            }
            let names = self.doc.page_names();
            let labels: Vec<(usize, String)> = names
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != self.doc.page_index)
                .map(|(i, name)| (i, name.to_string()))
                .collect();
            if labels.is_empty() {
                ui.label(t("Ajoutez d'abord une autre page."));
                ui.add_space(6.);
            } else {
                if !labels.iter().any(|(i, _)| *i == draft.target_index) {
                    draft.target_index = labels[0].0;
                }
                let selected = labels
                    .iter()
                    .find(|(i, _)| *i == draft.target_index)
                    .map(|(_, name)| name.clone());
                egui::ComboBox::from_id_salt("link_target_page")
                    .selected_text(selected.unwrap_or_default())
                    .show_ui(ui, |ui| {
                        for (i, name) in &labels {
                            ui.selectable_value(
                                &mut draft.target_index,
                                *i,
                                i18n::render(
                                    "Page {0} · {1}",
                                    &[("0", format!("{}", i + 1)), ("1", name.clone())],
                                ),
                            );
                        }
                    });
                ui.add_space(4.);
                ui.label(t("Étiquette facultative"));
                ui.add(
                    egui::TextEdit::singleline(&mut draft.label)
                        .char_limit(200)
                        .desired_width(f32::INFINITY),
                );
                ui.add_space(8.);
                ui.horizontal(|ui| {
                    if ui.button(t("Annuler")).clicked() {
                        keep = false;
                    }
                    if ui.button(t("Créer le lien")).clicked() {
                        changed = true;
                        draft.error = None;
                    }
                });
            }
        });
        if !keep {
            return false;
        }
        if changed {
            let target_page = self
                .doc
                .page(draft.target_index)
                .expect("page cible valide")
                .id;
            let label = draft.label.trim().to_string();
            let result = match draft.existing {
                None => self.doc.add_page_link(draft.source, target_page, 0, label),
                Some(id) => self.doc.update_page_link(id, target_page, 0, label),
            };
            match result {
                Ok(_) => {
                    self.commit();
                    return false;
                }
                Err(e) => {
                    draft.error = Some(e);
                    self.link_draft = Some(draft);
                    return true;
                }
            }
        }
        self.link_draft = Some(draft);
        true
    }

    /// Dispatch des formulaires de lien depuis la boucle principale.
    pub(super) fn page_links_dialog(&mut self, ctx: &egui::Context) {
        self.link_dialog_ui(ctx);
    }
}

/// Rayon (en pixels écran) autour de l'ancre dans lequel le badge d'un lien de
/// page réagit au survol et au clic.
const LINK_HIT_RADIUS_PX: f32 = 20.;

/// Couleur de texte lisible sur un fond de couleur donnée (noir ou blanc).
fn on_color(color: Color32) -> Color32 {
    let linear = |channel: u8| {
        let v = f64::from(channel) / 255.;
        if v <= 0.04045 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    let luminance =
        0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b());
    if luminance > 0.5 {
        Color32::from_rgb(20, 24, 28)
    } else {
        Color32::WHITE
    }
}

impl DrawApp {
    /// Peint les badges des liens de navigation de la page active, à l'ancre
    /// de leur objet source. Chaque badge affiche le nom de la page cible ; un
    /// double-clic navigue vers cette page (voir `navigate_link`).
    pub(super) fn paint_page_links(&self, painter: &egui::Painter) {
        let organization = self.doc.organization_view();
        for link in &self.doc.links {
            if link.from != 0 && !organization.is_object_visible(link.from) {
                continue;
            }
            let target_index = match self.doc.page_index_by_id(link.target_page) {
                Some(i) => i,
                // La page cible a disparu ; le nettoyage est censé retirer ce
                // lien, mais on ne dessine rien pour un lien orphelin.
                None => continue,
            };
            let name = self
                .doc
                .page_names()
                .get(target_index)
                .copied()
                .unwrap_or("");
            let color = rgb(link.color);
            let anchor = self.screen(self.doc.link_anchor(link));
            let label = if link.label.is_empty() {
                name.to_string()
            } else {
                link.label.clone()
            };
            let text_color = on_color(color);
            let galley =
                painter.layout_no_wrap(format!("⧉ {label}"), FontId::proportional(12.), text_color);
            let size = galley.size() + vec2(14., 8.);
            let rect = Rect::from_center_size(anchor + vec2(size.x / 2. + 4., 0.), size);
            painter.rect(
                rect,
                12.,
                color,
                Stroke::new(1., color.gamma_multiply(0.7)),
                StrokeKind::Middle,
            );
            painter.galley(rect.min + vec2(7., 4.), galley, text_color);
        }
    }

    /// Identifiant du lien dont le badge est sous le pointeur, dans le rayon
    /// d'interaction, si sa source existe encore sur la page active.
    pub(super) fn hit_page_link(&self, pos: Pos2) -> Option<u64> {
        let world = self.world(pos);
        let radius = LINK_HIT_RADIUS_PX / self.zoom.max(0.001);
        self.doc.hit_link(world, radius)
    }

    /// Navigue vers la page cible d'un lien (résolution par identifiant
    /// permanent de page). Aucun changement si la cible a disparu.
    pub(super) fn navigate_link(&mut self, id: u64) -> Option<usize> {
        let link = self.doc.link(id)?;
        let target_index = self.doc.page_index_by_id(link.target_page)?;
        if self.switch_page(target_index) {
            Some(target_index)
        } else {
            None
        }
    }

    /// Traite le double-clic sur un badge de lien : navigate vers la page
    /// cible. Appelé depuis la saisie du canevas, outil Sélection actif.
    pub(super) fn page_links_clicked(
        &mut self,
        ui: &egui::Ui,
        response: &egui::Response,
        pos: Option<Pos2>,
    ) {
        if !response.double_clicked() || self.tool != Tool::Select {
            return;
        }
        if ui.input(|i| i.modifiers.any()) {
            return;
        }
        let Some(pos) = pos else {
            return;
        };
        if let Some(id) = self.hit_page_link(pos) {
            self.navigate_link(id);
        }
    }
}
