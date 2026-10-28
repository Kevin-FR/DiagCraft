use diagcraft::{
    chart::{Chart, ChartKind, Primitive, Series},
    document::{
        self, ConnectorKind, Document, DrawingKind, GridStyle, Node, Point, Port, ShapeKind,
        TextAlign, TextStyle,
    },
    gallery, geometry,
    history::History,
    library::{self, Library},
    pdf,
    stroke_style::{self, ArrowHead, LinePattern},
    svg, symbols,
    templates::{self, Template},
};
use eframe::egui::{
    self, Align, Align2, Color32, CornerRadius, FontId, Key, Layout, Pos2, Rect, RichText, Sense,
    Shape, Stroke, StrokeKind, Vec2, pos2, vec2,
};
use i18n::{Language, t};
use std::path::PathBuf;

const INK: Color32 = Color32::from_rgb(35, 51, 65);
const MUTED: Color32 = Color32::from_rgb(131, 143, 155);
const ACCENT: Color32 = Color32::from_rgb(0, 125, 250);
const BORDER: Color32 = Color32::from_rgb(229, 234, 239);
const SURFACE: Color32 = Color32::from_rgb(248, 250, 252);
const PALETTE: [[u8; 3]; 8] = [
    [233, 246, 241],
    [237, 241, 254],
    [255, 246, 225],
    [252, 237, 236],
    [235, 246, 253],
    [246, 237, 252],
    [255, 255, 255],
    [42, 66, 80],
];

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Select,
    Shape(ShapeKind),
    Connect,
    Pan,
    Line,
    Bezier,
}
enum Drag {
    Marquee {
        start: Point,
        current: Point,
        initial: Vec<u64>,
        additive: bool,
    },
    Connect {
        press: Pos2,
    },
    FreeConnector {
        id: u64,
        start: Point,
        original: document::Connector,
    },
    Move {
        start: Point,
        original: Vec<(u64, Point)>,
        paths: Vec<(u64, Point, Point)>,
        charts: Vec<(u64, Point)>,
        edges: Vec<document::Connector>,
    },
    Resize {
        id: u64,
        start: Point,
        size: Point,
        origin: Point,
        handle: [i8; 2],
    },
    Endpoint {
        id: u64,
        start: bool,
        point: Point,
    },
    Pan {
        start: Pos2,
        original: Vec2,
    },
    Waypoint {
        id: u64,
        index: usize,
        start: Point,
        origin: Point,
    },
    DrawPath {
        id: u64,
    },
}
#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum LibraryPanel {
    Models,
    Symbols,
    Text,
    Charts,
    Projects,
}

#[derive(Clone)]
enum Pending {
    New,
    Open(PathBuf),
    OpenDialog,
    Close,
    CloseTab(usize),
}

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum RibbonTab {
    #[default]
    Home,
    Insert,
    Design,
    View,
    Symbols,
    Advanced,
}

#[derive(Clone)]
struct Session {
    recovered: bool,
    doc: Document,
    saved: Document,
    path: Option<PathBuf>,
    history: History,
    selected: Vec<u64>,
    zoom: f32,
    pan: Vec2,
    grid: bool,
    snap: bool,
    smart_guides: bool,
    rulers: bool,
    margins: bool,
    page_breaks: bool,
    fit_next: bool,
}

pub struct DrawApp {
    recovered: bool,
    recovery: recovery_ui::Recovery,
    doc: Document,
    saved: Document,
    history: History,
    path: Option<PathBuf>,
    selected: Vec<u64>,
    tool: Tool,
    connect_from: Option<u64>,
    connect_origin: Option<Point>,
    connect_port: Port,
    hover_link_node: Option<u64>,
    capture_link_preview: bool,
    connector_kind: ConnectorKind,
    drag: Option<Drag>,
    zoom: f32,
    pan: Vec2,
    fit_next: bool,
    canvas: Rect,
    grid: bool,
    snap: bool,
    smart_guides: bool,
    status: String,
    error: Option<String>,
    pending: Option<Pending>,
    allow_close: bool,
    screenshot: Option<PathBuf>,
    frame_count: u32,
    show_help: bool,
    command_palette: commands_ui::PaletteState,
    object_search: object_search_ui::Search,
    help_state: commands_ui::HelpState,
    ribbon_tab: RibbonTab,
    ribbon_expanded: Option<bool>,
    rulers: bool,
    margins: bool,
    page_breaks: bool,
    show_library: bool,
    show_inspector: bool,
    library_width: f32,
    inspector_width: f32,
    interface_path: Option<PathBuf>,
    language: Option<Language>,
    settings: settings_ui::Settings,
    symbol_search: String,
    clipboard: Option<Document>,
    tabs: Vec<Session>,
    active_tab: usize,
    format_brush: Option<Node>,
    table_rows: u32,
    table_columns: u32,
    library_panel: LibraryPanel,
    library: Library,
    library_path: Option<PathBuf>,
    builtin_templates: Vec<Template>,
    show_templates: bool,
    template_search: String,
    template_filter: String,
    save_template_open: bool,
    template_name: String,
    template_category: String,
    pending_symbol: Option<Node>,
    context_open: bool,
    context_position: Point,
    context_screen: Pos2,
    show_quick_toolbar: bool,
    paste_position: Option<Point>,
    focus_text: Option<u64>,
    rich_editor: Option<rich_text_ui::Editor>,
    text_click: Option<(u64, Pos2, f64)>,
    replay_input: Vec<egui::Event>,
    capture_menu: Option<String>,
    show_symbol_manager: bool,
    visible_draft: Vec<String>,
    manager_search: String,
    manager_category: String,
    gallery: gallery::Gallery,
    show_gallery: bool,
    gallery_search: String,
    gallery_versions: Option<String>,
    gallery_previews: Vec<(gallery::Project, Option<Document>)>,
    gallery_revision_previews: Vec<Option<Document>>,
    show_pdf_export: bool,
    pdf_format: pdf::PageFormat,
    pdf_all_pages: bool,
    show_pages: bool,
    page_draft: Option<pages_ui::PageDraft>,
    logo: Option<egui::TextureHandle>,
    toolbar_pinned: bool,
    toolbar_position: Option<Pos2>,
    quick_link_port: Option<Port>,
    edge_brush: Option<document::Connector>,
    agent_panel: Option<agent_ui::AgentPanel>,
    capture_shadow_presets: bool,
    capture_marquee: bool,
    style_tab: appearance_ui::StyleTab,
    styled_cache: appearance_ui::SharedCache,
    route_cache: std::cell::RefCell<routing_ui::Cache>,
    property_edit: bool,
    drag_snapshot: Option<gestures_ui::DragSnapshot>,
    keyboard_move: Option<keyboard_ui::Move>,
    keyboard_move_cancelled: bool,
    move_guides: Vec<alignment_ui::Guide>,
    capture_alignment: bool,
    show_layers: bool,
    drawing_layer: u64,
    organization_page: usize,
    layer_names: std::collections::BTreeMap<u64, String>,
}

