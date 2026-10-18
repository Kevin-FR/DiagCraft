//! Opt-in GitHub release checks. No document data, credentials or shell commands.
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::Duration,
};

const API: &str = "https://api.github.com/repos/Kevin-FR/DiagCraft/releases?per_page=30";
const DOWNLOADS: &str = "https://github.com/Kevin-FR/DiagCraft/releases/download/";
const MAX_METADATA: u64 = 1_048_576;
const MAX_DOWNLOAD: u64 = 256 * 1024 * 1024;

#[derive(Clone, Debug)]
pub(super) struct Release {
    pub version: Version,
    pub experimental: bool,
    pub filename: String,
    pub url: String,
    pub size: u64,
    pub digest: Option<String>,
    pub checksums_url: String,
}
impl Release {
    pub fn page(&self) -> String {
        format!(
            "https://github.com/Kevin-FR/DiagCraft/releases/tag/v{}",
            self.version
        )
    }
}
#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}
#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    size: u64,
    digest: Option<String>,
}

pub(super) fn package_name(version: &Version, platform: &str) -> Option<String> {
    match platform {
        "macos" => Some(format!("diagcraft-{version}-macos-arm64.zip")),
        "windows" => Some(format!("diagcraft-{version}-windows-x86_64.zip")),
        "linux" => Some(format!("diagcraft_{version}_amd64.deb")),
        _ => None,
    }
}
fn digest(text: &str) -> Option<String> {
    let text = text.strip_prefix("sha256:").unwrap_or(text);
    (text.len() == 64 && text.bytes().all(|b| b.is_ascii_hexdigit()))
        .then(|| text.to_ascii_lowercase())
}
pub(super) fn select(
    bytes: &[u8],
    current: &Version,
    experimental: bool,
    platform: &str,
) -> Result<Option<Release>, String> {
    if bytes.len() as u64 > MAX_METADATA {
        return Err("Réponse GitHub trop volumineuse".into());
    }
    let releases: Vec<GithubRelease> =
        serde_json::from_slice(bytes).map_err(|_| "Réponse GitHub invalide")?;
    let mut selected: Option<Release> = None;
    for r in releases {
        if r.draft || (!experimental && r.prerelease) {
            continue;
        }
        let Some(tag) = r.tag_name.strip_prefix('v') else {
            continue;
        };
        if tag.len() > 100 {
            continue;
        }
        let Ok(version) = Version::parse(tag) else {
            continue;
        };
        if (!experimental && !version.pre.is_empty()) || !version.cmp_precedence(current).is_gt() {
            continue;
        }
        let Some(filename) = package_name(&version, platform) else {
            continue;
        };
        let base = format!("{DOWNLOADS}v{version}/");
        let Some(asset) = r.assets.iter().find(|a| {
            a.name == filename
                && a.browser_download_url == format!("{base}{filename}")
                && a.size > 0
                && a.size <= MAX_DOWNLOAD
        }) else {
            continue;
        };
        if asset
            .digest
            .as_ref()
            .is_some_and(|hash| digest(hash).is_none())
        {
            continue;
        }
        let Some(checksums) = r.assets.iter().find(|a| {
            a.name == "SHA256SUMS.txt"
                && a.browser_download_url == format!("{base}SHA256SUMS.txt")
                && a.size <= 65536
        }) else {
            continue;
        };
        if selected
            .as_ref()
            .is_none_or(|previous| version.cmp_precedence(&previous.version).is_gt())
        {
            selected = Some(Release {
                version: version.clone(),
                experimental: r.prerelease || !version.pre.is_empty(),
                filename,
                url: asset.browser_download_url.clone(),
                size: asset.size,
                digest: asset.digest.as_deref().and_then(digest),
                checksums_url: checksums.browser_download_url.clone(),
            });
        }
    }
    Ok(selected)
}
fn checksum(bytes: &[u8], filename: &str) -> Result<String, String> {
    if bytes.len() > 65536 {
        return Err("Empreinte de téléchargement invalide".into());
    }
    let text = std::str::from_utf8(bytes).map_err(|_| "Empreinte de téléchargement invalide")?;
    let matches: Vec<_> = text
        .lines()
        .filter_map(|line| line.split_once("  "))
        .filter(|(_, name)| *name == filename)
        .collect();
    if matches.len() != 1 {
        return Err("Empreinte de téléchargement introuvable".into());
    }
    digest(matches[0].0).ok_or_else(|| "Empreinte de téléchargement invalide".into())
}
fn curl() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
            .join("System32/curl.exe")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/usr/bin/curl")
    }
}
fn fetch(
    url: &str,
    file: &Path,
    limit: u64,
    cancel: &AtomicBool,
    timeout: u32,
) -> Result<(), String> {
    if cancel.load(Ordering::Relaxed) {
        return Err("Téléchargement annulé".into());
    }
    let mut error = tempfile::tempfile().map_err(|_| "Impossible de préparer le téléchargement")?;
    let mut child = Command::new(curl())
        .args([
            "--disable", // Ignore curlrc: only the fixed HTTPS request below is allowed.
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-redirs",
            "3",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
            "--tlsv1.2",
            "--connect-timeout",
            "10",
            "--max-time",
            &timeout.to_string(),
            "--max-filesize",
            &limit.to_string(),
            "--user-agent",
            concat!("DiagCraft/", env!("CARGO_PKG_VERSION")),
            "--header",
            "Accept: application/vnd.github+json",
            "--header",
            "X-GitHub-Api-Version: 2026-03-10",
            "--output",
        ])
        .arg(file)
        .arg(url)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(
            error
                .try_clone()
                .map_err(|_| "Impossible de préparer le téléchargement")?,
        ))
        .spawn()
        .map_err(|_| "Le service de téléchargement du système est indisponible")?;
    loop {
        if cancel.load(Ordering::Relaxed) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("Téléchargement annulé".into());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    let _ = error.seek(SeekFrom::Start(0));
                    let mut details = String::new();
                    let _ = error.take(8192).read_to_string(&mut details);
                    return Err(if details.contains("403") || details.contains("429") {
                        "Limite GitHub atteinte. Réessayez plus tard."
                    } else {
                        "Connexion à GitHub impossible. Vérifiez votre connexion et réessayez."
                    }
                    .into());
                }
                if std::fs::metadata(file)
                    .map_err(|_| "Téléchargement incomplet")?
                    .len()
                    > limit
                {
                    return Err("Téléchargement trop volumineux".into());
                }
                return Ok(());
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(100)),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err("Téléchargement interrompu".into());
            }
        }
    }
}
fn fetch_bytes(url: &str, limit: u64, cancel: &AtomicBool) -> Result<Vec<u8>, String> {
    let file =
        tempfile::NamedTempFile::new().map_err(|_| "Impossible de préparer le téléchargement")?;
    fetch(url, file.path(), limit, cancel, 30)?;
    let mut bytes = vec![];
    File::open(file.path())
        .map_err(|_| "Téléchargement incomplet")?
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Téléchargement incomplet")?;
    if bytes.len() as u64 > limit {
        return Err("Téléchargement trop volumineux".into());
    }
    Ok(bytes)
}
pub(super) fn verify_and_persist(
    mut file: tempfile::NamedTempFile,
    destination: &Path,
    expected: &str,
    size: u64,
    cancel: &AtomicBool,
) -> Result<PathBuf, String> {
    file.as_file_mut()
        .seek(SeekFrom::Start(0))
        .map_err(|_| "Téléchargement incomplet")?;
    let mut hasher = Sha256::new();
    let mut buffer = [0; 65536];
    let mut total = 0;
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err("Téléchargement annulé".into());
        }
        let count = file
            .read(&mut buffer)
            .map_err(|_| "Téléchargement incomplet")?;
        if count == 0 {
            break;
        }
        total += count as u64;
        hasher.update(&buffer[..count]);
    }
    if total != size || format!("{:x}", hasher.finalize()) != expected {
        return Err("L’empreinte SHA-256 ne correspond pas. Le fichier a été supprimé.".into());
    }
    file.as_file()
        .sync_all()
        .map_err(|_| "Impossible d’enregistrer le téléchargement")?;
    if cancel.load(Ordering::Relaxed) {
        return Err("Téléchargement annulé".into());
    }
    file.persist(destination)
        .map_err(|_| "Impossible d’enregistrer le téléchargement")?;
    Ok(destination.to_owned())
}
#[derive(Debug)]
pub(super) enum Completed {
    Checked(Result<Option<Release>, String>),
    Downloaded(Result<PathBuf, String>),
}
pub(super) struct Job {
    pub receiver: mpsc::Receiver<Completed>,
    pub cancel: Arc<AtomicBool>,
}
impl Drop for Job {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
    }
}
pub(super) fn check(experimental: bool) -> Job {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        let supported = matches!(
            (std::env::consts::OS, std::env::consts::ARCH),
            ("macos", "aarch64") | ("windows" | "linux", "x86_64")
        );
        let result = if supported {
            fetch_bytes(API, MAX_METADATA, &flag)
        } else {
            Err("Cette architecture n’a pas de paquet de mise à jour.".into())
        }
        .and_then(|bytes| {
            select(
                &bytes,
                &Version::parse(env!("CARGO_PKG_VERSION")).unwrap(),
                experimental,
                std::env::consts::OS,
            )
        });
        let _ = sender.send(Completed::Checked(result));
    });
    Job { receiver, cancel }
}
pub(super) fn download(release: Release, destination: PathBuf) -> Job {
    let (sender, receiver) = mpsc::channel();
    let cancel = Arc::new(AtomicBool::new(false));
    let flag = cancel.clone();
    std::thread::spawn(move || {
        let result = (|| {
            let expected = checksum(
                &fetch_bytes(&release.checksums_url, 65536, &flag)?,
                &release.filename,
            )?;
            if release
                .digest
                .as_ref()
                .is_some_and(|hash| hash != &expected)
            {
                return Err("Les empreintes GitHub ne correspondent pas".into());
            }
            let parent = destination
                .parent()
                .ok_or("Emplacement de téléchargement invalide")?;
            let file = tempfile::NamedTempFile::new_in(parent)
                .map_err(|_| "Impossible de préparer le téléchargement")?;
            fetch(
                &release.url,
                file.path(),
                release.size.min(MAX_DOWNLOAD),
                &flag,
                300,
            )?;
            if flag.load(Ordering::Relaxed) {
                return Err("Téléchargement annulé".into());
            }
            verify_and_persist(file, &destination, &expected, release.size, &flag)
        })();
        let _ = sender.send(Completed::Downloaded(result));
    });
    Job { receiver, cancel }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};
    use std::io::Write;

    fn release(tag: &str, prerelease: bool, platform: &str) -> Value {
        let version = Version::parse(tag.strip_prefix('v').unwrap()).unwrap();
        let filename = package_name(&version, platform).unwrap();
        json!({"tag_name":tag,"draft":false,"prerelease":prerelease,"assets":[
            {"name":filename,"browser_download_url":format!("{DOWNLOADS}{tag}/{filename}"),"size":123,"digest":format!("sha256:{}", "a".repeat(64))},
            {"name":"SHA256SUMS.txt","browser_download_url":format!("{DOWNLOADS}{tag}/SHA256SUMS.txt"),"size":400}
        ]})
    }
    fn choose(releases: Vec<Value>, experimental: bool) -> Option<Release> {
        select(
            &serde_json::to_vec(&releases).unwrap(),
            &Version::parse("0.2.1").unwrap(),
            experimental,
            "macos",
        )
        .unwrap()
    }
    #[test]
    fn highest_semver_and_channel_are_selected_independently_of_api_order() {
        let stable = release("v0.3.0", false, "macos");
        let beta = release("v0.10.0-beta.2", true, "macos");
        let releases = vec![
            stable.clone(),
            beta.clone(),
            release("v0.9.0", false, "macos"),
        ];
        assert_eq!(
            choose(releases.clone(), true).unwrap().version.to_string(),
            "0.10.0-beta.2"
        );
        assert_eq!(
            choose(releases, false).unwrap().version.to_string(),
            "0.9.0"
        );
        let mut draft = stable.clone();
        draft["draft"] = json!(true);
        assert!(choose(vec![draft, beta], false).is_none());
        assert!(choose(vec![release("v0.2.1+metadata", false, "macos")], true).is_none());
        assert!(choose(vec![release("v0.2.0", false, "macos")], true).is_none());
    }
    #[test]
    fn mac_package_is_always_arm64_and_untrusted_urls_and_incomplete_releases_are_ignored() {
        assert_eq!(
            package_name(&Version::parse("0.3.0").unwrap(), "macos").unwrap(),
            "diagcraft-0.3.0-macos-arm64.zip"
        );
        for variant in 0..6 {
            let mut r = release("v0.3.0", false, "macos");
            match variant {
                0 => r["assets"][0]["browser_download_url"] = json!("https://example.org/malware"),
                1 => r["assets"][0]["name"] = json!("diagcraft-0.3.0-macos-x86_64.zip"),
                2 => r["assets"][0]["size"] = json!(MAX_DOWNLOAD + 1),
                3 => r["assets"][0]["digest"] = json!("not a checksum"),
                4 => {
                    r["assets"][1]["browser_download_url"] =
                        json!("https://example.org/SHA256SUMS.txt")
                }
                _ => {
                    r["assets"].as_array_mut().unwrap().pop();
                }
            }
            assert!(choose(vec![r], true).is_none(), "{variant}");
        }
        assert!(choose(vec![release("v0.3.0", false, "linux")], true).is_none());
        assert!(select(b"<html>error</html>", &Version::new(0, 2, 1), true, "macos").is_err());
    }
    #[test]
    fn checksums_require_one_exact_filename_and_sixty_four_hex_digits() {
        let valid = format!(
            "{}  app.zip\n{}  other.zip\n",
            "a".repeat(64),
            "b".repeat(64)
        );
        assert_eq!(
            checksum(valid.as_bytes(), "app.zip").unwrap(),
            "a".repeat(64)
        );
        assert!(checksum(valid.as_bytes(), "zip").is_err());
        assert!(checksum(format!("{valid}{valid}").as_bytes(), "app.zip").is_err());
        assert!(checksum(b"abc  app.zip", "app.zip").is_err());
    }
    #[test]
    fn only_verified_complete_downloads_replace_the_destination_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("package.zip");
        std::fs::write(&destination, b"existing package").unwrap();
        let expected = format!("{:x}", Sha256::digest(b"verified package"));
        for (data, size, canceled) in [
            (b"tampered package".as_slice(), 16, false),
            (b"verified package", 999, false),
            (b"verified package", 16, true),
        ] {
            let mut temp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
            temp.write_all(data).unwrap();
            let path = temp.path().to_owned();
            assert!(
                verify_and_persist(
                    temp,
                    &destination,
                    &expected,
                    size,
                    &AtomicBool::new(canceled)
                )
                .is_err()
            );
            assert!(!path.exists());
            assert_eq!(std::fs::read(&destination).unwrap(), b"existing package");
        }
        let mut temp = tempfile::NamedTempFile::new_in(dir.path()).unwrap();
        temp.write_all(b"verified package").unwrap();
        verify_and_persist(temp, &destination, &expected, 16, &AtomicBool::new(false)).unwrap();
        assert_eq!(std::fs::read(&destination).unwrap(), b"verified package");
    }
    #[test]
    fn cancel_before_fetch_does_not_start_a_process_or_write_a_file() {
        let dir = tempfile::tempdir().unwrap();
        let destination = dir.path().join("none");
        assert!(fetch(API, &destination, 100, &AtomicBool::new(true), 1).is_err());
        assert!(!destination.exists());
    }
    #[test]
    #[ignore = "Explicit live GitHub transport and SHA-256 integration check"]
    fn github_live_download_is_verified() {
        let bytes = fetch_bytes(API, MAX_METADATA, &AtomicBool::new(false)).unwrap();
        let release = select(&bytes, &Version::new(0, 1, 0), true, std::env::consts::OS)
            .unwrap()
            .unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(&release.filename);
        let job = download(release, path.clone());
        let result = job.receiver.recv_timeout(Duration::from_secs(360)).unwrap();
        match result {
            Completed::Downloaded(result) => assert_eq!(result.unwrap(), path),
            _ => panic!("unexpected completion"),
        }
    }
}
