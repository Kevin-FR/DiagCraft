//! Private, atomic checkpoints. OS locks distinguish a crashed process from a live one.
//! Documents use the canonical v7 serializer, never a second drawing representation.
use diagcraft::document::{self, Document};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, SyncSender},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub(super) const INTERVAL: Duration = Duration::from_secs(10);
const MAX_BYTES: u64 = 64 * 1024 * 1024;
const SCAN_BUDGET: u64 = 128 * 1024 * 1024;
const MAX_SESSIONS: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Copy {
    pub document: Document,
    /// Informational only: restoring always opens an unsaved copy.
    pub source: Option<PathBuf>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Snapshot {
    version: u32,
    pub updated: u64,
    pub copies: Vec<Copy>,
}

pub(super) struct Candidate {
    dir: PathBuf,
    // Keep the lease until the user decides, preventing two instances restoring it.
    _lease: File,
    pub snapshot: Result<Snapshot, String>,
    pub restored_at: Option<u64>,
    _preview_root: Option<tempfile::TempDir>,
}

impl Candidate {
    pub fn remove(&self) -> Result<(), String> {
        remove_snapshot(&self.dir)
    }
}

struct Store {
    dir: PathBuf,
    _lease: File,
    fingerprint: Vec<u8>,
}

struct BoundedWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for BoundedWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other(
                "Copie de récupération trop volumineuse (maximum 64 Mio).",
            ));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn encode(snapshot: &Snapshot) -> Result<Vec<u8>, String> {
    let mut writer = BoundedWriter {
        bytes: vec![],
        limit: MAX_BYTES as usize,
    };
    serde_json::to_writer(&mut writer, snapshot).map_err(|e| e.to_string())?;
    Ok(writer.bytes)
}

impl Store {
    fn new(root: &Path) -> Result<Self, String> {
        fs::create_dir_all(root).map_err(|e| e.to_string())?;
        // keep(): a dropped TempDir would delete the only recovery after a panic.
        let dir = tempfile::Builder::new()
            .prefix("session-")
            .tempdir_in(root)
            .map_err(|e| e.to_string())?
            .keep();
        // Publish the lease only after locking it: a concurrent scanner must not
        // mistake the small creation window for an empty, abandoned session.
        let lease = tempfile::NamedTempFile::new_in(&dir).map_err(|e| e.to_string())?;
        lease.as_file().try_lock().map_err(|e| e.to_string())?;
        let lease = lease
            .persist(dir.join("lease"))
            .map_err(|e| e.to_string())?;
        Ok(Self {
            dir,
            _lease: lease,
            fingerprint: vec![],
        })
    }

    fn checkpoint(&mut self, copies: Vec<Copy>) -> Result<(), String> {
        use sha2::{Digest, Sha256};
        if copies.is_empty() {
            remove_snapshot(&self.dir)?;
            self.fingerprint.clear();
            return Ok(());
        }
        if copies.len() > 100 {
            return Err("Limite de 100 documents de récupération atteinte.".into());
        }
        for copy in &copies {
            copy.document.validate()?;
        }
        let mut snapshot = Snapshot {
            version: 1,
            updated: 0,
            copies,
        };
        let bytes = encode(&snapshot)?;
        let fingerprint = Sha256::digest(&bytes).to_vec();
        drop(bytes);
        if fingerprint == self.fingerprint {
            return Ok(());
        }
        snapshot.updated = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let bytes = encode(&snapshot)?;
        document::atomic_write(&self.dir.join("snapshot.json"), &bytes)?;
        #[cfg(unix)]
        File::open(&self.dir)
            .and_then(|file| file.sync_all())
            .map_err(|e| e.to_string())?;
        self.fingerprint = fingerprint;
        Ok(())
    }
}

fn remove_snapshot(dir: &Path) -> Result<(), String> {
    match fs::remove_file(dir.join("snapshot.json")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}

fn load(path: &Path) -> Result<Snapshot, String> {
    let mut bytes = vec![];
    File::open(path)
        .map_err(|e| e.to_string())?
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Copie de récupération trop volumineuse (maximum 64 Mio).".into());
    }
    let snapshot: Snapshot = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if snapshot.version != 1 || snapshot.copies.is_empty() || snapshot.copies.len() > 100 {
        return Err("Version ou contenu de récupération non pris en charge.".into());
    }
    for copy in &snapshot.copies {
        copy.document.validate()?;
    }
    Ok(snapshot)
}

