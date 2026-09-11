//! Download, verify, extract, activate -- and undo.
//!
//! The shape is versioned directories plus a pointer, not an overwrite:
//!
//! ```text
//! tools\forgepact\1.3.16\     the version that is live
//! tools\forgepact\1.3.14\     the one before it, kept
//! tools\forgepact\current.json
//! ```
//!
//! An install extracts into `.staging-<version>` and becomes visible only when a
//! rename succeeds, so an interrupted install leaves rubbish in a staging
//! directory and a working installation untouched. Rollback is then a pointer
//! change rather than a re-download.

use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::catalog::Tool;
use crate::paths::Layout;
use crate::state::{self, Current, Installed};
use crate::verify::{self, VerifyError};

#[derive(Debug)]
pub enum InstallError {
    Io(String),
    Network(String),
    Verify(VerifyError),
    Archive(String),
    /// The catalog describes something the hub will not install -- an NSIS
    /// setup, or a tool for another platform.
    Unsupported(String),
    /// A D5 interlock said no. Carries the sentence the UI shows.
    Blocked(String),
}

impl std::fmt::Display for InstallError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InstallError::Io(e) => write!(f, "{e}"),
            InstallError::Network(e) => write!(f, "{e}"),
            InstallError::Verify(e) => write!(f, "{e}"),
            InstallError::Archive(e) => write!(f, "archive: {e}"),
            InstallError::Unsupported(e) => write!(f, "{e}"),
            InstallError::Blocked(e) => write!(f, "{e}"),
        }
    }
}

impl From<std::io::Error> for InstallError {
    fn from(value: std::io::Error) -> Self {
        InstallError::Io(value.to_string())
    }
}

impl From<VerifyError> for InstallError {
    fn from(value: VerifyError) -> Self {
        InstallError::Verify(value)
    }
}

pub type Result<T> = std::result::Result<T, InstallError>;

/// What the downloads drawer shows. `Verifying` is a step of its own rather than
/// part of `Downloading`, because hash checking is the reason any of this is
/// trustworthy and a progress bar that hides it implies it did not happen.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "lowercase")]
pub enum Progress {
    Started { id: String, total: u64 },
    Downloading { id: String, received: u64, total: u64 },
    Verifying { id: String },
    Extracting { id: String },
    Activating { id: String },
    Done { id: String, version: String },
    Failed { id: String, error: String },
}

/// A record of what was written, kept inside the version directory so it travels
/// with what it describes. `verify_installed` re-hashes against it, which is how
/// the overflow menu's Verify can answer offline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub version: String,
    pub artifact_sha256: String,
    pub files: BTreeMap<String, String>,
}

fn emit(on_progress: &dyn Fn(Progress), progress: Progress) {
    on_progress(progress);
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/// Fetch an artifact into the download cache and verify it against the catalog.
///
/// A file that hashes wrong is deleted rather than kept: leaving it invites a
/// later run to find it, skip the download, and trust it.
pub fn download(tool: &Tool, layout: &Layout, on_progress: &dyn Fn(Progress)) -> Result<PathBuf> {
    let downloads = layout.downloads();
    std::fs::create_dir_all(&downloads)?;
    let target = downloads.join(crate::paths::safe_component(&tool.artifact.name));

    // Already here and still correct? Then the bytes are the bytes.
    if target.exists() {
        emit(on_progress, Progress::Verifying { id: tool.id.clone() });
        if let Ok(actual) = verify::sha256_file(&target) {
            if verify::expect_sha256(&actual, &tool.artifact.sha256).is_ok() {
                return Ok(target);
            }
        }
        std::fs::remove_file(&target).ok();
    }

    emit(
        on_progress,
        Progress::Started {
            id: tool.id.clone(),
            total: tool.artifact.size,
        },
    );

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .user_agent(concat!("hero-siege-toolkit-hub/", env!("CARGO_PKG_VERSION")))
        .build();
    let response = agent
        .get(&tool.artifact.url)
        .call()
        .map_err(|e| InstallError::Network(format!("could not start the download: {e}")))?;

    // Written to `.part` so a half-finished download is never mistaken for a
    // complete one by the cache check above.
    let part = target.with_extension("part");
    let mut file = std::fs::File::create(&part)?;
    let mut reader = response.into_reader();
    let mut buffer = vec![0u8; 128 * 1024];
    let mut received: u64 = 0;
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|e| InstallError::Network(format!("the download stopped: {e}")))?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        received += read as u64;
        emit(
            on_progress,
            Progress::Downloading {
                id: tool.id.clone(),
                received,
                total: tool.artifact.size,
            },
        );
    }
    file.flush()?;
    drop(file);

    emit(on_progress, Progress::Verifying { id: tool.id.clone() });
    let actual = verify::sha256_file(&part)?;
    if let Err(error) = verify::expect_sha256(&actual, &tool.artifact.sha256) {
        std::fs::remove_file(&part).ok();
        return Err(error.into());
    }

    if target.exists() {
        std::fs::remove_file(&target)?;
    }
    std::fs::rename(&part, &target)?;
    Ok(target)
}

