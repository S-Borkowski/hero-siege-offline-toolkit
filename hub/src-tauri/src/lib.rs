//! Toolkit Hub -- the Tauri side.
//!
//! A library and process manager for the ten tools in the Hero Siege Offline
//! Toolkit: it installs them from a signed catalog, runs them, notices when they
//! have a newer release, and can put one back the way it was.

pub mod catalog;
pub mod game;
pub mod install;
pub mod launch;
pub mod log;
pub mod paths;
pub mod state;
pub mod verify;
pub mod version;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use catalog::{LoadedCatalog, Source};
use paths::Layout;
use state::HubState;

/// Everything the commands share. Managed as an `Arc` because installs run on a
/// worker thread -- a 50 MB download must not hold the UI still.
pub struct Hub {
    pub layout: Layout,
    pub state: Mutex<HubState>,
    pub catalog: Mutex<LoadedCatalog>,
    /// Tool id -> PID of the process this hub started.
    pub running: Mutex<BTreeMap<String, u32>>,
    /// The checkout this hub was built in, if it can still be found. Developer
    /// mode runs tools out of the submodules under it.
    pub repo_root: Option<PathBuf>,
    pub log: log::Log,
}

impl Hub {
    fn settings(&self) -> state::Settings {
        self.state
            .lock()
            .map(|s| s.settings.clone())
            .unwrap_or_default()
    }

    fn persist(&self) {
        if let Ok(state) = self.state.lock() {
            if let Err(error) = state.save(&self.layout.state_file()) {
                self.log.error(format!("could not write state.json: {error}"));
            }
        }
    }

    fn tool(&self, id: &str) -> Result<catalog::Tool, String> {
        self.catalog
            .lock()
            .map_err(|_| "the catalog is busy".to_string())?
            .catalog
            .tool(id)
            .cloned()
            .ok_or_else(|| format!("{id} is not in the catalog"))
    }
}

// ---------------------------------------------------------------------------
// What the frontend sees
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct HubInfo {
    pub version: String,
    pub install_root: String,
    pub log_path: String,
    pub repo_root: Option<String>,
    pub catalog_url: String,
}

