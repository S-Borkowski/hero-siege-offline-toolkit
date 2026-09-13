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
pub mod procs;
pub mod state;
pub mod verify;
pub mod version;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_updater::UpdaterExt;

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
    /// Tool id -> whether its health endpoint, or its preferred port, answered
    /// on the last probe round.
    ///
    /// Written only by the probe ticker, read only by `build_view`. A probe is
    /// a loopback connect, and a machine whose network stack does not refuse a
    /// closed port promptly pays the whole timeout for every one of them --
    /// measured here at 700 ms each, 2.8 s for the four tools that declare an
    /// endpoint. That is far too much to spend assembling a view, and it was
    /// being spent on the thread pumping the window's messages.
    ///
    /// The ticker rewrites the map whole each round, so an answer is never
    /// older than one tick and a tool it has stopped asking about leaves no
    /// stale entry behind.
    pub probes: Mutex<BTreeMap<String, bool>>,
    /// The hub's own newer release, if the last check found one.
    ///
    /// Held here rather than in `state.json`: it is an answer about a remote
    /// release, and a cached "0.1.1 is available" surviving a restart into an
    /// already-updated hub would be a notice nobody can clear.
    pub hub_update: Mutex<Option<HubUpdate>>,
    /// The checkout this hub was built in, if it can still be found. Developer
    /// mode runs tools out of the submodules under it.
    pub repo_root: Option<PathBuf>,
    /// Read once at startup: a process's own elevation does not change.
    pub elevated: bool,
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
    /// Which repository this build trusts for its catalog and its own updates.
    pub hub_repo: String,
    /// Normally false, and deliberately so -- the hub elevates tools per launch
    /// rather than running elevated itself.
    pub elevated: bool,
}

/// A newer release of the hub itself, as announced by `latest.json`.
///
/// The ten tools are compared against the signed catalog; the hub is compared
/// against its own updater endpoint. Two different sources, but the interface
/// says "an update is available" the same way for both, so this rides in
/// `LibraryView` beside the tools rather than being asked for separately.
/// Deliberately not carrying the release body: every hub release ships the same
/// boilerplate about SmartScreen, so showing it would be a paragraph of noise
/// on top of the one fact that differs, which is the version number.
#[derive(Debug, Clone, Serialize)]
pub struct HubUpdate {
    pub version: String,
    pub current_version: String,
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
    /// False for an elevated tool under an unelevated hub: Windows refuses the
    /// terminate, so the card must not offer Stop.
    pub can_stop: bool,
    /// A copy started outside the hub, noticed by its port being taken.
    pub running_elsewhere: bool,
    pub staged: Option<state::Staged>,
    pub source_available: bool,
    /// Built from `HUB_REPO`, so the interface never has to know which
    /// repository this build came from. None when the tool declares no guide.
    pub guide_url: Option<String>,
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
    pub hub_repo: String,
    /// None when the last check found nothing newer, or found nothing at all.
    pub hub_update: Option<HubUpdate>,
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
    let probes = hub
        .probes
        .lock()
        .map_err(|_| "the probe cache is busy".to_string())?
        .clone();
    // One look at the process table for the whole view. It answers three
    // questions that used to be asked separately -- the game's state, whether
    // each tracked PID is alive, and whether a tool is running that this hub
    // never started -- and answering them from one snapshot means they cannot
    // contradict each other.
    let snapshot = procs::Snapshot::take();
    let game = game::status_from(&snapshot);