pub(super) fn scan(root: &Path) -> Result<Vec<Candidate>, String> {
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut candidates = vec![];
    let mut budget = SCAN_BUDGET;
    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        // Do not follow external directories or interpret other application files.
        if !entry.file_type().map_err(|e| e.to_string())?.is_dir()
            || !entry.file_name().to_string_lossy().starts_with("session-")
        {
            continue;
        }
        let dir = entry.path();
        let lease_path = dir.join("lease");
        let snapshot_path = dir.join("snapshot.json");
        if !fs::symlink_metadata(&lease_path).is_ok_and(|m| m.is_file()) {
            continue;
        }
        let lease = File::options()
            .read(true)
            .write(true)
            .open(&lease_path)
            .map_err(|e| e.to_string())?;
        match lease.try_lock() {
            Ok(()) => {}
            Err(std::fs::TryLockError::WouldBlock) => continue,
            Err(e) => return Err(e.to_string()),
        }
        let metadata = match fs::symlink_metadata(&snapshot_path) {
            Ok(metadata) if metadata.is_file() => metadata,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Empty, unlocked session: no work to preserve. Close the lease for Windows.
                drop(lease);
                let _ = fs::remove_file(lease_path);
                let _ = fs::remove_dir(dir);
                continue;
            }
            _ => continue,
        };
        if candidates.len() >= MAX_SESSIONS || metadata.len() > budget {
            continue;
        }
        budget -= metadata.len();
        candidates.push(Candidate {
            snapshot: load(&snapshot_path),
            dir,
            _lease: lease,
            restored_at: None,
            _preview_root: None,
        });
    }
    candidates.sort_by_key(|c| std::cmp::Reverse(c.snapshot.as_ref().map_or(0, |s| s.updated)));
    Ok(candidates)
}

/// Documentation captures use only temporary copies of the project's own templates.
pub(super) fn preview(copies: Vec<Copy>) -> Result<Vec<Candidate>, String> {
    let root = tempfile::tempdir().map_err(|e| e.to_string())?;
    let mut store = Store::new(root.path())?;
    store.checkpoint(copies)?;
    drop(store);
    let mut candidates = scan(root.path())?;
    if let Some(candidate) = candidates.first_mut() {
        candidate._preview_root = Some(root);
    }
    Ok(candidates)
}

enum Request {
    Checkpoint(u64, Vec<Copy>),
    Clean,
}

pub(super) struct Worker {
    sender: SyncSender<Request>,
    receiver: Receiver<(u64, Result<(), String>)>,
    pub busy: bool,
}

