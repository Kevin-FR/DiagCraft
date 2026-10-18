use super::*;
use egui::{KeyboardShortcut, Modifiers};

const SECONDARY: Color32 = Color32::from_rgb(95, 111, 126);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Command {
    Palette,
    Help,
    New,
    Open,
    Save,
    SaveAs,
    Close,
    ExportSvg,
    ExportPdf,
    Gallery,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    Bold,
    Italic,
    Underline,
    Select,
    Rectangle,
    Text,
    Connect,
    Pan,
    Line,
    Bezier,
    FitPage,
    FitSelection,
    ActualSize,
    Library,
    Inspector,
    Ribbon,
    ResetInterface,
    Grid,
    Snap,
    Guides,
    Rulers,
    Templates,
    Symbols,
    Agents,
    Pages,
    PageSettings,
    AddPage,
    Validate,
}

pub(super) struct CommandSpec {
    pub command: Command,
    pub label: &'static str,
    pub category: &'static str,
    aliases: &'static str,
    shortcuts: &'static [KeyboardShortcut],
}

macro_rules! command {
    ($id:ident, $label:literal, $category:literal, $aliases:literal $(, $modifiers:expr => $key:ident)*) => {
        CommandSpec { command: Command::$id, label: $label, category: $category, aliases: $aliases,
            shortcuts: &[$(KeyboardShortcut::new($modifiers, Key::$key)),*] }
    };
}

pub(super) const COMMANDS: &[CommandSpec] = &[
    command!(Palette, "Rechercher une commande…", "Aide", "palette command search", Modifiers::COMMAND => K),
    command!(Help, "Raccourcis et gestes…", "Aide", "aide clavier trackpad help", Modifiers::NONE => F1),
    command!(New, "Nouveau document", "Fichier", "créer new", Modifiers::COMMAND => N),
    command!(Open, "Ouvrir un document…", "Fichier", "charger open", Modifiers::COMMAND => O),
    command!(Save, "Enregistrer", "Fichier", "sauvegarder sauver save", Modifiers::COMMAND => S),
    command!(SaveAs, "Enregistrer sous…", "Fichier", "sauvegarder copie save as", Modifiers::COMMAND.plus(Modifiers::SHIFT) => S),
    command!(ExportSvg, "Exporter en SVG…", "Fichier", "export vectoriel image", Modifiers::COMMAND => E),
    command!(ExportPdf, "Exporter en PDF…", "Fichier", "export imprimer impression", Modifiers::COMMAND.plus(Modifiers::SHIFT) => E),
    command!(
        Gallery,
        "Mes travaux enregistrés…",
        "Fichier",
        "galerie historique versions"
    ),
    command!(Close, "Fermer le document", "Fichier", "onglet close"),
    command!(Undo, "Annuler", "Édition", "undo revenir", Modifiers::COMMAND => Z),
    command!(Redo, "Rétablir", "Édition", "redo refaire", Modifiers::COMMAND.plus(Modifiers::SHIFT) => Z, Modifiers::COMMAND => Y),
    command!(Copy, "Copier", "Édition", "copy presse papiers", Modifiers::COMMAND => C),
    command!(Cut, "Couper", "Édition", "cut presse papiers", Modifiers::COMMAND => X),
    command!(Paste, "Coller", "Édition", "paste presse papiers", Modifiers::COMMAND => V),
    command!(Duplicate, "Dupliquer la sélection", "Édition", "copie duplicate", Modifiers::COMMAND => D),
    command!(Delete, "Supprimer la sélection", "Édition", "effacer delete", Modifiers::NONE => Delete, Modifiers::NONE => Backspace),
    command!(SelectAll, "Tout sélectionner", "Édition", "tous objets select all", Modifiers::COMMAND => A),
    command!(Bold, "Gras", "Texte", "bold", Modifiers::COMMAND => B),
    command!(Italic, "Italique", "Texte", "italic", Modifiers::COMMAND => I),
    command!(Underline, "Souligné", "Texte", "underline", Modifiers::COMMAND => U),
    command!(Select, "Outil Sélection", "Outils", "pointeur déplacer select", Modifiers::NONE => V),
    command!(Rectangle, "Outil Rectangle", "Outils", "bloc forme rectangle", Modifiers::NONE => R),
    command!(Text, "Outil Texte", "Outils", "ajouter écrire text", Modifiers::NONE => T),
    command!(Connect, "Relier des blocs", "Outils", "connecteur lien flèche connect", Modifiers::NONE => C),
    command!(Pan, "Outil Main", "Outils", "déplacer vue pan", Modifiers::NONE => H),
    command!(Line, "Outil Trait", "Outils", "ligne dessin line", Modifiers::NONE => L),
    command!(Bezier, "Outil Courbe de Bézier", "Outils", "dessin bezier", Modifiers::NONE => B),
    command!(FitPage, "Ajuster la page à la fenêtre", "Affichage", "zoom fit centrer", Modifiers::NONE => F),
    command!(FitSelection, "Zoomer sur la sélection", "Affichage", "zoom fit centrer", Modifiers::SHIFT => F),
    command!(
        ActualSize,
        "Zoom à 100 %",
        "Affichage",
        "taille réelle zoom"
    ),
    command!(
        Library,
        "Afficher / masquer la bibliothèque",
        "Affichage",
        "volet gauche symboles"
    ),
    command!(
        Inspector,
        "Afficher / masquer les propriétés",
        "Affichage",
        "volet droite document inspecteur"
    ),
    command!(
        Ribbon,
        "Déplier / replier le ruban",
        "Affichage",
        "compact barre outils"
    ),
    command!(
        ResetInterface,
        "Réinitialiser l’interface",
        "Affichage",
        "volets disposition ruban défaut reset"
    ),
    command!(
        Grid,
        "Afficher / masquer la grille",
        "Affichage",
        "quadrillage grid"
    ),
    command!(
        Snap,
        "Activer / désactiver le magnétisme",
        "Affichage",
        "aligner grille snap"
    ),
    command!(
        Guides,
        "Afficher / masquer les repères",
        "Affichage",
        "alignement smart guides"
    ),
    command!(
        Rulers,
        "Afficher / masquer les règles",
        "Affichage",
        "mesure ruler"
    ),
    command!(
        Templates,
        "Galerie de modèles…",
        "Document",
        "template exemple"
    ),
    command!(
        Symbols,
        "Gérer les bibliothèques de symboles…",
        "Document",
        "formes catalogue"
    ),
    command!(
        Agents,
        "Travailler avec un agent…",
        "Document",
        "ia claude chatgpt mcp json"
    ),
    command!(
        Pages,
        "Gérer les pages…",
        "Document",
        "multipage réordonner"
    ),
    command!(
        PageSettings,
        "Taille et paramètres de page…",
        "Document",
        "dimensions a4 format orientation"
    ),
    command!(
        AddPage,
        "Ajouter une page",
        "Document",
        "nouvelle feuille multipage"
    ),
    command!(
        Validate,
        "Vérifier le document",
        "Document",
        "valider géométrie références validate"
    ),
];