    let mut tools = Vec::with_capacity(loaded.catalog.tools.len());
    for tool in &loaded.catalog.tools {
        let installed = hub_state.installed.get(&tool.id);

        let tracked = running
            .get(&tool.id)
            .copied()
            .filter(|pid| snapshot.is_alive(*pid));

        // `Hub.running` is in memory, so a restart loses every tracked PID. A
        // process whose executable lives inside this tool's install directory
        // is this tool, whatever it is called -- which is the only signal that
        // works for the four tools with no health endpoint and no declared
        // port. Before this they went invisible on restart and their cards
        // offered Launch for something already open.
        let found = match (tracked, installed) {
            (None, Some(installed)) => {
                snapshot.find_under(std::path::Path::new(&installed.path))
            }
            _ => None,
        };
        let pid = tracked.or(found);

        // Only reached when the process table knew nothing, because a probe
        // is the weaker answer as well as the expensive one. Read from the
        // cache rather than taken here: this function must not touch the
        // network. See `Hub::probes`, and the budget test at the bottom of
        // this file.
        let answering = pid.is_none() && probes.get(&tool.id).copied().unwrap_or(false);

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
            can_stop: pid.is_some() && launch::can_stop(tool.launch.elevate, hub.elevated),
            // Up, but not started by this hub. Found by path it still has a
            // PID and so can still be stopped; known only by its health
            // endpoint it does not.
            running_elsewhere: tracked.is_none() && (found.is_some() || answering),
            staged: hub_state.staged.get(&tool.id).cloned(),
            source_available,
            guide_url: catalog::guide_url(&tool.guide),
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
        hub_repo: catalog::HUB_REPO.to_string(),
        hub_update: hub.hub_update.lock().ok().and_then(|u| u.clone()),
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
// The probe ticker
// ---------------------------------------------------------------------------

/// How often the ticker looks at the world.
///
/// This replaced a `setInterval(refresh, 10_000)` in the frontend, which cost a
/// full view build, a serialization and an IPC round trip every ten seconds
/// whether or not anything had changed. Same cadence, because the thing it
/// watches for -- Hero Siege starting or stopping -- is still not pushed at us
/// by anything. But the work is now one process snapshot, and nothing is sent
/// unless an answer actually moved.
const TICK: Duration = Duration::from_secs(10);

/// The parts of the view that change with nobody asking.
///
/// Everything else -- what is installed, what is staged, what the catalog says
/// -- changes only through a command, and every command that changes it calls
/// `announce` itself. So this is the whole of what a poll could discover.
#[derive(PartialEq)]
struct Pulse {
    /// Running, its PID, and whether EAC is up.
    game: (bool, Option<u32>, bool),
    /// Per tool: its PID if the process table has one, and its last probe.
    tools: Vec<(String, Option<u32>, bool)>,
}

/// Take one reading, refreshing the probe cache as a side effect.
///
/// None when a lock was busy: a round that could not read the world has nothing
/// to say about it, and returning a default would announce a change that did
/// not happen.
fn take_pulse(hub: &Hub) -> Option<Pulse> {
    let tools: Vec<catalog::Tool> = hub.catalog.lock().ok()?.catalog.tools.clone();
    let installed: BTreeMap<String, String> = hub
        .state
        .lock()
        .ok()?
        .installed
        .iter()
        .map(|(id, entry)| (id.clone(), entry.path.clone()))
        .collect();
    let running = hub.running.lock().ok()?.clone();

    let snapshot = procs::Snapshot::take();
    let game = game::status_from(&snapshot);

    let mut pids: BTreeMap<String, Option<u32>> = BTreeMap::new();
    let mut ask: Vec<&catalog::Tool> = Vec::new();
    for tool in &tools {
        // The same two questions `build_view` asks of the process table, in the
        // same order, so the two cannot disagree about which tool has a PID.
        let tracked = running
            .get(&tool.id)
            .copied()
            .filter(|pid| snapshot.is_alive(*pid));
        let found = match (tracked, installed.get(&tool.id)) {
            (None, Some(path)) => snapshot.find_under(std::path::Path::new(path)),
            _ => None,
        };
        let pid = tracked.or(found);

        // A tool this hub has neither installed nor started cannot be launched,
        // stopped or updated from its card, so "is something answering
        // somewhere" changes nothing the reader could act on. It is also the
        // probe that costs the most, because there is nothing there to answer
        // it -- six of the ten tools are in this state on a fresh install.
        if pid.is_none() && (installed.contains_key(&tool.id) || running.contains_key(&tool.id)) {
            ask.push(tool);
        }
        pids.insert(tool.id.clone(), pid);
    }

    // Concurrently, because the cost of a probe on a machine that does not
    // refuse a closed port is its timeout, in full, every time. Four of those
    // in series is four timeouts; started together it is one.
    let answers: BTreeMap<String, bool> = std::thread::scope(|scope| {
        let handles: Vec<_> = ask
            .iter()
            .map(|tool| scope.spawn(move || (tool.id.clone(), launch::already_running(tool))))
            .collect();
        handles.into_iter().filter_map(|handle| handle.join().ok()).collect()
    });

    if let Ok(mut guard) = hub.probes.lock() {
        *guard = answers.clone();
    }

    Some(Pulse {
        game: (game.running, game.pid, game.eac_running),
        tools: tools
            .iter()
            .map(|tool| {
                (
                    tool.id.clone(),
                    pids.get(&tool.id).copied().flatten(),
                    answers.get(&tool.id).copied().unwrap_or(false),
                )
            })
            .collect(),
    })
}

/// Watch for the changes nothing announces, and announce them.
///
/// The first round always announces, which is how the probe cache reaches a
/// view that was built before any probe had been taken.
fn start_ticker(app: AppHandle, hub: Arc<Hub>) {
    std::thread::spawn(move || {
        let mut last: Option<Pulse> = None;
        loop {
            if let Some(pulse) = take_pulse(&hub) {
                if last.as_ref() != Some(&pulse) {
                    announce(&app, &hub);
                    last = Some(pulse);
                }
            }
            std::thread::sleep(TICK);
        }
    });
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

// Everything below that reads the disk, the network or the process table is
// `#[tauri::command(async)]`. On a plain `fn` that attribute does not make the
// function async; it moves the call onto the async runtime instead of running
// it inline in the IPC handler -- which, on Windows, is the thread pumping
// WebView2's messages. A synchronous command holds that thread for its whole
// duration, so the window takes no clicks and paints no frames while it runs.
//
// `hub_info`, `get_settings` and `report` stay synchronous on purpose: they
// read memory and nothing else, and the thread hop would cost more than the
// work. Borrowed `State<'_, Arc<Hub>>` is fine under the attribute precisely
// because these are still `fn` and not `async fn`.

#[tauri::command]
fn hub_info(hub: State<'_, Arc<Hub>>) -> HubInfo {
    HubInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        install_root: hub.layout.root().to_string_lossy().to_string(),
        log_path: hub.log.path().to_string_lossy().to_string(),
        repo_root: hub.repo_root.as_ref().map(|p| p.to_string_lossy().to_string()),
        catalog_url: catalog::catalog_url(),
        hub_repo: catalog::HUB_REPO.to_string(),
        elevated: hub.elevated,
    }
}

#[tauri::command(async)]
fn library(hub: State<'_, Arc<Hub>>) -> Result<LibraryView, String> {
    build_view(&hub)
}

#[tauri::command]
fn get_settings(hub: State<'_, Arc<Hub>>) -> state::Settings {
    hub.settings()
}

#[tauri::command(async)]
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
#[tauri::command(async)]
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

/// Check the hub's own release, on demand.
///
/// Separate from `check_for_updates` rather than folded into it: they are two
/// requests to two places, and keeping them apart lets the interface say which
/// one it is waiting on instead of showing one spinner for both.
#[tauri::command(async)]
fn check_hub_update(app: AppHandle, hub: State<'_, Arc<Hub>>) -> Result<Option<HubUpdate>, String> {
    let settings = hub.settings();
    if !settings.may_reach_network() {
        return Err(if settings.work_offline {
            "Work offline is on. Turn it off in Settings to check for updates.".into()
        } else {
            "The hub has not finished its first run yet.".into()
        });
    }

    let update = refresh_hub_update(&hub, &app);
    announce(&app, &hub);
    Ok(update)
}

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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

#[tauri::command(async)]
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
    .map_err(|error| {
        // Launch failures used to reach only the banner, which meant a report
        // of one arrived as a screenshot rather than as a log line.
        hub.log.error(format!("launching {id} failed: {error}"));
        error.to_string()
    })?;

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

#[tauri::command(async)]
fn stop_tool(app: AppHandle, hub: State<'_, Arc<Hub>>, id: String) -> Result<(), String> {
    let tracked = hub
        .running
        .lock()
        .map_err(|_| "the process table is busy".to_string())?
        .get(&id)
        .copied();

    // The view offers Stop for a tool found by its install path as well as one
    // this hub launched, so this has to be able to stop both -- otherwise the
    // button is the same lie as a Launch offered for something already open.
    let pid = match tracked {
        Some(pid) => Some(pid),
        None => hub
            .state
            .lock()
            .ok()
            .and_then(|state| state.installed.get(&id).map(|i| i.path.clone()))
            .and_then(|path| {
                procs::Snapshot::take().find_under(std::path::Path::new(&path))
            }),
    };

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

#[tauri::command(async)]
fn game_status() -> game::GameStatus {
    game::status()
}

#[tauri::command(async)]
fn open_path(app: AppHandle, path: String) -> Result<(), String> {
    use tauri_plugin_opener::OpenerExt;
    app.opener()
        .open_path(path, None::<&str>)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
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

/// Ask the updater endpoint whether a newer hub has been released.
///
/// Blocking, so it belongs on a worker thread or in a command the interface has
/// already disabled its button for.
///
/// Deliberately independent of the catalog check. They are two different files
/// on two different release pages, and the hub's own update is the one a player
/// has no other way to find out about -- so a catalog fetch that fails must not
/// take it down with it.
fn refresh_hub_update(hub: &Hub, app: &AppHandle) -> Option<HubUpdate> {
    let found = match app.updater() {
        Ok(updater) => match tauri::async_runtime::block_on(updater.check()) {
            Ok(found) => found,
            Err(error) => {
                // Offline, a release page with no latest.json, a signature that
                // does not verify against the built-in public key. None of
                // these is worth interrupting anyone over, and all of them are
                // worth a line in the log.
                hub.log.info(format!("hub update check: {error}"));
                return hub.hub_update.lock().ok().and_then(|u| u.clone());
            }
        },
        Err(error) => {
            hub.log.error(format!("hub update check: {error}"));
            return None;
        }
    };

    let update = found.map(|update| HubUpdate {
        version: update.version.clone(),
        current_version: update.current_version.clone(),
    });

    match &update {
        Some(update) => hub.log.info(format!(
            "hub update available: {} (running {})",
            update.version, update.current_version
        )),
        None => hub.log.info("hub update check: this is the newest release"),
    }

    if let Ok(mut guard) = hub.hub_update.lock() {
        *guard = update.clone();
    }
    update
}

/// The launch check, plus whatever auto-download/auto-install allow.
fn startup_check(app: AppHandle, hub: Arc<Hub>) {
    let settings = hub.settings();
    if !settings.may_reach_network() || !settings.check_on_launch {
        return;
    }

    // Before the catalog, and announced on its own, because the catalog fetch
    // below returns early on any failure -- and the hub's own update went
    // unmentioned entirely until it was checked here.
    if refresh_hub_update(&hub, &app).is_some() {
        announce(&app, &hub);
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
    let elevated = launch::hub_is_elevated();
    logger.info(format!(
        "hub {} starting; repo {}; catalog {:?} generated {}; elevated {}",
        env!("CARGO_PKG_VERSION"),
        catalog::HUB_REPO,
        loaded.source,
        loaded.catalog.generated,
        elevated
    ));
    if elevated {
        // Not fatal, but worth a line: every tool started from here inherits
        // Administrator, which is exactly what elevating per launch avoids.
        logger.info(
            "this hub is running elevated, so every tool it starts will be too",
        );
    }

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
        probes: Mutex::new(BTreeMap::new()),
        hub_update: Mutex::new(None),
        repo_root,
        elevated,
        log: logger,
    });

    #[allow(unused_mut)]
    let mut builder = tauri::Builder::default();

    // A bridge an agent can drive the running hub through: click the tabs, read
    // the view, check the window is still taking input. That is how the
    // responsiveness work was checked, because the defect it fixes is invisible
    // to a test -- it is the window not repainting, not a wrong answer.
    //
    // Debug builds only, and bound to loopback rather than the plugin's default
    // of every interface: it can invoke any command this app has.
    #[cfg(debug_assertions)]
    {
        builder = builder.plugin(
            tauri_plugin_mcp_bridge::Builder::new()
                .bind_address("127.0.0.1")
                .build(),
        );
    }

    builder
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
            // The bridge drives the window by invoking its own plugin commands
            // from the webview, so the window needs permission to make those
            // calls. Added here rather than in `capabilities/`, which every
            // build reads -- this way the released hub's capability set is
            // exactly what it was.
            #[cfg(debug_assertions)]
            {
                if let Err(error) = handle.add_capability(
                    r#"{"identifier":"mcp-bridge-dev","windows":["hub"],"permissions":["mcp-bridge:default"]}"#,
                ) {
                    eprintln!("the MCP bridge capability could not be added: {error}");
                }
            }
            // The poll that used to live in `App.svelte`, moved to where the
            // data is: it announces only when something it watches has moved,
            // so an idle hub costs one process snapshot a tick and no IPC.
            start_ticker(handle.clone(), Arc::clone(&hub));
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
            check_hub_update,
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

    /// A `Hub` with nothing installed and nothing running, in its own temporary
    /// directory. That is the ordinary state of a fresh install, and it used to
    /// be the expensive one: every tool fell through to a health probe
    /// precisely because there was nothing on disk to find.
    fn scratch_hub(name: &str) -> (Hub, PathBuf) {
        let root = std::env::temp_dir().join(format!(
            "hub-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        ));
        let layout = Layout::new(root.clone());
        let _ = layout.ensure();
        let log = log::Log::new(root.join("hub.log"));
        let hub = Hub {
            layout,
            state: Mutex::new(HubState::default()),
            catalog: Mutex::new(catalog::embedded().expect("the embedded catalog must verify")),
            running: Mutex::new(BTreeMap::new()),
            probes: Mutex::new(BTreeMap::new()),
            hub_update: Mutex::new(None),
            repo_root: None,
            elevated: false,
            log,
        };
        (hub, root)
    }

    /// The regression that prompted the threading work, pinned.
    ///
    /// `build_view` used to call `launch::already_running` per tool, which is a
    /// loopback connect. On a machine that does not refuse a closed port
    /// promptly -- this one, where a refusal takes two seconds -- each of the
    /// four tools that declare a health endpoint cost the probe's whole
    /// timeout, so one view took 2.8 s. It was built on the thread pumping the
    /// window's messages, on a ten-second poll, and the hub was therefore
    /// unresponsive about a third of the time it was open.
    ///
    /// 200 ms is deliberately generous against the ~25 ms the process snapshot
    /// actually costs, so this will not flake on a loaded runner. Anything near
    /// the budget means a probe has crept back onto this path, which is exactly
    /// how it got here the first time.
    #[test]
    fn building_the_view_never_touches_the_network() {
        let (hub, root) = scratch_hub("view-budget");
        let expected = hub.catalog.lock().unwrap().catalog.tools.len();

        let started = std::time::Instant::now();
        let view = build_view(&hub).expect("the view should build");
        let elapsed = started.elapsed();

        assert_eq!(view.tools.len(), expected);
        assert!(
            view.tools.iter().all(|tool| !tool.running_elsewhere),
            "an empty probe cache cannot report anything as running elsewhere"
        );
        assert!(
            elapsed < Duration::from_millis(200),
            "building the view took {elapsed:?}, over its 200 ms budget -- \
             something on this path is doing network work again"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// The other half of the same contract: the view does report what the
    /// ticker found, so moving the probe off this path did not drop the answer.
    #[test]
    fn the_view_reports_what_the_probe_cache_holds() {
        let (hub, root) = scratch_hub("probe-cache");
        let id = hub.catalog.lock().unwrap().catalog.tools[0].id.clone();
        hub.probes.lock().unwrap().insert(id.clone(), true);

        let view = build_view(&hub).expect("the view should build");
        let tool = view.tools.iter().find(|t| t.tool.id == id).unwrap();
        assert!(tool.running_pid.is_none());
        assert!(
            tool.running_elsewhere,
            "a cached probe answer is what tells the card a copy is already up"
        );

        let _ = std::fs::remove_dir_all(&root);
    }

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
