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
    },
    DrawPath {
        id: u64,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
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

#[derive(Clone, Copy, Default, PartialEq, Eq)]
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
    help_state: commands_ui::HelpState,
    ribbon_tab: RibbonTab,
    ribbon_expanded: Option<bool>,
    rulers: bool,
    margins: bool,
    page_breaks: bool,
    show_library: bool,
    show_inspector: bool,
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
    property_edit: bool,
    drag_snapshot: Option<gestures_ui::DragSnapshot>,
    move_guides: Vec<alignment_ui::Guide>,
    capture_alignment: bool,
}

impl DrawApp {
    pub fn capture_tab(&mut self, name: &str) {
        self.ribbon_tab = match name {
            "inserer" => RibbonTab::Insert,
            "conception" => RibbonTab::Design,
            "affichage" => RibbonTab::View,
            "symboles" => RibbonTab::Symbols,
            "avance" => RibbonTab::Advanced,
            _ => RibbonTab::Home,
        };
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
                    Err(e) => app.error = Some(format!("Lecture de la bibliothèque : {e}")),
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
                app.error = Some(format!("Galerie : {e}"));
            }
        }
        app
    }
    fn from_context(
        ctx: &egui::Context,
        doc: Document,
        path: Option<PathBuf>,
        screenshot: Option<PathBuf>,
    ) -> Self {
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
        let mut app = Self {
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
            help_state: Default::default(),
            ribbon_tab: RibbonTab::Home,
            ribbon_expanded: None,
            rulers: true,
            margins: false,
            page_breaks: false,
            show_library: true,
            show_inspector: false,
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
            property_edit: false,
            drag_snapshot: None,
            move_guides: vec![],
            capture_alignment: false,
        };
        app.tabs.push(app.session());
        app
    }
    fn dirty(&self) -> bool {
        self.doc != self.saved
    }
    fn commit(&mut self) {
        if self.drag_snapshot.take().is_some() {
            self.drag = None;
            self.move_guides.clear();
        }
        self.doc.grow_page_to_content();
        self.history.record(&self.doc);
        self.property_edit = false;
    }
    fn finish_frame_edit(&mut self, ctx: &egui::Context, before: &Document) {
        if self.drag.is_none()
            && (self.property_edit || self.doc != *before)
            && !(self.property_edit && ctx.input(|i| i.pointer.primary_down() && i.focused))
        {
            self.commit();
        }
    }
    fn set_tool(&mut self, tool: Tool) {
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
        self.doc.remove(&self.selected);
        self.selected.clear();
        self.clear_connection();
        self.commit();
    }
    fn duplicate(&mut self) {
        let fragment = self.doc.selection(&self.selected);
        match self.doc.paste(&fragment, [24., 24.]) {
            Ok(ids) => {
                self.selected = ids;
                self.commit();
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn save(&mut self, save_as: bool) -> bool {
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .set_title("Enregistrer le diagramme")
                .add_filter("Diagramme DiagCraft", &["draw"])
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
                self.status = "Diagramme enregistré sur cet ordinateur".into();
                let error = self
                    .gallery
                    .record(&path, &self.doc)
                    .err()
                    .or(previous_error);
                if let Some(e) = error {
                    self.error = Some(format!("Document enregistré ; galerie indisponible : {e}"));
                }
                true
            }
            Err(e) => {
                self.error = Some(format!("Impossible d’enregistrer : {e}"));
                false
            }
        }
    }
    fn export(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title("Exporter en SVG")
            .add_filter("Image vectorielle SVG", &["svg"])
            .set_file_name("diagramme.svg")
            .save_file()
        else {
            return;
        };
        let result =
            svg::export(&self.doc).and_then(|s| document::atomic_write(&path, s.as_bytes()));
        match result {
            Ok(()) => {
                self.status = format!(
                    "SVG exporté : {}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                )
            }
            Err(e) => self.error = Some(format!("Impossible d’exporter : {e}")),
        }
    }
    fn request(&mut self, action: Pending) {
        self.context_open = false;
        self.commit();
        match action {
            Pending::Close => {
                self.stash_tab();
                if let Some(index) = self.tabs.iter().position(|t| t.doc != t.saved) {
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
                self.request(Pending::Close);
            }
            Pending::CloseTab(index) => self.close_tab(index),
            Pending::New => self.open_tab(Document::default(), None),
            Pending::OpenDialog => {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title("Ouvrir un diagramme DiagCraft")
                    .add_filter("Diagramme DiagCraft", &["draw", "json"])
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
                Err(e) => self.error = Some(format!("Impossible d’ouvrir : {e}")),
            },
        }
    }
    fn replace(&mut self, doc: Document, path: Option<PathBuf>) {
        self.property_edit = false;
        self.drag_snapshot = None;
        self.context_open = false;
        self.paste_position = None;
        self.focus_text = None;
        self.saved = doc.clone();
        self.history = History::new(&doc);
        self.doc = doc;
        self.path = path;
        self.selected.clear();
        self.set_tool(Tool::Select);
        self.drag = None;
        self.fit_next = true;
        self.status = "Document ouvert".into();
    }
    fn session(&self) -> Session {
        Session {
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
        self.commit();
        self.stash_tab();
        let s = self.tabs[index].clone();
        self.doc = s.doc;
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
        self.context_open = false;
        self.paste_position = None;
        self.focus_text = None;
        if index >= self.tabs.len() {
            return;
        }
        self.commit();
        self.stash_tab();
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.replace(Document::default(), None);
            self.tabs.push(self.session());
            self.active_tab = 0;
            return;
        }
        let target = index.min(self.tabs.len() - 1);
        self.active_tab = usize::MAX;
        let s = self.tabs[target].clone();
        self.doc = s.doc;
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
        let shift = ctx.input(|i| i.modifiers.shift);
        if !ctx.input(|i| i.modifiers.command || i.modifiers.ctrl || i.modifiers.alt) {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.set_tool(Tool::Select);
                self.format_brush = None;
                self.edge_brush = None;
                self.selected.clear();
            }
            let step = if shift { 10. } else { 1. };
            let mut delta = [0., 0.];
            for (key, x, y) in [
                (Key::ArrowLeft, -step, 0.),
                (Key::ArrowRight, step, 0.),
                (Key::ArrowUp, 0., -step),
                (Key::ArrowDown, 0., step),
            ] {
                if ctx.input(|i| i.key_pressed(key)) {
                    delta[0] += x;
                    delta[1] += y;
                }
            }
            if delta != [0., 0.] {
                let page = self.doc.edit_bounds();
                for n in &mut self.doc.nodes {
                    if self.selected.contains(&n.id) {
                        n.position[0] =
                            (n.position[0] + delta[0]).clamp(0., (page[0] - n.size[0]).max(0.));
                        n.position[1] =
                            (n.position[1] + delta[1]).clamp(0., (page[1] - n.size[1]).max(0.));
                    }
                }
                for c in &mut self.doc.charts {
                    if self.selected.contains(&c.id) {
                        for a in 0..2 {
                            c.position[a] =
                                (c.position[a] + delta[a]).clamp(0., (page[a] - c.size[a]).max(0.));
                        }
                    }
                }
                for e in &mut self.doc.connectors {
                    if self.selected.contains(&e.id) && (e.from == 0 || e.to == 0) {
                        for p in [&mut e.free_start, &mut e.free_end]
                            .into_iter()
                            .flatten()
                            .chain(e.waypoints.iter_mut())
                        {
                            for a in 0..2 {
                                p[a] = (p[a] + delta[a]).clamp(-100_000., 100_000.);
                            }
                        }
                    }
                }
                for d in &mut self.doc.drawings {
                    if self.selected.contains(&d.id) {
                        for p in [&mut d.start, &mut d.end] {
                            p[0] = (p[0] + delta[0]).clamp(-100_000., 100_000.);
                            p[1] = (p[1] + delta[1]).clamp(-100_000., 100_000.);
                        }
                    }
                }
                self.commit();
            }
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
        egui::SidePanel::right("inspector").exact_width(254.).resizable(false).show_separator_line(false).frame(egui::Frame::new().fill(Color32::WHITE).inner_margin(egui::Margin::symmetric(20,22)).stroke(Stroke::new(1.,BORDER))).show(ctx,|ui| {
            ui.spacing_mut().item_spacing.y=6.;ui.spacing_mut().interact_size.y=26.;ui.spacing_mut().button_padding.y=4.;
            ui.horizontal(|ui|{ui.label(RichText::new("PROPRIÉTÉS").size(11.).strong().color(MUTED));ui.with_layout(Layout::right_to_left(Align::Center),|ui|{if ui.button("×").on_hover_text("Masquer les propriétés").clicked(){self.show_inspector=false;}});});ui.add_space(10.);
            egui::ScrollArea::vertical().show(ui,|ui| {
                if self.selected.len()==1 {
                    let id=self.selected[0];
                    ui.horizontal(|ui|{ui.monospace(format!("page:{} / objet:{id}",self.doc.page_index));if ui.small_button("Copier").on_hover_text("Référence JSON pour un agent").clicked(){ctx.copy_text(serde_json::json!({"page":self.doc.page_index,"target":id}).to_string());}});ui.add_space(10.);
                    if let Some(n)=self.doc.node_mut(id) {
                        let (front,back)=appearance_ui::node_inspector(ui,n,&mut self.style_tab,&self.styled_cache,&mut self.focus_text,self.capture_shadow_presets);
                        if (front || back) && let Some(index)=self.doc.nodes.iter().position(|n|n.id==id) {let node=self.doc.nodes.remove(index);if front{self.doc.nodes.push(node);}else{self.doc.nodes.insert(0,node);}}
                    } else if self.doc.charts.iter().any(|c|c.id==id) { self.chart_inspector(ui,id);
                    } else if let Some(edge)=self.doc.connectors.iter_mut().find(|e|e.id==id) {
                        ui.label(RichText::new("Connecteur").size(20.).strong());ui.add_space(18.);field_label(ui,"LIBELLÉ");
                        let response=ui.add(egui::TextEdit::singleline(&mut edge.label).id(egui::Id::new(("edge_label",id))).desired_width(f32::INFINITY).char_limit(1000));if self.focus_text==Some(id){response.request_focus();self.focus_text=None;}
                        appearance_ui::edge_controls(ui,edge);
                        ui.add_space(18.);field_label(ui,"TRACÉ");
                        ui.selectable_value(&mut edge.kind,ConnectorKind::Elbow,"À angle droit");
                        ui.selectable_value(&mut edge.kind,ConnectorKind::Straight,"Ligne directe");
                        ui.checkbox(&mut edge.arrow_start,"Flèche au départ");ui.checkbox(&mut edge.arrow,"Flèche à l’arrivée");
                        for (label,port) in [("Départ",&mut edge.from_port),("Arrivée",&mut edge.to_port)] {
                            egui::ComboBox::from_id_salt(label).selected_text(format!("{label} : {}",port.label())).show_ui(ui,|ui|{for p in Port::ALL {ui.selectable_value(port,p,p.label());}});
                        }
                        ui.add(egui::Slider::new(&mut edge.label_t,0.0..=1.0).clamping(egui::SliderClamping::Edits).text("Position du texte"));
                        let mut remove=None;
                        for (i,p) in edge.waypoints.iter_mut().enumerate() { ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut p[0]).clamp_existing_to_range(false).range(-100_000.0..=100_000.0).prefix("X "));ui.add(egui::DragValue::new(&mut p[1]).clamp_existing_to_range(false).range(-100_000.0..=100_000.0).prefix("Y "));if ui.small_button("×").clicked(){remove=Some(i);}}); }
                        if let Some(i)=remove{edge.waypoints.remove(i);}
                        if edge.waypoints.len()<16 && ui.button("Ajouter un point de passage").clicked(){edge.waypoints.push(self.doc.page_size.map(|v|v/2.));}
                        for (label,p) in [("Départ libre",&mut edge.free_start),("Arrivée libre",&mut edge.free_end)] {if let Some(p)=p {ui.label(label);ui.horizontal(|ui|{for (a,axis) in [(0,"X "),(1,"Y ")] {ui.add(egui::DragValue::new(&mut p[a]).range(-100_000.0..=100_000.0).prefix(axis));}});}}
                        ui.label("Glissez une extrémité sur une forme pour l’attacher, ou dans le vide pour la libérer.");

                    } else if let Some(d)=self.doc.drawings.iter_mut().find(|d|d.id==id) {
                        ui.label(RichText::new(if d.kind==DrawingKind::Bezier{"Courbe de Bézier"}else{"Trait"}).size(20.).strong());ui.add_space(18.);field_label(ui,"TRACÉ");
                        appearance_ui::line_controls(ui,&mut d.appearance,&mut d.color,&mut d.width,None,None,0.5);
                        ui.checkbox(&mut d.arrow,"Flèche à l’arrivée");
                        ui.add(egui::Slider::new(&mut d.appearance.end_scale,0.5..=3.0).text("Taille de flèche"));
                        if d.kind==DrawingKind::Bezier {ui.add(egui::Slider::new(&mut d.bend,-300.0..=300.0).clamping(egui::SliderClamping::Edits).text("Courbure"));}
                        for (label,p) in [("Départ",&mut d.start),("Arrivée",&mut d.end)] {ui.label(label);ui.horizontal(|ui|{ui.add(egui::DragValue::new(&mut p[0]).clamp_existing_to_range(false).prefix("X ").range(-100_000.0..=100_000.0));ui.add(egui::DragValue::new(&mut p[1]).clamp_existing_to_range(false).prefix("Y ").range(-100_000.0..=100_000.0));});}
                    }
                } else if self.selected.len()>1 {
                    self.selection_inspector(ui);
                } else {
                    ui.label(RichText::new("Votre document").size(20.).strong());
                    ui.label(RichText::new("Les détails font la différence.").size(12.).color(MUTED));ui.add_space(22.);
                    field_label(ui,"NOM DU DIAGRAMME");
                    ui.add(egui::TextEdit::singleline(&mut self.doc.title).desired_width(f32::INFINITY).char_limit(200));
                    ui.add_space(22.);field_label(ui,"PAGE");
                    ui.label(format!("{} · {:.0} × {:.0} px",self.doc.page_name,self.doc.page_size[0],self.doc.page_size[1]));
                    if ui.button("Nom et dimensions de page…").clicked(){self.open_page_settings();}
                    if ui.button("Gérer les pages…").clicked(){self.show_pages=true;}
                    ui.add_space(18.);
                    ui.label(RichText::new("À vous de jouer").strong().size(14.));
                    for text in ["01   Choisissez une forme.","02   Cliquez sur la page.","03   Reliez vos idées.","04   Exportez votre diagramme."] {ui.add_space(8.);ui.label(RichText::new(text).size(12.).color(MUTED));}
                    ui.add_space(20.);
                    ui.label(RichText::new("Sélectionnez un objet pour modifier son texte, ses couleurs et sa taille.").size(12.).color(MUTED));
                }
            });
        });
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
                    ui.label(RichText::new(&self.status).size(11.).color(MUTED));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "{} formes · {} liens · {} tracés · {} graphiques",
                                self.doc.nodes.len(),
                                self.doc.connectors.len(),
                                self.doc.drawings.len(),
                                self.doc.charts.len()
                            ))
                            .size(11.)
                            .color(MUTED),
                        );
                        ui.separator();
                        ui.label(
                            RichText::new("En local, simplement.")
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
                let response = ui.allocate_rect(rect, Sense::click_and_drag());
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
                if interactive && !self.context_open && !self.capture_marquee && !self.capture_alignment {
                    self.canvas_input(ui, &response);
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
                for d in &self.doc.drawings {
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
                for edge in &self.doc.connectors {
                    let pts: Vec<_> = self
                        .doc
                        .route(edge)
                        .into_iter()
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
                        self.styled_cache.borrow_mut().edge(&p,edge,&self.doc.route(edge),self.canvas.min+self.pan,self.zoom,self.doc.page_style.background);
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
                        let pos = self.screen(self.doc.edge_label_point(edge));
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
                for node in &self.doc.nodes {
                    self.paint_node(&p, node);
                }
                for chart in &self.doc.charts {
                    let r = Rect::from_min_size(
                        self.screen(chart.position),
                        vec2(chart.size[0], chart.size[1]) * self.zoom,
                    );
                    paint_chart(&p, chart, r);
                    if self.selected.contains(&chart.id) {
                        paint_selection(&p, r, self.selected.len() == 1 && !matches!(self.drag, Some(Drag::Marquee { .. })));
                    }
                }
                if !matches!(self.drag, Some(Drag::Marquee { .. })) {
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
                        Align2::CENTER_CENTER,
                        "Tout commence par une idée.",
                        FontId::proportional(23.),
                        MUTED,
                    );
                    p.text(
                        page.center() + vec2(0., 22.),
                        Align2::CENTER_CENTER,
                        "Choisissez une forme à gauche, puis cliquez ici.",
                        FontId::proportional(13.),
                        MUTED,
                    );
                }
                if response.hovered() || self.drag.is_some() {
                    ctx.set_cursor_icon(if self.drag.is_some() {
                        self.manipulation_cursor(ui.input(|i| i.pointer.hover_pos()))
                    } else { match self.tool {
                        Tool::Shape(_) | Tool::Connect | Tool::Line | Tool::Bezier => egui::CursorIcon::Crosshair,
                        Tool::Pan => egui::CursorIcon::Grab,
                        _ => self.manipulation_cursor(ui.input(|i| i.pointer.hover_pos())),
                    }});
                }
                let hint = match self.tool {
                    Tool::Shape(k) => format!("Cliquez pour ajouter : {}", k.label()),
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
                    hint,
                    FontId::proportional(11.),
                    MUTED,
                );
                if interactive {
                    if let Some(name) = self.capture_menu.take() {
                        let point = match name.as_str() {
                            "contexte-connecteur" => {
                                let e = &self.doc.connectors[2];
                                let route = self.doc.route(e);
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
                                    ).on_hover_text("Trackpad : deux doigts pour déplacer, pincer pour zoomer autour du pointeur. F pour recentrer la page.");
                                    if ui.button("+").clicked() {
                                        self.set_zoom(self.zoom * 1.2, rect.center());
                                    }
                                    ui.separator();
                                    if ui.button("Ajuster").on_hover_text("F").clicked() {
                                        self.fit_next = true;
                                    }
                                    if !self.selected.is_empty() && ui.button("Sélection").on_hover_text("Zoom sur la sélection · Maj + F").clicked() {
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
                    let id = self.doc.add_node(kind, world);
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
                    if let Some((id, port)) = self.connection_handle_at(pos) {
                        self.begin_connection(id, port, None, Some(pos));
                        return;
                    }
                    let handle = self.resize_handle_at(pos);
                    let endpoint = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1)
                        .and_then(|id| self.doc.connectors.iter().find(|e| e.id == *id))
                        .and_then(|e| {
                            let r = self.doc.route(e);
                            [r.first().map(|p| (true, *p)), r.last().map(|p| (false, *p))]
                                .into_iter()
                                .flatten()
                                .find(|(_, p)| self.screen(*p).distance(pos) <= 9.)
                                .map(|(start, point)| (e.id, start, point))
                        });
                    let segment = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1)
                        .and_then(|id| {
                            self.segment_handles(*id)
                                .into_iter()
                                .find(|p| self.screen(*p).distance(pos) <= 7.)
                                .map(|p| (*id, p))
                        });
                    let waypoint = self
                        .selected
                        .first()
                        .filter(|_| self.selected.len() == 1)
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
                        self.drag = Some(Drag::Waypoint { id, index });
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
                                self.drag = Some(Drag::Waypoint { id, index });
                            }
                            Err(e) => {
                                self.drag_snapshot = None;
                                self.error = Some(e);
                            }
                        }
                    } else if let Some((id, origin, size, handle)) = handle {
                        self.begin_edit_drag();
                        self.drag = Some(Drag::Resize {
                            id,
                            start: world,
                            size,
                            origin,
                            handle,
                        });
                    } else if let Some(id) = self.doc.hit_chart(world) {
                        self.select_for_drag(id, world, ui.input(|i| i.modifiers.shift));
                    } else if let Some(id) = self.doc.hit_node(world) {
                        if let Some(source) = self.format_brush.take() {
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
                    } else if let Some(id) = self.doc.hit_connector(world, 7. / self.zoom) {
                        if let Some(source) = self.edge_brush.take() {
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
                        if self.selected.len() == 1 && self.selected.contains(&id) {
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
                    } else if let Some(id) = self.doc.hit_drawing(world, 7. / self.zoom) {
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
                Drag::Waypoint { id, index } => {
                    if let Some(e) = self.doc.connectors.iter_mut().find(|e| e.id == *id)
                        && let Some(p) = e.waypoints.get_mut(*index)
                    {
                        for a in 0..2 {
                            p[a] = if self.snap && !ui.input(|i| i.modifiers.alt) {
                                (world[a] / snap_step).round() * snap_step
                            } else {
                                world[a]
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
        if response.double_clicked()
            && self.tool == Tool::Select
            && let Some(pos) = pos
            && let Some(id) = self
                .doc
                .hit_node(self.world(pos))
                .filter(|id| self.selected.contains(id))
        {
            self.show_inspector = true;
            self.focus_text = Some(id);
            self.status = "Modifier le texte · refermez les propriétés avec × pour retrouver toute la largeur.".into();
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
        // Explicit line breaks are preserved in both the canvas and SVG.
        let family = if n.text_style.monospace {
            egui::FontFamily::Monospace
        } else {
            egui::FontFamily::Name(font_name(&n.text_style).into())
        };
        let format = egui::TextFormat {
            font_id: FontId::new(n.font_size * self.zoom, family),
            color: rgb(n.text_color),
            italics: n.text_style.monospace && n.text_style.italic,
            underline: if n.text_style.underline {
                Stroke::new(self.zoom.max(1.), rgb(n.text_color))
            } else {
                Stroke::NONE
            },
            strikethrough: if n.text_style.strikethrough {
                Stroke::new(self.zoom.max(1.), rgb(n.text_color))
            } else {
                Stroke::NONE
            },
            ..Default::default()
        };
        let mut job = egui::text::LayoutJob::simple_format(n.display_text(), format);
        job.halign = match n.text_style.align {
            TextAlign::Left => Align::LEFT,
            TextAlign::Center => Align::Center,
            TextAlign::Right => Align::RIGHT,
        };
        let galley = p.layout_job(job);
        let x = match n.text_style.align {
            TextAlign::Left => r.left() + 12. * self.zoom,
            TextAlign::Center => r.center().x - galley.size().x / 2.,
            TextAlign::Right => r.right() - 12. * self.zoom - galley.size().x,
        };
        let pos = pos2(x, r.center().y - galley.size().y / 2.) - galley.rect.min.to_vec2();
        if n.text_style.monospace && n.text_style.bold {
            p.galley(
                pos + vec2(self.zoom * 0.6, 0.),
                galley.clone(),
                rgb(n.text_color),
            );
        }
        p.galley(pos, galley, rgb(n.text_color));
        if self.selected.contains(&n.id) && self.tool != Tool::Connect {
            paint_selection(
                p,
                r,
                self.selected.len() == 1 && !matches!(self.drag, Some(Drag::Marquee { .. })),
            );
        }
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if let Some(action) = self.pending.clone() {
            egui::Modal::new("unsaved".into()).show(ctx,|ui| {
                ui.set_width(420.);ui.heading(format!("Conserver « {} » ?",self.doc.title));ui.add_space(10.);
                ui.label("Ce diagramme contient des modifications qui n’ont pas encore été enregistrées.");ui.add_space(16.);
                ui.horizontal(|ui| {
                    if ui.button("Annuler").clicked(){self.pending=None;}
                    if ui.button("Ne pas enregistrer").clicked(){self.pending=None;self.execute(action.clone());}
                    if ui.add(egui::Button::new(RichText::new("Enregistrer").color(Color32::WHITE)).fill(ACCENT)).clicked() && self.save(false) {self.pending=None;self.execute(action);}
                });
            });
        }
        if let Some(error) = self.error.clone() {
            egui::Modal::new("error".into()).show(ctx, |ui| {
                ui.set_width(420.);
                ui.heading("L’opération n’a pas abouti");
                ui.add_space(10.);
                ui.label(error);
                ui.add_space(12.);
                if ui.button("Fermer").clicked() {
                    self.error = None;
                }
            });
        }
        self.help_dialog(ctx);
    }
}

impl eframe::App for DrawApp {
    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        visuals.panel_fill.to_normalized_gamma_f32()
    }
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
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
            self.request(Pending::Close);
        }
        if self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if let Some(path) = ctx.input(|i| i.raw.dropped_files.first().and_then(|f| f.path.clone()))
        {
            self.request(Pending::Open(path));
        }
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
                && !self.show_help
                && !self.command_palette.open
                && !self.show_templates
                && !self.save_template_open
                && !self.show_symbol_manager
                && !self.show_gallery
                && !self.show_pdf_export
                && !self.show_pages
                && self.page_draft.is_none()
                && self.agent_panel.is_none(),
        );
        self.finish_frame_edit(ctx, &before);
        self.command_dialog(ctx);
        self.library_dialogs(ctx);
        self.workspace_dialogs(ctx);
        self.pages_dialogs(ctx);
        self.agent_dialog(ctx);
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
    ctx.set_fonts(fonts);
}
fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(RichText::new(label).size(10.).strong().color(MUTED));
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

#[path = "commands_ui.rs"]
mod commands_ui;