impl DrawApp {
    pub fn capture_tab(&mut self, name: &str) {
        if name == "routage" {
            self.prepare_routing_capture();
            return;
        }
        if name == "organisation" {
            self.prepare_organization_capture();
            return;
        }
        if name == "recuperation" {
            self.prepare_recovery_capture();
        }
        self.ribbon_tab = match name {
            "inserer" | "ruban-inserer" => RibbonTab::Insert,
            "conception" => RibbonTab::Design,
            "affichage" => RibbonTab::View,
            "symboles" => RibbonTab::Symbols,
            "avance" => RibbonTab::Advanced,
            _ => RibbonTab::Home,
        };
        if name == "parametres" {
            self.settings.open = true;
            self.show_library = false;
        }
        if matches!(name, "interface" | "ruban-inserer") {
            self.ribbon_expanded = Some(false);
            self.show_library = true;
            self.show_inspector = name == "interface";
            self.show_quick_toolbar = false;
            if name == "interface" {
                self.selected = self
                    .doc
                    .nodes
                    .first()
                    .map(|n| vec![n.id])
                    .unwrap_or_default();
            }
        }
        if matches!(name, "volets" | "volets-fermes") {
            self.show_library = name == "volets";
            self.show_inspector = name == "volets";
            self.library_panel = LibraryPanel::Projects;
            self.show_quick_toolbar = false;
            self.ribbon_tab = RibbonTab::Advanced;
            self.ribbon_expanded = Some(name == "volets");
        }
        match name {
            "remplissage" | "contours" | "ombres" | "ombres-prereglages" | "selection-multiple"
            | "selection-contours" | "selection-ombres" | "selection-cadre" => {
                self.replace(diagcraft::appearance::demo(), None);
                self.show_library = false;
                self.show_inspector = true;
                self.show_quick_toolbar = false;
            }
            "graphiques" => {
                self.library_panel = LibraryPanel::Charts;
                self.replace(templates::charts(), None);
            }
            "bibliotheques" => self.open_symbol_manager(),
            "nouveaux-symboles" => {
                self.replace(symbols::demo(), None);
                self.show_inspector = false;
                self.library.visible_libraries =
                    vec!["uml".into(), "blocks".into(), "basic".into()];
            }
            "travaux" | "versions" => {
                for (i, t) in templates::catalog().into_iter().take(6).enumerate() {
                    let path = PathBuf::from(format!("/exemples/{}.draw", t.id));
                    let _ = self.gallery.record(&path, &t.document);
                    if i == 0 {
                        let mut edited = t.document;
                        edited.title = "Traitement d’une demande".into();
                        if let Some(n) = edited.node_mut(1) {
                            n.text = "Demande précisée".into();
                        }
                        let _ = self.gallery.record(&path, &edited);
                    }
                }
                self.open_gallery();
                if name == "versions"
                    && let Some(key) = self.gallery.projects().last().map(|p| p.key.clone())
                {
                    self.open_versions(key);
                }
            }
            "pages" | "tailles-pages" | "connecteurs-libres" => {
                self.replace(templates::multipage(), None);
                self.show_library = false;
                self.show_inspector = false;
                if name == "pages" {
                    self.show_pages = true;
                } else {
                    self.doc
                        .switch_page(if name == "tailles-pages" { 2 } else { 1 })
                        .unwrap();
                    if name == "tailles-pages" {
                        self.open_page_settings();
                    }
                }
            }
            "lier-blocs" | "lier-apercu" => {
                let mut d = Document {
                    title: "Relier deux blocs".into(),
                    page_size: [1200., 760.],
                    ..Document::default()
                };
                for (kind, at, text) in [
                    (ShapeKind::Rounded, [260., 330.], "Demande reçue"),
                    (ShapeKind::Rectangle, [820., 420.], "Traiter la demande"),
                ] {
                    let id = d.add_node(kind, at);
                    d.node_mut(id).unwrap().text = text.into();
                }
                self.replace(d, None);
                self.show_inspector = false;
            }
            "modeles" => self.show_templates = true,
            "support"
            | "contexte-forme"
            | "contexte-connecteur"
            | "contexte-page"
            | "barre-contextuelle"
            | "barre-connecteur"
            | "connexions-rapides" => {
                self.replace(templates::support(), None);
                self.show_inspector = false;
            }
            _ => {}
        }
        self.selected = self
            .doc
            .nodes
            .iter()
            .find(|n| n.kind != ShapeKind::Text)
            .map(|n| vec![n.id])
            .unwrap_or_default();
        if name == "support"
            || name.starts_with("contexte-")
            || name == "barre-contextuelle"
            || name == "barre-connecteur"
            || name == "connexions-rapides"
        {
            self.selected.clear();
            if name != "support" {
                self.show_library = false;
                self.capture_menu = Some(name.into());
            }
        }
        if name == "connecteurs-libres" {
            self.selected = vec![self.doc.connectors[1].id];
            self.toolbar_pinned = true;
            self.toolbar_position = Some(pos2(65., 265.));
        }
        if name == "pages" || name == "tailles-pages" {
            self.selected.clear();
        }
        if name == "graphiques" {
            self.selected = vec![self.doc.charts[0].id];
        }
        let second = Document {
            title: "Diagramme de flux".into(),
            ..Document::default()
        };
        self.open_tab(second, None);
        self.switch_tab(0);
        self.fit_next = true;
        if name == "commandes" {
            self.show_library = false;
            self.show_inspector = false;
            self.open_command_palette();
            self.command_palette.query = "page".into();
        }
        if name == "raccourcis" {
            self.show_help = true;
            self.help_state = Default::default();
        }
        if name == "agents" {
            self.capture_agents();
        }
        if matches!(name, "compact" | "trackpad" | "ruban-compact") {
            self.show_library = false;
            self.show_inspector = false;
            self.show_quick_toolbar = false;
            self.selected.clear();
            self.show_help = name == "trackpad";
            self.help_state.gestures = name == "trackpad";
        }
        if name == "proprietes" {
            self.show_library = false;
            self.show_inspector = true;
            self.show_quick_toolbar = false;
        }
        if matches!(
            name,
            "remplissage" | "contours" | "ombres" | "ombres-prereglages"
        ) {
            self.capture_shadow_presets = name == "ombres-prereglages";
            self.style_tab = match name {
                "contours" => appearance_ui::StyleTab::Line,
                "ombres" | "ombres-prereglages" => appearance_ui::StyleTab::Shadow,
                _ => appearance_ui::StyleTab::Fill,
            };
            self.selected = vec![
                self.doc.nodes[if name.starts_with("ombres") {
                    3
                } else if name == "contours" {
                    6
                } else {
                    0
                }]
                .id,
            ];
        }
        if name.starts_with("selection-") {
            self.style_tab = match name {
                "selection-contours" => appearance_ui::StyleTab::Line,
                "selection-ombres" => appearance_ui::StyleTab::Shadow,
                _ => appearance_ui::StyleTab::Fill,
            };
            self.selected = vec![1, 2, 4];
            if name == "selection-contours" {
                let e = self.doc.connect(1, 2, ConnectorKind::Elbow).unwrap();
                let d = self
                    .doc
                    .add_drawing(DrawingKind::Bezier, [220., 810.], [960., 810.]);
                self.selected.extend([e, d]);
                self.commit();
            }
        }

        if name == "ruban-compact" || name == "selection-cadre" {
            self.ribbon_expanded = Some(false);
        }
        if name == "selection-cadre" {
            self.capture_marquee = true;
            self.show_inspector = false;
        }
        if matches!(name, "alignement" | "reperes") {
            self.prepare_alignment_capture(name == "reperes");
        }
        if name == "redimensionnement" {
            self.prepare_resize_capture();
        }
        if name == "lier-blocs" || name == "lier-apercu" {
            self.show_quick_toolbar = false;
            self.capture_link_preview = name == "lier-apercu";
        }
        if matches!(name, "contenu-bloc" | "editeur-texte") {
            self.prepare_rich_text_capture(name == "editeur-texte");
        }
        if name == "ajuster-texte" {
            self.prepare_text_fit_capture();
        }
        if name == "recherche-objets" {
            self.show_library = false;
            self.show_inspector = false;
            self.ribbon_expanded = Some(false);
            self.open_object_search();
        }
        if matches!(name, "volets" | "volets-fermes") {
            self.selected.clear();
        }
        if name == "placement-clavier" {
            self.prepare_keyboard_capture();
        }
    }
    fn prepare_rich_text_capture(&mut self, editor: bool) {
        let mut doc = Document {
            title: "Un bloc, plusieurs styles".into(),
            page_size: [1000., 650.],
            ..Default::default()
        };
        let title = doc.add_node(ShapeKind::Text, [500., 90.]);
        let node = doc.node_mut(title).unwrap();
        node.text = "Votre contenu, mot par mot".into();
        node.size = [800., 60.];
        node.position = [100., 60.];
        node.font_size = 28.;
        node.text_style.bold = true;
        let id = doc.add_node(ShapeKind::Rounded, [420., 310.]);
        let node = doc.node_mut(id).unwrap();
        node.size = [360., 180.];
        node.position = [240., 220.];
        node.text = "test Le BLOC\n\nSuperbe".into();
        node.font_size = 24.;
        node.fill = [255, 255, 255];
        node.stroke = [35, 51, 65];
        node.text_style.wrap = true;
        node.format_text(
            5..7,
            Some(&diagcraft::rich_text::InlineStyle {
                bold: Some(true),
                ..Default::default()
            }),
        )
        .unwrap();
        node.format_text(
            14..21,
            Some(&diagcraft::rich_text::InlineStyle {
                italic: Some(true),
                underline: Some(true),
                font_size: Some(28.),
                ..Default::default()
            }),
        )
        .unwrap();
        node.format_text(
            19..21,
            Some(&diagcraft::rich_text::InlineStyle {
                bold: Some(true),
                ..Default::default()
            }),
        )
        .unwrap();
        self.replace(doc, None);
        self.selected = vec![id];
        self.show_library = false;
        self.show_inspector = !editor;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        self.focus_text = (!editor).then_some(id);
        self.fit_next = true;
        if editor {
            self.open_node_text(id);
        }
    }
    fn prepare_text_fit_capture(&mut self) {
        let mut doc = Document {
            title: "Du texte, de la place".into(),
            page_size: [1000., 680.],
            ..Default::default()
        };
        let heading = doc.add_node(ShapeKind::Text, [500., 85.]);
        let node = doc.node_mut(heading).unwrap();
        node.text = "Ajuster le bloc, garder les liens".into();
        node.size = [850., 60.];
        node.position = [75., 55.];
        node.font_size = 28.;
        node.text_style.bold = true;
        let a = doc.add_node(ShapeKind::Rounded, [220., 250.]);
        let b = doc.add_node(ShapeKind::Rounded, [680., 250.]);
        for id in [a, b] {
            let node = doc.node_mut(id).unwrap();
            node.size = [240., 72.];
            node.position = [if id == a { 100. } else { 560. }, 190.];
            node.text = "Un libellé peut contenir plusieurs lignes, des accents et du chinois : 中文绘图. Le bloc s’adapte au contenu.".into();
            node.text_style.wrap = true;
        }
        doc.node_mut(b).unwrap().text_style.auto_fit = true;
        doc.fit_automatic_text().unwrap();
        doc.connect(a, b, ConnectorKind::Elbow).unwrap();
        self.replace(doc, None);
        self.selected = vec![b];
        self.focus_text = Some(b);
        self.show_library = false;
        self.show_inspector = true;
        self.show_quick_toolbar = false;
        self.ribbon_expanded = Some(false);
        self.fit_next = true;
    }
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        doc: Document,
        path: Option<PathBuf>,
        screenshot: Option<PathBuf>,
    ) -> Self {
        let persist = screenshot.is_none();
        let mut app = Self::from_context(&cc.egui_ctx, doc, path, screenshot);
        if persist {
            i18n::activate(app.language.unwrap_or_else(Language::system));
            app.interface_path =
                library::default_path().map(|p| p.with_file_name("interface-v1.json"));
            app.restore_interface();
            app.library_path = library::default_path();
            let load_path = app.library_path.as_ref().and_then(|p| {
                if p.exists() {
                    Some(p.clone())
                } else {
                    [
                        "library-v6.json",
                        "library-v5.json",
                        "library-v4.json",
                        "library-v3.json",
                    ]
                    .into_iter()
                    .map(|name| p.with_file_name(name))
                    .find(|p| p.exists())
                }
            });
            if let Some(path) = load_path {
                match Library::load(&path) {
                    Ok(lib) => app.library = lib,
                    Err(e) => {
                        app.error = Some(i18n::render(
                            "Lecture de la bibliothèque : {0}",
                            &[("0", e.to_string())],
                        ))
                    }
                }
            }
            if let Some(root) = gallery::default_root() {
                match gallery::Gallery::open(root) {
                    Ok(gallery) => app.gallery = gallery,
                    Err(e) => {
                        app.gallery = gallery::Gallery::unavailable(e.clone());
                        app.error = Some(format!("Lecture de la galerie : {e}"));
                    }
                }
            }
            if let Some(path) = app.path.clone()
                && let Err(e) = app.gallery.record(&path, &app.doc)
            {
                app.error = Some(i18n::render("Galerie : {0}", &[("0", e.to_string())]));
            }
        }
        if persist {
            app.initialize_recovery();
            app.maybe_check_updates();
        }
        app
    }
    fn from_context(
        ctx: &egui::Context,
        doc: Document,
        path: Option<PathBuf>,
        screenshot: Option<PathBuf>,
    ) -> Self {
        i18n::activate(Language::French);
        ctx.set_theme(egui::Theme::Light);
        ctx.set_visuals(egui::Visuals::light());
        install_fonts(ctx);
        let mut style = (*ctx.style()).clone();
        style.spacing.item_spacing = vec2(10., 9.);
        style.spacing.button_padding = vec2(10., 5.);
        style.spacing.interact_size.y = 30.;
        style.visuals.panel_fill = Color32::WHITE;
        style.visuals.override_text_color = Some(INK);
        style.visuals.widgets.inactive.weak_bg_fill = SURFACE;
        style.visuals.widgets.inactive.bg_fill = SURFACE;
        style.visuals.widgets.inactive.bg_stroke = Stroke::new(1., BORDER);
        style.visuals.widgets.noninteractive.bg_stroke = Stroke::new(1., BORDER);
        style.visuals.widgets.noninteractive.fg_stroke = Stroke::new(1., INK);
        style.visuals.widgets.hovered.weak_bg_fill = Color32::from_rgb(237, 244, 255);
        style.visuals.widgets.hovered.bg_stroke = Stroke::new(1., ACCENT);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(237, 244, 255);
        style.visuals.widgets.active.weak_bg_fill = Color32::from_rgb(224, 238, 255);
        style.visuals.widgets.active.bg_fill = Color32::from_rgb(224, 238, 255);
        style.visuals.widgets.active.bg_stroke = Stroke::new(1., ACCENT);
        style.visuals.selection.bg_fill = Color32::from_rgb(224, 238, 255);
        style.visuals.selection.stroke = Stroke::new(1., ACCENT);
        style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6);
        style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6);
        style.visuals.widgets.active.corner_radius = CornerRadius::same(6);
        style
            .text_styles
            .insert(egui::TextStyle::Body, FontId::proportional(14.));
        style
            .text_styles
            .insert(egui::TextStyle::Button, FontId::proportional(14.));
        if screenshot.is_some() {
            style.animation_time = 0.;
        }
        ctx.set_style(style);
        let organization_page = doc.page_index;
        let mut app = Self {
            recovered: false,
            recovery: Default::default(),
            saved: doc.clone(),
            history: History::new(&doc),
            doc,
            path,
            selected: vec![],
            tool: Tool::Select,
            connect_from: None,
            connect_origin: None,
            connect_port: Port::Auto,
            hover_link_node: None,
            capture_link_preview: false,
            connector_kind: ConnectorKind::Elbow,
            drag: None,
            zoom: 0.8,
            pan: Vec2::ZERO,
            fit_next: true,
            canvas: Rect::NOTHING,
            grid: true,
            snap: true,
            smart_guides: true,
            status: "Tout est prêt. Faites place à vos idées.".into(),
            error: None,
            pending: None,
            allow_close: false,
            screenshot,
            frame_count: 0,
            show_help: false,
            command_palette: Default::default(),
            object_search: Default::default(),
            help_state: Default::default(),
            ribbon_tab: RibbonTab::Home,
            ribbon_expanded: None,
            rulers: true,
            margins: false,
            page_breaks: false,
            show_library: true,
            show_inspector: false,
            library_width: 256.,
            inspector_width: 280.,
            interface_path: None,
            language: None,
            settings: settings_ui::Settings {
                experimental: true,
                ..Default::default()
            },
            symbol_search: String::new(),
            clipboard: None,
            tabs: vec![],
            active_tab: 0,
            format_brush: None,
            table_rows: 3,
            table_columns: 3,
            library_panel: LibraryPanel::Symbols,
            library: Library::default(),
            library_path: None,
            builtin_templates: templates::catalog(),
            show_templates: false,
            template_search: String::new(),
            template_filter: "Recommandé".into(),
            save_template_open: false,
            template_name: String::new(),
            template_category: "Général".into(),
            pending_symbol: None,
            context_open: false,
            context_position: [0., 0.],
            context_screen: Pos2::ZERO,
            show_quick_toolbar: true,
            paste_position: None,
            focus_text: None,
            rich_editor: None,
            text_click: None,
            replay_input: vec![],
            capture_menu: None,
            show_symbol_manager: false,
            visible_draft: vec![],
            manager_search: String::new(),
            manager_category: "Toutes".into(),
            gallery: gallery::Gallery::default(),
            show_gallery: false,
            gallery_search: String::new(),
            gallery_versions: None,
            gallery_previews: vec![],
            gallery_revision_previews: vec![],
            show_pdf_export: false,
            pdf_format: pdf::PageFormat::Drawing,
            pdf_all_pages: true,
            show_pages: false,
            page_draft: None,
            logo: None,
            toolbar_pinned: false,
            toolbar_position: None,
            quick_link_port: None,
            edge_brush: None,
            agent_panel: None,
            capture_shadow_presets: false,
            capture_marquee: false,
            style_tab: Default::default(),
            styled_cache: Default::default(),
            route_cache: Default::default(),
            property_edit: false,
            drag_snapshot: None,
            keyboard_move: None,
            keyboard_move_cancelled: false,
            move_guides: vec![],
            capture_alignment: false,
            show_layers: false,
            drawing_layer: 0,
            organization_page,
            layer_names: Default::default(),
        };
        app.tabs.push(app.session());
        app
    }
    fn add_node(&mut self, kind: ShapeKind, center: Point) -> u64 {
        let id = self.doc.add_node(kind, center);
        let node = self.doc.node_mut(id).unwrap();
        node.text = t(&node.text);
        id
    }
    fn dirty(&self) -> bool {
        self.recovered || self.doc != self.saved
    }
    fn commit(&mut self) {
        let before = self.history.current().clone();
        self.assign_new_objects(&before);
        if let Err(error) = before.check_locked_changes(&self.doc) {
            self.doc = before;
            self.drag = None;
            self.drag_snapshot = None;
            self.keyboard_move = None;
            self.move_guides.clear();
            self.property_edit = false;
            self.normalize_selection();
            self.status = error;
            return;
        }
        let keyboard_move = self.keyboard_move.take();
        if self.drag_snapshot.take().is_some() {
            self.drag = None;
            self.move_guides.clear();
        }
        if let Err(error) = self.doc.fit_automatic_text() {
            self.status = error;
        }
        self.doc.grow_page_to_content();
        if let Err(error) = before.check_locked_changes(&self.doc) {
            self.doc = before;
            self.normalize_selection();
            self.property_edit = false;
            self.status = error;
            return;
        }
        self.normalize_selection();
        self.history.record(&self.doc);
        self.property_edit = false;
        if let Some(movement) = keyboard_move {
            self.status = if movement.changed(&self.doc) {
                "Déplacement terminé · Ctrl/⌘ Z pour annuler"
            } else {
                "Position d’origine retrouvée"
            }
            .into();
        }
    }
    fn finish_frame_edit(&mut self, ctx: &egui::Context, before: &Document) {
        if self.drag.is_none()
            && self.keyboard_move.is_none()
            && (self.property_edit || self.doc != *before)
            && !(self.property_edit && ctx.input(|i| i.pointer.primary_down() && i.focused))
        {
            self.commit();
        }
    }
    fn fit_selection_text(&mut self) {
        if !self.selection_editable() {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        match self.doc.fit_nodes_to_text(&self.selected) {
            Ok(_) => {
                self.commit();
                self.status = "Dimensions ajustées au texte. Les liens suivent les formes.".into();
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn set_tool(&mut self, tool: Tool) {
        self.text_click = None;
        self.tool = tool;
        self.pending_symbol = None;
        self.clear_connection();
    }
    fn undo(&mut self) {
        self.commit();
        if let Some(d) = self.history.undo() {
            if self.doc.page_index != d.page_index {
                self.fit_next = true;
            }
            self.doc = d;
            self.selected.clear();
            self.clear_connection();
            self.status = "Modification annulée".into();
        }
    }
    fn redo(&mut self) {
        self.commit();
        if let Some(d) = self.history.redo() {
            if self.doc.page_index != d.page_index {
                self.fit_next = true;
            }
            self.doc = d;
            self.selected.clear();
            self.clear_connection();
            self.status = "Modification rétablie".into();
        }
    }
    fn delete(&mut self) {
        self.normalize_selection();
        if !self.selection_editable() {
            self.status = "La sélection est verrouillée".into();
            return;
        }
        if let Err(error) = self.doc.try_remove(&self.selected) {
            self.status = error;
            return;
        }
        self.selected.clear();
        self.clear_connection();
        self.commit();
    }
    fn duplicate(&mut self) {
        self.normalize_selection();
        match self.doc.duplicate_objects(&self.selected, [24., 24.]) {
            Ok(ids) => {
                self.selected = ids;
                // Duplicates keep their source layers, including the base layer,
                // regardless of the target selected for newly drawn objects.
                let drawing_layer = std::mem::replace(&mut self.drawing_layer, 0);
                self.commit();
                self.drawing_layer = drawing_layer;
                self.normalize_selection();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn save(&mut self, save_as: bool) -> bool {
        if !self.commit_inline_if_needed() {
            return false;
        }
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .set_title(t("Enregistrer le diagramme"))
                .add_filter(t("Diagramme DiagCraft"), &["draw"])
                .set_file_name(
                    self.path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|n| n.to_str())
                        .unwrap_or("diagramme.draw"),
                )
                .save_file()
        } else {
            self.path.clone()
        };
        let Some(path) = path else {
            return false;
        };
        self.save_to(path)
    }
    fn save_to(&mut self, path: PathBuf) -> bool {
        self.commit();
        let previous_error = self.gallery.preserve_existing(&path).err();
        match self.doc.save(&path) {
            Ok(()) => {
                self.path = Some(path.clone());
                self.saved = self.doc.clone();
                self.recovered = false;
                self.recovery.force = true;
                self.status = "Diagramme enregistré sur cet ordinateur".into();
                let error = self
                    .gallery
                    .record(&path, &self.doc)
                    .err()
                    .or(previous_error);
                if let Some(e) = error {
                    self.error = Some(i18n::render(
                        "Document enregistré ; galerie indisponible : {0}",
                        &[("0", e.to_string())],
                    ));
                }
                true
            }
            Err(e) => {
                self.error = Some(i18n::render(
                    "Impossible d’enregistrer : {0}",
                    &[("0", e.to_string())],
                ));
                false
            }
        }
    }
    fn export(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(t("Exporter en SVG"))
            .add_filter(t("Image vectorielle SVG"), &["svg"])
            .set_file_name("diagramme.svg")
            .save_file()
        else {
            return;
        };
        let result =
            svg::export(&self.doc).and_then(|s| document::atomic_write(&path, s.as_bytes()));
        match result {
            Ok(()) => {
                self.status = i18n::render(
                    "SVG exporté : {0}",
                    &[(
                        "0",
                        format!("{}", path.file_name().unwrap_or_default().to_string_lossy()),
                    )],
                )
            }
            Err(e) => {
                self.error = Some(i18n::render(
                    "Impossible d’exporter : {0}",
                    &[("0", e.to_string())],
                ))
            }
        }
    }
    fn request(&mut self, action: Pending) {
        if !self.commit_inline_if_needed() {
            return;
        }
        self.context_open = false;
        self.commit();
        match action {
            Pending::Close => {
                self.stash_tab();
                if let Some(index) = self
                    .tabs
                    .iter()
                    .position(|t| t.recovered || t.doc != t.saved)
                {
                    self.switch_tab(index);
                    self.pending = Some(Pending::Close);
                } else {
                    self.allow_close = true;
                }
            }
            Pending::CloseTab(index) => {
                self.switch_tab(index);
                if self.dirty() {
                    self.pending = Some(Pending::CloseTab(index));
                } else {
                    self.execute(Pending::CloseTab(index));
                }
            }
            _ => self.execute(action),
        }
    }
    fn execute(&mut self, action: Pending) {
        match action {
            Pending::Close => {
                self.doc = self.saved.clone();
                self.recovered = false;
                self.request(Pending::Close);
            }
            Pending::CloseTab(index) => self.close_tab(index),
            Pending::New => {
                let document = Document {
                    title: t("Sans titre"),
                    page_name: i18n::render("Page {0}", &[("0", "1".into())]),
                    ..Default::default()
                };
                self.open_tab(document, None);
            }
            Pending::OpenDialog => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(t("Ouvrir un diagramme DiagCraft"))
                    .add_filter(t("Diagramme DiagCraft"), &["draw", "json"])
                    .pick_file()
                {
                    self.execute(Pending::Open(path));
                }
            }
            Pending::Open(path) => match Document::load(&path) {
                Ok(doc) => {
                    if let Err(e) = self.gallery.record(&path, &doc) {
                        self.error = Some(format!("Document ouvert ; galerie indisponible : {e}"));
                    }
                    self.open_tab(doc, Some(path));
                }
                Err(e) => {
                    self.error = Some(i18n::render(
                        "Impossible d’ouvrir : {0}",
                        &[("0", e.to_string())],
                    ))
                }
            },
        }
    }
    fn replace(&mut self, doc: Document, path: Option<PathBuf>) {
        self.keyboard_move = None;
        self.keyboard_move_cancelled = false;
        self.property_edit = false;
        self.drag_snapshot = None;
        self.context_open = false;
        self.paste_position = None;
        self.focus_text = None;
        self.recovered = false;
        self.saved = doc.clone();
        self.history = History::new(&doc);
        self.doc = doc;
        self.reset_organization_view();
        self.path = path;
        self.selected.clear();
        self.set_tool(Tool::Select);
        self.drag = None;
        self.fit_next = true;
        self.status = "Document ouvert".into();
    }
    fn session(&self) -> Session {
        Session {
            recovered: self.recovered,
            doc: self.doc.clone(),
            saved: self.saved.clone(),
            path: self.path.clone(),
            history: self.history.clone(),
            selected: self.selected.clone(),
            zoom: self.zoom,
            pan: self.pan,
            grid: self.grid,
            snap: self.snap,
            smart_guides: self.smart_guides,
            rulers: self.rulers,
            margins: self.margins,
            page_breaks: self.page_breaks,
            fit_next: self.fit_next,
        }
    }
    fn stash_tab(&mut self) {
        self.tabs[self.active_tab] = self.session();
    }
    fn switch_tab(&mut self, index: usize) {
        if index >= self.tabs.len() || index == self.active_tab {
            return;
        }
        if !self.commit_inline_if_needed() {
            return;
        }
        self.commit();
        self.stash_tab();
        let s = self.tabs[index].clone();
        self.recovered = s.recovered;
        self.doc = s.doc;
        self.reset_organization_view();
        self.saved = s.saved;
        self.path = s.path;
        self.history = s.history;
        self.selected = s.selected;
        self.zoom = s.zoom;
        self.pan = s.pan;
        self.grid = s.grid;
        self.snap = s.snap;
        self.smart_guides = s.smart_guides;
        self.rulers = s.rulers;
        self.margins = s.margins;
        self.page_breaks = s.page_breaks;
        self.context_open = false;
        self.paste_position = None;
        self.focus_text = None;
        self.active_tab = index;
        self.drag = None;
        self.clear_connection();
        self.tool = Tool::Select;
        self.fit_next = s.fit_next;
    }
    fn open_tab(&mut self, doc: Document, path: Option<PathBuf>) {
        if !self.commit_inline_if_needed() {
            return;
        }
        if let Some(ref path) = path
            && let Some(index) = self.tabs.iter().position(|t| t.path.as_ref() == Some(path))
        {
            self.switch_tab(index);
            return;
        }
        self.commit();
        self.stash_tab();
        self.replace(doc, path);
        self.tabs.push(self.session());
        self.active_tab = self.tabs.len() - 1;
    }
    fn close_tab(&mut self, index: usize) {
        if !self.commit_inline_if_needed() {
            return;
        }
        self.context_open = false;
        self.paste_position = None;
        self.focus_text = None;
        if index >= self.tabs.len() {
            return;
        }
        self.commit();
        self.stash_tab();
        self.tabs.remove(index);
        self.recovery.force = true;
        if self.tabs.is_empty() {
            self.replace(Document::default(), None);
            self.tabs.push(self.session());
            self.active_tab = 0;
            return;
        }
        let target = index.min(self.tabs.len() - 1);
        self.active_tab = usize::MAX;
        let s = self.tabs[target].clone();
        self.recovered = s.recovered;
        self.doc = s.doc;
        self.reset_organization_view();
        self.saved = s.saved;
        self.path = s.path;
        self.history = s.history;
        self.selected = s.selected;
        self.zoom = s.zoom;
        self.pan = s.pan;
        self.grid = s.grid;
        self.snap = s.snap;
        self.smart_guides = s.smart_guides;
        self.rulers = s.rulers;
        self.margins = s.margins;
        self.page_breaks = s.page_breaks;
        self.active_tab = target;
        self.drag = None;
        self.tool = Tool::Select;
        self.clear_connection();
        self.fit_next = s.fit_next;
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.keyboard_move_shortcuts(ctx) {
            return;
        }
        if self.rich_editor.is_some() {
            self.inline_text_shortcuts(ctx);
            if self.rich_editor.is_some() {
                return;
            }
        }
        if self.drag_shortcuts(ctx) {
            return;
        }
        if self.command_palette.open || self.show_help || self.command_dialog_blocked() {
            return;
        }
        if self.context_open {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.context_open = false;
                return;
            }
            if !ctx.wants_keyboard_input() {
                use context_menu::Action;
                let action = ctx.input(|i| {
                    if i.modifiers.command {
                        if i.key_pressed(Key::Z) {
                            return Some(if i.modifiers.shift {
                                Action::Redo
                            } else {
                                Action::Undo
                            });
                        }
                        [
                            (Key::C, Action::Copy),
                            (Key::X, Action::Cut),
                            (Key::V, Action::Paste),
                            (Key::D, Action::Duplicate),
                        ]
                        .into_iter()
                        .find_map(|(key, action)| i.key_pressed(key).then_some(action))
                    } else if i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace) {
                        Some(Action::Delete)
                    } else {
                        None
                    }
                });
                if let Some(action) = action {
                    self.perform_action(action, ctx);
                }
            }
            return;
        }
        if matches!(self.drag, Some(Drag::Marquee { .. })) {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.cancel_marquee();
            }
            return;
        }
        let editing_text = ctx.wants_keyboard_input();
        if !editing_text && ctx.input(|i| i.key_pressed(Key::F10) && i.modifiers.shift) {
            let screen = self
                .selection_rect()
                .map(|r| r.center())
                .unwrap_or(self.canvas.center());
            self.open_context(screen, false);
            return;
        }
        if self.command_shortcut(ctx, editing_text) {
            return;
        }
        if editing_text {
            self.paste_position = None;
            return;
        }
        if !ctx.input(|i| i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt)
            && ctx.input(|i| i.key_pressed(Key::Escape))
        {
            self.set_tool(Tool::Select);
            self.format_brush = None;
            self.edge_brush = None;
            self.selected.clear();
        }
        for event in ctx.input(|i| i.events.clone()) {
            if let egui::Event::Paste(text) = event {
                self.paste_text(&text);
            }
        }
    }
    fn header(&mut self, ctx: &egui::Context) {
        self.ribbon(ctx);
    }
    fn sidebar(&mut self, ctx: &egui::Context) {
        self.library_sidebar(ctx);
    }
    fn inspector(&mut self, ctx: &egui::Context) {
        let before = self.doc.clone();
        let max_width = (ctx.available_rect().width() - panels_ui::MIN_CANVAS).clamp(260., 400.);
        panels_ui::prepare_width(ctx, "inspector", self.inspector_width, 260., max_width);
        let panel = egui::SidePanel::right("inspector").default_width(self.inspector_width).width_range(260. ..=max_width).resizable(true).frame(panels_ui::frame(true, true)).show(ctx,|ui| {
            ui.spacing_mut().item_spacing.y=6.;ui.spacing_mut().interact_size.y=30.;ui.spacing_mut().button_padding.y=5.;
            panels_ui::heading(ui, if self.selected.is_empty() { "Document" } else { "Propriétés" }, Some(&mut self.show_inspector));
            egui::ScrollArea::vertical().id_salt("inspector_scroll").auto_shrink([false, false]).show(ui,|ui| {
                self.organization_inspector(ui);
                self.routing_notice(ui);
                let editable = self.selected.is_empty() || self.selection_editable();
                ui.add_enabled_ui(editable, |ui| {
                if self.selected.len()==1 {
                    let id=self.selected[0];
                    ui.horizontal(|ui|{ui.monospace(i18n::render("page:{0} / objet:{1}", &[("0", format!("{}", self.doc.page_index)),("1", format!("{}", id))]));if ui.small_button(t("Copier")).on_hover_text(t("Référence JSON pour un agent")).clicked(){ctx.copy_text(serde_json::json!({"page":self.doc.page_index,"target":id}).to_string());}});ui.add_space(10.);
                    if let Some(n)=self.doc.node_mut(id) {
                        let (front,back,fit_text,edit_content)=appearance_ui::node_inspector(ui,n,&mut self.style_tab,&self.styled_cache,&mut self.focus_text,self.capture_shadow_presets);
                        if fit_text { self.fit_selection_text(); }
                        if edit_content { self.open_node_text(id); }
                        if (front || back) && let Some(index)=self.doc.nodes.iter().position(|n|n.id==id) {let node=self.doc.nodes.remove(index);if front{self.doc.nodes.push(node);}else{self.doc.nodes.insert(0,node);}}
                    } else if self.doc.charts.iter().any(|c|c.id==id) { self.chart_inspector(ui,id);
                    } else if let Some(edge)=self.doc.connectors.iter_mut().find(|e|e.id==id) {
                        ui.label(RichText::new(t("Connecteur")).size(20.).strong());ui.add_space(18.);field_label(ui,"LIBELLÉ");
                        let response=ui.add(egui::TextEdit::singleline(&mut edge.label).id(egui::Id::new(("edge_label",id))).desired_width(f32::INFINITY).char_limit(1000));if self.focus_text==Some(id){response.request_focus();self.focus_text=None;}
                        appearance_ui::edge_controls(ui,edge);
                        ui.add_space(18.);field_label(ui,"TRACÉ");
                        ui.selectable_value(&mut edge.kind,ConnectorKind::Elbow,t("À angle droit"));
                        ui.selectable_value(&mut edge.kind,ConnectorKind::Straight,t("Ligne directe"));
                        ui.checkbox(&mut edge.arrow_start,t("Flèche au départ"));ui.checkbox(&mut edge.arrow,t("Flèche à l’arrivée"));
                        for (label,port) in [("Départ",&mut edge.from_port),("Arrivée",&mut edge.to_port)] {
                            egui::ComboBox::from_id_salt(label).selected_text(format!("{label} : {}",port.label())).show_ui(ui,|ui|{for p in Port::ALL {ui.selectable_value(port,p,t(p.label()));}});
                        }
                        ui.add(egui::Slider::new(&mut edge.label_t,0.0..=1.0).clamping(egui::SliderClamping::Edits).text(t("Position du texte")));
                        let mut remove=None;
                        for (i,p) in edge.waypoints.iter_mut().enumerate() { ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut p[0]).clamp_existing_to_range(false).range(-100_000.0..=100_000.0).prefix("X "));ui.add(egui::DragValue::new(&mut p[1]).clamp_existing_to_range(false).range(-100_000.0..=100_000.0).prefix("Y "));if ui.small_button("×").clicked(){remove=Some(i);}}); }
                        if let Some(i)=remove{edge.waypoints.remove(i);}
                        if edge.waypoints.len()<16 && ui.button(t("Ajouter un point de passage")).clicked(){edge.waypoints.push(self.doc.page_size.map(|v|v/2.));}
                        for (label,p) in [("Départ libre",&mut edge.free_start),("Arrivée libre",&mut edge.free_end)] {if let Some(p)=p {ui.label(t(label));ui.horizontal(|ui|{for (a,axis) in [(0,"X "),(1,"Y ")] {ui.add(egui::DragValue::new(&mut p[a]).range(-100_000.0..=100_000.0).prefix(axis));}});}}
                        ui.label(t("Glissez une extrémité sur une forme pour l’attacher, ou dans le vide pour la libérer."));

                    } else if let Some(d)=self.doc.drawings.iter_mut().find(|d|d.id==id) {
                        ui.label(RichText::new(t(if d.kind==DrawingKind::Bezier{"Courbe de Bézier"}else{"Trait"})).size(20.).strong());ui.add_space(18.);field_label(ui,"TRACÉ");
                        appearance_ui::line_controls(ui,&mut d.appearance,&mut d.color,&mut d.width,None,None,0.5);
                        ui.checkbox(&mut d.arrow,t("Flèche à l’arrivée"));
                        ui.add(egui::Slider::new(&mut d.appearance.end_scale,0.5..=3.0).text(t("Taille de flèche")));
                        if d.kind==DrawingKind::Bezier {ui.add(egui::Slider::new(&mut d.bend,-300.0..=300.0).clamping(egui::SliderClamping::Edits).text(t("Courbure")));}
                        for (label,p) in [("Départ",&mut d.start),("Arrivée",&mut d.end)] {ui.label(t(label));ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut p[0]).clamp_existing_to_range(false).prefix("X ").range(-100_000.0..=100_000.0));ui.add(egui::DragValue::new(&mut p[1]).clamp_existing_to_range(false).prefix("Y ").range(-100_000.0..=100_000.0));});}
                    }
                } else if self.selected.len()>1 {
                    self.selection_inspector(ui);
                } else {
                    field_label(ui,"NOM DU DIAGRAMME");
                    ui.add(egui::TextEdit::singleline(&mut self.doc.title).id(egui::Id::new("inspector_document_title")).desired_width(f32::INFINITY).char_limit(200));
                    ui.add_space(12.);ui.separator();field_label(ui,"PAGE");
                    ui.label(format!("{} · {:.0} × {:.0} px",self.doc.page_name,self.doc.page_size[0],self.doc.page_size[1]));
                    if ui.button(t("Nom et dimensions de page…")).clicked(){self.open_page_settings();}
                    if ui.button(t("Gérer les pages…")).clicked(){self.show_pages=true;}
                    ui.add_space(12.);ui.separator();
                    ui.label(RichText::new(t("Sélectionnez un objet pour modifier son texte, ses couleurs et sa taille.")).size(12.).color(MUTED));
                }
                });
            });
        });
        panels_ui::remember_width(
            ctx,
            "inspector",
            panel.response.rect,
            &mut self.inspector_width,
        );
        if self.doc != before && ctx.input(|i| i.pointer.primary_down()) {
            self.property_edit = true;
        }
    }
    fn footer(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::bottom("status")
            .exact_height(38.)
            .resizable(false)
            .show_separator_line(false)
            .frame(
                egui::Frame::new()
                    .fill(Color32::WHITE)
                    .inner_margin(egui::Margin::symmetric(20, 8))
                    .stroke(Stroke::new(1., BORDER)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("•").size(9.).color(ACCENT));
                    ui.label(RichText::new(t(&self.status)).size(11.).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(i18n::render(
                                "{0} formes · {1} liens · {2} tracés · {3} graphiques",
                                &[
                                    ("0", format!("{}", self.doc.nodes.len())),
                                    ("1", format!("{}", self.doc.connectors.len())),
                                    ("2", format!("{}", self.doc.drawings.len())),
                                    ("3", format!("{}", self.doc.charts.len())),
                                ],
                            ))
                            .size(11.)
                            .color(MUTED),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new(t("En local, simplement."))
                                .size(11.)
                                .color(MUTED),
                        );
                    });
                });
            });
    }
    fn screen(&self, p: Point) -> Pos2 {
        self.canvas.min + self.pan + vec2(p[0], p[1]) * self.zoom
    }
    fn world(&self, p: Pos2) -> Point {
        let v = (p - self.canvas.min - self.pan) / self.zoom;
        [v.x, v.y]
    }
    fn fit(&mut self) {
        self.zoom = ((self.canvas.width() - 100.) / self.doc.page_size[0])
            .min((self.canvas.height() - 100.) / self.doc.page_size[1])
            .clamp(0.2, 2.5);
        self.pan = (self.canvas.size()
            - vec2(self.doc.page_size[0], self.doc.page_size[1]) * self.zoom)
            / 2.;
        self.fit_next = false;
    }
    fn set_zoom(&mut self, new: f32, anchor: Pos2) {
        if !new.is_finite() || !anchor.is_finite() {
            return;
        }
        let world = self.world(anchor);
        self.zoom = new.clamp(0.2, 2.5);
        self.pan = anchor - self.canvas.min - vec2(world[0], world[1]) * self.zoom;
    }
    fn canvas(&mut self, ctx: &egui::Context, interactive: bool) {
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(241, 244, 247)))
            .show(ctx, |ui| {
                let outer = ui.available_rect_before_wrap();
                let inset = if self.rulers { 24. } else { 0. };
                let rect = Rect::from_min_max(outer.min + vec2(inset, inset), outer.max);
                // Reserve panel resize grips: the canvas is registered after the
                // panels and must not steal drags at their shared boundary.
                let mut hit_rect = rect;
                let grip = ui.style().interaction.resize_grab_radius_side + 1.;
                if self.show_inspector { hit_rect.max.x -= grip; }
                if self.show_library && !self.rulers { hit_rect.min.x += grip; }
                let response = ui.allocate_rect(hit_rect, Sense::click_and_drag());
                self.canvas = rect;
                if self.fit_next {
                    self.fit();
                }
                if self.rulers {
                    self.paint_rulers(ui.painter(), outer);
                }
                if self.capture_link_preview {
                    let source = self.doc.nodes[0].id;
                    let press = connection_buttons(self.node_screen_rect(&self.doc.nodes[0]))[1].0;
                    self.begin_connection(source, Port::Right, None, Some(press));
                }
                if self.capture_marquee {
                    self.selected=self.marquee_hits([70.,70.],[750.,530.]);
                    self.drag=Some(Drag::Marquee {start:[70.,70.],current:[750.,530.],initial:vec![],additive:false});
                }
                if interactive && self.rich_editor.is_none() && !self.context_open && !self.capture_marquee && !self.capture_alignment {
                    self.canvas_input(ui, &response);
                } else if self.rich_editor.is_some() {
                    let pos = ui.input(|i|i.pointer.hover_pos());
                    self.canvas_navigation(ui,ui.rect_contains_pointer(rect),pos);
                }
                if self.drag.is_none() { self.drag_snapshot = None; }
                if !matches!(self.drag, Some(Drag::Move { .. })) { self.move_guides.clear(); }
                let p = ui.painter_at(rect);
                let page = Rect::from_min_size(
                    self.screen([0., 0.]),
                    vec2(self.doc.page_size[0], self.doc.page_size[1]) * self.zoom,
                );
                p.rect_filled(
                    page.translate(vec2(0., 3.)).expand(2.),
                    3,
                    Color32::from_black_alpha(9),
                );
                p.rect_filled(page, 1, rgb(self.doc.page_style.background));
                p.rect_stroke(page, 1, Stroke::new(1., BORDER), StrokeKind::Outside);
                if self.grid {
                    let clipped = page.intersect(rect);
                    let step = self.doc.page_style.grid_spacing * self.zoom;
                    if step >= 5. {
                        let mut x =
                            page.min.x + ((clipped.min.x - page.min.x) / step).ceil() * step;
                        while x < clipped.max.x {
                            if self.doc.page_style.grid_style == GridStyle::Lines {
                                p.line_segment(
                                    [pos2(x, clipped.top()), pos2(x, clipped.bottom())],
                                    Stroke::new(0.6, Color32::from_rgb(222, 225, 230)),
                                );
                            }
                            let mut y =
                                page.min.y + ((clipped.min.y - page.min.y) / step).ceil() * step;
                            while y < clipped.max.y {
                                if self.doc.page_style.grid_style == GridStyle::Dots {
                                    p.circle_filled(
                                        pos2(x, y),
                                        0.75,
                                        Color32::from_rgb(222, 228, 233),
                                    );
                                } else if x
                                    == page.min.x
                                        + ((clipped.min.x - page.min.x) / step).ceil() * step
                                {
                                    p.line_segment(
                                        [pos2(clipped.left(), y), pos2(clipped.right(), y)],
                                        Stroke::new(0.6, Color32::from_rgb(222, 225, 230)),
                                    );
                                }
                                y += step;
                            }
                            x += step;
                        }
                    }
                }
                let p = p.with_clip_rect(page.intersect(rect));
                if self.margins {
                    let r = page.shrink(self.doc.page_style.margin * self.zoom);
                    if r.is_positive() {
                        p.extend(Shape::dashed_line(
                            &[
                                r.left_top(),
                                r.right_top(),
                                r.right_bottom(),
                                r.left_bottom(),
                                r.left_top(),
                            ],
                            Stroke::new(1., Color32::from_rgb(170, 183, 200)),
                            5.,
                            5.,
                        ));
                    }
                }
                if self.page_breaks {
                    let step = 900. * self.zoom;
                    let mut y = page.top() + step;
                    while y < page.bottom() {
                        p.extend(Shape::dashed_line(
                            &[pos2(page.left(), y), pos2(page.right(), y)],
                            Stroke::new(1., Color32::from_rgb(173, 187, 207)),
                            8.,
                            5.,
                        ));
                        y += step;
                    }
                }
                if !self.doc.page_style.watermark.is_empty() {
                    p.text(
                        page.center(),
                        Align2::CENTER_CENTER,
                        &self.doc.page_style.watermark,
                        FontId::proportional(64. * self.zoom),
                        Color32::from_black_alpha(20),
                    );
                }
                let preview = self.inline_preview_document();
                let rendered = preview.as_ref().unwrap_or(&self.doc);
                let organization = rendered.organization_view();
                let routes = self.route_cache.borrow_mut().scene(rendered);
                let mut paths = std::collections::BTreeMap::<u64, Vec<_>>::new();
                let mut edges = std::collections::BTreeMap::<u64, Vec<_>>::new();
                let mut nodes = std::collections::BTreeMap::<u64, Vec<_>>::new();
                let mut charts = std::collections::BTreeMap::<u64, Vec<_>>::new();
                for drawing in &self.doc.drawings { if organization.is_object_visible(drawing.id) { paths.entry(organization.object_layer(drawing.id)).or_default().push(drawing); } }
                for edge in &rendered.connectors { if organization.is_object_visible(edge.id) { edges.entry(organization.object_layer(edge.id)).or_default().push(edge); } }
                for node in &rendered.nodes { if organization.is_object_visible(node.id) { nodes.entry(organization.object_layer(node.id)).or_default().push(node); } }
                for chart in &self.doc.charts { if organization.is_object_visible(chart.id) { charts.entry(organization.object_layer(chart.id)).or_default().push(chart); } }
                for layer in rendered.layer_ids() {
                for d in paths.get(&layer).into_iter().flatten() {
                    let points: Vec<_> = d.points().into_iter().map(|v| self.screen(v)).collect();
                    let color = if self.selected.contains(&d.id) {
                        ACCENT
                    } else {
                        rgb(d.color)
                    };
                    if !d.appearance.is_default() {
                        let edge=diagcraft::appearance_svg::drawing_edge(d);
                        self.styled_cache.borrow_mut().edge(&p,&edge,&d.points(),self.canvas.min+self.pan,self.zoom,self.doc.page_style.background);
                    } else {
                    p.add(Shape::line(
                        points.clone(),
                        Stroke::new(d.width * self.zoom, color),
                    ));
                    if d.arrow {
                        paint_arrow(&p, &points, color, (8. + d.width) * self.zoom);
                    }
                    }
                    if self.selected.contains(&d.id) {
                        for pt in [self.screen(d.start), self.screen(d.end)] {
                            p.circle(pt, 4., Color32::WHITE, Stroke::new(1.5, ACCENT));
                        }
                    }
                }
                for edge in edges.get(&layer).into_iter().flatten() {
                    let routed = &routes[&edge.id];
                    let pts: Vec<_> = routed.points
                        .iter().copied()
                        .map(|v| self.screen(v))
                        .collect();
                    let mut color = if self.selected.contains(&edge.id) {
                        ACCENT
                    } else {
                        rgb(edge.color)
                    };
                    if matches!(self.drag,Some(Drag::Endpoint{id,..}) if id==edge.id) {
                        color = color.gamma_multiply(0.25);
                    }
                    if !edge.appearance.is_default() {
                        self.styled_cache.borrow_mut().edge(&p,edge,&routed.points,self.canvas.min+self.pan,self.zoom,self.doc.page_style.background);
                    } else {
                    paint_pattern(
                        &p,
                        &pts,
                        Stroke::new((edge.width * self.zoom).max(0.5), color),
                        edge.pattern,
                    );
                    if edge.arrow {
                        paint_head(
                            &p,
                            &pts,
                            color,
                            (8. + edge.width) * self.zoom,
                            (edge.width * self.zoom).max(0.5),
                            edge.end_head,
                            rgb(self.doc.page_style.background),
                        );
                    }
                    if edge.arrow_start {
                        let reversed: Vec<_> = pts.iter().rev().copied().collect();
                        paint_head(
                            &p,
                            &reversed,
                            color,
                            (8. + edge.width) * self.zoom,
                            (edge.width * self.zoom).max(0.5),
                            edge.start_head,
                            rgb(self.doc.page_style.background),
                        );
                    }
                    }
                    if !edge.label.is_empty() && pts.len() >= 2 {
                        let pos = self.screen(routed.label_position(edge.label_t));
                        let galley = p.layout_no_wrap(
                            edge.label.clone(),
                            FontId::proportional(14. * self.zoom),
                            color,
                        );
                        let text_rect = Rect::from_center_size(pos, galley.size());
                        p.rect_filled(text_rect.expand(3. * self.zoom), 2, Color32::WHITE);
                        p.galley(text_rect.min, galley, color);
                    }
                }
                for node in nodes.get(&layer).into_iter().flatten() { self.paint_node(&p,node); }
                for chart in charts.get(&layer).into_iter().flatten() {
                    let r = Rect::from_min_size(
                        self.screen(chart.position),
                        vec2(chart.size[0], chart.size[1]) * self.zoom,
                    );
                    paint_chart(&p, chart, r);
                    if self.selected.contains(&chart.id) {
                        paint_selection(&p, r, self.selected.len() == 1 && self.selection_editable() && !matches!(self.drag, Some(Drag::Marquee { .. })));
                    }
                }
                }
                self.paint_group_selection(&p);
                if self.rich_editor.is_none() && !matches!(self.drag, Some(Drag::Marquee { .. })) {
                    self.paint_connector_handles(&p);
                    self.paint_connection_buttons(&p);
                }
                self.paint_connection_preview(&p);
                self.paint_marquee(&p);
                self.paint_alignment_guides(&p);
                self.paint_manipulation_hint(&p);
                if self.doc.nodes.is_empty()
                    && self.doc.drawings.is_empty()
                    && self.doc.charts.is_empty()
                    && self.doc.connectors.is_empty()
                {
                    p.text(
                        page.center() - vec2(0., 14.),
                        Align2::CENTER_CENTER,t("Tout commence par une idée."),
                        FontId::proportional(23.),
                        MUTED,
                    );
                    p.text(
                        page.center() + vec2(0., 22.),
                        Align2::CENTER_CENTER,t("Choisissez une forme à gauche, puis cliquez ici."),
                        FontId::proportional(13.),
                        MUTED,
                    );
                }
                if self.rich_editor.is_none() && (response.hovered() || self.drag.is_some()) {
                    ctx.set_cursor_icon(if self.drag.is_some() {
                        self.manipulation_cursor(ui.input(|i| i.pointer.hover_pos()))
                    } else { match self.tool {
                        Tool::Shape(_) | Tool::Connect | Tool::Line | Tool::Bezier => egui::CursorIcon::Crosshair,
                        Tool::Pan => egui::CursorIcon::Grab,
                        _ => self.manipulation_cursor(ui.input(|i| i.pointer.hover_pos())),
                    }});
                }
                let hint = match self.tool {
                    Tool::Shape(k) => i18n::render("Cliquez pour ajouter : {0} · Entrée pour insérer dans la vue", &[("0", t(k.label()))]),
                    Tool::Connect => {
                        if self.connect_from.is_some() {
                            "Choisissez le bloc d’arrivée · glissez ou cliquez · Échap pour annuler"
                                .into()
                        } else {
                            "Glissez d’un bloc à l’autre, ou cliquez sur le départ puis l’arrivée"
                                .into()
                        }
                    }
                    Tool::Pan => "Glissez ou utilisez deux doigts pour déplacer la vue".into(),
                    Tool::Line => "Glissez pour dessiner un trait".into(),
                    Tool::Bezier => {
                        "Glissez pour dessiner une courbe · Réglez la courbure dans Propriétés"
                            .into()
                    }
                    _ => "Glisser dans le vide : sélectionner · Deux doigts : déplacer · Pincer : zoomer"
                        .into(),
                };
                ui.painter().text(
                    rect.min + vec2(20., 19.),
                    Align2::LEFT_TOP,
                    t(hint),
                    FontId::proportional(11.),
                    MUTED,
                );
                if interactive && self.rich_editor.is_none() {
                    if let Some(name) = self.capture_menu.take() {
                        let point = match name.as_str() {
                            "contexte-connecteur" => {
                                let e = &self.doc.connectors[2];
                                let route = self.cached_route(e.id);
                                [
                                    (route[0][0] + route[1][0]) / 2.,
                                    (route[0][1] + route[1][1]) / 2.,
                                ]
                            }
                            "contexte-page" => [1000., 1250.],
                            _ => self.doc.nodes[2].center(),
                        };
                        if name == "barre-contextuelle" || name == "connexions-rapides" {
                            self.selected = vec![self.doc.nodes[3].id];
                            if name == "connexions-rapides" {
                                self.open_context(
                                    self.screen(self.doc.nodes[3].center()) + vec2(100., 70.),
                                    false,
                                );
                                self.quick_link_port = Some(Port::Bottom);
                            }
                        } else if name == "barre-connecteur" {
                            let id = self.doc.connectors[3].id;
                            self.doc.set_automatic_route(id, false).unwrap();
                            let e = &mut self.doc.connectors[3];
                            e.pattern = LinePattern::Dashed;
                            e.start_head = ArrowHead::Circle;
                            e.end_head = ArrowHead::Open;
                            e.arrow_start = true;
                            self.selected = vec![id];
                            self.toolbar_pinned = true;
                            self.toolbar_position = Some(rect.min + vec2(24., 35.));
                        } else {
                            self.open_context(self.screen(point), true);
                        }
                    }
                    self.canvas_context(ui, &response);
                    if !self.context_open {
                        self.quick_toolbar(ctx);
                    }
                }
                self.inline_text(ui);
                egui::Area::new("zoom_controls".into())
                    .order(egui::Order::Foreground)
                    .default_size(vec2(340., 40.))
                    .constrain_to(rect)
                    .pivot(Align2::RIGHT_BOTTOM)
                    .fixed_pos(rect.right_bottom() - vec2(16., 20.))
                    .show(ctx, |ui| {
                        egui::Frame::new()
                            .fill(Color32::WHITE)
                            .stroke(Stroke::new(1., BORDER))
                            .corner_radius(9)
                            .inner_margin(egui::Margin::symmetric(8, 3))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    if ui.button("−").clicked() {
                                        self.set_zoom(self.zoom / 1.2, rect.center());
                                    }
                                    ui.label(
                                        RichText::new(format!(
                                            "{} %",
                                            (self.zoom * 100.).round() as i32
                                        ))
                                        .size(12.),
                                    ).on_hover_text(t("Trackpad : deux doigts pour déplacer, pincer pour zoomer autour du pointeur. F pour recentrer la page."));
                                    if ui.button("+").clicked() {
                                        self.set_zoom(self.zoom * 1.2, rect.center());
                                    }
                                    ui.separator();
                                    if ui.button(t("Ajuster")).on_hover_text(t("F")).clicked() {
                                        self.fit_next = true;
                                    }
                                    if !self.selected.is_empty() && ui.button(t("Sélection")).on_hover_text(t("Zoom sur la sélection · Maj + F")).clicked() {
                                        self.fit_selection(ctx);
                                    }
                                });
                            });
                    });
            });
    }
    fn canvas_input(&mut self, ui: &egui::Ui, response: &egui::Response) {
        let snap_step = self.doc.page_style.grid_spacing / 2.;
        let pos = ui.input(|i| i.pointer.interact_pos());
        let primary_pressed = ui.input(|i| i.pointer.primary_pressed());
        let over = response.hovered()
            || (response.contains_pointer()
                && pos.is_some_and(|p| ui.ctx().layer_id_at(p) == Some(response.layer_id)));
        self.canvas_navigation(ui, over, pos);
        if self.update_marquee(ui, pos) {
            return;
        }
        self.update_link_hover(pos, over);
        let pan_pressed = ui.input(|i| {
            i.pointer.button_pressed(egui::PointerButton::Middle)
                || (primary_pressed && (i.key_down(Key::Space) || self.tool == Tool::Pan))
        });
        if over
            && pan_pressed
            && let Some(pos) = pos
        {
            self.drag = Some(Drag::Pan {
                start: pos,
                original: self.pan,
            });
        } else if over
            && primary_pressed
            && let Some(pos) = pos
        {
            let world = self.world(pos);
            match self.tool {
                Tool::Shape(kind) => {
                    if self.doc.object_count() >= 10_000 {
                        self.error = Some("Limite de 10 000 objets atteinte.".into());
                        return;
                    }
                    let id = self.add_node(kind, world);
                    let page = self.doc.edit_bounds();
                    let n = self.doc.node_mut(id).unwrap();
                    if let Some(mut custom) = self.pending_symbol.take() {
                        custom.id = id;
                        custom.position = [
                            world[0] - custom.size[0] / 2.,
                            world[1] - custom.size[1] / 2.,
                        ];
                        *n = custom;
                    } else {
                        n.text_style.wrap = true;
                    }
                    n.position[0] = n.position[0].clamp(0., (page[0] - n.size[0]).max(0.));
                    n.position[1] = n.position[1].clamp(0., (page[1] - n.size[1]).max(0.));
                    self.selected = vec![id];
                    self.set_tool(Tool::Select);
                    self.commit();
                }
                Tool::Line | Tool::Bezier => {
                    if self.doc.object_count() >= 10_000 {
                        self.error = Some("Limite de 10 000 objets atteinte.".into());
                        return;
                    }
                    self.begin_edit_drag();
                    let id = self.doc.add_drawing(
                        if self.tool == Tool::Line {
                            DrawingKind::Line
                        } else {
                            DrawingKind::Bezier
                        },
                        world,
                        world,
                    );
                    self.selected = vec![id];
                    self.drag = Some(Drag::DrawPath { id });
                }
                Tool::Connect => {
                    if self.connect_from.is_some() {
                        if self
                            .connection_target(pos)
                            .is_some_and(|t| Some(t.id) == self.connect_from)
                        {
                            self.drag = Some(Drag::Connect { press: pos });
                        } else {
                            self.finish_connection(pos);
                        }
                    } else {
                        let target = self.connection_target(pos);
                        self.begin_connection(
                            target.map_or(0, |t| t.id),
                            target.map_or(Port::Auto, |t| t.port),
                            target.is_none().then_some(world),
                            Some(pos),
                        );
                    }
                }
                Tool::Select => {
                    let top_hit = self.organized_hit(world);
                    if let Some((id, port)) = self.connection_handle_at(pos) {
                        self.begin_connection(id, port, None, Some(pos));
                        return;
                    }
                    let handle = self.resize_handle_at(pos);
                    let endpoint = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1 && self.selection_editable())
                        .and_then(|id| self.doc.connectors.iter().find(|e| e.id == *id))
                        .and_then(|e| {
                            let r = self.cached_route(e.id);
                            [r.first().map(|p| (true, *p)), r.last().map(|p| (false, *p))]
                                .into_iter()
                                .flatten()
                                .find(|(_, p)| self.screen(*p).distance(pos) <= 9.)
                                .map(|(start, point)| (e.id, start, point))
                        });
                    let segment = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1 && self.selection_editable())
                        .and_then(|id| {
                            self.segment_handles(*id)
                                .into_iter()
                                .find(|p| self.screen(*p).distance(pos) <= 7.)
                                .map(|p| (*id, p))
                        });
                    let waypoint = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1 && self.selection_editable())
                        .and_then(|id| {
                            self.doc
                                .connectors
                                .iter()
                                .find(|e| e.id == *id && e.kind == ConnectorKind::Elbow)
                        })
                        .and_then(|e| {
                            e.waypoints
                                .iter()
                                .enumerate()
                                .find(|(_, p)| self.screen(**p).distance(pos) <= 9.)
                                .map(|(index, _)| (e.id, index))
                        });
                    if let Some((id, start, point)) = endpoint {
                        self.begin_edit_drag();
                        self.drag = Some(Drag::Endpoint { id, start, point });
                    } else if let Some((id, index)) = waypoint {
                        self.begin_edit_drag();
                        let origin = self
                            .doc
                            .connectors
                            .iter()
                            .find(|e| e.id == id)
                            .unwrap()
                            .waypoints[index];
                        self.drag = Some(Drag::Waypoint {
                            id,
                            index,
                            start: world,
                            origin,
                        });
                    } else if let Some((id, point)) = segment {
                        self.begin_edit_drag();
                        match self.doc.add_connector_waypoint(id, point) {
                            Ok(()) => {
                                let index = self
                                    .doc
                                    .connectors
                                    .iter()
                                    .find(|e| e.id == id)
                                    .unwrap()
                                    .waypoints
                                    .iter()
                                    .position(|p| *p == point)
                                    .unwrap();
                                self.drag = Some(Drag::Waypoint {
                                    id,
                                    index,
                                    start: world,
                                    origin: point,
                                });
                            }
                            Err(e) => {
                                self.drag_snapshot = None;
                                self.error = Some(e);
                            }
                        }
                    } else if let Some((id, origin, size, handle)) = handle {
                        self.begin_edit_drag();
                        if let Some(node) = self.doc.node_mut(id) {
                            node.text_style.auto_fit = false;
                        }
                        self.drag = Some(Drag::Resize {
                            id,
                            start: world,
                            size,
                            origin,
                            handle,
                        });
                    } else if let Some(id) =
                        self.doc.hit_chart(world).filter(|id| Some(*id) == top_hit)
                    {
                        self.select_for_drag(id, world, ui.input(|i| i.modifiers.shift));
                    } else if let Some(id) =
                        self.doc.hit_node(world).filter(|id| Some(*id) == top_hit)
                    {
                        if let Some(source) = self
                            .format_brush
                            .take()
                            .filter(|_| !self.doc.is_object_locked(id))
                        {
                            if let Some(n) = self.doc.node_mut(id) {
                                n.fill = source.fill;
                                n.stroke = source.stroke;
                                n.stroke_width = source.stroke_width;
                                n.text_color = source.text_color;
                                n.font_size = source.font_size;
                                n.text_style = source.text_style;
                                n.appearance = source.appearance;
                            }
                            self.commit();
                        }
                        self.select_for_drag(id, world, ui.input(|i| i.modifiers.shift));
                    } else if let Some(id) = self
                        .hit_connector_cached(world, 7. / self.zoom)
                        .filter(|id| Some(*id) == top_hit)
                    {
                        if let Some(source) = self
                            .edge_brush
                            .take()
                            .filter(|_| !self.doc.is_object_locked(id))
                        {
                            let target =
                                self.doc.connectors.iter_mut().find(|e| e.id == id).unwrap();
                            target.color = source.color;
                            target.width = source.width;
                            target.arrow = source.arrow;
                            target.arrow_start = source.arrow_start;
                            target.start_head = source.start_head;
                            target.end_head = source.end_head;
                            target.pattern = source.pattern;
                            target.appearance = source.appearance;
                            self.commit();
                        }
                        self.select_for_drag(id, world, ui.input(|i| i.modifiers.shift));
                        if self.selected.len() == 1
                            && self.selected.contains(&id)
                            && self.selection_editable()
                        {
                            let e = self.doc.connectors.iter().find(|e| e.id == id).unwrap();
                            self.drag = if e.from == 0 || e.to == 0 {
                                Some(Drag::FreeConnector {
                                    id,
                                    start: world,
                                    original: e.clone(),
                                })
                            } else {
                                None
                            };
                        }
                    } else if let Some(id) = self
                        .doc
                        .hit_drawing(world, 7. / self.zoom)
                        .filter(|id| Some(*id) == top_hit)
                    {
                        self.select_for_drag(id, world, ui.input(|i| i.modifiers.shift));
                    } else {
                        self.drag = Some(Drag::Marquee {
                            start: world,
                            current: world,
                            initial: self.selected.clone(),
                            additive: ui.input(|i| i.modifiers.shift),
                        });
                        self.hover_link_node = None;
                    }
                }
                Tool::Pan => {}
            }
        }
        if let (Some(drag), Some(pos)) = (&self.drag, pos) {
            let world = self.world(pos);
            let page = self.doc.edit_bounds();
            match drag {
                Drag::Marquee { .. } => {}
                Drag::Move {
                    start,
                    original,
                    paths,
                    charts,
                    edges,
                } => {
                    let (delta, guides) = self.move_placement(
                        original,
                        charts,
                        [world[0] - start[0], world[1] - start[1]],
                        ui.input(|i| i.modifiers.alt),
                    );
                    self.move_guides = guides;
                    for (id, old) in original {
                        if let Some(n) = self.doc.node_mut(*id) {
                            n.position[0] = (old[0] + delta[0]).clamp(-100_000., 100_000.);
                            n.position[1] = (old[1] + delta[1]).clamp(-100_000., 100_000.);
                        }
                    }
                    for (id, old) in charts {
                        if let Some(c) = self.doc.charts.iter_mut().find(|c| c.id == *id) {
                            for a in 0..2 {
                                c.position[a] = (old[a] + delta[a]).clamp(-100_000., 100_000.);
                            }
                        }
                    }
                    for original in edges {
                        if let Some(e) =
                            self.doc.connectors.iter_mut().find(|e| e.id == original.id)
                        {
                            for (target, source) in [
                                (&mut e.free_start, original.free_start),
                                (&mut e.free_end, original.free_end),
                            ] {
                                *target = source.map(|p| {
                                    [0, 1].map(|a| (p[a] + delta[a]).clamp(-100_000., 100_000.))
                                });
                            }
                            e.waypoints = original
                                .waypoints
                                .iter()
                                .map(|p| {
                                    [0, 1].map(|a| (p[a] + delta[a]).clamp(-100_000., 100_000.))
                                })
                                .collect();
                        }
                    }
                    for (id, start, end) in paths {
                        if let Some(d) = self.doc.drawings.iter_mut().find(|d| d.id == *id) {
                            for a in 0..2 {
                                d.start[a] = (start[a] + delta[a]).clamp(-100_000., 100_000.);
                                d.end[a] = (end[a] + delta[a]).clamp(-100_000., 100_000.);
                            }
                        }
                    }
                }
                Drag::Resize {
                    id,
                    start,
                    size,
                    origin,
                    handle,
                } => {
                    let minimum = if self.doc.node(*id).is_some() {
                        24.
                    } else {
                        120.
                    };
                    let modifiers = ui.input(|i| i.modifiers);
                    let (position, extent) = gestures_ui::resize_geometry(
                        *origin,
                        *size,
                        *handle,
                        [world[0] - start[0], world[1] - start[1]],
                        gestures_ui::ResizeSettings {
                            bounds: page,
                            minimum,
                            grid: (self.snap && !modifiers.alt).then_some(snap_step),
                            proportional: modifiers.shift,
                        },
                    );
                    if let Some(n) = self.doc.node_mut(*id) {
                        n.position = position;
                        n.size = extent;
                    } else if let Some(c) = self.doc.charts.iter_mut().find(|c| c.id == *id) {
                        c.position = position;
                        c.size = extent;
                    }
                }
                Drag::FreeConnector {
                    id,
                    start,
                    original,
                } => {
                    let mut delta = [world[0] - start[0], world[1] - start[1]];
                    if self.snap && !ui.input(|i| i.modifiers.alt) {
                        delta = delta.map(|v| (v / snap_step).round() * snap_step);
                    }
                    if let Some(e) = self.doc.connectors.iter_mut().find(|e| e.id == *id) {
                        for (target, source) in [
                            (&mut e.free_start, original.free_start),
                            (&mut e.free_end, original.free_end),
                        ] {
                            *target = source.map(|p| {
                                [0, 1].map(|a| (p[a] + delta[a]).clamp(-100_000., 100_000.))
                            });
                        }
                        e.waypoints = original
                            .waypoints
                            .iter()
                            .map(|p| [0, 1].map(|a| (p[a] + delta[a]).clamp(-100_000., 100_000.)))
                            .collect();
                    }
                }
                Drag::Endpoint { .. } | Drag::Connect { .. } => {}
                Drag::Waypoint {
                    id,
                    index,
                    start,
                    origin,
                } => {
                    if let Some(e) = self.doc.connectors.iter_mut().find(|e| e.id == *id)
                        && let Some(p) = e.waypoints.get_mut(*index)
                    {
                        for a in 0..2 {
                            p[a] = if self.snap && !ui.input(|i| i.modifiers.alt) {
                                origin[a] + ((world[a] - start[a]) / snap_step).round() * snap_step
                            } else {
                                origin[a] + world[a] - start[a]
                            };
                            p[a] = p[a].clamp(-100_000., 100_000.);
                        }
                    }
                }
                Drag::Pan { start, original } => self.pan = *original + (pos - *start),
                Drag::DrawPath { id } => {
                    if let Some(d) = self.doc.drawings.iter_mut().find(|d| d.id == *id) {
                        d.end = world.map(|v| v.clamp(-100_000., 100_000.));
                    }
                }
            }
        }
        if self.update_marquee(ui, pos) {
            return;
        }
        if let Some(pos) = pos {
            let world = self.world(pos);
            if let Some(Drag::Endpoint { point, .. }) = &mut self.drag {
                *point = world;
            }
        }
        if let Some(Drag::Connect { press }) = self.drag {
            if ui.input(|i| i.pointer.primary_released()) {
                self.drag = None;
                if !over || pos.is_none() {
                    self.set_tool(Tool::Select);
                    self.status = "Création du lien annulée.".into();
                } else if let Some(pos) = pos
                    && press.distance(pos) >= 6.
                {
                    self.finish_connection(pos);
                }
            }
            return;
        }
        let text_click = response.clicked()
            && self.tool == Tool::Select
            && matches!(self.drag, Some(Drag::Move { .. }));
        if self.drag.is_some() && ui.input(|i| i.pointer.any_released()) {
            if let Some(Drag::Endpoint { id, start, point }) = self.drag {
                if !over {
                    self.drag = None;
                    self.status =
                        "Extrémité conservée : relâchez dans le canevas pour la déplacer.".into();
                    return;
                }
                if let Some(target) = self.connection_target(self.screen(point)) {
                    match self
                        .doc
                        .reconnect_connector(id, start, target.id, target.port)
                    {
                        Ok(()) => self.status = "Extrémité reconnectée".into(),
                        Err(e) => self.status = e,
                    }
                } else {
                    match self.doc.release_connector_end(id, start, point) {
                        Ok(()) => {
                            self.status =
                                "Extrémité libre · glissez-la sur une forme pour la reconnecter."
                                    .into()
                        }
                        Err(e) => self.status = e,
                    }
                }
            }
            if let Some(Drag::DrawPath { id }) = self.drag
                && self.doc.drawings.iter().any(|d| {
                    d.id == id && (d.start[0] - d.end[0]).hypot(d.start[1] - d.end[1]) < 2.
                })
            {
                self.cancel_edit_drag();
                self.status = "Tracé trop court · dessin précédent conservé".into();
                return;
            }
            self.drag = None;
            self.commit();
        }
        if response.clicked() {
            let previous = self.text_click.take();
            if text_click
                && let Some(pos) = pos
                && let Some(id) = self.doc.hit_node(self.world(pos)).filter(|id| {
                    self.selected.contains(id) && self.organized_hit(self.world(pos)) == Some(*id)
                })
            {
                let time = ui.input(|i| i.time);
                let delay = ui.ctx().options(|o| o.input_options.max_double_click_delay);
                if previous.is_some_and(|(last_id, last_pos, last_time)| {
                    last_id == id && last_pos.distance(pos) <= 6. && time - last_time < delay
                }) {
                    self.open_node_text_at(id, Some(pos));
                } else {
                    self.text_click = Some((id, pos, time));
                }
            }
        }
    }
    fn paint_node(&self, p: &egui::Painter, n: &Node) {
        let r = Rect::from_min_size(
            self.screen(n.position),
            vec2(n.size[0], n.size[1]) * self.zoom,
        );
        let fill = rgb(n.fill);
        let stroke = Stroke::new(n.stroke_width * self.zoom, rgb(n.stroke));
        let styled = !n.appearance.is_default()
            && self.styled_cache.borrow_mut().node(p, n, r.min, self.zoom);
        if !styled {
            match n.kind {
                ShapeKind::Rectangle | ShapeKind::Rounded => {
                    p.rect(
                        r,
                        if n.kind == ShapeKind::Rounded {
                            (20. * self.zoom) as u8
                        } else {
                            2
                        },
                        fill,
                        stroke,
                        StrokeKind::Middle,
                    );
                }
                ShapeKind::Ellipse => {
                    p.add(egui::epaint::EllipseShape {
                        center: r.center(),
                        radius: r.size() / 2.,
                        fill,
                        stroke,
                    });
                }
                ShapeKind::Diamond => {
                    p.add(Shape::convex_polygon(
                        vec![
                            r.center_top(),
                            r.right_center(),
                            r.center_bottom(),
                            r.left_center(),
                        ],
                        fill,
                        stroke,
                    ));
                }
                ShapeKind::Text => {}
                kind => paint_geometry(p, kind, r, fill, stroke),
            }
        }
        let editing = self
            .rich_editor
            .as_ref()
            .is_some_and(|editor| editor.draft.id == n.id);
        if !editing {
            let text = n.text_layout();
            let (text_origin, text_size) = diagcraft::text_layout::area(n);
            let job = rich_text_ui::document_job(n, self.zoom, false);
            let galley = p.layout_job(job);
            let x = match n.text_style.align {
                TextAlign::Left => self.screen(text_origin).x,
                TextAlign::Center => r.center().x - galley.size().x / 2.,
                TextAlign::Right => {
                    self.screen([text_origin[0] + text_size[0], 0.]).x - galley.size().x
                }
            };
            let pos = pos2(x, r.center().y - galley.size().y / 2.) - galley.rect.min.to_vec2();
            if text
                .runs
                .iter()
                .flatten()
                .any(|run| run.style.text_style.monospace && run.style.text_style.bold)
            {
                let bold = p.layout_job(rich_text_ui::document_job(n, self.zoom, true));
                p.galley(pos + vec2(self.zoom * 0.6, 0.), bold, rgb(n.text_color));
            }
            p.galley(pos, galley, rgb(n.text_color));
        }
        if editing {
            p.rect_stroke(
                r.expand(2.),
                3,
                Stroke::new(1.5, ACCENT),
                StrokeKind::Outside,
            );
        }
        if !editing && self.selected.contains(&n.id) && self.tool != Tool::Connect {
            paint_selection(
                p,
                r,
                self.selected.len() == 1
                    && self.selection_editable()
                    && !matches!(self.drag, Some(Drag::Marquee { .. })),
            );
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending.clone() {
            egui::Modal::new("unsaved".into()).show(ctx,|ui| {
                ui.set_width(420.);ui.heading(i18n::render("Conserver « {0} » ?", &[("0", self.doc.title.clone())]));ui.add_space(10.);
                ui.label(t("Ce diagramme contient des modifications qui n’ont pas encore été enregistrées."));ui.add_space(16.);
                ui.horizontal(|ui| {
                    if ui.button(t("Annuler")).clicked(){self.pending=None;}
                    if ui.button(t("Ne pas enregistrer")).clicked(){self.pending=None;self.execute(action.clone());}
                    if ui.add(egui::Button::new(RichText::new(t("Enregistrer")).color(Color32::WHITE)).fill(ACCENT)).clicked() && self.save(false) {self.pending=None;self.execute(action);}
                });
            });
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new("error".into()).show(ctx, |ui| {
                ui.set_width(420.);
                ui.heading(t("L’opération n’a pas abouti"));
                ui.add_space(10.);
                ui.label(t(error));
                ui.add_space(12.);
                if ui.button(t("Fermer")).clicked() {
                    self.error = None;
                }
            });
        }
        self.help_dialog(ctx);
    }
}

