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
    #[serde(default = "default_name")]
    pub page_name: String,
    pub page_size: Point,
    pub nodes: Vec<Node>,
    pub connectors: Vec<Connector>,
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
    page_name: &'a str,
    page_size: Point,
    nodes: &'a [Node],
    connectors: &'a [Connector],
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
            page_name: &p.page_name,
            page_size: p.page_size,
            nodes: &p.nodes,
            connectors: &p.connectors,
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
                page_name: &self.page_name,
                page_size: self.page_size,
                nodes: &self.nodes,
                connectors: &self.connectors,
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
            page_name: p.page_name.into(),
            page_size: p.page_size,
            nodes: p.nodes.to_vec(),
            connectors: p.connectors.to_vec(),
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
            page_name: p.page_name,
            page_size: p.page_size,
            nodes: p.nodes,
            connectors: p.connectors,
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
    fn replace_pages(&mut self, mut pages: Vec<Page>, active: usize) {
        let title = self.title.clone();
        let p = pages.remove(active);
        *self = Self::from_page(p, &title);
        self.pages = pages;
        self.page_index = active;
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
        if page.organization_view().editable_ids.len() < page.object_ids().len() {
            return Err(
                "Déverrouillez les objets et affichez leur calque avant de les modifier.".into(),
            );
        }
        let mut pages = (0..self.page_count())
            .map(|i| self.page(i).unwrap())
            .collect::<Vec<_>>();
        pages.remove(index);
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
            #[serde(flatten)]
            first: PageRef<'a>,
            pages: Vec<PageRef<'a>>,
        }
        File {
            format_version: self.format_version,
            title: &self.title,
            first: self.page_ref(0),
            pages: (1..self.page_count()).map(|i| self.page_ref(i)).collect(),
        }
        .serialize(s)
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
