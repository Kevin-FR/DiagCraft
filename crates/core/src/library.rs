use crate::document::{Document, Node, atomic_write};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PersonalTemplate {
    pub name: String,
    pub category: String,
    pub document: Document,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub templates: Vec<PersonalTemplate>,
    pub symbols: Vec<Node>,
    pub recent: Vec<String>,
    pub visible_libraries: Vec<String>,
}
impl Default for Library {
    fn default() -> Self {
        Self {
            templates: vec![],
            symbols: vec![],
            recent: vec![],
            visible_libraries: crate::symbols::default_visible(),
        }
    }
}
impl Library {
    pub fn visible(&self, id: &str) -> bool {
        self.visible_libraries.iter().any(|s| s == id)
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        if self
            .visible_libraries
            .iter()
            .any(|s| !crate::symbols::known(s) || !seen.insert(s))
        {
            return Err("Choix de bibliothèques invalide.".into());
        }

        if self.templates.len() > 20 || self.symbols.len() > 64 || self.recent.len() > 12 {
            return Err("Bibliothèque trop volumineuse.".into());
        }
        for t in &self.templates {
            if t.name.len() > 150 || t.name.trim().is_empty() || t.category.len() > 100 {
                return Err("Nom de modèle invalide.".into());
            }
            t.document.validate()?;
        }
        for s in &self.symbols {
            let d = Document {
                nodes: vec![s.clone()],
                ..Document::default()
            };
            d.validate()?;
        }
        if self.recent.iter().any(|s| s.len() > 200) {
            return Err("Historique de modèles invalide.".into());
        }
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut bytes = vec![];
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(64 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Bibliothèque trop volumineuse.".into());
        }
        let mut library: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        for t in &mut library.templates {
            t.document.upgrade_legacy();
        }
        library.validate()?;
        Ok(library)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > 64 * 1024 * 1024 {
            return Err("Bibliothèque trop volumineuse.".into());
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        atomic_write(path, &bytes)
    }
    pub fn remember(&mut self, id: &str) {
        self.recent.retain(|s| s != id);
        self.recent.insert(0, id.into());
        self.recent.truncate(12);
    }
    pub fn add_template(
        &mut self,
        name: &str,
        category: &str,
        doc: &Document,
    ) -> Result<(), String> {
        if self.templates.iter().any(|t| t.name == name.trim()) {
            return Err("Un modèle porte déjà ce nom.".into());
        }
        let mut next = self.clone();
        next.templates.push(PersonalTemplate {
            name: name.trim().into(),
            category: category.into(),
            document: doc.clone(),
        });
        next.validate()?;
        *self = next;
        Ok(())
    }
    pub fn add_symbol(&mut self, node: &Node) -> Result<(), String> {
        let mut next = self.clone();
        let mut n = node.clone();
        n.id = (next.symbols.len() + 1) as u64;
        next.symbols.push(n);
        next.validate()?;
        *self = next;
        Ok(())
    }
}
// Keep the library and gallery together. Existing installations retain their
// original data directory; fresh installations use the DiagCraft name. Never
// move, merge or overwrite user data as a side effect of resolving a path.
fn existing_data_root(primary: PathBuf, legacy: PathBuf) -> PathBuf {
    if !primary.exists() && legacy.exists() {
        legacy
    } else {
        primary
    }
}

pub fn default_path() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var_os("HOME").map(|p| {
            let base = PathBuf::from(p).join("Library/Application Support");
            existing_data_root(base.join("DiagCraft"), base.join("Rust Draw"))
                .join("library-v7.json")
        })
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var_os("APPDATA").map(|p| {
            let base = PathBuf::from(p);
            existing_data_root(base.join("DiagCraft"), base.join("Rust Draw"))
                .join("library-v7.json")
        })
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")))
            .map(|p| {
                existing_data_root(p.join("diagcraft"), p.join("rust-draw")).join("library-v7.json")
            })
    }
}

#[cfg(test)]
mod storage_compatibility_tests {
    use super::*;

    #[test]
    fn fresh_installation_uses_new_name_without_creating_files() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("DiagCraft");
        let legacy = dir.path().join("Rust Draw");
        assert_eq!(existing_data_root(primary.clone(), legacy), primary);
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    }

    #[test]
    fn existing_library_and_gallery_keep_the_same_root_and_content() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("DiagCraft");
        let legacy = dir.path().join("Rust Draw");
        let projects = legacy.join("projects");
        std::fs::create_dir_all(&projects).unwrap();
        std::fs::write(legacy.join("library-v6.json"), b"previous library").unwrap();
        std::fs::write(projects.join("index-v1.json"), b"previous gallery").unwrap();
        assert_eq!(existing_data_root(primary.clone(), legacy.clone()), legacy);
        assert!(!primary.exists());
        assert_eq!(
            std::fs::read(legacy.join("library-v6.json")).unwrap(),
            b"previous library"
        );
        assert_eq!(
            std::fs::read(projects.join("index-v1.json")).unwrap(),
            b"previous gallery"
        );
    }

    #[test]
    fn gallery_only_installation_is_not_split_into_a_new_directory() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("diagcraft");
        let legacy = dir.path().join("rust-draw");
        std::fs::create_dir_all(legacy.join("projects")).unwrap();
        assert_eq!(existing_data_root(primary.clone(), legacy.clone()), legacy);
        assert!(!primary.exists());
    }

    #[test]
    fn new_directory_takes_precedence_without_merging_old_data() {
        let dir = tempfile::tempdir().unwrap();
        let primary = dir.path().join("DiagCraft");
        let legacy = dir.path().join("Rust Draw");
        std::fs::create_dir_all(&primary).unwrap();
        std::fs::create_dir_all(&legacy).unwrap();
        std::fs::write(primary.join("library-v7.json"), b"new library").unwrap();
        std::fs::write(legacy.join("library-v7.json"), b"old library").unwrap();
        assert_eq!(existing_data_root(primary.clone(), legacy.clone()), primary);
        assert_eq!(
            std::fs::read(primary.join("library-v7.json")).unwrap(),
            b"new library"
        );
        assert_eq!(
            std::fs::read(legacy.join("library-v7.json")).unwrap(),
            b"old library"
        );
    }
}