impl Worker {
    pub fn start(root: &Path) -> Result<Self, String> {
        let mut store = Store::new(root)?;
        let (sender, requests) = mpsc::sync_channel(1);
        let (results, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("diagcraft-recovery".into())
            .spawn(move || {
                while let Ok(request) = requests.recv() {
                    match request {
                        Request::Checkpoint(serial, copies) => {
                            let _ = results.send((serial, store.checkpoint(copies)));
                        }
                        Request::Clean => {
                            let _ = results.send((u64::MAX, remove_snapshot(&store.dir)));
                            break;
                        }
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Self {
            sender,
            receiver,
            busy: false,
        })
    }
    pub fn checkpoint(&mut self, serial: u64, copies: Vec<Copy>) -> bool {
        if !self.busy
            && self
                .sender
                .try_send(Request::Checkpoint(serial, copies))
                .is_ok()
        {
            self.busy = true;
            true
        } else {
            false
        }
    }
    pub fn poll(&mut self) -> Option<(u64, Result<(), String>)> {
        match self.receiver.try_recv() {
            Ok(result) => {
                self.busy = false;
                Some(result)
            }
            Err(mpsc::TryRecvError::Disconnected) if self.busy => {
                self.busy = false;
                Some((0, Err("Service de récupération interrompu.".into())))
            }
            _ => None,
        }
    }
    pub fn clean(&mut self) {
        // A queued write must finish before cleanup. Never block the UI indefinitely.
        if self.busy {
            let _ = self.receiver.recv_timeout(Duration::from_millis(500));
        }
        if self.sender.try_send(Request::Clean).is_ok() {
            let _ = self.receiver.recv_timeout(Duration::from_millis(500));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use diagcraft::document::{ConnectorKind, ShapeKind};

    fn copies() -> Vec<Copy> {
        let mut document = Document::default();
        let a = document.add_node(ShapeKind::Rectangle, [180., 180.]);
        let b = document.add_node(ShapeKind::Diamond, [480., 180.]);
        document.connect(a, b, ConnectorKind::Elbow).unwrap();
        document.node_mut(a).unwrap().text = "中文 Grüße {0}".into();
        document.add_page(true).unwrap();
        vec![Copy {
            document,
            source: Some("original.draw".into()),
        }]
    }

    #[test]
    fn serializer_bound_rejects_a_chunk_before_allocating_it() {
        let mut writer = BoundedWriter {
            bytes: vec![],
            limit: 8,
        };
        assert!(writer.write_all(b"123456789").is_err());
        assert!(writer.bytes.is_empty());
        writer.write_all(b"1234").unwrap();
        assert!(writer.write_all(b"56789").is_err());
        assert_eq!(writer.bytes, b"1234");
    }

    #[test]
    fn locks_hide_live_sessions_and_released_sessions_keep_canonical_multipage_content() {
        let root = tempfile::tempdir().unwrap();
        let mut store = Store::new(root.path()).unwrap();
        let copies = copies();
        store.checkpoint(copies.clone()).unwrap();
        assert!(scan(root.path()).unwrap().is_empty());
        drop(store);
        let claimed = scan(root.path()).unwrap();
        let snapshot = claimed[0].snapshot.as_ref().unwrap();
        assert_eq!(snapshot.copies[0].document, copies[0].document);
        assert_eq!(snapshot.copies[0].source, copies[0].source);
        assert_eq!(snapshot.copies[0].document.page_count(), 2);
        assert!(
            scan(root.path()).unwrap().is_empty(),
            "a second instance cannot claim it"
        );
        drop(claimed);
        assert_eq!(scan(root.path()).unwrap().len(), 1);
    }

    #[test]
    fn invalid_checkpoint_preserves_previous_copy_and_empty_checkpoint_cleans_it() {
        let root = tempfile::tempdir().unwrap();
        let mut store = Store::new(root.path()).unwrap();
        store.checkpoint(copies()).unwrap();
        let path = store.dir.join("snapshot.json");
        let bytes = fs::read(&path).unwrap();
        store.checkpoint(copies()).unwrap();
        assert_eq!(
            fs::read(&path).unwrap(),
            bytes,
            "unchanged drawings are not rewritten"
        );
        let mut bad = copies();
        bad[0].document.nodes[0].position[0] = f32::NAN;
        assert!(store.checkpoint(bad).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        store.checkpoint(vec![]).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn corrupt_newer_and_oversized_checkpoints_are_preserved() {
        let root = tempfile::tempdir().unwrap();
        let mut store = Store::new(root.path()).unwrap();
        store.checkpoint(copies()).unwrap();
        let path = store.dir.join("snapshot.json");
        drop(store);
        for bytes in [
            b"broken".to_vec(),
            br#"{"version":99,"updated":0,"copies":[]}"#.to_vec(),
        ] {
            fs::write(&path, &bytes).unwrap();
            let candidates = scan(root.path()).unwrap();
            assert!(candidates[0].snapshot.is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        let file = File::create(&path).unwrap();
        file.set_len(MAX_BYTES + 1).unwrap();
        assert!(scan(root.path()).unwrap()[0].snapshot.is_err());
        assert_eq!(fs::metadata(&path).unwrap().len(), MAX_BYTES + 1);
    }

    #[cfg(unix)]
    #[test]
    fn scan_does_not_follow_symlinks() {
        use std::os::unix::fs::symlink;
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let mut store = Store::new(outside.path()).unwrap();
        store.checkpoint(copies()).unwrap();
        let dir = store.dir.clone();
        drop(store);
        symlink(&dir, root.path().join("session-external")).unwrap();
        assert!(scan(root.path()).unwrap().is_empty());
        assert!(dir.join("snapshot.json").exists());
    }

    #[test]
    fn worker_writes_off_thread_and_cleans_on_explicit_shutdown() {
        let root = tempfile::tempdir().unwrap();
        let mut worker = Worker::start(root.path()).unwrap();
        assert!(worker.checkpoint(3, copies()));
        assert!(!worker.checkpoint(4, vec![]), "no unbounded write queue");
        let (serial, result) = worker
            .receiver
            .recv_timeout(Duration::from_secs(5))
            .unwrap();
        assert_eq!(serial, 3);
        result.unwrap();
        worker.busy = false;
        assert!(scan(root.path()).unwrap().is_empty());
        worker.clean();
        drop(worker);
        assert!(scan(root.path()).unwrap().is_empty());
    }

    // Executed as a subprocess by the next test; normal test discovery does no IO.
    #[test]
    fn crash_child() {
        let Some(root) = std::env::var_os("DIAGCRAFT_TEST_CRASH_ROOT") else {
            return;
        };
        let root = PathBuf::from(root);
        let mut store = Store::new(&root).unwrap();
        store.checkpoint(copies()).unwrap();
        fs::write(root.join("ready"), b"ready").unwrap();
        loop {
            std::thread::park();
        }
    }

    #[test]
    fn killed_process_releases_lease_and_preserves_checkpoint() {
        let root = tempfile::tempdir().unwrap();
        let mut child = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "app::recovery::tests::crash_child",
                "--nocapture",
            ])
            .env("DIAGCRAFT_TEST_CRASH_ROOT", root.path())
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !root.path().join("ready").exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let ready = root.path().join("ready").exists();
        let live = if ready {
            scan(root.path()).unwrap().len()
        } else {
            usize::MAX
        };
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(ready, "child never checkpointed");
        assert_eq!(live, 0);
        let candidates = scan(root.path()).unwrap();
        assert_eq!(candidates.len(), 1);
        assert_eq!(
            candidates[0].snapshot.as_ref().unwrap().copies[0].document,
            copies()[0].document
        );
    }
}
