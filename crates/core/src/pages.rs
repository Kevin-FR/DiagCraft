//! Ordered pages. The editor keeps the active page's fields directly on Document;
//! serialization always writes page 1 at the root, followed by the other pages.
use crate::document::*;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Unit {
    #[default]
    Pixels,
    Millimeters,
    Inches,
}
impl Unit {
    pub const ALL: [Self; 3] = [Self::Pixels, Self::Millimeters, Self::Inches];
    pub fn label(self) -> &'static str {
        match self {
            Self::Pixels => "px",
            Self::Millimeters => "mm",
            Self::Inches => "in",
        }
    }
    pub fn to_px(self, v: f32) -> f32 {
        v * match self {
            Self::Pixels => 1.,
            Self::Millimeters => 96. / 25.4,
            Self::Inches => 96.,
        }
    }
    pub fn from_px(self, v: f32) -> f32 {
        v / self.to_px(1.)
    }
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PageLayout {
    pub unit: Unit,
    pub auto_size: bool,
}
fn default_name() -> String {
    "Page 1".into()
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    #[serde(default)]
    pub id: u64,
    #[serde(default = "default_name")]
    pub page_name: String,
    pub page_size: Point,
    pub nodes: Vec<Node>,
    pub connectors: Vec<Connector>,
    #[serde(default)]
    pub links: Vec<crate::document::PageLink>,
    #[serde(default)]
    pub drawings: Vec<Drawing>,
    #[serde(default)]
    pub charts: Vec<crate::chart::Chart>,
    #[serde(default)]
    pub page_style: PageStyle,
    #[serde(default)]
    pub page_layout: PageLayout,
    #[serde(
        default,
        skip_serializing_if = "crate::organization::Organization::is_empty"
    )]
    pub organization: crate::organization::Organization,
}
#[derive(Serialize, PartialEq)]
struct PageRef<'a> {
    id: u64,
    page_name: &'a str,
    page_size: Point,
    nodes: &'a [Node],
    connectors: &'a [Connector],
    #[serde(skip_serializing_if = "Vec::is_empty")]
    links: &'a Vec<crate::document::PageLink>,
    drawings: &'a [Drawing],
    charts: &'a [crate::chart::Chart],
    page_style: &'a PageStyle,
    page_layout: &'a PageLayout,
    #[serde(skip_serializing_if = "crate::organization::Organization::is_empty")]
    organization: &'a crate::organization::Organization,
}
impl<'a> From<&'a Page> for PageRef<'a> {
    fn from(p: &'a Page) -> Self {
        Self {
            id: p.id,
            page_name: &p.page_name,
            page_size: p.page_size,
            nodes: &p.nodes,
            connectors: &p.connectors,
            links: &p.links,
            drawings: &p.drawings,
            charts: &p.charts,
            page_style: &p.page_style,
            page_layout: &p.page_layout,
            organization: &p.organization,
        }
    }
}
impl Document {
    fn page_ref(&self, index: usize) -> PageRef<'_> {
        if index == self.page_index {
            PageRef {
                id: self.page_id,
                page_name: &self.page_name,
                page_size: self.page_size,
                nodes: &self.nodes,
                connectors: &self.connectors,
                links: &self.links,
                drawings: &self.drawings,
                charts: &self.charts,
                page_style: &self.page_style,
                page_layout: &self.page_layout,
                organization: &self.organization,
            }
        } else {
            PageRef::from(
                &self.pages[if index < self.page_index {
                    index
                } else {
                    index - 1
                }],
            )
        }
    }
    pub fn page_count(&self) -> usize {
        self.pages.len() + 1
    }
    pub fn page(&self, index: usize) -> Option<Page> {
        if index >= self.page_count() {
            return None;
        }
        let p = self.page_ref(index);
        Some(Page {
            id: p.id,
            page_name: p.page_name.into(),
            page_size: p.page_size,
            nodes: p.nodes.to_vec(),
            connectors: p.connectors.to_vec(),
            links: p.links.to_vec(),
            drawings: p.drawings.to_vec(),
            charts: p.charts.to_vec(),
            page_style: p.page_style.clone(),
            page_layout: p.page_layout.clone(),
            organization: p.organization.clone(),
        })
    }
    pub fn page_document(&self, index: usize) -> Option<Self> {
        self.page(index).map(|p| Self::from_page(p, &self.title))
    }
    pub fn from_page(p: Page, title: &str) -> Self {
        Self {
            title: title.into(),
            page_id: p.id,
            page_name: p.page_name,
            page_size: p.page_size,
            nodes: p.nodes,
            connectors: p.connectors,
            links: p.links,
            drawings: p.drawings,
            charts: p.charts,
            page_style: p.page_style,
            page_layout: p.page_layout,
            organization: p.organization,
            ..Self::default()
        }
    }
    pub fn page_names(&self) -> Vec<&str> {
        (0..self.page_count())
            .map(|i| self.page_ref(i).page_name)
            .collect()
    }
    /// Index de la page portant l'identifiant permanent donné, ou `None`.
    /// Les identifiants de page sont permanents (v8) : les index changent après
    /// insertion, suppression ou réordonnancement, pas les identifiants. Les
    /// liens inter-pages stockent la cible par identifiant et la résolvent en
    /// index au moment de la navigation.
    pub fn page_index_by_id(&self, id: u64) -> Option<usize> {
        if id == 0 {
            return None;
        }
        (0..self.page_count()).find(|&i| self.page_ref(i).id == id)
    }
    fn replace_pages(&mut self, mut pages: Vec<Page>, active: usize) {
        let title = self.title.clone();
        // Les styles nommés sont document-globaux : `from_page` les réinitialise
        // via `..Self::default()`, on les préserve explicitement.
        let named_styles = std::mem::take(&mut self.named_styles);
        let p = pages.remove(active);
        *self = Self::from_page(p, &title);
        self.pages = pages;
        self.page_index = active;
        self.named_styles = named_styles;
    }
    pub fn switch_page(&mut self, index: usize) -> Result<(), String> {
        if index >= self.page_count() {
            return Err("Page introuvable.".into());
        }
        if index != self.page_index {
            let pages = (0..self.page_count())
                .map(|i| self.page(i).unwrap())
                .collect();
            self.replace_pages(pages, index);
        }
        Ok(())
    }
    pub fn add_page(&mut self, duplicate: bool) -> Result<(), String> {
        self.validate()?;
        if self.page_count() >= 100 {
            return Err("Maximum de 100 pages atteint.".into());
        }
        let mut pages = (0..self.page_count())
            .map(|i| self.page(i).unwrap())
            .collect::<Vec<_>>();
        let mut p = if duplicate {
            self.page(self.page_index).unwrap()
        } else {
            let mut p = Document::default().page(0).unwrap();
            p.page_size = self.page_size;
            p.page_style = self.page_style.clone();
            p.page_layout = self.page_layout.clone();
            p
        };
        p.page_name = if duplicate {
            format!("Copie — {}", self.page_name)
        } else {
            format!("Page {}", self.page_count() + 1)
        };
        p.page_name = p.page_name.chars().take(200).collect();
        p.id = self.next_page_id();
        let index = self.page_index + 1;
        pages.insert(index, p);
        let mut next = self.clone();
        next.replace_pages(pages, index);
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn remove_page(&mut self, index: usize) -> Result<(), String> {
        self.validate()?;
        if self.page_count() == 1 {
            return Err("Le document doit garder au moins une page.".into());
        }
        if index >= self.page_count() {
            return Err("Page introuvable.".into());
        }
        let page = self.page_document(index).unwrap();
        let removed_id = page.page_id;
        if page.organization_view().editable_ids.len() < page.object_ids().len() {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let mut pages = (0..self.page_count())
            .map(|i| self.page(i).unwrap())
            .collect::<Vec<_>>();
        pages.remove(index);
        // Les liens qui pointaient vers la page supprimée deviennent orphelins :
        // on les retire de toutes les pages restantes, car la navigation vers
        // une destination disparue serait invalide.
        for p in &mut pages {
            p.links.retain(|l| l.target_page != removed_id);
        }
        let active = if self.page_index > index {
            self.page_index - 1
        } else if self.page_index == index {
            index.min(pages.len() - 1)
        } else {
            self.page_index
        };
        self.replace_pages(pages, active);
        Ok(())
    }
    pub fn move_page(&mut self, from: usize, to: usize) -> Result<(), String> {
        self.validate()?;
        if from >= self.page_count() || to >= self.page_count() {
            return Err("Page introuvable.".into());
        }
        let mut pages = (0..self.page_count())
            .map(|i| self.page(i).unwrap())
            .collect::<Vec<_>>();
        let p = pages.remove(from);
        pages.insert(to, p);
        let active = if self.page_index == from {
            to
        } else if from < self.page_index && to >= self.page_index {
            self.page_index - 1
        } else if from > self.page_index && to <= self.page_index {
            self.page_index + 1
        } else {
            self.page_index
        };
        self.replace_pages(pages, active);
        Ok(())
    }

    // ---- Styles nommés (lot 111) ----

    fn apply_style_to_nodes(nodes: &mut [Node], name: &str, style: &NamedStyle) {
        for n in nodes.iter_mut() {
            if n.style_ref.as_deref() == Some(name) {
                n.fill = style.fill;
                n.stroke = style.stroke;
                n.text_color = style.text_color;
                n.stroke_width = style.stroke_width;
                n.font_size = style.font_size;
            }
        }
    }
    fn style_nodes_mut(&mut self) -> impl Iterator<Item = &mut Node> {
        self.pages
            .iter_mut()
            .flat_map(|p| p.nodes.iter_mut())
            .chain(self.nodes.iter_mut())
    }
    /// Liste des noms de styles nommés du document, dans l’ordre.
    pub fn named_style_names(&self) -> Vec<&str> {
        self.named_styles
            .0
            .iter()
            .map(|s| s.name.as_str())
            .collect()
    }
    /// Retourne la définition d’un style nommé, ou `None`.
    pub fn named_style(&self, name: &str) -> Option<&NamedStyle> {
        self.named_styles.0.iter().find(|s| s.name == name)
    }
    /// Applique les valeurs d’un style nommé à toutes les formes (toutes pages)
    /// qui le référencent. Appelée après toute édition de la définition.
    fn propagate_style(&mut self, name: &str) {
        let Some(style) = self.named_styles.0.iter().find(|s| s.name == name).cloned() else {
            return;
        };
        Self::apply_style_to_nodes(&mut self.nodes, name, &style);
        for p in self.pages.iter_mut() {
            Self::apply_style_to_nodes(&mut p.nodes, name, &style);
        }
    }
    /// Crée un style nommé à partir d’une apparence. `Err` si le nom est vide,
    /// trop long ou déjà pris.
    pub fn create_named_style(
        &mut self,
        name: &str,
        fill: Color,
        stroke: Color,
        text_color: Color,
        stroke_width: f32,
        font_size: f32,
    ) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() || name.len() > 64 {
            return Err("Nom de style invalide.".into());
        }
        if self.named_styles.0.iter().any(|s| s.name == name) {
            return Err("Un style porte déjà ce nom.".into());
        }
        let style = NamedStyle {
            name: name.to_owned(),
            fill,
            stroke,
            text_color,
            stroke_width,
            font_size,
        };
        style.validate()?;
        self.named_styles.0.push(style);
        Ok(())
    }
    /// Renomme un style et met à jour toutes les références, ou `Err` si le
    /// nouveau nom est vide/trop long/déjà pris.
    pub fn rename_named_style(&mut self, old: &str, new: &str) -> Result<(), String> {
        let new = new.trim();
        if new.is_empty() || new.len() > 64 {
            return Err("Nom de style invalide.".into());
        }
        if old == new {
            return Ok(());
        }
        if self.named_styles.0.iter().any(|s| s.name == new) {
            return Err("Un style porte déjà ce nom.".into());
        }
        for s in self.named_styles.0.iter_mut() {
            if s.name == old {
                s.name = new.to_owned();
            }
        }
        for n in self.style_nodes_mut() {
            if n.style_ref.as_deref() == Some(old) {
                n.style_ref = Some(new.to_owned());
            }
        }
        Ok(())
    }
    /// Supprime un style nommé et détache toutes les formes qui le référencent.
    pub fn remove_named_style(&mut self, name: &str) -> Result<(), String> {
        self.named_styles.0.retain(|s| s.name != name);
        for n in self.style_nodes_mut() {
            if n.style_ref.as_deref() == Some(name) {
                n.style_ref = None;
            }
        }
        Ok(())
    }
    /// Met à jour une valeur d’un style nommé et la propage à toutes les formes
    /// qui le référencent.
    pub fn edit_named_style_fill(&mut self, name: &str, fill: Color) -> Result<(), String> {
        self.set_style_prop(name, |s| s.fill = fill)
    }
    pub fn edit_named_style_stroke(&mut self, name: &str, stroke: Color) -> Result<(), String> {
        self.set_style_prop(name, |s| s.stroke = stroke)
    }
    pub fn edit_named_style_text_color(&mut self, name: &str, c: Color) -> Result<(), String> {
        self.set_style_prop(name, |s| s.text_color = c)
    }
    pub fn edit_named_style_stroke_width(&mut self, name: &str, w: f32) -> Result<(), String> {
        self.set_style_prop(name, |s| s.stroke_width = w)
    }
    pub fn edit_named_style_font_size(&mut self, name: &str, size: f32) -> Result<(), String> {
        self.set_style_prop(name, |s| s.font_size = size)
    }
    fn set_style_prop(&mut self, name: &str, f: impl Fn(&mut NamedStyle)) -> Result<(), String> {
        let idx = self
            .named_styles
            .0
            .iter()
            .position(|s| s.name == name)
            .ok_or_else(|| format!("Style «{name}» introuvable."))?;
        let mut next = self.named_styles.0[idx].clone();
        f(&mut next);
        next.validate()?;
        self.named_styles.0[idx] = next;
        self.propagate_style(name);
        Ok(())
    }
    /// Applique un style nommé aux formes données de la page active : pose la
    /// référence et recopie les valeurs du style. `Err` si le style n’existe pas.
    pub fn apply_named_style_to_nodes(&mut self, ids: &[u64], name: &str) -> Result<(), String> {
        let style = self
            .named_styles
            .0
            .iter()
            .find(|s| s.name == name)
            .cloned()
            .ok_or_else(|| format!("Style «{name}» introuvable."))?;
        style.validate()?;
        for n in self.nodes.iter_mut() {
            if ids.contains(&n.id) {
                n.style_ref = Some(name.to_owned());
                n.fill = style.fill;
                n.stroke = style.stroke;
                n.text_color = style.text_color;
                n.stroke_width = style.stroke_width;
                n.font_size = style.font_size;
            }
        }
        Ok(())
    }
    /// Détache les formes données de tout style nommé (sans changer les valeurs).
    pub fn detach_named_style(&mut self, ids: &[u64]) {
        for n in self.nodes.iter_mut() {
            if ids.contains(&n.id) {
                n.style_ref = None;
            }
        }
    }

    pub fn resize_page(
        &mut self,
        name: String,
        size: Point,
        layout: PageLayout,
    ) -> Result<(), String> {
        let mut next = self.clone();
        next.page_name = name.trim().into();
        next.page_size = size;
        next.page_layout = layout;
        next.grow_page_to_content();
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn grow_page_to_content(&mut self) {
        if !self.page_layout.auto_size {
            return;
        }
        let mut extent = self.page_size;
        for n in &self.nodes {
            let bounds = if n.appearance.is_default() {
                [0., 0., n.size[0], n.size[1]]
            } else {
                crate::appearance_svg::node_bounds(n)
            };
            for (a, value) in extent.iter_mut().enumerate() {
                *value = value.max(n.position[a] + bounds[a + 2] + self.page_style.margin);
            }
        }
        for c in &self.charts {
            for (a, value) in extent.iter_mut().enumerate() {
                *value = value.max(c.position[a] + c.size[a] + self.page_style.margin);
            }
        }
        for d in &self.drawings {
            for p in d.controls() {
                for a in 0..2 {
                    extent[a] = extent[a].max(p[a] + self.page_style.margin);
                }
            }
        }
        for e in &self.connectors {
            for p in self.route(e) {
                for a in 0..2 {
                    extent[a] = extent[a].max(p[a] + self.page_style.margin);
                }
            }
        }
        self.page_size = extent.map(|v| v.min(100_000.));
    }
}
impl PartialEq for Document {
    fn eq(&self, other: &Self) -> bool {
        self.format_version == other.format_version
            && self.title == other.title
            && self.description == other.description
            && self.page_count() == other.page_count()
            && (0..self.page_count()).all(|i| self.page_ref(i) == other.page_ref(i))
    }
}
impl Serialize for Document {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        #[derive(Serialize)]
        struct File<'a> {
            format_version: u32,
            title: &'a str,
            #[serde(skip_serializing_if = "str::is_empty")]
            description: &'a str,
            #[serde(flatten)]
            first: PageRef<'a>,
            pages: Vec<PageRef<'a>>,
            #[serde(skip_serializing_if = "crate::document::NamedStyles::is_empty")]
            named_styles: &'a crate::document::NamedStyles,
        }
        File {
            format_version: self.format_version,
            title: &self.title,
            description: &self.description,
            first: self.page_ref(0),
            pages: (1..self.page_count()).map(|i| self.page_ref(i)).collect(),
            named_styles: &self.named_styles,
        }
        .serialize(s)
    }
}
impl<'de> Deserialize<'de> for Document {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        struct Raw {
            format_version: u32,
            title: String,
            #[serde(default)]
            description: String,
            #[serde(default = "crate::document::default_page_name")]
            page_name: String,
            #[serde(default)]
            page_layout: PageLayout,
            #[serde(default)]
            pages: Vec<Page>,
            page_size: Point,
            nodes: Vec<Node>,
            connectors: Vec<Connector>,
            #[serde(default)]
            links: Vec<crate::document::PageLink>,
            #[serde(default)]
            drawings: Vec<Drawing>,
            #[serde(default)]
            page_style: PageStyle,
            #[serde(default)]
            charts: Vec<crate::chart::Chart>,
            #[serde(default)]
            organization: crate::organization::Organization,
            // Identifiant stable de la page 0 (champ racine 'id' produit par
            // la sérialisation canonique v8). La page active après chargement
            // étant toujours l'index 0, cet identifiant est celui de la page 0.
            #[serde(default)]
            id: u64,
            #[serde(
                default,
                skip_serializing_if = "crate::document::NamedStyles::is_empty"
            )]
            named_styles: crate::document::NamedStyles,
        }
        let raw = Raw::deserialize(d)?;
        Ok(Document {
            format_version: raw.format_version,
            title: raw.title,
            description: raw.description,
            page_name: raw.page_name,
            page_layout: raw.page_layout,
            pages: raw.pages,
            page_index: 0,
            page_size: raw.page_size,
            nodes: raw.nodes,
            connectors: raw.connectors,
            links: raw.links,
            drawings: raw.drawings,
            page_style: raw.page_style,
            charts: raw.charts,
            organization: raw.organization,
            page_id: raw.id,
            named_styles: raw.named_styles,
        })
    }
}