#[derive(Default)]
pub(super) struct PaletteState {
    pub open: bool,
    pub query: String,
    pub selected: usize,
    focus: bool,
}

#[derive(Default)]
pub(super) struct HelpState {
    query: String,
    pub gestures: bool,
}

pub(super) fn normalize(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'à' | 'â' | 'ä' => 'a',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'œ' => 'o',
            '’' | '\'' | '/' | '…' => ' ',
            _ => c,
        })
        .collect()
}

fn matches(spec: &CommandSpec, query: &str) -> bool {
    let haystack = normalize(&format!(
        "{} {} {}",
        spec.label, spec.category, spec.aliases
    ));
    normalize(query)
        .split_whitespace()
        .all(|word| haystack.contains(word))
}

pub(super) fn results(query: &str) -> Vec<&'static CommandSpec> {
    let mut results: Vec<_> = COMMANDS
        .iter()
        .filter(|s| s.command != Command::Palette && matches(s, query))
        .collect();
    let query = normalize(query);
    if !query.trim().is_empty() {
        results.sort_by_key(|s| !normalize(s.label).starts_with(query.trim()));
    }
    results
}

fn shortcut_text(ctx: &egui::Context, spec: &CommandSpec) -> String {
    spec.shortcuts
        .iter()
        .map(|s| ctx.format_shortcut(s))
        .collect::<Vec<_>>()
        .join(" / ")
}

impl DrawApp {
    pub(super) fn command_dialog_blocked(&self) -> bool {
        self.pending.is_some()
            || self.error.is_some()
            || self.show_templates
            || self.save_template_open
            || self.show_symbol_manager
            || self.show_gallery
            || self.show_pdf_export
            || self.show_pages
            || self.page_draft.is_some()
            || self.agent_panel.is_some()
    }

