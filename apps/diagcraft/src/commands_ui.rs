use super::*;
use egui::{KeyboardShortcut, Modifiers};

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
    ExportPng,
    ImportMermaid,
    ExportMermaid,
    ImportImage,
    PasteImage,
    Gallery,
    Recovery,
    Undo,
    Redo,
    Copy,
    Cut,
    Paste,
    Duplicate,
    Group,
    Ungroup,
    Lock,
    Unlock,
    Layers,
    Delete,
    SelectAll,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    Bold,
    Italic,
    Underline,
    FitText,
    EditText,
    FindObject,
    PlaceShape,
    AutoLayout,
    Outline,
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
    Minimap,
    Ribbon,
    ResetInterface,
    Settings,
    Updates,
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
    NextRegion,
    PreviousRegion,
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
    command!(
        Outline,
        "Structure du document",
        "Affichage",
        "outline tree objects layers groups structure arborescence"
    ),
    command!(
        AutoLayout,
        "Mise en page automatique…",
        "Organisation",
        "layout arrange flow organize horizontal vertical"
    ),
    command!(
        Settings,
        "Langue et paramètres…",
        "Application",
        "language langue idioma sprache lingua 中文 settings preferences"
    ),
    command!(
        Updates,
        "Vérifier les mises à jour…",
        "Application",
        "update github version mise à jour actualiser"
    ),
    command!(Palette, "Rechercher une commande…", "Aide", "palette command search", Modifiers::COMMAND => K),
    command!(Help, "Raccourcis et gestes…", "Aide", "aide clavier trackpad help", Modifiers::NONE => F1),
    command!(NextRegion, "Activer la zone suivante", "Affichage", "focus clavier navigation volet ribbon pane next", Modifiers::NONE => F6),
    command!(PreviousRegion, "Activer la zone précédente", "Affichage", "focus clavier navigation volet ribbon pane previous", Modifiers::SHIFT => F6),
    command!(New, "Nouveau document", "Fichier", "créer new", Modifiers::COMMAND => N),
    command!(Open, "Ouvrir un document…", "Fichier", "charger open", Modifiers::COMMAND => O),
    command!(Save, "Enregistrer", "Fichier", "sauvegarder sauver save", Modifiers::COMMAND => S),
    command!(SaveAs, "Enregistrer sous…", "Fichier", "sauvegarder copie save as", Modifiers::COMMAND.plus(Modifiers::SHIFT) => S),
    command!(ExportSvg, "Exporter en SVG…", "Fichier", "export vectoriel image", Modifiers::COMMAND => E),
    command!(ExportPdf, "Exporter en PDF…", "Fichier", "export imprimer impression", Modifiers::COMMAND.plus(Modifiers::SHIFT) => E),
    command!(
        ExportPng,
        "Exporter en PNG…",
        "Fichier",
        "image pixels transparent png"
    ),
    command!(
        ImportMermaid,
        "Importer du Mermaid…",
        "Fichier",
        "mermaid flowchart graph import texte"
    ),
    command!(
        ExportMermaid,
        "Exporter en Mermaid…",
        "Fichier",
        "mermaid flowchart graph export texte"
    ),
    command!(ImportImage,"Importer des images…","Insertion","image png jpeg fichier",Modifiers::COMMAND=>I),
    command!(PasteImage,"Coller une image","Insertion","image clipboard presse papiers",Modifiers::COMMAND.plus(Modifiers::SHIFT)=>I),
    command!(
        Gallery,
        "Mes travaux enregistrés…",
        "Fichier",
        "galerie historique versions"
    ),
    command!(
        Recovery,
        "Récupérer des dessins…",
        "Fichier",
        "recovery crash récupération sauvegarde secours"
    ),
    command!(Close, "Fermer le document", "Fichier", "onglet close"),
    command!(Undo, "Annuler la modification", "Édition", "undo revenir", Modifiers::COMMAND => Z),
    command!(Redo, "Rétablir", "Édition", "redo refaire", Modifiers::COMMAND.plus(Modifiers::SHIFT) => Z, Modifiers::COMMAND => Y),
    command!(Copy, "Copier", "Édition", "copy presse papiers", Modifiers::COMMAND => C),
    command!(Cut, "Couper", "Édition", "cut presse papiers", Modifiers::COMMAND => X),
    command!(Paste, "Coller", "Édition", "paste presse papiers", Modifiers::COMMAND => V),
    command!(Duplicate, "Dupliquer la sélection", "Édition", "copie duplicate", Modifiers::COMMAND => D),
    command!(Group, "Grouper la sélection", "Organisation", "groupe group ensemble", Modifiers::COMMAND => G),
    command!(Ungroup, "Dissocier le groupe", "Organisation", "dégrouper ungroup dissocier", Modifiers::COMMAND.plus(Modifiers::SHIFT) => G),
    command!(Lock, "Verrouiller la sélection", "Organisation", "lock protéger figer", Modifiers::COMMAND => L),
    command!(Unlock, "Déverrouiller la sélection", "Organisation", "unlock libérer", Modifiers::COMMAND.plus(Modifiers::SHIFT) => L),
    command!(
        Layers,
        "Gérer les calques…",
        "Organisation",
        "layers calque visible masquer ordre"
    ),
    command!(Delete, "Supprimer la sélection", "Édition", "effacer delete", Modifiers::NONE => Delete, Modifiers::NONE => Backspace),
    command!(SelectAll, "Tout sélectionner", "Édition", "tous objets select all", Modifiers::COMMAND => A),
    command!(NudgeLeft, "Déplacer la sélection à gauche", "Placement", "position clavier déplacer précision nudge left", Modifiers::NONE => ArrowLeft, Modifiers::SHIFT => ArrowLeft),
    command!(NudgeRight, "Déplacer la sélection à droite", "Placement", "position clavier déplacer précision nudge right", Modifiers::NONE => ArrowRight, Modifiers::SHIFT => ArrowRight),
    command!(NudgeUp, "Déplacer la sélection vers le haut", "Placement", "position clavier déplacer précision nudge up", Modifiers::NONE => ArrowUp, Modifiers::SHIFT => ArrowUp),
    command!(NudgeDown, "Déplacer la sélection vers le bas", "Placement", "position clavier déplacer précision nudge down", Modifiers::NONE => ArrowDown, Modifiers::SHIFT => ArrowDown),
    command!(Bold, "Gras", "Texte", "bold", Modifiers::COMMAND => B),
    command!(Italic, "Italique", "Texte", "italic", Modifiers::COMMAND => I),
    command!(Underline, "Souligné", "Texte", "underline", Modifiers::COMMAND => U),
    command!(
        FitText,
        "Ajuster la forme au texte",
        "Texte",
        "fit text auto size hauteur largeur ajustement dimension"
    ),
    command!(Select, "Outil Sélection", "Outils", "pointeur déplacer select", Modifiers::NONE => V),
    command!(FindObject, "Rechercher un objet…", "Affichage", "trouver texte bloc connecteur identifiant search find object", Modifiers::COMMAND => F),
    command!(PlaceShape, "Placer la forme dans la vue", "Outils", "insérer créer clavier rectangle texte center", Modifiers::NONE => Enter),
    command!(Rectangle, "Outil Rectangle", "Outils", "bloc forme rectangle", Modifiers::NONE => R),
    command!(EditText, "Modifier le texte", "Texte", "édition bloc contenu inline", Modifiers::NONE => F2, Modifiers::NONE => Enter),
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
        Minimap,
        "Afficher / masquer la mini-carte",
        "Affichage",
        "minimap mini map navigation carte overview aperçu vue"
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
    origin: Option<focus_ui::Region>,
}

