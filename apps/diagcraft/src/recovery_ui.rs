use super::*;
use std::time::Instant;

pub(super) struct Recovery {
    pub open: bool,
    pub force: bool,
    worker: Option<recovery::Worker>,
    candidates: Vec<recovery::Candidate>,
    error: Option<String>,
    discovery_error: Option<String>,
    confirm_delete: Option<usize>,
    last_attempt: Instant,
    serial: u64,
}

impl Default for Recovery {
    fn default() -> Self {
        Self {
            open: false,
            force: false,
            worker: None,
            candidates: vec![],
            error: None,
            discovery_error: None,
            confirm_delete: None,
            last_attempt: Instant::now(),
            serial: 0,
        }
    }
}

impl DrawApp {
    pub(super) fn prepare_recovery_capture(&mut self) {
        match recovery::preview(vec![
            recovery::Copy {
                document: Document::demo(),
                source: Some(PathBuf::from("Documents/mon-diagramme.draw")),
            },
            recovery::Copy {
                document: templates::support(),
                source: None,
            },
        ]) {
            Ok(candidates) => {
                self.recovery.candidates = candidates;
                self.recovery.open = true;
            }
            Err(e) => self.error = Some(e),
        }
    }
    pub(super) fn initialize_recovery(&mut self) {
        if let Some(root) =
            library::default_path().and_then(|p| p.parent().map(|p| p.join("recovery-v1")))
        {
            self.start_recovery(&root);
        }
    }

    fn start_recovery(&mut self, root: &std::path::Path) {
        match recovery::scan(root) {
            Ok(candidates) => {
                self.recovery.open = !candidates.is_empty();
                self.recovery.candidates = candidates;
            }
            Err(e) => self.recovery.discovery_error = Some(e),
        }
        match recovery::Worker::start(root) {
            Ok(worker) => self.recovery.worker = Some(worker),
            Err(e) => self.recovery.error = Some(e),
        }
        if self.recovery.error.is_some() || self.recovery.discovery_error.is_some() {
            self.status = "Récupération automatique indisponible. Consultez Récupérer des dessins dans le menu Fichier.".into();
        }
    }

    fn recovery_copies(&self) -> Vec<recovery::Copy> {
        self.tabs
            .iter()
            .enumerate()
            .filter_map(|(index, tab)| {
                let (doc, saved, source, recovered) = if index == self.active_tab {
                    (&self.doc, &self.saved, &self.path, self.recovered)
                } else {
                    (&tab.doc, &tab.saved, &tab.path, tab.recovered)
                };
                (recovered || doc != saved).then(|| recovery::Copy {
                    document: doc.clone(),
                    source: source.clone(),
                })
            })
            .collect()
    }

    pub(super) fn tick_recovery(&mut self, ctx: &egui::Context) {
        let result = self
            .recovery
            .worker
            .as_mut()
            .and_then(|worker| worker.poll());
        if let Some((serial, result)) = result {
            match result {
                Ok(()) => {
                    self.recovery.error = None;
                    // Remove the old copy only after a new, durable checkpoint acknowledged it.
                    let previous_count = self.recovery.candidates.len();
                    let mut retained = vec![];
                    for candidate in self.recovery.candidates.drain(..) {
                        if candidate.restored_at.is_some_and(|s| s <= serial) {
                            if let Err(e) = candidate.remove() {
                                self.recovery.error = Some(e);
                                retained.push(candidate);
                            }
                        } else {
                            retained.push(candidate);
                        }
                    }
                    self.recovery.candidates = retained;
                    if self.recovery.candidates.len() != previous_count {
                        self.recovery.confirm_delete = None;
                    }
                }
                Err(e) => {
                    self.recovery.error = Some(e);
                    self.status = "La copie de récupération n’a pas pu être mise à jour. Enregistrez votre travail.".into();
                }
            }
        }
        if self.recovery.worker.is_none() {
            return;
        }
        // A partial pointer gesture must never replace the last committed drawing.
        if self.drag.is_none()
            && !self.property_edit
            && (self.recovery.force || self.recovery.last_attempt.elapsed() >= recovery::INTERVAL)
            && self
                .recovery
                .worker
                .as_ref()
                .is_some_and(|worker| !worker.busy)
        {
            let copies = self.recovery_copies();
            let serial = self.recovery.serial + 1;
            if self
                .recovery
                .worker
                .as_mut()
                .unwrap()
                .checkpoint(serial, copies)
            {
                self.recovery.serial = serial;
                self.recovery.force = false;
                self.recovery.last_attempt = Instant::now();
            }
        }
        ctx.request_repaint_after(if self.recovery.worker.as_ref().is_some_and(|w| w.busy) {
            std::time::Duration::from_millis(100)
        } else {
            recovery::INTERVAL
                .saturating_sub(self.recovery.last_attempt.elapsed())
                .max(std::time::Duration::from_millis(100))
        });
    }

