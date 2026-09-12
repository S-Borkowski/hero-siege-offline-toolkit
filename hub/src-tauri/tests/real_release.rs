//! The engine against the actual published releases.
//!
//! Ignored by default: these download real artifacts from GitHub, which is not
//! something `cargo test` should do on every run, in CI, or on an aeroplane.
//! They exist because the fixtures in `install_e2e.rs` can only prove the
//! pipeline is self-consistent -- they cannot prove the catalog describes the
//! releases correctly, and that is the thing most likely to rot.
//!
//! Run them deliberately:
//!
//! ```text
//! cd hub
//! cargo test --manifest-path src-tauri/Cargo.toml --test real_release -- --ignored --nocapture
//! cargo test --manifest-path src-tauri/Cargo.toml --test real_release -- --ignored --nocapture forgepact
//! ```
//!
//! Roughly 210 MB across all ten. Nothing is launched and nothing is written
//! outside a temporary directory.

use std::path::PathBuf;

use hero_siege_toolkit_hub_lib::catalog;
use hero_siege_toolkit_hub_lib::install::{self, Progress};
use hero_siege_toolkit_hub_lib::paths::Layout;

fn temp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("hub-real-{tag}-{}", std::process::id()));
    std::fs::remove_dir_all(&dir).ok();
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn noisy() -> impl Fn(Progress) {
    |progress: Progress| {
        if let Progress::Downloading { id, received, total } = &progress {
            // One line per 8 MB: enough to see it moving, not a wall of text.
            if total > &0 && received % (8 * 1024 * 1024) < 131_072 {
                println!("  {id}: {} / {} MB", received / 1_048_576, total / 1_048_576);
            }
        } else {
            println!("  {progress:?}");
        }
    }
}

#[test]
#[ignore = "downloads the real ForgePact release"]
fn forgepact_installs_from_its_real_release() {
    let loaded = catalog::embedded().expect("embedded catalog");
    let tool = loaded.catalog.tool("forgepact").expect("forgepact in catalog");
    let root = temp_root("forgepact");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    println!("installing {} v{} from {}", tool.name, tool.version, tool.artifact.url);
    let installed = install::install(&layout, tool, &noisy()).expect("install ForgePact");

    // build_release.py documents the layout as ForgePact.exe beside modfiles/,
    // with four DLLs and the patcher in it.
    let dir = PathBuf::from(&installed.path);
    assert!(dir.join("ForgePact.exe").is_file(), "no ForgePact.exe in {dir:?}");
    let modfiles = dir.join("modfiles");
    assert!(modfiles.is_dir(), "no modfiles/ directory");
    for expected in [
        "AurieCore.dll",
        "AuriePatcher.exe",
        "YYToolkit.dll",
        "BloodPactPlugin.dll",
    ] {
        assert!(
            modfiles.join(expected).is_file(),
            "modfiles/{expected} is missing -- build_release.py says it should be there"
        );
    }

    assert!(install::verify_installed(&layout, "forgepact", &tool.version).ok);
    println!("installed {} bytes to {}", tool.artifact.size, installed.path);
    std::fs::remove_dir_all(&root).ok();
}

#[test]
#[ignore = "downloads every real release (~210 MB)"]
fn every_tool_in_the_catalog_installs_and_its_entry_point_is_there() {
    let loaded = catalog::embedded().expect("embedded catalog");
    let root = temp_root("all");
    let layout = Layout::new(&root);
    layout.ensure().unwrap();

    let mut failures = Vec::new();
    for tool in &loaded.catalog.tools {
        if !tool.is_installable() {
            println!("skipping {} (kind {})", tool.id, tool.artifact.kind);
            continue;
        }
        println!("\n=== {} v{}", tool.id, tool.version);
        match install::install(&layout, tool, &noisy()) {
            Ok(installed) => {
                let entry = PathBuf::from(&installed.path)
                    .join(tool.launch.exe.replace('/', std::path::MAIN_SEPARATOR_STR));
                if entry.exists() {
                    println!("  ok -> {}", entry.display());
                } else {
                    failures.push(format!("{}: {} is not in the tree", tool.id, tool.launch.exe));
                }
            }
            Err(error) => failures.push(format!("{}: {error}", tool.id)),
        }
    }

    std::fs::remove_dir_all(&root).ok();
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}