// ---------------------------------------------------------------------------
// Extract
// ---------------------------------------------------------------------------

/// Strip the archive's wrapping directory and refuse anything that tries to
/// leave the destination.
///
/// The second half is zip-slip: an entry named `..\..\Windows\System32\x.dll`
/// is a normal zip and a hostile one. The catalog is signed, so this is not the
/// expected path -- it is the one that matters when an artifact is not what the
/// catalog thought it was.
fn safe_entry_path(name: &str, strip_prefix: &str) -> Option<PathBuf> {
    let normalized = name.replace('\\', "/");
    let relative = if strip_prefix.is_empty() {
        normalized.as_str()
    } else {
        normalized.strip_prefix(strip_prefix)?
    };
    if relative.is_empty() {
        return None;
    }
    let mut out = PathBuf::new();
    for component in relative.split('/') {
        match component {
            "" | "." => continue,
            ".." => return None,
            other => {
                if other.contains(':') {
                    return None; // a drive letter or an NTFS stream
                }
                out.push(other);
            }
        }
    }
    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

fn extract_zip(archive: &Path, destination: &Path, strip_prefix: &str) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(std::io::BufReader::new(file))
        .map_err(|e| InstallError::Archive(e.to_string()))?;

    let mut wrote_anything = false;
    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|e| InstallError::Archive(e.to_string()))?;
        let name = entry.name().to_string();
        let Some(relative) = safe_entry_path(&name, strip_prefix) else {
            if entry.is_dir() || name.replace('\\', "/") == strip_prefix {
                continue;
            }
            return Err(InstallError::Archive(format!(
                "the archive contains an entry that would be written outside the \
                 install directory: {name}"
            )));
        };
        let out = destination.join(&relative);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut writer = std::io::BufWriter::new(std::fs::File::create(&out)?);
        std::io::copy(&mut entry, &mut writer)?;
        writer.flush()?;
        wrote_anything = true;
    }

    if !wrote_anything {
        return Err(InstallError::Archive(format!(
            "nothing was extracted. The catalog says this archive is wrapped in \
             {strip_prefix:?}; it is not."
        )));
    }
    Ok(())
}

fn hash_tree(root: &Path) -> Result<BTreeMap<String, String>> {
    let mut files = BTreeMap::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            if relative == ".hub-manifest.json" {
                continue;
            }
            files.insert(relative, verify::sha256_file(&path)?);
        }
    }
    Ok(files)
}

// ---------------------------------------------------------------------------
// Install
// ---------------------------------------------------------------------------

