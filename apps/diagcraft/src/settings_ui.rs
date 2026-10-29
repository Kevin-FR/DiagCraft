use super::*;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Default)]
pub(super) struct Settings {
    pub open: bool,
    pub check_at_startup: bool,
    pub experimental: bool,
    pub last_check: u64,
    pub job: Option<updates::Job>,
    pub release: Option<updates::Release>,
    pub checked: bool,
    pub error: Option<String>,
    pub downloaded: Option<PathBuf>,
    pub downloading: bool,
}
impl DrawApp {
    pub(crate) fn set_language(&mut self, language: Option<Language>) {
        self.language = language;
        i18n::activate(language.unwrap_or_else(Language::system));
        self.builtin_templates = welcome_ui::catalog();
        self.status = t("Langue de l’interface mise à jour");
        self.symbol_search.clear();
        self.command_palette.query.clear();
        self.help_state.query.clear();
    }
    pub(super) fn check_updates(&mut self) {
        if self.settings.job.is_some() {
            return;
        }
        self.settings.error = None;
        self.settings.checked = false;
        self.settings.downloaded = None;
        self.settings.last_check = now();
        self.settings.job = Some(updates::check(self.settings.experimental));
    }
    pub(super) fn poll_updates(&mut self, ctx: &egui::Context) {
        let result = self
            .settings
            .job
            .as_ref()
            .map(|job| job.receiver.try_recv());
        match result {
            Some(Ok(updates::Completed::Checked(result))) => {
                self.settings.job = None;
                self.settings.checked = true;
                match result {
                    Ok(release) => {
                        self.settings.release = release;
                        if let Some(release) = &self.settings.release {
                            self.status = i18n::render(
                                "Nouveautés disponibles : {0} · Fichier → Mises à jour",
                                &[("0", release.version.to_string())],
                            );
                        }
                    }
                    Err(error) => self.settings.error = Some(error),
                }
            }
            Some(Ok(updates::Completed::Downloaded(result))) => {
                self.settings.job = None;
                self.settings.downloading = false;
                match result {
                    Ok(path) => self.settings.downloaded = Some(path),
                    Err(error) => self.settings.error = Some(error),
                }
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.settings.job = None;
                self.settings.error = Some("Le service de téléchargement est indisponible".into());
            }
            _ => {}
        }
        if self.settings.job.is_some() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    pub(super) fn settings_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings.open {
            return;
        }
        let modal = egui::Modal::new("application_settings".into()).show(ctx, |ui| {
            ui.set_width(560_f32.min(ctx.content_rect().width() - 60.));
            ui.set_height((ctx.content_rect().height() - 120.).clamp(300., 560.));
            ui.horizontal(|ui| {
                ui.heading(t("Paramètres de l’application"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| { if ui.button(t("Fermer")).clicked() { self.settings.open = false; } });
            });
            ui.separator();
            egui::ScrollArea::vertical().scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible).max_height((ctx.content_rect().height() - 170.).max(200.)).show(ui, |ui| {
            ui.heading(t("Langue"));
            let previous = self.language;
            egui::ComboBox::from_id_salt("interface_language").selected_text(self.language.map_or_else(|| t("Système"), |language| language.native().into())).width(ui.available_width()).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.language, None, t("Système"));
                for language in Language::ALL { ui.selectable_value(&mut self.language, Some(language), language.native()); }
            });
            if self.language != previous { self.set_language(self.language); ctx.request_repaint(); }
            ui.label(t("La langue change immédiatement. Les textes de vos dessins sont conservés."));
            ui.add_space(14.);
            ui.heading(t("Mises à jour"));
            ui.label(i18n::render("Version installée : {0}", &[("0", env!("CARGO_PKG_VERSION").into())]));
            ui.checkbox(&mut self.settings.check_at_startup, t("Vérifier au démarrage (au plus une fois par jour)"));
            ui.label(RichText::new(t("La vérification contacte uniquement GitHub. Aucun dessin n’est envoyé.")).small());
            ui.add_enabled_ui(self.settings.job.is_none(), |ui| {
                if ui.checkbox(&mut self.settings.experimental, t("Inclure les versions expérimentales")).changed() { self.settings.release = None; self.settings.checked = false; }
            });
            if let Some(error) = &self.settings.error { ui.colored_label(Color32::DARK_RED, t(error)); }
            if self.settings.job.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(t(if self.settings.downloading { "Téléchargement et vérification SHA-256…" } else { "Vérification sur GitHub…" }));
                    if ui.button(t("Annuler")).clicked() { self.settings.job = None; self.settings.downloading = false; }
                });
            } else {
                if ui.button(t("Vérifier maintenant")).clicked() { self.check_updates(); }
                if self.settings.checked && self.settings.release.is_none() && self.settings.error.is_none() { ui.label(t("Vous utilisez la version la plus récente de ce canal.")); }
            }
            if let Some(release) = self.settings.release.clone() {
                ui.separator();
                ui.heading(i18n::render("Nouvelle version : {0}", &[("0", release.version.to_string())]));
                if release.experimental { ui.label(t("Version expérimentale")); }
                ui.hyperlink_to(t("Lire les notes sur GitHub"), release.page());
                ui.label(&release.filename);
                if self.settings.job.is_none() && self.settings.downloaded.is_none() && ui.button(t("Télécharger le paquet vérifié…")).clicked()
                    && let Some(destination) = rfd::FileDialog::new().set_title(t("Enregistrer la mise à jour")).set_file_name(&release.filename).save_file()
                {
                    self.settings.error = None;
                    self.settings.downloading = true;
                    self.settings.job = Some(updates::download(release, destination));
                }
            }
            if let Some(path) = &self.settings.downloaded {
                ui.label(t("Téléchargement vérifié. Votre travail est conservé."));
                ui.label(path.to_string_lossy());
                ui.label(t("Fermez DiagCraft après avoir enregistré vos dessins, puis installez le nouveau paquet."));
                if ui.button(t("Afficher le téléchargement")).clicked() { reveal(path); }
            }
            ui.add_space(12.);
            ui.label(t("L’installation reste une action explicite. Aucun fichier de travail n’est remplacé par la mise à jour."));
            });
        });
        if modal.should_close() {
            self.settings.open = false;
        }
    }
    pub(super) fn maybe_check_updates(&mut self) {
        if self.screenshot.is_none()
            && self.settings.check_at_startup
            && now().saturating_sub(self.settings.last_check) >= 24 * 60 * 60
        {
            self.check_updates();
        }
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}
fn reveal(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("/usr/bin/open")
        .arg("-R")
        .arg(path)
        .spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new(
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("explorer.exe"),
    )
    .arg(format!("/select,{}", path.display()))
    .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    if let Some(parent) = path.parent() {
        let _ = std::process::Command::new("/usr/bin/xdg-open")
            .arg(parent)
            .spawn();
    }
}