impl eframe::App for DrawApp {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        let mut replay = std::mem::take(&mut self.replay_input);
        replay.append(&mut input.events);
        input.events = replay;
    }
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        if self.allow_close {
            self.clean_recovery();
        }
        if let Some(job) = self.settings.job.take() {
            job.cancel.store(true, std::sync::atomic::Ordering::Relaxed);
            // Give the worker a bounded opportunity to kill/reap its curl child.
            let _ = job
                .receiver
                .recv_timeout(std::time::Duration::from_millis(500));
        }
        if let Err(error) = self.save_interface() {
            eprintln!("Préférences d’interface : {error}");
        }
    }
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_updates(ctx);
        self.frame_count += 1;
        if let Some(path) = self.screenshot.clone() {
            if self.frame_count == 5 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            }
            for event in ctx.input(|i| i.events.clone()) {
                if let egui::Event::Screenshot { image, .. } = event {
                    let bytes: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
                    match image::save_buffer(
                        &path,
                        &bytes,
                        image.size[0] as u32,
                        image.size[1] as u32,
                        image::ColorType::Rgba8,
                    ) {
                        Ok(()) => {
                            println!("Capture : {}", path.display());
                            self.allow_close = true;
                            self.screenshot = None;
                        }
                        Err(e) => {
                            self.error = Some(e.to_string());
                            self.screenshot = None;
                        }
                    }
                }
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(50));
        }
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            if self.rich_editor.is_some() {
                self.defer_inline_close();
            } else {
                self.request(Pending::Close);
            }
        }
        if self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.inline_outside_press(ctx);
        if let Some(path) = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()))
        {
            self.inline_open(path);
        }
        self.normalize_selection();
        self.shortcuts(ctx);
        let before = self.doc.clone();
        self.header(ctx);
        self.footer(ctx);
        self.pages_bar(ctx);
        self.side_panels(ctx);
        self.canvas(
            ctx,
            self.pending.is_none()
                && self.error.is_none()
                && !self.settings.open
                && !self.recovery.open
                && !self.show_help
                && !self.command_palette.open
                && !self.object_search.open
                && !self.show_templates
                && !self.save_template_open
                && !self.show_symbol_manager
                && !self.show_gallery
                && !self.show_pdf_export
                && !self.show_pages
                && !self.show_layers
                && self.page_draft.is_none()
                && self.agent_panel.is_none()
                && self.rich_editor.is_none(),
        );
        self.finish_frame_edit(ctx, &before);
        self.recovery_dialog(ctx);
        self.tick_recovery(ctx);
        self.settings_dialog(ctx);
        self.command_dialog(ctx);
        self.object_search_dialog(ctx);
        self.library_dialogs(ctx);
        self.workspace_dialogs(ctx);
        self.pages_dialogs(ctx);
        self.layers_dialog(ctx);
        self.agent_dialog(ctx);
        self.inline_text_toolbar(ctx);
        self.dialogs(ctx);
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(format!(
            "{}{} — DiagCraft",
            if self.dirty() { "* " } else { "" },
            self.doc.title
        )));
    }
}