/// Put a verified artifact in place and make it the live version.
///
/// The artifact must already have been checked by `download`; this re-checks it
/// anyway, because a staged install can sit in the cache across a reboot and the
/// cost of hashing it again is a second against installing something that was
/// modified while it waited.
pub fn install_artifact(
    layout: &Layout,
    tool: &Tool,
    artifact: &Path,
    on_progress: &dyn Fn(Progress),
) -> Result<Installed> {
    if !tool.is_installable() {
        return Err(InstallError::Unsupported(format!(
            "{} ships an installer the hub does not run. Install it from its \
             release page instead.",
            tool.name
        )));
    }

    emit(on_progress, Progress::Verifying { id: tool.id.clone() });
    let actual = verify::sha256_file(artifact)?;
    verify::expect_sha256(&actual, &tool.artifact.sha256)?;

    let staging = layout.staging_dir(&tool.id, &tool.version);
    if staging.exists() {
        // Left over from an install that did not finish. There is nothing in it
        // worth keeping -- the previous attempt never reached the rename.
        std::fs::remove_dir_all(&staging)?;
    }
    std::fs::create_dir_all(&staging)?;

    emit(on_progress, Progress::Extracting { id: tool.id.clone() });
    let outcome = if tool.is_archive() {
        extract_zip(artifact, &staging, &tool.artifact.strip_prefix)
    } else {
        let name = tool
            .artifact
            .install_as
            .clone()
            .unwrap_or_else(|| tool.artifact.name.clone());
        let Some(target) = safe_entry_path(&name, "") else {
            return Err(InstallError::Archive(format!(
                "{name:?} is not a usable filename"
            )));
        };
        std::fs::copy(artifact, staging.join(target)).map(|_| ()).map_err(Into::into)
    };
    if let Err(error) = outcome {
        std::fs::remove_dir_all(&staging).ok();
        return Err(error);
    }

    // The launch path has to exist before this is called an installation --
    // otherwise the failure surfaces as a Launch button that does nothing.
    if !tool.launch.exe.is_empty() {
        let entry = staging.join(tool.launch.exe.replace('/', std::path::MAIN_SEPARATOR_STR));
        if !entry.exists() {
            std::fs::remove_dir_all(&staging).ok();
            return Err(InstallError::Archive(format!(
                "{} is not in the installed tree. The catalog and the artifact \
                 disagree about where the entry point is.",
                tool.launch.exe
            )));
        }
    }

    let manifest = Manifest {
        version: tool.version.clone(),
        artifact_sha256: actual.clone(),
        files: hash_tree(&staging)?,
    };
    std::fs::write(
        staging.join(".hub-manifest.json"),
        serde_json::to_string_pretty(&manifest)
            .map_err(|e| InstallError::Io(e.to_string()))?,
    )?;

    emit(on_progress, Progress::Activating { id: tool.id.clone() });
    let final_dir = layout.version_dir(&tool.id, &tool.version);
    let previous = state::read_current(&layout.current_file(&tool.id)).map(|c| c.version);

    if final_dir.exists() {
        // Reinstalling the same version -- a repair. Move the old tree aside and
        // only delete it once the new one is in place.
        let condemned = final_dir.with_extension("replaced");
        std::fs::remove_dir_all(&condemned).ok();
        std::fs::rename(&final_dir, &condemned)?;
        match std::fs::rename(&staging, &final_dir) {
            Ok(()) => {
                std::fs::remove_dir_all(&condemned).ok();
            }
            Err(error) => {
                std::fs::rename(&condemned, &final_dir).ok();
                std::fs::remove_dir_all(&staging).ok();
                return Err(error.into());
            }
        }
    } else {
        std::fs::rename(&staging, &final_dir)?;
    }

    let previous = previous.filter(|v| v != &tool.version);
    state::write_current(
        &layout.current_file(&tool.id),
        &Current {
            version: tool.version.clone(),
            previous: previous.clone(),
        },
    )?;
    prune_old_versions(layout, &tool.id, &tool.version, previous.as_deref());

    emit(
        on_progress,
        Progress::Done {
            id: tool.id.clone(),
            version: tool.version.clone(),
        },
    );

    Ok(Installed {
        version: tool.version.clone(),
        installed_at: state::now_iso(),
        sha256: actual,
        path: final_dir.to_string_lossy().to_string(),
        previous,
    })
}

/// Download and install in one go.
pub fn install(layout: &Layout, tool: &Tool, on_progress: &dyn Fn(Progress)) -> Result<Installed> {
    let artifact = match download(tool, layout, on_progress) {
        Ok(path) => path,
        Err(error) => {
            emit(
                on_progress,
                Progress::Failed {
                    id: tool.id.clone(),
                    error: error.to_string(),
                },
            );
            return Err(error);
        }
    };
    match install_artifact(layout, tool, &artifact, on_progress) {
        Ok(installed) => Ok(installed),
        Err(error) => {
            emit(
                on_progress,
                Progress::Failed {
                    id: tool.id.clone(),
                    error: error.to_string(),
                },
            );
            Err(error)
        }
    }
}

/// Keep the live version and the one behind it. Anything older cannot be rolled
/// back to through the UI, so it is disk a player did not ask to spend.
fn prune_old_versions(layout: &Layout, id: &str, keep: &str, also_keep: Option<&str>) {
    let Ok(entries) = std::fs::read_dir(layout.tool_dir(id)) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.starts_with('.') || name == keep || Some(name) == also_keep {
            continue;
        }
        std::fs::remove_dir_all(&path).ok();
    }
}