pub struct Preset {
    pub label: &'static str,
    pub size: Point,
}
pub fn presets() -> Vec<Preset> {
    let mm = Unit::Millimeters;
    let inch = Unit::Inches;
    vec![
        Preset {
            label: "A3 · 297 × 420 mm",
            size: [mm.to_px(297.), mm.to_px(420.)],
        },
        Preset {
            label: "A4 · 210 × 297 mm",
            size: [mm.to_px(210.), mm.to_px(297.)],
        },
        Preset {
            label: "A5 · 148 × 210 mm",
            size: [mm.to_px(148.), mm.to_px(210.)],
        },
        Preset {
            label: "Lettre · 8,5 × 11 in",
            size: [inch.to_px(8.5), inch.to_px(11.)],
        },
        Preset {
            label: "Legal · 8,5 × 14 in",
            size: [inch.to_px(8.5), inch.to_px(14.)],
        },
        Preset {
            label: "Tabloïd · 11 × 17 in",
            size: [inch.to_px(11.), inch.to_px(17.)],
        },
        Preset {
            label: "Executive · 7,25 × 10,5 in",
            size: [inch.to_px(7.25), inch.to_px(10.5)],
        },
        Preset {
            label: "Présentation · 16:9",
            size: [1600., 900.],
        },
        Preset {
            label: "Présentation · 4:3",
            size: [1200., 900.],
        },
        Preset {
            label: "Carré · 1080 × 1080 px",
            size: [1080., 1080.],
        },
    ]
}