fn rgb(c: [u8; 3]) -> Color32 {
    Color32::from_rgb(c[0], c[1], c[2])
}
fn font_name(style: &TextStyle) -> &'static str {
    match (style.bold, style.italic) {
        (true, true) => "SourceSans3-BoldIt",
        (true, false) => "SourceSans3-Bold",
        (false, true) => "SourceSans3-It",
        _ => "SourceSans3-Regular",
    }
}
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for (name, bytes) in [
        (
            "SourceSans3-Regular",
            include_bytes!("../../../assets/fonts/SourceSans3-Regular.ttf").as_slice(),
        ),
        (
            "SourceSans3-Bold",
            include_bytes!("../../../assets/fonts/SourceSans3-Bold.ttf").as_slice(),
        ),
        (
            "SourceSans3-It",
            include_bytes!("../../../assets/fonts/SourceSans3-It.ttf").as_slice(),
        ),
        (
            "SourceSans3-BoldIt",
            include_bytes!("../../../assets/fonts/SourceSans3-BoldIt.ttf").as_slice(),
        ),
    ] {
        fonts
            .font_data
            .insert(name.into(), egui::FontData::from_static(bytes).into());
        let mut names = vec![name.into()];
        names.extend(fonts.families[&egui::FontFamily::Proportional].clone());
        fonts
            .families
            .insert(egui::FontFamily::Name(name.into()), names);
    }
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "SourceSans3-Regular".into());
    fonts.font_data.insert(
        "DiagCraftHack".into(),
        egui::FontData::from_static(include_bytes!("../../../assets/fonts/Hack-Regular.ttf"))
            .into(),
    );
    fonts.families.insert(
        egui::FontFamily::Name("DiagCraftMono".into()),
        vec!["DiagCraftHack".into()],
    );
    fonts.font_data.insert(
        "NotoSansSC".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansSC-Regular.otf"
        ))
        .into(),
    );
    for family in fonts.families.values_mut() {
        family.push("NotoSansSC".into());
    }
    ctx.set_fonts(fonts);
}
fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(t(label)).size(10.).strong().color(MUTED));
    ui.add_space(4.);
}
fn swatch(ui: &mut egui::Ui, c: [u8; 3], active: bool) -> egui::Response {
    let (r, response) = ui.allocate_exact_size(vec2(21., 21.), Sense::click());
    ui.painter().rect(
        r.shrink(1.),
        5,
        rgb(c),
        Stroke::new(
            if active { 2. } else { 1. },
            if active { ACCENT } else { BORDER },
        ),
        StrokeKind::Middle,
    );
    response
}
fn paint_arrow(p: &egui::Painter, points: &[Pos2], color: Color32, size: f32) {
    if let Some(tip) = points.last()
        && let Some(previous) = points.iter().rev().find(|v| v.distance(*tip) > 0.1)
    {
        let direction = (*tip - *previous).normalized();
        let side = vec2(-direction.y, direction.x);
        p.add(Shape::convex_polygon(
            vec![
                *tip,
                *tip - direction * size + side * size * 0.45,
                *tip - direction * size - side * size * 0.45,
            ],
            color,
            Stroke::NONE,
        ));
    }
}