// ---------------------------------------------------------------------------
// Rollback, verify, uninstall
// ---------------------------------------------------------------------------

pub fn rollback(layout: &Layout, id: &str) -> Result<Installed> {
    let current_path = layout.current_file(id);
    let Some(current) = state::read_current(&current_path) else {
        return Err(InstallError::Io(format!("{id} is not installed")));
    };
    let Some(previous) = current.previous.clone() else {
        return Err(InstallError::Io(format!(
            "there is no earlier version of {id} to go back to"
        )));
    };
    let target = layout.version_dir(id, &previous);
    if !target.is_dir() {
        return Err(InstallError::Io(format!(
            "version {previous} of {id} is no longer on disk"
        )));
    }

    state::write_current(
        &current_path,
        &Current {
            version: previous.clone(),
            // The version we just came from becomes the thing to roll *forward*
            // to, so a rollback can be undone with a second rollback.
            previous: Some(current.version.clone()),
        },
    )?;

    let manifest = read_manifest(layout, id, &previous);
    Ok(Installed {
        version: previous.clone(),
        installed_at: state::now_iso(),
        sha256: manifest.map(|m| m.artifact_sha256).unwrap_or_default(),
        path: target.to_string_lossy().to_string(),
        previous: Some(current.version),
    })
}

fn read_manifest(layout: &Layout, id: &str, version: &str) -> Option<Manifest> {
    serde_json::from_str(&std::fs::read_to_string(layout.manifest_file(id, version)).ok()?).ok()
}

#[derive(Debug, Clone, Serialize)]
pub struct VerifyReport {
    pub ok: bool,
    pub checked: usize,
    pub changed: Vec<String>,
    pub missing: Vec<String>,
    pub message: String,
}

/// Re-hash an installed tree against the manifest written when it was installed.
///
/// Offline by design -- it answers "is what I installed still what I installed",
/// which is the question a player actually has, and does it without a download.
pub fn verify_installed(layout: &Layout, id: &str, version: &str) -> VerifyReport {
    let Some(manifest) = read_manifest(layout, id, version) else {
        return VerifyReport {
            ok: false,
            checked: 0,
            changed: vec![],
            missing: vec![],
            message: "this version was installed without a manifest; reinstall to \
                      make it checkable"
                .into(),
        };
    };
    let root = layout.version_dir(id, version);
    let mut changed = Vec::new();
    let mut missing = Vec::new();
    for (relative, expected) in &manifest.files {
        let path = root.join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
        match verify::sha256_file(&path) {
            Ok(actual) if actual.eq_ignore_ascii_case(expected) => {}
            Ok(_) => changed.push(relative.clone()),
            Err(_) => missing.push(relative.clone()),
        }
    }
    let ok = changed.is_empty() && missing.is_empty();
    let message = if ok {
        format!("all {} files match the manifest", manifest.files.len())
    } else {
        format!(
            "{} changed, {} missing out of {}",
            changed.len(),
            missing.len(),
            manifest.files.len()
        )
    };
    VerifyReport {
        ok,
        checked: manifest.files.len(),
        changed,
        missing,
        message,
    }
}

