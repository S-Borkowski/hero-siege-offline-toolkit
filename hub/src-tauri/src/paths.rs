//! Where the hub keeps things.
//!
//! ```text
//! %LOCALAPPDATA%\Hero Siege Toolkit\
//!   tools\<id>\<version>\      extracted artifact
//!   tools\<id>\current.json    which version is active, and what to roll back to
//!   cache\downloads\           hash-verified before anything uses them
//!   cache\catalog.json         last catalog that verified, with its signature
//!   state.json                 installed set, settings, last check
//!   logs\hub.log
//!   bundle\                    optional offline-bundle payload, consulted first
//! ```
//!
//! Program files only. `%LOCALAPPDATA%\Hero_Siege\forgepact.json`,
//! `%LOCALAPPDATA%\HSCraftSim\session.json`, each tool's saves and backups are
//! left exactly where the tool puts them (D6): uninstalling the hub never costs a
//! player their settings, and a tool started outside the hub behaves identically.

use std::path::{Path, PathBuf};

/// `%LOCALAPPDATA%\Hero Siege Toolkit`, or the platform equivalent.
pub fn default_install_root() -> PathBuf {
    #[cfg(windows)]
    {
        if let Ok(local) = std::env::var("LOCALAPPDATA") {
            if !local.is_empty() {
                return PathBuf::from(local).join("Hero Siege Toolkit");
            }
        }
    }
    if let Some(home) = home_dir() {
        #[cfg(windows)]
        return home.join("AppData").join("Local").join("Hero Siege Toolkit");
        #[cfg(not(windows))]
        return home.join(".local").join("share").join("hero-siege-toolkit");
    }
    std::env::temp_dir().join("hero-siege-toolkit")
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

/// A tool id, reduced to something that cannot escape the install root.
///
/// The catalog is signed, so an id is not attacker-controlled in the normal
/// case. This exists for the abnormal one: a hub running with signature
/// verification relaxed for local development, or a bundle directory somebody
/// dropped a file into. `..` and separators would otherwise let an id write
/// anywhere the user can.
pub fn safe_component(id: &str) -> String {
    let cleaned: String = id
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    // Mapping separators to `_` is already enough to keep the result inside the
    // parent directory, but it leaves `../../evil` looking like `_.._evil`.
    // Collapsing `..` as well means nothing downstream -- a log line, an error
    // message, a path handed to a shell -- ever carries a traversal fragment.
    let mut cleaned = cleaned;
    while let Some(at) = cleaned.find("..") {
        cleaned.replace_range(at..at + 2, "_");
    }
    let cleaned = cleaned.trim_matches('.').to_string();
    if cleaned.is_empty() {
        "_".to_string()
    } else {
        cleaned
    }
}

#[derive(Debug, Clone)]
pub struct Layout {
    root: PathBuf,
}

impl Layout {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn tools(&self) -> PathBuf {
        self.root.join("tools")
    }

    pub fn tool_dir(&self, id: &str) -> PathBuf {
        self.tools().join(safe_component(id))
    }

    /// Where a specific version lives. Versions are directories rather than an
    /// in-place overwrite so that rollback is a rename, not a re-download.
    pub fn version_dir(&self, id: &str, version: &str) -> PathBuf {
        self.tool_dir(id).join(safe_component(version))
    }

    /// The staging directory an install extracts into before it is made live.
    /// Never renamed into place until the whole extraction succeeded, so an
    /// interrupted install leaves rubbish here and nothing in `version_dir`.
    pub fn staging_dir(&self, id: &str, version: &str) -> PathBuf {
        self.tool_dir(id)
            .join(format!(".staging-{}", safe_component(version)))
    }

    pub fn current_file(&self, id: &str) -> PathBuf {
        self.tool_dir(id).join("current.json")
    }

    /// The per-install file manifest, written inside the version directory so it
    /// travels with what it describes. `verify_tool` re-hashes against it.
    pub fn manifest_file(&self, id: &str, version: &str) -> PathBuf {
        self.version_dir(id, version).join(".hub-manifest.json")
    }

    pub fn cache(&self) -> PathBuf {
        self.root.join("cache")
    }

    pub fn downloads(&self) -> PathBuf {
        self.cache().join("downloads")
    }

    pub fn cached_catalog(&self) -> PathBuf {
        self.cache().join("catalog.json")
    }

    pub fn cached_catalog_signature(&self) -> PathBuf {
        self.cache().join("catalog.json.minisig")
    }

    pub fn state_file(&self) -> PathBuf {
        self.root.join("state.json")
    }

    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn log_file(&self) -> PathBuf {
        self.logs().join("hub.log")
    }

    /// Consulted before the network, so an offline bundle installs with no
    /// outbound request at all.
    pub fn bundle(&self) -> PathBuf {
        self.root.join("bundle")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [self.tools(), self.downloads(), self.logs()] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traversal_in_an_id_cannot_escape_the_install_root() {
        let layout = Layout::new("C:/root");
        for hostile in ["../../evil", "..\\..\\evil", "/etc/passwd", "..", "."] {
            let dir = layout.tool_dir(hostile);
            assert!(
                dir.starts_with("C:/root/tools"),
                "{hostile:?} escaped to {dir:?}"
            );
            assert!(!dir.to_string_lossy().contains(".."));
        }
    }

    #[test]
    fn traversal_in_a_version_cannot_escape_either() {
        let layout = Layout::new("C:/root");
        let dir = layout.version_dir("forgepact", "../../../evil");
        assert!(dir.starts_with("C:/root/tools/forgepact"));
        assert!(!dir.to_string_lossy().contains(".."));
    }

    #[test]
    fn ordinary_ids_and_versions_are_left_alone() {
        assert_eq!(safe_component("hs-offline-tracker"), "hs-offline-tracker");
        assert_eq!(safe_component("hssaveeditor-steamdeck"), "hssaveeditor-steamdeck");
        assert_eq!(safe_component("1.3.16"), "1.3.16");
        assert_eq!(safe_component("2.15.4"), "2.15.4");
    }

    #[test]
    fn an_id_that_sanitizes_to_nothing_still_yields_a_directory() {
        assert_eq!(safe_component(""), "_");
        assert_eq!(safe_component("..."), "_");
    }

    #[test]
    fn staging_is_a_sibling_of_the_version_directory_not_a_child() {
        let layout = Layout::new("C:/root");
        let staging = layout.staging_dir("forgepact", "1.3.16");
        let final_dir = layout.version_dir("forgepact", "1.3.16");
        assert_ne!(staging, final_dir);
        assert_eq!(staging.parent(), final_dir.parent());
    }
}
