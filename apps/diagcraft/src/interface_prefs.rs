//! Local UI preferences: independent of documents, selection and undo history.
use super::*;
use serde::{Deserialize, Serialize};
use std::io::Read;

#[derive(Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    version: u32,
    language: Option<Language>,
    update_check: bool,
    update_experimental: bool,
    last_update_check: u64,
    library: bool,
    inspector: bool,
    minimap: bool,
    library_width: f32,
    inspector_width: f32,
    ribbon: Option<bool>,
    tab: RibbonTab,
    panel: LibraryPanel,
    floating_toolbar: bool,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            version: 1,
            language: None,
            update_check: false,
            update_experimental: true,
            last_update_check: 0,
            library: true,
            inspector: false,
            minimap: false,
            library_width: 256.,
            inspector_width: 280.,
            ribbon: None,
            tab: RibbonTab::Home,
            panel: LibraryPanel::Symbols,
            floating_toolbar: true,
        }
    }
}

impl DrawApp {
    fn interface_preferences(&self) -> Preferences {
        Preferences {
            language: self.language,
            update_check: self.settings.check_at_startup,
            update_experimental: self.settings.experimental,
            last_update_check: self.settings.last_check,
            library: self.show_library,
            inspector: self.show_inspector,
            minimap: self.show_minimap,
            library_width: self.library_width,
            inspector_width: self.inspector_width,
            ribbon: self.ribbon_expanded,
            tab: self.ribbon_tab,
            // Outline is a transient navigator, so older app versions can still
            // read the existing version-1 panel preference.
            panel: if self.library_panel == LibraryPanel::Outline {
                LibraryPanel::Symbols
            } else {
                self.library_panel
            },
            floating_toolbar: self.show_quick_toolbar,
            ..Default::default()
        }
    }

    fn apply_interface(&mut self, prefs: Preferences) {
        self.settings.check_at_startup = prefs.update_check;
        self.settings.experimental = prefs.update_experimental;
        self.settings.last_check = prefs.last_update_check;
        self.language = prefs.language;
        i18n::activate(self.language.unwrap_or_else(Language::system));
        self.show_library = prefs.library;
        self.show_inspector = prefs.inspector;
        self.show_minimap = prefs.minimap;
        self.library_width = bounded(prefs.library_width, 240., 256.);
        self.inspector_width = bounded(prefs.inspector_width, 260., 280.);
        self.ribbon_expanded = prefs.ribbon;
        self.ribbon_tab = prefs.tab;
        self.library_panel = prefs.panel;
        self.show_quick_toolbar = prefs.floating_toolbar;
    }

    pub(super) fn restore_interface(&mut self) {
        let Some(path) = &self.interface_path else {
            return;
        };
        if !path.exists() {
            return;
        }
        let result: Result<Preferences, String> = (|| {
            let mut bytes = vec![];
            std::fs::File::open(path)
                .map_err(|e| e.to_string())?
                .take(8193)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 8192 {
                return Err("Fichier trop volumineux".into());
            }
            let prefs: Preferences = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if prefs.version != 1 {
                return Err("Version de préférences inconnue".into());
            }
            Ok(prefs)
        })();
        match result {
            Ok(prefs) => self.apply_interface(prefs),
            Err(error) => {
                // Preserve an unreadable or newer file; an explicit reset re-enables saving.
                self.interface_path = None;
                self.status = i18n::render(
                    "Interface par défaut : {0}. Réinitialisez l’interface pour mémoriser les réglages.",
                    &[("0", error.to_string())],
                );
            }
        }
    }

    pub(super) fn save_interface(&self) -> Result<(), String> {
        let Some(path) = &self.interface_path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let bytes =
            serde_json::to_vec_pretty(&self.interface_preferences()).map_err(|e| e.to_string())?;
        document::atomic_write(path, &bytes)
    }

    pub(super) fn reset_interface(&mut self, ctx: &egui::Context) {
        self.apply_interface(Preferences {
            language: self.language,
            update_check: self.settings.check_at_startup,
            update_experimental: self.settings.experimental,
            last_update_check: self.settings.last_check,
            ..Default::default()
        });
        for name in ["symbol_library", "inspector"] {
            ctx.data_mut(|data| {
                data.remove::<egui::containers::panel::PanelState>(egui::Id::new(name))
            });
        }
        // Screenshot and test sessions must never write user preferences.
        if self.interface_path.is_none() && self.library_path.is_some() {
            self.interface_path = self
                .library_path
                .as_ref()
                .map(|p| p.with_file_name("interface-v1.json"));
        }
        self.status = "Interface réinitialisée. Le dessin est conservé.".into();
    }
}

fn bounded(value: f32, min: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, 400.)
    } else {
        fallback
    }
}