/// Remove every version of a tool.
///
/// Program files only: the tool's own `%LOCALAPPDATA%` directory, its saves and
/// its backups are not the hub's to delete (D6).
pub fn uninstall(layout: &Layout, id: &str) -> Result<()> {
    let dir = layout.tool_dir(id);
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{Artifact, Launch, Requires};

    fn temp_root(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "hub-install-{tag}-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        dir
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

    fn tool(id: &str, kind: &str, sha: &str, size: u64, prefix: &str, exe: &str) -> Tool {
        Tool {
            id: id.into(),
            name: id.into(),
            summary: String::new(),
            repo: "o/r".into(),
            submodule: String::new(),
            version: "1.0.0".into(),
            tag: "v1.0.0".into(),
            published: String::new(),
            license: "MIT".into(),
            requires: Requires::default(),
            artifact: Artifact {
                kind: kind.into(),
                name: format!("{id}.zip"),
                url: "https://example.invalid/x".into(),
                size,
                sha256: sha.into(),
                sha256_source: "computed".into(),
                strip_prefix: prefix.into(),
                install_as: None,
            },
            launch: Launch {
                exe: exe.into(),
                ..Launch::default()
            },
            source_launch: None,
            notes_url: String::new(),
            notes: String::new(),
            guide: String::new(),
        }
    }

    fn silent() -> impl Fn(Progress) {
        |_| {}
    }

    #[test]
    fn a_wrapping_directory_is_stripped_and_the_entry_point_lands_at_the_root() {
        let root = temp_root("strip");
        let layout = Layout::new(&root);
        let archive_bytes = zip_bytes(&[
            ("ForgePact-1.3.16/ForgePact.exe", b"MZ"),
            ("ForgePact-1.3.16/modfiles/AurieCore.dll", b"MZ dll"),
        ]);
        let archive = root.join("a.zip");
        std::fs::write(&archive, &archive_bytes).unwrap();

        let tool = tool(
            "forgepact",
            "zip",
            &verify::sha256_bytes(&archive_bytes),
            archive_bytes.len() as u64,
            "ForgePact-1.3.16/",
            "ForgePact.exe",
        );
        let installed = install_artifact(&layout, &tool, &archive, &silent()).unwrap();

        let dir = PathBuf::from(&installed.path);
        assert!(dir.join("ForgePact.exe").is_file());
        assert!(dir.join("modfiles/AurieCore.dll").is_file());
        assert!(!dir.join("ForgePact-1.3.16").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_artifact_that_does_not_match_the_pinned_hash_is_refused() {
        let root = temp_root("badhash");
        let layout = Layout::new(&root);
        let archive_bytes = zip_bytes(&[("x.exe", b"MZ")]);
        let archive = root.join("a.zip");
        std::fs::write(&archive, &archive_bytes).unwrap();

        let tool = tool("t", "zip", &"0".repeat(64), 2, "", "x.exe");
        let error = install_artifact(&layout, &tool, &archive, &silent()).unwrap_err();
        assert!(
            matches!(error, InstallError::Verify(VerifyError::HashMismatch { .. })),
            "{error}"
        );
        // Nothing was created: a refused install is not a partial one.
        assert!(!layout.version_dir("t", "1.0.0").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_archive_that_would_write_outside_the_install_directory_is_refused() {
        let root = temp_root("zipslip");
        let layout = Layout::new(&root);
        let archive_bytes = zip_bytes(&[("../../escaped.txt", b"nope"), ("x.exe", b"MZ")]);
        let archive = root.join("a.zip");
        std::fs::write(&archive, &archive_bytes).unwrap();

        let tool = tool(
            "t",
            "zip",
            &verify::sha256_bytes(&archive_bytes),
            archive_bytes.len() as u64,
            "",
            "x.exe",
        );
        let error = install_artifact(&layout, &tool, &archive, &silent()).unwrap_err();
        assert!(matches!(error, InstallError::Archive(_)), "{error}");
        assert!(!root.join("escaped.txt").exists());
        assert!(!layout.version_dir("t", "1.0.0").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_archive_missing_the_declared_entry_point_is_refused() {
        let root = temp_root("noentry");
        let layout = Layout::new(&root);
        let archive_bytes = zip_bytes(&[("something-else.exe", b"MZ")]);
        let archive = root.join("a.zip");
        std::fs::write(&archive, &archive_bytes).unwrap();

        let tool = tool(
            "t",
            "zip",
            &verify::sha256_bytes(&archive_bytes),
            archive_bytes.len() as u64,
            "",
            "expected.exe",
        );
        let error = install_artifact(&layout, &tool, &archive, &silent()).unwrap_err();
        assert!(error.to_string().contains("expected.exe"), "{error}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn a_bare_exe_is_installed_under_its_stable_name() {
        let root = temp_root("bareexe");
        let layout = Layout::new(&root);
        let payload = b"MZ item editor".to_vec();
        let artifact = root.join("HeroSiegeItemEditor-v2.15.4-s10.exe");
        std::fs::write(&artifact, &payload).unwrap();

        let mut tool = tool(
            "item-editor",
            "exe",
            &verify::sha256_bytes(&payload),
            payload.len() as u64,
            "",
            "HeroSiegeItemEditor.exe",
        );
        tool.artifact.name = "HeroSiegeItemEditor-v2.15.4-s10.exe".into();
        tool.artifact.install_as = Some("HeroSiegeItemEditor.exe".into());

        let installed = install_artifact(&layout, &tool, &artifact, &silent()).unwrap();
        assert!(PathBuf::from(&installed.path)
            .join("HeroSiegeItemEditor.exe")
            .is_file());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_nsis_installer_is_not_something_the_hub_installs() {
        let root = temp_root("nsis");
        let layout = Layout::new(&root);
        let artifact = root.join("setup.exe");
        std::fs::write(&artifact, b"MZ").unwrap();
        let tool = tool("t", "nsis", &verify::sha256_bytes(b"MZ"), 2, "", "setup.exe");
        let error = install_artifact(&layout, &tool, &artifact, &silent()).unwrap_err();
        assert!(matches!(error, InstallError::Unsupported(_)), "{error}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_interrupted_extract_leaves_the_previous_install_alone() {
        let root = temp_root("interrupted");
        let layout = Layout::new(&root);

        // Install 1.0.0 properly.
        let good = zip_bytes(&[("x.exe", b"MZ v1")]);
        let good_path = root.join("good.zip");
        std::fs::write(&good_path, &good).unwrap();
        let mut tool_v1 = tool(
            "t",
            "zip",
            &verify::sha256_bytes(&good),
            good.len() as u64,
            "",
            "x.exe",
        );
        tool_v1.version = "1.0.0".into();
        install_artifact(&layout, &tool_v1, &good_path, &silent()).unwrap();

        // A 2.0.0 whose archive is broken must not disturb it.
        let broken = root.join("broken.zip");
        std::fs::write(&broken, b"this is not a zip file at all").unwrap();
        let mut tool_v2 = tool_v1.clone();
        tool_v2.version = "2.0.0".into();
        tool_v2.artifact.sha256 = verify::sha256_bytes(b"this is not a zip file at all");
        let error = install_artifact(&layout, &tool_v2, &broken, &silent()).unwrap_err();
        assert!(matches!(error, InstallError::Archive(_)), "{error}");

        assert_eq!(
            std::fs::read(layout.version_dir("t", "1.0.0").join("x.exe")).unwrap(),
            b"MZ v1"
        );
        assert!(!layout.version_dir("t", "2.0.0").exists());
        assert!(!layout.staging_dir("t", "2.0.0").exists());
        assert_eq!(
            state::read_current(&layout.current_file("t")).unwrap().version,
            "1.0.0"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn stale_staging_from_a_previous_attempt_does_not_poison_the_next_install() {
        let root = temp_root("stalestaging");
        let layout = Layout::new(&root);
        let staging = layout.staging_dir("t", "1.0.0");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::write(staging.join("left-over.txt"), b"junk").unwrap();

        let archive_bytes = zip_bytes(&[("x.exe", b"MZ")]);
        let archive = root.join("a.zip");
        std::fs::write(&archive, &archive_bytes).unwrap();
        let tool = tool(
            "t",
            "zip",
            &verify::sha256_bytes(&archive_bytes),
            archive_bytes.len() as u64,
            "",
            "x.exe",
        );
        let installed = install_artifact(&layout, &tool, &archive, &silent()).unwrap();
        assert!(!PathBuf::from(&installed.path).join("left-over.txt").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    fn install_version(layout: &Layout, version: &str, body: &[u8]) -> Installed {
        let archive_bytes = zip_bytes(&[("x.exe", body)]);
        let archive = layout.root().join(format!("{version}.zip"));
        std::fs::write(&archive, &archive_bytes).unwrap();
        let mut t = tool(
            "t",
            "zip",
            &verify::sha256_bytes(&archive_bytes),
            archive_bytes.len() as u64,
            "",
            "x.exe",
        );
        t.version = version.into();
        install_artifact(layout, &t, &archive, &silent()).unwrap()
    }

    #[test]
    fn rollback_returns_to_the_previous_version_and_can_be_undone() {
        let root = temp_root("rollback");
        let layout = Layout::new(&root);
        install_version(&layout, "1.0.0", b"MZ v1");
        let second = install_version(&layout, "2.0.0", b"MZ v2");
        assert_eq!(second.previous.as_deref(), Some("1.0.0"));

        let back = rollback(&layout, "t").unwrap();
        assert_eq!(back.version, "1.0.0");
        assert_eq!(
            std::fs::read(PathBuf::from(&back.path).join("x.exe")).unwrap(),
            b"MZ v1"
        );

        // Rolling back again goes forward: 2.0.0 is still on disk.
        let forward = rollback(&layout, "t").unwrap();
        assert_eq!(forward.version, "2.0.0");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rollback_with_nothing_behind_it_is_an_error_not_a_deletion() {
        let root = temp_root("rollback-none");
        let layout = Layout::new(&root);
        install_version(&layout, "1.0.0", b"MZ v1");
        assert!(rollback(&layout, "t").is_err());
        assert!(layout.version_dir("t", "1.0.0").is_dir());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn only_the_live_version_and_the_one_behind_it_are_kept() {
        let root = temp_root("prune");
        let layout = Layout::new(&root);
        install_version(&layout, "1.0.0", b"MZ v1");
        install_version(&layout, "2.0.0", b"MZ v2");
        install_version(&layout, "3.0.0", b"MZ v3");
        assert!(layout.version_dir("t", "3.0.0").is_dir());
        assert!(layout.version_dir("t", "2.0.0").is_dir());
        assert!(!layout.version_dir("t", "1.0.0").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn verify_notices_a_file_that_changed_under_it() {
        let root = temp_root("verify");
        let layout = Layout::new(&root);
        let installed = install_version(&layout, "1.0.0", b"MZ v1");

        let report = verify_installed(&layout, "t", "1.0.0");
        assert!(report.ok, "{}", report.message);
        assert_eq!(report.checked, 1);

        std::fs::write(PathBuf::from(&installed.path).join("x.exe"), b"tampered").unwrap();
        let report = verify_installed(&layout, "t", "1.0.0");
        assert!(!report.ok);
        assert_eq!(report.changed, vec!["x.exe".to_string()]);

        std::fs::remove_file(PathBuf::from(&installed.path).join("x.exe")).unwrap();
        let report = verify_installed(&layout, "t", "1.0.0");
        assert_eq!(report.missing, vec!["x.exe".to_string()]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn uninstall_removes_every_version_and_nothing_else() {
        let root = temp_root("uninstall");
        let layout = Layout::new(&root);
        install_version(&layout, "1.0.0", b"MZ v1");
        install_version(&layout, "2.0.0", b"MZ v2");
        let other = layout.tool_dir("other");
        std::fs::create_dir_all(&other).unwrap();

        uninstall(&layout, "t").unwrap();
        assert!(!layout.tool_dir("t").exists());
        assert!(other.is_dir(), "uninstalling one tool removed another");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn reinstalling_the_same_version_repairs_it_without_losing_it_on_failure() {
        let root = temp_root("repair");
        let layout = Layout::new(&root);
        let installed = install_version(&layout, "1.0.0", b"MZ v1");
        std::fs::write(PathBuf::from(&installed.path).join("x.exe"), b"tampered").unwrap();
        assert!(!verify_installed(&layout, "t", "1.0.0").ok);

        install_version(&layout, "1.0.0", b"MZ v1");
        assert!(verify_installed(&layout, "t", "1.0.0").ok);
        // A repair is not a downgrade: there is nothing to roll back to.
        let current = state::read_current(&layout.current_file("t")).unwrap();
        assert_eq!(current.version, "1.0.0");
        assert_eq!(current.previous, None);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn entry_paths_that_try_to_escape_are_rejected_and_ordinary_ones_are_not() {
        assert_eq!(
            safe_entry_path("ForgePact-1.3.16/ForgePact.exe", "ForgePact-1.3.16/"),
            Some(PathBuf::from("ForgePact.exe"))
        );
        assert_eq!(
            safe_entry_path("a/b/c.txt", ""),
            Some(PathBuf::from("a").join("b").join("c.txt"))
        );
        assert_eq!(safe_entry_path("../escape", ""), None);
        assert_eq!(safe_entry_path("..\\escape", ""), None);
        assert_eq!(safe_entry_path("a/../../escape", ""), None);
        assert_eq!(safe_entry_path("C:/windows/x.dll", ""), None);
        assert_eq!(safe_entry_path("x.txt:stream", ""), None);
        // An entry outside the declared prefix is not ours to write.
        assert_eq!(safe_entry_path("elsewhere/x", "wrapper/"), None);
    }
}