#[derive(Default)]
pub(super) struct HelpState {
    pub(super) query: String,
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
        t(spec.label),
        t(spec.category),
        spec.aliases
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
        results.sort_by_key(|s| !normalize(&t(s.label)).starts_with(query.trim()));
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
        self.settings.open
            || self.recovery.open
            || self.pending.is_some()
            || self.error.is_some()
            || self.show_templates
            || self.save_template_open
            || self.show_symbol_manager
            || self.show_gallery
            || self.show_pdf_export
            || self.image_ui.png_open
            || self.mermaid_ui.is_some()
            || self.auto_layout_ui.is_some()
            || self.show_pages
            || self.show_layers
            || self.page_draft.is_some()
            || self.agent_panel.is_some()
            || self.rich_editor.is_some()
            || self.object_search.open
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
        self.command_palette.origin = None;
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("command_query")));
    }

    pub(super) fn command_unavailable(&self, command: Command) -> Option<&'static str> {
        use Command::*;
        if matches!(
            command,
            Cut | Delete
                | NudgeLeft
                | NudgeRight
                | NudgeUp
                | NudgeDown
                | Bold
                | Italic
                | Underline
                | FitText
                | EditText
                | Group
                | Ungroup
        ) && !self.selected.is_empty()
            && !self.selection_editable()
        {
            return Some("La sélection est verrouillée");
        }
        match command {
            Duplicate if !self.selected.is_empty() && !self.selection_duplicable() => {
                Some("Affichez et déverrouillez le calque avant de dupliquer")
            }
            Group
                if self.selected.len() < 2
                    || self.selected.iter().any(|id| {
                        self.doc.object_layer(*id) != self.doc.object_layer(self.selected[0])
                    }) =>
            {
                Some("Sélectionnez au moins deux objets modifiables du même calque")
            }
            Ungroup
                if !self
                    .doc
                    .organization
                    .groups
                    .iter()
                    .any(|g| g.members.iter().any(|id| self.selected.contains(id))) =>
            {
                Some("Sélectionnez un groupe")
            }
            Lock | Unlock if self.selected.is_empty() => Some("Sélectionnez au moins un objet"),
            PlaceShape if !matches!(self.tool, Tool::Shape(_)) => {
                Some("Choisissez d’abord une forme à insérer")
            }
            PlaceShape if self.doc.object_count() >= 10_000 => {
                Some("Limite de 10 000 objets atteinte.")
            }
            EditText if self.selected.len() != 1 || self.doc.node(self.selected[0]).is_none() => {
                Some("Sélectionnez une forme ou un texte")
            }
            Undo if !self.history.can_undo()
                && !self.property_edit
                && self.keyboard_move.is_none() =>
            {
                Some("Aucune modification à annuler")
            }
            Redo if !self.history.can_redo() => Some("Aucune modification à rétablir"),
            NudgeLeft | NudgeRight | NudgeUp | NudgeDown if !self.selection_has_movable() => {
                Some("Sélectionnez une forme, un graphique, un tracé ou un lien libre")
            }
            Copy | Cut | Duplicate | Delete | FitSelection if self.selected.is_empty() => {
                Some("Sélectionnez au moins un objet")
            }
            Bold | Italic | Underline | FitText
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
        self.run_command_with_origin(command, ctx, None);
    }

    fn run_command_with_origin(
        &mut self,
        command: Command,
        ctx: &egui::Context,
        origin: Option<focus_ui::Region>,
    ) {
        if self.keyboard_move.is_some()
            && !matches!(
                command,
                Command::NudgeLeft | Command::NudgeRight | Command::NudgeUp | Command::NudgeDown
            )
        {
            self.commit();
        }
        if let Some(reason) = self.command_unavailable(command) {
            self.status = reason.into();
            return;
        }
        use Command::*;
        use context_menu::Action;
        match command {
            Settings => self.settings.open = true,
            Recovery => self.recovery.open = true,
            Updates => {
                self.settings.open = true;
                self.check_updates();
            }
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
            ExportPng => self.open_png_export(),
            ImportMermaid => self.open_mermaid(false),
            ExportMermaid => self.open_mermaid(true),
            ImportImage => self.import_image(),
            PasteImage => {
                if !self.paste_image() {
                    self.status = "Aucune image disponible dans le presse-papiers.".into();
                }
            }
            Gallery => self.open_gallery(),
            Undo => self.undo(),
            Redo => self.redo(),
            Copy => self.perform_action(Action::Copy, ctx),
            Cut => self.perform_action(Action::Cut, ctx),
            Duplicate => self.perform_action(Action::Duplicate, ctx),
            Group => self.group_selection(),
            Ungroup => self.ungroup_selection(),
            Lock => self.set_selection_locked(true),
            Unlock => self.set_selection_locked(false),
            Layers => self.open_layers(),
            Delete => self.perform_action(Action::Delete, ctx),
            SelectAll => self.perform_action(Action::SelectAll, ctx),
            NudgeLeft | NudgeRight | NudgeUp | NudgeDown => {
                let step = if ctx.input(|i| i.modifiers.shift) {
                    10.
                } else {
                    1.
                };
                self.nudge_selection(
                    match command {
                        NudgeLeft => [-step, 0.],
                        NudgeRight => [step, 0.],
                        NudgeUp => [0., -step],
                        _ => [0., step],
                    },
                    ctx.input(|i| {
                        [
                            Key::ArrowLeft,
                            Key::ArrowRight,
                            Key::ArrowUp,
                            Key::ArrowDown,
                        ]
                        .iter()
                        .any(|key| i.key_down(*key))
                    }),
                );
            }
            Paste => {
                self.paste_position = None;
                if self.paste_image() {
                    return;
                }
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
            FitText => self.fit_selection_text(),
            EditText => self.open_node_text(self.selected[0]),
            FindObject => self.open_object_search(),
            PlaceShape => self.place_shape_in_view(ctx),
            AutoLayout => self.open_auto_layout(),
            Outline => {
                self.library_panel = LibraryPanel::Outline;
                self.show_library = true;
            }
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
            Minimap => self.show_minimap = !self.show_minimap,
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
            NextRegion => self.cycle_workspace_focus(ctx, false, origin),
            PreviousRegion => self.cycle_workspace_focus(ctx, true, origin),
        }
    }

    pub(super) fn command_shortcut(&mut self, ctx: &egui::Context, editing: bool) -> bool {
        for spec in COMMANDS {
            if self.keyboard_move_cancelled
                && matches!(
                    spec.command,
                    Command::NudgeLeft
                        | Command::NudgeRight
                        | Command::NudgeUp
                        | Command::NudgeDown
                )
            {
                continue;
            }
            if spec.command == Command::PlaceShape
                && self.command_unavailable(spec.command).is_some()
            {
                continue;
            }
            if editing
                && !matches!(
                    spec.command,
                    Command::Palette
                        | Command::FindObject
                        | Command::Help
                        | Command::New
                        | Command::Open
                        | Command::Save
                        | Command::SaveAs
                        | Command::ExportSvg
                        | Command::ExportPdf
                        | Command::NextRegion
                        | Command::PreviousRegion
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
        if self.command_palette.focus {
            // Capture the real workspace widget before the query takes focus.
            // The modal's position and its result/close buttons are not a zone.
            self.command_palette.origin = Some(self.workspace_focus_region(ctx));
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
        if up || down {
            ctx.memory_mut(|m| m.move_focus(egui::FocusDirection::None));
        }
        let enter = search_focused && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        let mut chosen = None;
        let mut close = false;
        let modal = egui::Modal::new("command_palette".into()).show(ctx, |ui| {
            ui.set_width(620_f32.min(ctx.content_rect().width() - 48.));
            ui.horizontal(|ui| {
                ui.heading(t("Rechercher une commande"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.button(t("Fermer")).clicked();
                });
            });
            ui.label(RichText::new(t("Outils, fichiers, pages et affichage")).color(INFO_TEXT));
            ui.add_space(8.);
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.command_palette.query)
                    .id(egui::Id::new("command_query"))
                    .hint_text(t("Ex. exporter PDF, taille de page, propriétés…"))
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
                            t(spec.label),
                            t(reason.unwrap_or(spec.category))
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
                        ui.label(t("Aucune commande trouvée."));
                        ui.label(t("Essayez un outil, un format d’export ou un réglage."));
                        ui.add_space(16.);
                    }
                });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t("↑ ↓ Choisir · Entrée Exécuter · Échap Fermer"))
                        .size(12.)
                        .color(INFO_TEXT),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button(t("Tous les raccourcis")).clicked() {
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
        let origin = self.command_palette.origin;
        if close || modal.should_close() || chosen.is_some() {
            self.close_command_palette(ctx);
        }
        if let Some(command) = chosen {
            self.run_command_with_origin(command, ctx, origin);
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
                ui.heading(t("Raccourcis et gestes"));
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    close = ui.button(t("Fermer")).clicked();
                });
            });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut self.help_state.gestures, false, t("Clavier"));
                ui.selectable_value(&mut self.help_state.gestures, true, t("Souris et trackpad"));
            });
            ui.add_space(8.);
            ui.add(
                egui::TextEdit::singleline(&mut self.help_state.query)
                    .hint_text(t("Filtrer l’aide…"))
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
                            let text = normalize(&format!("{} {}", t(gesture), t(action)));
                            if !normalize(&self.help_state.query)
                                .split_whitespace()
                                .all(|q| text.contains(q))
                            {
                                continue;
                            }
                            ui.label(RichText::new(t(*gesture)).strong());
                            ui.label(t(*action));
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
                                ui.label(RichText::new(t(category)).strong().color(ACTION_TEXT));
                            }
                            ui.horizontal(|ui| {
                                ui.label(t(spec.label));
                                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                    ui.label(RichText::new(keys).strong());
                                });
                            });
                            count += 1;
                        }
                    }
                    if count == 0 {
                        ui.label(t("Aucun raccourci trouvé pour cette recherche."));
                    }
                });
            ui.separator();
            ui.label(
                RichText::new(t(
                    "Les lettres seules agissent sur la feuille, hors saisie de texte.",
                ))
                .size(12.)
                .color(INFO_TEXT),
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
        "Déplacer de 1 ou 10 pixels. Maintenir une flèche : une seule annulation. Échap annule le déplacement.",
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
