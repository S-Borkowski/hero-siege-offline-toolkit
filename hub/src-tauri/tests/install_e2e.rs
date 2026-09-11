//! The whole install pipeline, over real HTTP, against a server this test owns.
//!
//! The unit tests in `install.rs` start from a file already on disk, which skips
//! the half that talks to the network: streaming, the `.part` file, the cache
//! hit on a second run, and the hash check that stands between a download and an
//! extraction. This covers that, and it does it without a real release -- so the
//! loop stays fast and works offline.

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use hero_siege_toolkit_hub_lib::catalog::{Artifact, Launch, Requires, Tool};
use hero_siege_toolkit_hub_lib::install::{self, InstallError, Progress};
use hero_siege_toolkit_hub_lib::paths::Layout;
use hero_siege_toolkit_hub_lib::verify;

/// A one-route HTTP server: `GET /artifact.zip` returns `body`, anything else
/// 404s. Counts requests so a test can prove the cache was used.
struct Server {
    port: u16,
    hits: Arc<AtomicUsize>,
}

impl Server {
    fn start(body: Vec<u8>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let port = listener.local_addr().unwrap().port();
        let hits = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&hits);

        std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(mut stream) = stream else { continue };
                let mut buffer = [0u8; 2048];
                let read = stream.read(&mut buffer).unwrap_or(0);
                let request = String::from_utf8_lossy(&buffer[..read]);
                if request.starts_with("GET /artifact.zip") {
                    counter.fetch_add(1, Ordering::SeqCst);
                    respond(&mut stream, 200, "OK", &body);
                } else if request.starts_with("GET /gone.zip") {
                    respond(&mut stream, 404, "Not Found", b"no");
                } else if request.starts_with("GET /truncated.zip") {
                    // Claims more than it sends, then hangs up: what a dropped
                    // connection mid-download looks like to the client.
                    let head = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        body.len() + 4096
                    );
                    let _ = stream.write_all(head.as_bytes());
                    let _ = stream.write_all(&body);
                    let _ = stream.flush();
                } else {
                    respond(&mut stream, 404, "Not Found", b"no");
                }
            }
        });

        Self { port, hits }
    }

    fn url(&self, path: &str) -> String {
        format!("http://127.0.0.1:{}/{path}", self.port)
    }

    fn hits(&self) -> usize {
        self.hits.load(Ordering::SeqCst)
    }
}

fn respond(stream: &mut TcpStream, code: u16, reason: &str, body: &[u8]) {
    let head = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let _ = stream.write_all(head.as_bytes());
    let _ = stream.write_all(body);
    let _ = stream.flush();
}

fn zip_bytes(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (name, data) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(data).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer.into_inner()
}

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hub-e2e-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn tool(url: String, sha256: String, size: u64) -> Tool {
    Tool {
        id: "forgepact".into(),
        name: "ForgePact".into(),
        summary: "Offline gameplay modifiers.".into(),
        repo: "falorfrozen-cmd/ForgePact".into(),
        submodule: "ForgePact".into(),
        version: "1.3.16".into(),
        tag: "v1.3.16".into(),
        published: "2026-09-10T06:24:46Z".into(),
        license: "AGPL-3.0".into(),
        requires: Requires::default(),
        artifact: Artifact {
            kind: "zip".into(),
            name: "ForgePact-1.3.16.zip".into(),
            url,
            size,
            sha256,
            sha256_source: "published+verified".into(),
            // The real release wraps its tree in a versioned directory, which is
            // the case a declared `strip_prefix` got wrong.
            strip_prefix: "ForgePact-1.3.16/".into(),
            install_as: None,
        },
        launch: Launch {
            exe: "ForgePact.exe".into(),
            ports: vec![8766],
            ..Launch::default()
        },
        source_launch: None,
        notes_url: String::new(),
        notes: String::new(),
        guide: String::new(),
    }
}