    fn restore_recovery(&mut self, index: usize) {
        let Some(candidate) = self.recovery.candidates.get(index) else {
            return;
        };
        if candidate.restored_at.is_some() {
            return;
        }
        let copies = match &candidate.snapshot {
            Ok(snapshot) => snapshot.copies.clone(),
            Err(_) => return,
        };
        for copy in copies {
            // Source paths are never used as destinations, even if the source has disappeared.
            self.open_tab(copy.document, None);
            self.recovered = true;
            self.stash_tab();
        }
        self.recovery.candidates[index].restored_at = Some(self.recovery.serial + 1);
        self.recovery.force = true;
        self.recovery.confirm_delete = None;
        self.recovery.open = false;
        self.status = "Dessins récupérés dans de nouveaux onglets. Enregistrez les copies à l’emplacement de votre choix.".into();
    }

    pub(super) fn clean_recovery(&mut self) {
        if let Some(worker) = &mut self.recovery.worker {
            worker.clean();
        }
        // Unopened recoveries are preserved, including when Later was selected.
        for candidate in &self.recovery.candidates {
            if candidate.restored_at.is_some() {
                let _ = candidate.remove();
            }
        }
    }

    pub(super) fn recovery_dialog(&mut self, ctx: &egui::Context) {
        if !self.recovery.open {
            return;
        }
        let mut restore = None;
        let mut delete = None;
        let modal = egui::Modal::new("recovery".into()).show(ctx, |ui| {
            ui.set_width(560_f32.min(ctx.content_rect().width() - 60.));
            ui.heading(t("Récupérer des dessins"));
            ui.label(t("Des copies locales peuvent être disponibles après une fermeture inattendue. Les fichiers d’origine restent inchangés."));
            ui.add_space(8.);
            egui::ScrollArea::vertical().max_height((ctx.content_rect().height() - 270.).clamp(150., 380.)).show(ui, |ui| {
                for e in self.recovery.discovery_error.iter().chain(self.recovery.error.iter()) {
                    ui.colored_label(Color32::DARK_RED, i18n::render("Récupération indisponible : {0}", &[("0", e.clone())]));
                }
                if self.recovery.candidates.is_empty() {
                    ui.label(t("Aucun dessin à récupérer."));
                }
                for (index, candidate) in self.recovery.candidates.iter().enumerate() {
                    ui.push_id(index, |ui| {
                        ui.separator();
                        match &candidate.snapshot {
                            Ok(snapshot) => {
                                ui.strong(i18n::render("Dessins à récupérer : {0}", &[("0", snapshot.copies.len().to_string())]));
                                let minutes = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default().as_secs().saturating_sub(snapshot.updated) / 60;
                                ui.label(RichText::new(if minutes == 0 { t("Dernière copie : moins d’une minute") }
                                    else { i18n::render("Dernière copie : il y a {0} min", &[("0", minutes.to_string())]) }).small().color(MUTED));
                                for copy in &snapshot.copies {
                                    ui.label(&copy.document.title);
                                    if let Some(path) = &copy.source { ui.label(RichText::new(path.to_string_lossy()).small().color(MUTED)); }
                                    else { ui.label(RichText::new(t("Jamais enregistré")).small().color(MUTED)); }
                                }
                                if candidate.restored_at.is_some() { ui.label(t("Copies déjà ouvertes")); }
                                else if ui.button(RichText::new(t("Récupérer cette session")).color(ACCENT)).clicked() { restore = Some(index); }
                            },
                            Err(e) => {
                                ui.label(t("Copie illisible ou issue d’une version plus récente. Elle est conservée."));
                                ui.collapsing(t("Détails"), |ui| { ui.label(e); });
                            },
                        }
                        if candidate.restored_at.is_none() {
                            if self.recovery.confirm_delete == Some(index) {
                                ui.label(t("Supprimer définitivement ces copies de récupération ?"));
                                ui.horizontal_wrapped(|ui| {
                                    if ui.button(t("Supprimer les copies")).clicked() { delete = Some(index); }
                                    if ui.button(t("Annuler")).clicked() { self.recovery.confirm_delete = None; }
                                });
                            } else if ui.small_button(t("Supprimer…")).clicked() { self.recovery.confirm_delete = Some(index); }
                        }
                    });
                }
            });
            ui.separator();
            ui.label(RichText::new(t("Copie automatique toutes les 10 secondes, après les gestes terminés. Elle ne remplace pas Enregistrer.")).small());
            if ui.button(t("Plus tard")).clicked() { self.recovery.open = false; }
        });
        if modal.should_close() {
            self.recovery.open = false;
        }
        if let Some(index) = restore {
            self.restore_recovery(index);
        }
        if let Some(index) = delete {
            match self.recovery.candidates[index].remove() {
                Ok(()) => {
                    self.recovery.candidates.remove(index);
                    self.recovery.confirm_delete = None;
                }
                Err(e) => self.recovery.error = Some(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn app() -> (egui::Context, DrawApp) {
        let ctx = egui::Context::default();
        let app = DrawApp::from_context(&ctx, Document::default(), None, None);
        (ctx, app)
    }

    fn wait(app: &mut DrawApp, ctx: &egui::Context) {
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.recovery.worker.as_ref().is_some_and(|w| w.busy) && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(10));
            app.tick_recovery(ctx);
        }
        assert!(!app.recovery.worker.as_ref().unwrap().busy);
        assert!(app.recovery.error.is_none(), "{:?}", app.recovery.error);
    }

    fn abandoned(root: &std::path::Path, copies: Vec<recovery::Copy>) {
        let mut worker = recovery::Worker::start(root).unwrap();
        assert!(worker.checkpoint(1, copies));
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some((_, result)) = worker.poll() {
                result.unwrap();
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
        drop(worker);
        // The worker releases its OS lease when the sender disconnects.
        while recovery::scan(root).unwrap().is_empty() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn restore_opens_dirty_copies_without_source_paths_and_preserves_all_tabs() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("source.draw");
        let original = Document::default();
        original.save(&source).unwrap();
        let before = std::fs::read(&source).unwrap();
        let mut changed = original.clone();
        changed.add_node(ShapeKind::Rectangle, [200., 200.]);
        changed.add_page(true).unwrap();
        let unsaved = Document {
            title: "Enregistrer {0} 中文".into(),
            ..Default::default()
        };
        abandoned(
            root.path(),
            vec![
                recovery::Copy {
                    document: changed.clone(),
                    source: Some(source.clone()),
                },
                recovery::Copy {
                    document: unsaved.clone(),
                    source: None,
                },
            ],
        );
        let (ctx, mut app) = app();
        app.start_recovery(root.path());
        assert!(app.recovery.open);
        app.restore_recovery(0);
        assert_eq!(app.tabs.len(), 3);
        assert_eq!(app.tabs[1].doc, changed);
        assert_eq!(app.doc, unsaved);
        assert!(app.dirty() && app.recovered && app.path.is_none());
        assert_eq!(std::fs::read(&source).unwrap(), before);
        assert!(
            !app.history.can_undo(),
            "recovery is the initial copy, not an undo to empty"
        );
        app.tick_recovery(&ctx);
        wait(&mut app, &ctx);
        assert!(
            app.recovery.candidates.is_empty(),
            "old copy transfers only after persistence"
        );
        app.switch_tab(1);
        assert!(app.dirty() && app.path.is_none());
        let destination = root.path().join("recovered.draw");
        assert!(app.save_to(destination.clone()));
        assert!(!app.dirty() && !app.recovered);
        assert_eq!(Document::load(&destination).unwrap(), changed);
        assert_eq!(std::fs::read(source).unwrap(), before);
        app.request(Pending::Close);
        assert!(matches!(app.pending, Some(Pending::Close)));
        assert_eq!(
            app.doc, unsaved,
            "unsaved restored copies still trigger confirmation"
        );
        app.clean_recovery();
    }

    #[test]
    fn checkpoints_skip_live_gestures_and_clean_saved_or_closed_tabs() {
        let root = tempfile::tempdir().unwrap();
        let (ctx, mut app) = app();
        app.start_recovery(root.path());
        app.doc.title = "changed".into();
        app.recovery.force = true;
        app.property_edit = true;
        app.tick_recovery(&ctx);
        assert_eq!(app.recovery.serial, 0);
        app.commit();
        app.tick_recovery(&ctx);
        wait(&mut app, &ctx);
        assert_eq!(app.recovery.serial, 1);
        app.open_tab(Document::default(), None);
        app.doc.title = "second".into();
        assert_eq!(app.recovery_copies().len(), 2);
        app.close_tab(0);
        assert_eq!(app.recovery_copies().len(), 1);
        assert!(app.save_to(root.path().join("saved.draw")));
        assert!(app.recovery_copies().is_empty());
        app.tick_recovery(&ctx);
        wait(&mut app, &ctx);
        app.clean_recovery();
    }

    #[test]
    fn later_preserves_unopened_recoveries_and_modal_blocks_drawing_commands() {
        let root = tempfile::tempdir().unwrap();
        abandoned(
            root.path(),
            vec![recovery::Copy {
                document: Document::default(),
                source: None,
            }],
        );
        let (ctx, mut app) = app();
        app.start_recovery(root.path());
        assert!(app.command_dialog_blocked());
        let before = app.doc.clone();
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1280., 760.))),
                events: vec![egui::Event::Key {
                    key: Key::N,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: egui::Modifiers::COMMAND,
                }],
                ..Default::default()
            },
            |ctx| {
                app.shortcuts(ctx);
                app.recovery_dialog(ctx);
            },
        );
        assert_eq!(app.doc, before);
        assert_eq!(app.tabs.len(), 1);
        app.recovery.open = false;
        app.clean_recovery();
        drop(app);
        assert_eq!(recovery::scan(root.path()).unwrap().len(), 1);
    }

    #[test]
    fn earlier_acknowledgment_and_failed_transfer_never_delete_the_old_copy() {
        let root = tempfile::tempdir().unwrap();
        let original = Document {
            title: "recovery".into(),
            ..Default::default()
        };
        abandoned(
            root.path(),
            vec![recovery::Copy {
                document: original.clone(),
                source: None,
            }],
        );
        let (ctx, mut app) = app();
        app.start_recovery(root.path());
        app.doc.title = "other tab".into();
        app.recovery.force = true;
        app.tick_recovery(&ctx); // write 1 contains only the other tab
        app.restore_recovery(0); // transfer requires write 2 or later
        app.property_edit = true; // prevent write 2 while acknowledging 1
        wait(&mut app, &ctx);
        assert_eq!(app.recovery.serial, 1);
        assert_eq!(app.recovery.candidates.len(), 1);
        app.doc.add_node(ShapeKind::Rectangle, [200., 200.]);
        app.doc.nodes[0].position[0] = f32::NAN;
        app.property_edit = false;
        app.tick_recovery(&ctx);
        let deadline = Instant::now() + Duration::from_secs(5);
        while app.recovery.worker.as_ref().unwrap().busy {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(10));
            app.tick_recovery(&ctx);
        }
        assert!(app.recovery.error.is_some());
        assert_eq!(app.recovery.candidates.len(), 1);
        assert_eq!(
            app.recovery.candidates[0].snapshot.as_ref().unwrap().copies[0].document,
            original
        );
        app.doc.nodes[0].position[0] = 200.;
        app.recovery.force = true;
        app.tick_recovery(&ctx);
        wait(&mut app, &ctx);
        assert!(app.recovery.candidates.is_empty());
        app.clean_recovery();
    }
}