/// One row of the Library grid, with everything the card needs already decided.
///
/// Assembled here rather than in Svelte so that "is this an update" has exactly
/// one implementation -- the version comparison that `About.svelte` had to get
/// right for `0.9.10` against `0.9.8`.
#[derive(Debug, Clone, Serialize)]
pub struct ToolView {
    #[serde(flatten)]
    pub tool: catalog::Tool,
    pub installed_version: Option<String>,
    pub installed_at: Option<String>,
    pub installed_sha256: Option<String>,
    pub install_path: Option<String>,
    pub can_roll_back: bool,
    pub update_available: bool,
    pub running_pid: Option<u32>,
    /// A copy started outside the hub, noticed by its port being taken.
    pub running_elsewhere: bool,
    pub staged: Option<state::Staged>,
    pub source_available: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LibraryView {
    pub tools: Vec<ToolView>,
    pub catalog_generated: String,
    pub catalog_source: Source,
    pub catalog_trusted_comment: String,
    pub last_check: Option<String>,
    pub settings: state::Settings,
    pub game: game::GameStatus,
}

fn build_view(hub: &Hub) -> Result<LibraryView, String> {
    let loaded = hub
        .catalog
        .lock()
        .map_err(|_| "the catalog is busy".to_string())?
        .clone();
    let hub_state = hub
        .state
        .lock()
        .map_err(|_| "the state is busy".to_string())?
        .clone();
    let running = hub
        .running
        .lock()
        .map_err(|_| "the process table is busy".to_string())?
        .clone();
    let game = game::status();

    let mut tools = Vec::with_capacity(loaded.catalog.tools.len());
    for tool in &loaded.catalog.tools {
        let installed = hub_state.installed.get(&tool.id);
        let pid = running.get(&tool.id).copied().filter(|pid| launch::is_alive(*pid));
        let source_available = tool.source_launch.is_some()
            && hub
                .repo_root
                .as_ref()
                .map(|root| launch::submodule_path(root, &tool.submodule).is_dir())
                .unwrap_or(false);

        tools.push(ToolView {
            installed_version: installed.map(|i| i.version.clone()),
            installed_at: installed.map(|i| i.installed_at.clone()),
            installed_sha256: installed.map(|i| i.sha256.clone()),
            install_path: installed.map(|i| i.path.clone()),
            can_roll_back: installed.and_then(|i| i.previous.as_ref()).is_some(),
            update_available: installed
                .map(|i| version::is_newer(&tool.version, &i.version))
                .unwrap_or(false),
            running_pid: pid,
            running_elsewhere: pid.is_none() && launch::any_port_in_use(&tool.launch.ports),
            staged: hub_state.staged.get(&tool.id).cloned(),
            source_available,
            tool: tool.clone(),
        });
    }

    Ok(LibraryView {
        tools,
        catalog_generated: loaded.catalog.generated,
        catalog_source: loaded.source,
        catalog_trusted_comment: loaded.trusted_comment,
        last_check: hub_state.last_check,
        settings: hub_state.settings,
        game,
    })
}

fn announce(app: &AppHandle, hub: &Hub) {
    match build_view(hub) {
        Ok(view) => {
            let _ = app.emit("library-changed", view);
        }
        Err(error) => hub.log.error(format!("could not rebuild the library: {error}")),
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
fn hub_info(hub: State<'_, Arc<Hub>>) -> HubInfo {
    HubInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        install_root: hub.layout.root().to_string_lossy().to_string(),
        log_path: hub.log.path().to_string_lossy().to_string(),
        repo_root: hub.repo_root.as_ref().map(|p| p.to_string_lossy().to_string()),
        catalog_url: catalog::CATALOG_URL.to_string(),
    }
}

#[tauri::command]
fn library(hub: State<'_, Arc<Hub>>) -> Result<LibraryView, String> {
    build_view(&hub)
}

#[tauri::command]
fn get_settings(hub: State<'_, Arc<Hub>>) -> state::Settings {
    hub.settings()
}

#[tauri::command]
fn set_settings(
    app: AppHandle,
    hub: State<'_, Arc<Hub>>,
    settings: state::Settings,
) -> Result<state::Settings, String> {
    {
        let mut guard = hub.state.lock().map_err(|_| "the state is busy".to_string())?;
        guard.settings = settings;
    }
    hub.persist();
    let settings = hub.settings();
    let _ = app.emit("settings-changed", settings.clone());
    announce(&app, &hub);
    Ok(settings)
}

/// Refresh the catalog from the network, if the settings permit it.
///
/// Work offline is checked here rather than only in the UI, so a stale frontend
/// or a hand-edited state file cannot produce a request the player disabled.
#[tauri::command]
fn check_for_updates(app: AppHandle, hub: State<'_, Arc<Hub>>) -> Result<LibraryView, String> {
    let settings = hub.settings();
    if !settings.may_reach_network() {
        return Err(if settings.work_offline {
            "Work offline is on. Turn it off in Settings to check for updates.".into()
        } else {
            "The hub has not finished its first run yet.".into()
        });
    }

    let (payload, signature) = catalog::fetch_remote(Duration::from_secs(30))?;
    let loaded = catalog::accept(&payload, &signature, Source::Remote)?;

    // Cached only once it verified. A catalog that failed its signature is not
    // written anywhere the hub would read it back from.
    let _ = std::fs::create_dir_all(hub.layout.cache());
    let _ = std::fs::write(hub.layout.cached_catalog(), &payload);
    let _ = std::fs::write(hub.layout.cached_catalog_signature(), &signature);

    hub.log.info(format!(
        "catalog refreshed: generated {}, {} tools",
        loaded.catalog.generated,
        loaded.catalog.tools.len()
    ));

    *hub.catalog.lock().map_err(|_| "the catalog is busy".to_string())? = loaded;
    {
        let mut guard = hub.state.lock().map_err(|_| "the state is busy".to_string())?;
        guard.last_check = Some(state::now_iso());
    }
    hub.persist();

    let view = build_view(&hub)?;
    let _ = app.emit("library-changed", view.clone());
    Ok(view)
}

#[tauri::command]
fn install_tool(app: AppHandle, hub: State<'_, Arc<Hub>>, id: String) -> Result<(), String> {
    let tool = hub.tool(&id)?;
    let settings = hub.settings();
    if !settings.may_reach_network() && bundled_artifact(&hub, &tool).is_none() {
        return Err(
            "Work offline is on and this artifact is not in the offline bundle.".into(),
        );
    }

    let hub = Arc::clone(&hub);
    let app_for_thread = app.clone();
    std::thread::spawn(move || {
        let emitter = {
            let app = app_for_thread.clone();
            move |progress: install::Progress| {
                let _ = app.emit("install-progress", progress);
            }
        };

        let outcome = match bundled_artifact(&hub, &tool) {
            Some(path) => install::install_artifact(&hub.layout, &tool, &path, &emitter),
            None => install::install(&hub.layout, &tool, &emitter),
        };

        match outcome {
            Ok(installed) => {
                hub.log.info(format!(
                    "installed {} {} ({})",
                    tool.id, installed.version, installed.sha256
                ));
                if let Ok(mut guard) = hub.state.lock() {
                    guard.installed.insert(tool.id.clone(), installed);
                    guard.staged.remove(&tool.id);
                }
                hub.persist();
            }
            Err(error) => {
                hub.log
                    .error(format!("installing {} failed: {error}", tool.id));
            }
        }
        announce(&app_for_thread, &hub);
    });
    Ok(())
}

/// An artifact that came with an offline bundle, if it is there and correct.
fn bundled_artifact(hub: &Hub, tool: &catalog::Tool) -> Option<PathBuf> {
    let path = hub.layout.bundle().join(&tool.artifact.name);
    if !path.is_file() {
        return None;
    }
    match verify::sha256_file(&path) {
        Ok(actual) if verify::expect_sha256(&actual, &tool.artifact.sha256).is_ok() => Some(path),
        _ => None,
    }
}

#[tauri::command]
fn uninstall_tool(app: AppHandle, hub: State<'_, Arc<Hub>>, id: String) -> Result<(), String> {
    if let Some(pid) = hub.running.lock().ok().and_then(|r| r.get(&id).copied()) {
        if launch::is_alive(pid) {
            return Err("Stop the tool before uninstalling it.".into());
        }
    }
    install::uninstall(&hub.layout, &id).map_err(|e| e.to_string())?;
    {
        let mut guard = hub.state.lock().map_err(|_| "the state is busy".to_string())?;
        guard.installed.remove(&id);
        guard.staged.remove(&id);
    }
    hub.persist();
    hub.log.info(format!("uninstalled {id}"));
    announce(&app, &hub);
    Ok(())
}

#[tauri::command]
fn rollback_tool(app: AppHandle, hub: State<'_, Arc<Hub>>, id: String) -> Result<(), String> {
    let installed = install::rollback(&hub.layout, &id).map_err(|e| e.to_string())?;
    hub.log
        .info(format!("rolled {id} back to {}", installed.version));
    {
        let mut guard = hub.state.lock().map_err(|_| "the state is busy".to_string())?;
        guard.installed.insert(id, installed);
    }
    hub.persist();
    announce(&app, &hub);
    Ok(())
}

#[tauri::command]
fn verify_tool(hub: State<'_, Arc<Hub>>, id: String) -> Result<install::VerifyReport, String> {
    let version = hub
        .state
        .lock()
        .map_err(|_| "the state is busy".to_string())?
        .installed
        .get(&id)
        .map(|i| i.version.clone())
        .ok_or_else(|| format!("{id} is not installed"))?;
    Ok(install::verify_installed(&hub.layout, &id, &version))
}

#[tauri::command]
fn launch_tool(
    app: AppHandle,
    hub: State<'_, Arc<Hub>>,
    id: String,
    from_source: Option<bool>,
) -> Result<launch::Started, String> {
    let tool = hub.tool(&id)?;
    let from_source = from_source.unwrap_or(false);

    let started = if from_source {
        let root = hub
            .repo_root
            .as_ref()
            .ok_or("this hub was not built inside the toolkit checkout")?;
        launch::launch_from_source(&tool, &launch::submodule_path(root, &tool.submodule))
    } else {
        let dir = hub
            .state
            .lock()
            .map_err(|_| "the state is busy".to_string())?
            .installed
            .get(&id)
            .map(|i| PathBuf::from(&i.path))
            .ok_or_else(|| format!("{} is not installed", tool.name))?;
        launch::launch(&tool, &dir)
    }
    .map_err(|e| e.to_string())?;

    if let launch::Started::Process { pid } = started {
        if let Ok(mut running) = hub.running.lock() {
            running.insert(id.clone(), pid);
        }
        hub.log.info(format!("launched {id} as pid {pid}"));

        // Wait for the tool's own health endpoint on a worker thread, so the
        // card can turn from Starting to Running without the UI blocking for
        // the twenty-odd seconds a frozen Python app takes to bind its port.
        let app = app.clone();
        let hub = Arc::clone(&hub);
        std::thread::spawn(move || {
            let healthy = launch::wait_until_healthy(&tool);
            let _ = app.emit(
                "tool-health",
                serde_json::json!({ "id": tool.id, "healthy": healthy, "pid": pid }),
            );
            announce(&app, &hub);
        });
    }

    announce(&app, &hub);
    Ok(started)
}

#[tauri::command]
fn stop_tool(app: AppHandle, hub: State<'_, Arc<Hub>>, id: String) -> Result<(), String> {
    let pid = hub
        .running
        .lock()
        .map_err(|_| "the process table is busy".to_string())?
        .get(&id)
        .copied();
    if let Some(pid) = pid {
        launch::stop(pid).map_err(|e| e.to_string())?;
        if let Ok(mut running) = hub.running.lock() {
            running.remove(&id);
        }
        hub.log.info(format!("stopped {id} (pid {pid})"));
    }
    // Anything staged behind this tool can go in now.
    apply_staged(&app, &hub);
    announce(&app, &hub);
    Ok(())
}

#[tauri::command]
fn game_status() -> game::GameStatus {
    game::status()
}

#[tauri::command]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> Result<(), String> {
    // Only ever http(s): a catalog field is not a reason to hand an arbitrary
    // scheme to the shell.
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err(format!("refusing to open {url}"));
    }
    use tauri_plugin_opener::OpenerExt;
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

/// Errors from the web side, into the same log as everything else.
#[tauri::command]
fn report(hub: State<'_, Arc<Hub>>, level: String, message: String) {
    hub.log.write(&level, &message);
}

/// Apply anything that was staged, now that whatever blocked it may be gone.
fn apply_staged(app: &AppHandle, hub: &Arc<Hub>) {
    let staged: Vec<(String, state::Staged)> = match hub.state.lock() {
        Ok(guard) => guard
            .staged
            .iter()
            .map(|(id, s)| (id.clone(), s.clone()))
            .collect(),
        Err(_) => return,
    };
    if staged.is_empty() {
        return;
    }
    let game = game::status();
    for (id, entry) in staged {
        let Ok(tool) = hub.tool(&id) else { continue };
        let tool_running = hub
            .running
            .lock()
            .ok()
            .and_then(|r| r.get(&id).copied())
            .map(launch::is_alive)
            .unwrap_or(false);
        if game::install_blocked_by(&tool.name, tool_running, &game).is_some() {
            continue;
        }
        let artifact = PathBuf::from(&entry.artifact_path);
        let emitter = {
            let app = app.clone();
            move |progress: install::Progress| {
                let _ = app.emit("install-progress", progress);
            }
        };
        match install::install_artifact(&hub.layout, &tool, &artifact, &emitter) {
            Ok(installed) => {
                hub.log
                    .info(format!("applied staged update {} {}", id, installed.version));
                if let Ok(mut guard) = hub.state.lock() {
                    guard.installed.insert(id.clone(), installed);
                    guard.staged.remove(&id);
                }
                hub.persist();
            }
            Err(error) => hub
                .log
                .error(format!("staged update for {id} failed: {error}")),
        }
    }
}

/// The launch check, plus whatever auto-download/auto-install allow.
fn startup_check(app: AppHandle, hub: Arc<Hub>) {
    let settings = hub.settings();
    if !settings.may_reach_network() || !settings.check_on_launch {
        return;
    }

    let Ok((payload, signature)) = catalog::fetch_remote(Duration::from_secs(30)) else {
        hub.log.info("launch check: the catalog could not be fetched");
        return;
    };
    let loaded = match catalog::accept(&payload, &signature, Source::Remote) {
        Ok(loaded) => loaded,
        Err(error) => {
            hub.log.error(format!("launch check: {error}"));
            return;
        }
    };
    let _ = std::fs::create_dir_all(hub.layout.cache());
    let _ = std::fs::write(hub.layout.cached_catalog(), &payload);
    let _ = std::fs::write(hub.layout.cached_catalog_signature(), &signature);

    let updates: Vec<catalog::Tool> = {
        let Ok(guard) = hub.state.lock() else { return };
        loaded
            .catalog
            .tools
            .iter()
            .filter(|tool| {
                guard
                    .installed
                    .get(&tool.id)
                    .map(|i| version::is_newer(&tool.version, &i.version))
                    .unwrap_or(false)
            })
            .cloned()
            .collect()
    };

    if let Ok(mut guard) = hub.catalog.lock() {
        *guard = loaded;
    }
    if let Ok(mut guard) = hub.state.lock() {
        guard.last_check = Some(state::now_iso());
    }
    hub.persist();
    announce(&app, &hub);

    if !settings.auto_download || updates.is_empty() {
        return;
    }

    let game = game::status();
    for tool in updates {
        let emitter = {
            let app = app.clone();
            move |progress: install::Progress| {
                let _ = app.emit("install-progress", progress);
            }
        };
        let Ok(artifact) = install::download(&tool, &hub.layout, &emitter) else {
            continue;
        };

        if !settings.effective_auto_install() {
            continue;
        }

        let tool_running = hub
            .running
            .lock()
            .ok()
            .and_then(|r| r.get(&tool.id).copied())
            .map(launch::is_alive)
            .unwrap_or(false);

        // D5's interlocks. An update that cannot be applied safely waits and
        // says why, rather than being written over a running tool or over a
        // game whose PE ForgePact has patched.
        if let Some(reason) = game::install_blocked_by(&tool.name, tool_running, &game) {
            hub.log.info(format!("staged {}: {reason}", tool.id));
            if let Ok(mut guard) = hub.state.lock() {
                guard.staged.insert(
                    tool.id.clone(),
                    state::Staged {
                        version: tool.version.clone(),
                        artifact_path: artifact.to_string_lossy().to_string(),
                        sha256: tool.artifact.sha256.clone(),
                        staged_at: state::now_iso(),
                        blocked_by: reason,
                    },
                );
            }
            hub.persist();
            continue;
        }

        match install::install_artifact(&hub.layout, &tool, &artifact, &emitter) {
            Ok(installed) => {
                if let Ok(mut guard) = hub.state.lock() {
                    guard.installed.insert(tool.id.clone(), installed);
                }
                hub.persist();
            }
            Err(error) => hub
                .log
                .error(format!("auto-install of {} failed: {error}", tool.id)),
        }
    }
    announce(&app, &hub);
}

/// Walk up from a starting directory looking for the toolkit checkout.
///
/// `tauri dev` runs with the working directory at `hub/src-tauri`, and a
/// released hub is installed nowhere near a checkout -- so this returns None
/// often, and developer mode is simply unavailable when it does.
fn find_repo_root(start: PathBuf) -> Option<PathBuf> {
    let mut current = start;
    for _ in 0..8 {
        if current.join(".gitmodules").is_file() && current.join("catalog").is_dir() {
            return Some(current);
        }
        current = current.parent()?.to_path_buf();
    }
    None
}

pub fn run() {
    let install_root = paths::default_install_root();

    // Settings can move the install root, so the state file has to be read from
    // the default location first to find out where everything else lives.
    let bootstrap = HubState::load(&Layout::new(&install_root).state_file());
    let layout = Layout::new(
        bootstrap
            .settings
            .install_root
            .clone()
            .map(PathBuf::from)
            .unwrap_or(install_root),
    );
    let _ = layout.ensure();

    let hub_state = HubState::load(&layout.state_file());
    let loaded = catalog::load_local(
        &layout.bundle(),
        &layout.cached_catalog(),
        &layout.cached_catalog_signature(),
    );
    let logger = log::Log::new(layout.log_file());
    logger.info(format!(
        "hub {} starting; catalog {:?} generated {}",
        env!("CARGO_PKG_VERSION"),
        loaded.source,
        loaded.catalog.generated
    ));

    let repo_root = std::env::current_dir()
        .ok()
        .and_then(find_repo_root)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
                .and_then(find_repo_root)
        });

    let hub = Arc::new(Hub {
        layout,
        state: Mutex::new(hub_state),
        catalog: Mutex::new(loaded),
        running: Mutex::new(BTreeMap::new()),
        repo_root,
        log: logger,
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _argv, _cwd| {
            // A second launch raises the window already open rather than racing
            // it for state.json.
            if let Some(window) = app.get_webview_window("hub") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(Arc::clone(&hub))
        .setup(move |app| {
            let handle = app.handle().clone();
            let hub = Arc::clone(&hub);
            std::thread::spawn(move || {
                // Anything staged from a previous session goes in first, before
                // the network is touched -- a pending install the user already
                // agreed to should not wait on a check succeeding.
                apply_staged(&handle, &hub);
                startup_check(handle, hub);
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            hub_info,
            library,
            get_settings,
            set_settings,
            check_for_updates,
            install_tool,
            uninstall_tool,
            rollback_tool,
            verify_tool,
            launch_tool,
            stop_tool,
            game_status,
            open_path,
            open_url,
            report,
        ])
        .run(tauri::generate_context!())
        .expect("the hub could not start");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repo_root_is_found_from_inside_the_crate() {
        // The tests run with the working directory at hub/src-tauri, which is
        // two levels under the checkout.
        let root = find_repo_root(std::env::current_dir().unwrap());
        assert!(root.is_some(), "the toolkit checkout should be findable from here");
        assert!(root.unwrap().join("catalog/sources.toml").is_file());
    }

    #[test]
    fn a_directory_outside_any_checkout_has_no_repo_root() {
        assert!(find_repo_root(std::env::temp_dir()).is_none());
    }

    #[test]
    fn only_http_urls_are_openable() {
        for url in [
            "file:///C:/Windows/System32/calc.exe",
            "javascript:alert(1)",
            "ms-settings:",
            "\\\\server\\share",
        ] {
            assert!(
                !(url.starts_with("https://") || url.starts_with("http://")),
                "{url}"
            );
        }
    }
}
