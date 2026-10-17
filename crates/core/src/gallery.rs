//! Local saved-work index with immutable, bounded document revisions.
use crate::document::{Document, MAX_FILE_BYTES, atomic_write};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    io::Read,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
const MAX_PROJECTS: usize = 40;
const MAX_VERSIONS: usize = 10;
const MAX_CACHE: u64 = 128 * 1024 * 1024;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Revision {
    pub hash: String,
    pub saved_at: u64,
    pub bytes: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Project {
    pub key: String,
    pub path: PathBuf,
    pub title: String,
    pub revisions: Vec<Revision>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
struct Index {
    projects: Vec<Project>,
}
#[derive(Default)]
pub struct Gallery {
    root: Option<PathBuf>,
    unavailable: Option<String>,
    index: Index,
    memory: HashMap<String, Document>,
}
fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn hex_key(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}
fn file_name(p: &Project, r: &Revision) -> String {
    format!("{}-{}.draw", p.key, r.hash)
}
impl Index {
    fn validate(&self) -> Result<(), String> {
        if self.projects.len() > MAX_PROJECTS {
            return Err("Galerie trop volumineuse.".into());
        }
        let mut keys = HashSet::new();
        let mut total = 0;
        for p in &self.projects {
            if !hex_key(&p.key)
                || !keys.insert(&p.key)
                || p.title.len() > 4096
                || p.path.as_os_str().len() > 32768
                || !p.path.is_absolute()
                || p.revisions.is_empty()
                || p.revisions.len() > MAX_VERSIONS
            {
                return Err("Entrée de galerie invalide.".into());
            }
            for r in &p.revisions {
                if !hex_key(&r.hash) || r.bytes > MAX_FILE_BYTES {
                    return Err("Version de galerie invalide.".into());
                }
                total += r.bytes;
            }
        }
        if total > MAX_CACHE {
            return Err("Cache de galerie trop volumineux.".into());
        }
        Ok(())
    }
    fn files(&self) -> HashSet<String> {
        self.projects
            .iter()
            .flat_map(|p| p.revisions.iter().map(move |r| file_name(p, r)))
            .collect()
    }
    fn trim(&mut self) {
        self.projects.truncate(MAX_PROJECTS);
        for p in &mut self.projects {
            p.revisions.truncate(MAX_VERSIONS);
        }
        while self
            .projects
            .iter()
            .flat_map(|p| &p.revisions)
            .map(|r| r.bytes)
            .sum::<u64>()
            > MAX_CACHE
        {
            let i = self.projects.len() - 1;
            if self.projects[i].revisions.len() > 1 {
                self.projects[i].revisions.pop();
            } else {
                self.projects.pop();
            }
        }
    }
}
impl Gallery {
    pub fn unavailable(error: String) -> Self {
        Self {
            unavailable: Some(error),
            ..Default::default()
        }
    }
    pub fn open(root: PathBuf) -> Result<Self, String> {
        let path = root.join("index-v1.json");
        let index = if path.exists() {
            let mut b = vec![];
            std::fs::File::open(path)
                .map_err(|e| e.to_string())?
                .take(2 * 1024 * 1024 + 1)
                .read_to_end(&mut b)
                .map_err(|e| e.to_string())?;
            if b.len() > 2 * 1024 * 1024 {
                return Err("Index de galerie trop volumineux.".into());
            }
            let index: Index = serde_json::from_slice(&b).map_err(|e| e.to_string())?;
            index.validate()?;
            index
        } else {
            Index::default()
        };
        Ok(Self {
            root: Some(root),
            unavailable: None,
            index,
            memory: HashMap::new(),
        })
    }
    pub fn projects(&self) -> &[Project] {
        &self.index.projects
    }
    pub fn preserve_existing(&mut self, path: &Path) -> Result<(), String> {
        if path.is_file() {
            let d = Document::load(path)?;
            self.record(path, &d)?;
        }
        Ok(())
    }
    pub fn record(&mut self, path: &Path, doc: &Document) -> Result<(), String> {
        if let Some(e) = &self.unavailable {
            return Err(e.clone());
        }
        doc.validate()?;
        let path = std::fs::canonicalize(path).unwrap_or(if path.is_absolute() {
            path.to_owned()
        } else {
            std::env::current_dir()
                .map_err(|e| e.to_string())?
                .join(path)
        });
        let key = digest(path.to_string_lossy().as_bytes());
        let bytes = serde_json::to_vec_pretty(doc).map_err(|e| e.to_string())?;
        if bytes.len() as u64 > MAX_FILE_BYTES {
            return Err("Document trop volumineux pour la galerie.".into());
        }
        let hash = digest(&bytes);
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut next = self.index.clone();
        let mut p = next
            .projects
            .iter()
            .find(|p| p.key == key)
            .cloned()
            .unwrap_or(Project {
                key: key.clone(),
                path,
                title: doc.title.clone(),
                revisions: vec![],
            });
        p.title = doc.title.clone();
        // Consecutive saves of unchanged content do not consume a revision.
        if p.revisions.first().is_none_or(|r| r.hash != hash) {
            p.revisions.insert(
                0,
                Revision {
                    hash: hash.clone(),
                    saved_at: time,
                    bytes: bytes.len() as u64,
                },
            );
        }
        let filename = format!("{key}-{hash}.draw");
        next.projects.retain(|p| p.key != key);
        next.projects.insert(0, p);
        next.trim();
        next.validate()?;
        if let Some(root) = &self.root {
            std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
            let snapshot = root.join(&filename);
            let existed = snapshot.exists();
            atomic_write(&snapshot, &bytes)?;
            if let Err(e) = atomic_write(
                &root.join("index-v1.json"),
                &serde_json::to_vec_pretty(&next).map_err(|e| e.to_string())?,
            ) {
                if !existed {
                    let _ = std::fs::remove_file(snapshot);
                }
                return Err(e);
            }
            // Only remove files previously owned by this validated index.
            let keep = next.files();
            for name in self.index.files().difference(&keep) {
                let _ = std::fs::remove_file(root.join(name));
            }
        } else {
            self.memory.insert(filename, doc.clone());
            let keep = next.files();
            self.memory.retain(|k, _| keep.contains(k));
        }
        self.index = next;
        Ok(())
    }
    pub fn revision(&self, key: &str, index: usize) -> Result<Document, String> {
        let p = self
            .index
            .projects
            .iter()
            .find(|p| p.key == key)
            .ok_or("Travail introuvable")?;
        let r = p.revisions.get(index).ok_or("Version introuvable")?;
        let filename = file_name(p, r);
        if let Some(root) = &self.root {
            let mut bytes = vec![];
            std::fs::File::open(root.join(filename))
                .map_err(|e| format!("Version indisponible : {e}"))?
                .take(MAX_FILE_BYTES + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() as u64 > MAX_FILE_BYTES
                || bytes.len() as u64 != r.bytes
                || digest(&bytes) != r.hash
            {
                return Err("La version enregistrée a été altérée.".into());
            }
            let mut doc: Document = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            doc.upgrade_legacy();
            doc.validate()?;
            Ok(doc)
        } else {
            self.memory
                .get(&filename)
                .cloned()
                .ok_or("Version indisponible".into())
        }
    }
}
pub fn default_root() -> Option<PathBuf> {
    crate::library::default_path().and_then(|p| p.parent().map(|p| p.join("projects")))
}