/// The shape of the real ForgePact release: `ForgePact.exe` beside `modfiles/`,
/// inside one versioned directory.
fn forgepact_archive() -> Vec<u8> {
    zip_bytes(&[
        ("ForgePact-1.3.16/ForgePact.exe", b"MZ forgepact"),
        ("ForgePact-1.3.16/modfiles/AurieCore.dll", b"MZ aurie"),
        ("ForgePact-1.3.16/modfiles/BloodPactPlugin.dll", b"MZ plugin"),
        ("ForgePact-1.3.16/modfiles/AuriePatcher.exe", b"MZ patcher"),
    ])
}

fn record() -> (impl Fn(Progress), std::sync::mpsc::Receiver<String>) {
    let (tx, rx) = std::sync::mpsc::channel();
    let emit = move |progress: Progress| {
        let phase = match progress {
            Progress::Started { .. } => "started",
            Progress::Downloading { .. } => "downloading",
            Progress::Verifying { .. } => "verifying",
            Progress::Extracting { .. } => "extracting",
            Progress::Activating { .. } => "activating",
            Progress::Done { .. } => "done",
            Progress::Failed { .. } => "failed",
        };
        let _ = tx.send(phase.to_string());
    };
    (emit, rx)
}

#[test]
fn a_release_downloads_verifies_extracts_and_becomes_live() {
    let archive = forgepact_archive();
    let digest = verify::sha256_bytes(&archive);
    let size = archive.len() as u64;
    let server = Server::start(archive);
    let root = temp_root("happy");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let tool = tool(server.url("artifact.zip"), digest.clone(), size);
    let (emit, phases) = record();
    let installed = install::install(&layout, &tool, &emit).expect("install");

    assert_eq!(installed.version, "1.3.16");
    assert_eq!(installed.sha256, digest);

    let dir = PathBuf::from(&installed.path);
    assert!(dir.join("ForgePact.exe").is_file());
    assert!(dir.join("modfiles/BloodPactPlugin.dll").is_file());
    // The wrapping directory came off; the exe is where the launch rule says.
    assert!(!dir.join("ForgePact-1.3.16").exists());

    let seen: Vec<String> = phases.try_iter().collect();
    for expected in ["started", "downloading", "verifying", "extracting", "activating", "done"] {
        assert!(seen.iter().any(|p| p == expected), "never reported {expected}: {seen:?}");
    }
    // Verifying comes between downloading and extracting, so the downloads
    // drawer can show a hash check happening rather than implying one did.
    let at = |name: &str| seen.iter().position(|p| p == name).unwrap();
    assert!(at("downloading") < at("verifying"));
    assert!(at("verifying") < at("extracting"));

    assert!(install::verify_installed(&layout, "forgepact", "1.3.16").ok);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_artifact_that_does_not_match_the_catalog_is_refused_and_not_kept() {
    let archive = forgepact_archive();
    let size = archive.len() as u64;
    let server = Server::start(archive);
    let root = temp_root("badhash");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    // What a swapped release asset looks like from here: the catalog pins one
    // hash, the bytes are something else.
    let tool = tool(server.url("artifact.zip"), "0".repeat(64), size);
    let (emit, _phases) = record();
    let error = install::install(&layout, &tool, &emit).unwrap_err();
    assert!(matches!(error, InstallError::Verify(_)), "{error}");

    assert!(!layout.version_dir("forgepact", "1.3.16").exists());
    // The bad download is gone rather than sitting in the cache waiting for a
    // later run to find it and skip the download.
    let cached: Vec<_> = std::fs::read_dir(layout.downloads())
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();
    assert!(cached.is_empty(), "a refused download was kept: {cached:?}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_second_install_reuses_the_verified_download_instead_of_fetching_again() {
    let archive = forgepact_archive();
    let digest = verify::sha256_bytes(&archive);
    let size = archive.len() as u64;
    let server = Server::start(archive);
    let root = temp_root("cache");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let tool = tool(server.url("artifact.zip"), digest, size);
    let (emit, _p) = record();
    install::install(&layout, &tool, &emit).expect("first install");
    assert_eq!(server.hits(), 1);

    install::install(&layout, &tool, &emit).expect("second install");
    assert_eq!(server.hits(), 1, "the cached artifact should not be re-downloaded");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_cached_file_that_no_longer_matches_is_re_downloaded_rather_than_trusted() {
    let archive = forgepact_archive();
    let digest = verify::sha256_bytes(&archive);
    let size = archive.len() as u64;
    let server = Server::start(archive);
    let root = temp_root("stalecache");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    // Something in the cache under the right name but with the wrong contents:
    // a truncated earlier download, or a file somebody dropped there.
    std::fs::write(
        layout.downloads().join("ForgePact-1.3.16.zip"),
        b"not the artifact",
    )
    .unwrap();

    let tool = tool(server.url("artifact.zip"), digest, size);
    let (emit, _p) = record();
    let installed = install::install(&layout, &tool, &emit).expect("install");
    assert_eq!(server.hits(), 1, "the stale cache entry should have been discarded");
    assert!(PathBuf::from(&installed.path).join("ForgePact.exe").is_file());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_download_that_stops_early_fails_and_leaves_nothing_installable() {
    let archive = forgepact_archive();
    let size = archive.len() as u64 + 4096;
    let digest = verify::sha256_bytes(&archive);
    let server = Server::start(archive);
    let root = temp_root("truncated");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let tool = tool(server.url("truncated.zip"), digest, size);
    let (emit, _p) = record();
    assert!(install::install(&layout, &tool, &emit).is_err());
    assert!(!layout.version_dir("forgepact", "1.3.16").exists());
    // Whatever arrived stayed in `.part` and was never promoted to the name a
    // later run would treat as a complete download.
    assert!(!layout.downloads().join("ForgePact-1.3.16.zip").exists());
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn a_missing_artifact_reports_a_network_error_rather_than_a_hash_error() {
    let archive = forgepact_archive();
    let digest = verify::sha256_bytes(&archive);
    let size = archive.len() as u64;
    let server = Server::start(archive);
    let root = temp_root("404");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let tool = tool(server.url("gone.zip"), digest, size);
    let (emit, _p) = record();
    let error = install::install(&layout, &tool, &emit).unwrap_err();
    assert!(matches!(error, InstallError::Network(_)), "{error}");
    std::fs::remove_dir_all(&root).ok();
}

#[test]
fn an_update_keeps_the_previous_version_and_can_be_rolled_back() {
    let v1 = zip_bytes(&[("ForgePact-1.3.16/ForgePact.exe", b"MZ v1")]);
    let digest = verify::sha256_bytes(&v1);
    let size = v1.len() as u64;
    let server = Server::start(v1);
    let root = temp_root("update");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let first = tool(server.url("artifact.zip"), digest, size);
    let (emit, _p) = record();
    install::install(&layout, &first, &emit).expect("install 1.3.16");

    // A newer release, served by a second server so both remain fetchable.
    let v2 = zip_bytes(&[("ForgePact-1.3.18/ForgePact.exe", b"MZ v2")]);
    let digest2 = verify::sha256_bytes(&v2);
    let size2 = v2.len() as u64;
    let server2 = Server::start(v2);
    let mut second = tool(server2.url("artifact.zip"), digest2, size2);
    second.version = "1.3.18".into();
    second.artifact.name = "ForgePact-1.3.18.zip".into();
    second.artifact.strip_prefix = "ForgePact-1.3.18/".into();

    let updated = install::install(&layout, &second, &emit).expect("install 1.3.18");
    assert_eq!(updated.version, "1.3.18");
    assert_eq!(updated.previous.as_deref(), Some("1.3.16"));
    assert_eq!(
        std::fs::read(PathBuf::from(&updated.path).join("ForgePact.exe")).unwrap(),
        b"MZ v2"
    );

    let back = install::rollback(&layout, "forgepact").expect("rollback");
    assert_eq!(back.version, "1.3.16");
    assert_eq!(
        std::fs::read(PathBuf::from(&back.path).join("ForgePact.exe")).unwrap(),
        b"MZ v1"
    );
    std::fs::remove_dir_all(&root).ok();
}