    pub(super) fn open_command_palette(&mut self) {
        if self.command_dialog_blocked() || self.drag.is_some() || self.show_help {
            return;
        }
        self.commit();
        self.context_open = false;
        self.clear_connection();
        self.command_palette = PaletteState {
            open: true,
            focus: true,
            ..Default::default()
        };
    }

    fn close_command_palette(&mut self, ctx: &egui::Context) {
        self.command_palette.open = false;
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("command_query")));
    }

    pub(super) fn command_unavailable(&self, command: Command) -> Option<&'static str> {
        use Command::*;
        match command {
            Undo if !self.history.can_undo() && !self.property_edit => {
                Some("Aucune modification à annuler")
            }
            Redo if !self.history.can_redo() => Some("Aucune modification à rétablir"),
            Copy | Cut | Duplicate | Delete | FitSelection if self.selected.is_empty() => {
                Some("Sélectionnez au moins un objet")
            }
            Bold | Italic | Underline
                if !self.selected.iter().any(|id| self.doc.node(*id).is_some()) =>
            {
                Some("Sélectionnez une forme ou un texte")
            }
            SelectAll if self.doc.object_count() == 0 => Some("La page ne contient aucun objet"),
            AddPage if self.doc.page_count() >= 100 => Some("La limite de 100 pages est atteinte"),
            _ => None,
        }
    }

    pub(super) fn run_command(&mut self, command: Command, ctx: &egui::Context) {
        if let Some(reason) = self.command_unavailable(command) {
            self.status = reason.into();
            return;
        }
        use Command::*;
        use context_menu::Action;
        match command {
            Palette => self.open_command_palette(),
            Help => {
                self.help_state = HelpState::default();
                self.show_help = true;
            }
            New => self.request(Pending::New),
            Open => self.request(Pending::OpenDialog),
            Save => {
                self.save(false);
            }
            SaveAs => {
                self.save(true);
            }
            Close => self.request(Pending::CloseTab(self.active_tab)),
            ExportSvg => self.export(),
            ExportPdf => self.show_pdf_export = true,
            Gallery => self.open_gallery(),
            Undo => self.undo(),
            Redo => self.redo(),
            Copy => self.perform_action(Action::Copy, ctx),
            Cut => self.perform_action(Action::Cut, ctx),
            Duplicate => self.perform_action(Action::Duplicate, ctx),
            Delete => self.perform_action(Action::Delete, ctx),
            SelectAll => self.perform_action(Action::SelectAll, ctx),
            Paste => {
                self.paste_position = None;
                if let Some(text) = ctx.input(|i| {
                    i.events.iter().find_map(|e| {
                        if let egui::Event::Paste(text) = e {
                            Some(text.clone())
                        } else {
                            None
                        }
                    })
                }) {
                    self.paste_text(&text);
                } else {
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
                }
            }
            Bold => self.change_text(|s| s.bold = !s.bold),
            Italic => self.change_text(|s| s.italic = !s.italic),
            Underline => self.change_text(|s| s.underline = !s.underline),
            Select => self.set_tool(Tool::Select),
            Rectangle => self.set_tool(Tool::Shape(ShapeKind::Rectangle)),
            Text => self.set_tool(Tool::Shape(ShapeKind::Text)),
            Connect => self.activate_connect_tool(),
            Pan => self.set_tool(Tool::Pan),
            Line => self.set_tool(Tool::Line),
            Bezier => self.set_tool(Tool::Bezier),
            FitPage => self.fit_next = true,
            FitSelection => self.fit_selection(ctx),
            ActualSize => {
                self.set_zoom(1., self.canvas.center());
                self.fit_next = false;
            }
            Library => self.show_library = !self.show_library,
            Inspector => self.show_inspector = !self.show_inspector,
            Ribbon => self.ribbon_expanded = Some(!self.ribbon_is_expanded(ctx)),
            ResetInterface => self.reset_interface(ctx),
            Grid => self.grid = !self.grid,
            Snap => self.snap = !self.snap,
            Guides => self.smart_guides = !self.smart_guides,
            Rulers => self.rulers = !self.rulers,
            Templates => self.show_templates = true,
            Symbols => self.open_symbol_manager(),
            Agents => self.open_agent_panel(),
            Pages => self.show_pages = true,
            PageSettings => self.open_page_settings(),
            AddPage => self.add_blank_page(),
            Validate => match self.doc.validate() {
                Ok(()) => {
                    self.status = "Document vérifié : géométrie et références valides.".into()
                }
                Err(e) => self.error = Some(e),
            },
        }
    }

    pub(super) fn command_shortcut(&mut self, ctx: &egui::Context, editing: bool) -> bool {
        for spec in COMMANDS {
            if editing
                && !matches!(
                    spec.command,
                    Command::Palette
                        | Command::Help
                        | Command::New
                        | Command::Open
                        | Command::Save
                        | Command::SaveAs
                        | Command::ExportSvg
                        | Command::ExportPdf
                )
            {
                continue;
            }
            for shortcut in spec.shortcuts {
                let exact = ctx.input(|i| {
                    let m = i.modifiers;
                    m.shift == shortcut.modifiers.shift
                        && m.alt == shortcut.modifiers.alt
                        && m.command == shortcut.modifiers.command
                        && (shortcut.modifiers.command || (!m.ctrl && !m.mac_cmd))
                });
                if exact && ctx.input_mut(|i| i.consume_shortcut(shortcut)) {
                    self.run_command(spec.command, ctx);
                    return true;
                }
            }
        }
        false
    }

    pub(super) fn command_dialog(&mut self, ctx: &egui::Context) {
        if !self.command_palette.open {
            return;
        }
        if self.command_dialog_blocked() {
            self.close_command_palette(ctx);
            return;
        }
        if ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(Modifiers::COMMAND, Key::K)))
        {
            self.close_command_palette(ctx);
            return;
        }
        // Tab can move focus to a result or to Fermer. Let that widget own
        // Enter; search navigation must not execute a different row behind it.
        let search_focused = ctx.memory(|m| {
            m.focused()
                .is_none_or(|id| id == egui::Id::new("command_query"))
        }) || self.command_palette.focus;
        let up = search_focused && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp));
        let down =
            search_focused && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown));
        let enter = search_focused && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let mut chosen = None;
        let mut close = false;
        let modal = egui::Modal::new("command_palette".into()).show(ctx, |ui| {
            ui.set_width(620_f32.min(ctx.content_rect().width() - 48.));
            ui.horizontal(|ui| {
                ui.heading("Rechercher une commande");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.button("Fermer").clicked();
                });
            });
            ui.label(RichText::new("Outils, fichiers, pages et affichage").color(SECONDARY));
            ui.add_space(8.);
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.command_palette.query)
                    .id(egui::Id::new("command_query"))
                    .hint_text("Ex. exporter PDF, taille de page, propriétés…")
                    .char_limit(120)
                    .desired_width(f32::INFINITY),
            );
            if self.command_palette.focus {
                response.request_focus();
                self.command_palette.focus = false;
            }
            if response.changed() {
                self.command_palette.selected = 0;
            }
            let entries = results(&self.command_palette.query);
            self.command_palette.selected = self
                .command_palette
                .selected
                .min(entries.len().saturating_sub(1));
            if up || down {
                let count = entries.len();
                if count > 0 {
                    for _ in 0..count {
                        self.command_palette.selected = (self.command_palette.selected
                            + if up { count - 1 } else { 1 })
                            % count;
                        if self
                            .command_unavailable(entries[self.command_palette.selected].command)
                            .is_none()
                        {
                            break;
                        }
                    }
                }
            }
            ui.add_space(8.);
            egui::ScrollArea::vertical()
                .id_salt("command_results")
                .max_height((ctx.content_rect().height() - 250.).clamp(160., 420.))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    for (index, spec) in entries.iter().enumerate() {
                        let reason = self.command_unavailable(spec.command);
                        let selected = index == self.command_palette.selected;
                        let button = egui::Button::new(format!(
                            "{}\n{}",
                            spec.label,
                            reason.unwrap_or(spec.category)
                        ))
                        .selected(selected)
                        .shortcut_text(shortcut_text(ctx, spec));
                        let row = ui.add_enabled(
                            reason.is_none(),
                            button.min_size(vec2(ui.available_width(), 46.)),
                        );
                        if selected && (up || down || response.changed()) {
                            row.scroll_to_me(Some(Align::Center));
                        }
                        if row.clicked() {
                            chosen = Some(spec.command);
                        }
                    }
                    if entries.is_empty() {
                        ui.add_space(16.);
                        ui.label("Aucune commande trouvée.");
                        ui.label("Essayez un outil, un format d’export ou un réglage.");
                        ui.add_space(16.);
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("↑ ↓ Choisir · Entrée Exécuter · Échap Fermer")
                        .size(12.)
                        .color(SECONDARY),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Tous les raccourcis").clicked() {
                        chosen = Some(Command::Help);
                    }
                });
            });
            if enter
                && let Some(spec) = entries.get(self.command_palette.selected)
                && self.command_unavailable(spec.command).is_none()
            {
                chosen = Some(spec.command);
            }
        });
        if close || modal.should_close() || chosen.is_some() {
            self.close_command_palette(ctx);
        }
        if let Some(command) = chosen {
            self.run_command(command, ctx);
        }
    }

    pub(super) fn help_dialog(&mut self, ctx: &egui::Context) {
        if !self.show_help {
            return;
        }
        if self.command_dialog_blocked() {
            self.show_help = false;
            return;
        }
        let mut close = false;
        let modal = egui::Modal::new("keyboard_help".into()).show(ctx, |ui| {
            ui.set_width(660_f32.min(ctx.content_rect().width() - 48.));
            ui.horizontal(|ui| {
                ui.heading("Raccourcis et gestes");
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.button("Fermer").clicked();
                });
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.help_state.gestures, false, "Clavier");
                ui.selectable_value(&mut self.help_state.gestures, true, "Souris et trackpad");
            });
            ui.add_space(8.);
            ui.add(
                egui::TextEdit::singleline(&mut self.help_state.query)
                    .hint_text("Filtrer l’aide…")
                    .char_limit(120)
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.);
            egui::ScrollArea::vertical()
                .id_salt(("shortcut_help", self.help_state.gestures))
                .max_height((ctx.content_rect().height() - 235.).clamp(180., 500.))
                .show(ui, |ui| {
                    let mut count = 0;
                    if self.help_state.gestures {
                        for (gesture, action) in GESTURES {
                            let text = normalize(&format!("{gesture} {action}"));
                            if !normalize(&self.help_state.query)
                                .split_whitespace()
                                .all(|q| text.contains(q))
                            {
                                continue;
                            }
                            ui.label(RichText::new(*gesture).strong());
                            ui.label(*action);
                            ui.add_space(8.);
                            count += 1;
                        }
                    } else {
                        let mut category = "";
                        for spec in COMMANDS.iter().filter(|s| !s.shortcuts.is_empty()) {
                            let keys = shortcut_text(ctx, spec);
                            if !matches(spec, &self.help_state.query)
                                && !normalize(&keys)
                                    .contains(normalize(&self.help_state.query).trim())
                            {
                                continue;
                            }
                            if category != spec.category {
                                category = spec.category;
                                ui.add_space(6.);
                                ui.label(RichText::new(category).strong().color(ACCENT));
                            }
                            ui.horizontal(|ui| {
                                ui.label(spec.label);
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(keys).strong());
                                });
                            });
                            count += 1;
                        }
                    }
                    if count == 0 {
                        ui.label("Aucun raccourci trouvé pour cette recherche.");
                    }
                });
            ui.separator();
            ui.label(
                RichText::new("Les lettres seules agissent sur la feuille, hors saisie de texte.")
                    .size(12.)
                    .color(SECONDARY),
            );
        });
        if close || modal.should_close() {
            self.show_help = false;
        }
    }
}

const GESTURES: &[(&str, &str)] = &[
    (
        "Deux doigts sur le trackpad",
        "Déplacer la feuille dans toutes les directions.",
    ),
    (
        "Pincer / écarter · Ctrl/⌘ + molette",
        "Zoomer autour du pointeur.",
    ),
    (
        "Espace + glisser · bouton milieu",
        "Déplacer la vue sans changer d’outil.",
    ),
    (
        "Clic droit · Maj + F10",
        "Ouvrir le menu contextuel de la sélection.",
    ),
    (
        "Glisser dans le vide",
        "Tracer un cadre de sélection. Maj ajoute les objets à la sélection.",
    ),
    ("Maj + clic", "Sélectionner plusieurs objets."),
    (
        "Flèches · Maj + flèches",
        "Déplacer les objets sélectionnés de 1 ou 10 pixels.",
    ),
    (
        "Maj + poignée",
        "Redimensionner en conservant les proportions.",
    ),
    (
        "Alt / Option + glisser",
        "Placer ou redimensionner sans magnétisme.",
    ),
    (
        "Échap pendant un geste",
        "Annuler ce geste en gardant les modifications précédentes.",
    ),
];