#[cfg(test)]
#[path = "ui_tests.rs"]
mod tests;

#[path = "ribbon.rs"]
mod ribbon;

#[path = "library_ui.rs"]
mod library_ui;
use library_ui::{paint_chart, paint_geometry, paint_selection};

#[path = "context_menu.rs"]
mod context_menu;

#[path = "workspace_ui.rs"]
mod workspace_ui;

#[path = "interaction_ui.rs"]
mod interaction_ui;
use interaction_ui::{connection_buttons, paint_head, paint_pattern, resize_handles};

#[path = "contextual_toolbar.rs"]
mod contextual_toolbar;

#[path = "pages_ui.rs"]
mod pages_ui;

#[path = "agent_ui.rs"]
mod agent_ui;

#[path = "connection_ui.rs"]
mod connection_ui;

#[path = "panels_ui.rs"]
mod panels_ui;

#[path = "interface_prefs.rs"]
mod interface_prefs;

#[path = "navigation_ui.rs"]
mod navigation_ui;

#[path = "appearance_ui.rs"]
mod appearance_ui;
#[path = "selection_style_ui.rs"]
mod selection_style_ui;
#[path = "selection_ui.rs"]
mod selection_ui;

#[path = "alignment_ui.rs"]
mod alignment_ui;

#[path = "gestures_ui.rs"]
mod gestures_ui;
#[path = "keyboard_ui.rs"]
mod keyboard_ui;

#[path = "commands_ui.rs"]
mod commands_ui;

#[path = "i18n.rs"]
pub(crate) mod i18n;

#[path = "settings_ui.rs"]
mod settings_ui;
#[path = "updates.rs"]
mod updates;

#[path = "recovery.rs"]
mod recovery;
#[path = "recovery_ui.rs"]
mod recovery_ui;

#[path = "object_search_ui.rs"]
mod object_search_ui;
#[path = "rich_text_ui.rs"]
mod rich_text_ui;

#[path = "organization_ui.rs"]
mod organization_ui;
#[path = "routing_ui.rs"]
mod routing_ui;

#[cfg(test)]
#[path = "organization_tests.rs"]
mod organization_tests;
