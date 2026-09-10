# ForgePact Module Development Guide

## Module Overview & Metadata
- **Module Name:** ForgePact (Hero Siege Season 10 Offline Mod Panel & BloodPactPlugin)
- **Submodule Path:** `ForgePact`
- **Reviewed Git Revision (upstream, `origin`):** `2751679` (Tag: `v1.3.16`, Branch: `main`) — `falorfrozen-cmd/ForgePact`.
- **Revision Date:** `2026-09-10 09:21:05 +0300`
- **Commit Message:** `Prepare ForgePact 1.3.16 Headhunter release` — includes upstream's independent fix for the `VALUE_REF` player-resolution bug class in the Headhunter kill/steal path (`HhResolveInstance`, commits `7743a99`/`4ae4e30`), the same bug class described below.
- **Fork & Branch:** This guide additionally tracks work rebased onto that revision and pushed to `fork` (`S-Borkowski/ForgePact`) as **`release/v1.3.17`** — the version is 1.3.17, not 1.3.16, precisely because upstream had already shipped 1.3.16 by the time this branch was rebased onto it. See `release-notes-v1.3.17.md`.
- **Source Availability:** Full application source is present (Python control panel `src/forgepact.py`, C++20 native mod plugin `plugin/ModuleMain.cpp`, modified YYToolkit patches `yytoolkit-modified/`, build scripts `plugin_build/build.bat` and `build_release.py`, Python contract tests `tests/`, and reverse-engineering research notes `docs/`).
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or remote CI configurations exist; verification is conducted locally via Python unittest test suites and static build audits).
- **Purpose & Scope:** Standalone offline control panel and native runtime hook plugin providing runtime modifiers for Hero Siege single-player sessions. Controls monster density, special content spawns (Rift Portals, Battlefields, Cursed Orbs, Chaos Tower, Shadow Realm, etc.), drop rate multipliers and gated drop families (Keys, Relics, Angelic/Unholy uniques), gameplay mods (relic drop pool filter excluding maxed 10/10 relics, orb pickup radius), player/combat stat scaling, full map reveal, and custom forge mechanics (Headhunter, Tyrant's Crown, Beacon, Item Editor base stat export) without permanently altering save files or the base game executable. Integrated with `hs-game-sdk`.
- **Fork Branch vs. This Guide:** `release/v1.3.17` (`fork`) carries the Mods tab (relic filter, orb pickup, the Headhunter/Tyrant's Crown/Beacon/Map Reveal relocation), the build-order packaging guard, the stall watchdog, the `SafeF()` crash guard, and a second, complementary `VALUE_REF` player-resolution fix (`HhUsableInstance`, used by orb pickup and the relic filter's `HhResolveLocalPlayer` calls — distinct from upstream's `HhResolveInstance`, which fixed the same bug class for the Headhunter kill/steal path only). All covered by `tests/test_relic_filter_contract.py` (updated to match the merge) and documented in Known Limitations items 4-9 below; none are optional cleanup, all were needed to reach a working build.

---

## Architecture & Repository Map

### Repository Layout
- `src/`: Python application frontend and control panel runtime.
  - `forgepact.py`: Single-file local web/desktop application (`http://127.0.0.1:8766`). Manages port selection, configuration persistence (`%LOCALAPPDATA%\Hero_Siege\forgepact.json`), background game process detection / auto-apply watcher, PE binary patching / backup / restoration via `AuriePatcher.exe`, and file-based IPC dispatch to `bp_ipc/cmd.txt`.
- `plugin/`: Native GameMaker mod plugin implementation.
  - `ModuleMain.cpp`: C++20 dynamic library source for `BloodPactPlugin`. Implements Aurie module lifecycle (`ModuleInitialize`), hooks GameMaker engine routines via YYToolkit, manages frame event callbacks (`EVENT_FRAME`), polls commands from `bp_ipc/cmd.txt`, logs responses to `bp_ipc/out.txt`, applies throttled density/spawn overrides, manipulates drop tables and LoadDrops gates, projects HUD head labels, and exports live item statistics to `bp_ipc/itemstats.json`.
  - `BUILD.md`: Build requirements, compilation instructions, and differences between shipping (`/DFORGEPACT_RELEASE`) and research builds.
- `plugin_build/`: Plugin compiler script and build workspace.
  - `build.bat`: MSVC x64 batch script compiling `plugin/ModuleMain.cpp` into `BloodPactPlugin_ship.dll` (player build) or `BloodPactPlugin_rel.dll` (research build).
- `modfiles_shipped/`: Shipped binaries deployed to the game's `bin/` directory upon mod installation.
  - `AurieCore.dll`: Aurie Framework core loader binary (unmodified AGPL-3.0).
  - `AuriePatcher.exe`: Aurie PE import/bootstrap patcher (unmodified AGPL-3.0).
  - `YYToolkit.dll`: GameMaker runtime interface library built with modified startup cache and disabled `ExecuteIt` hook (AGPL-3.0).
  - `BloodPactPlugin.dll`: Pre-compiled release build of the mod plugin.
  - `HSOfflineTrackerProducer.dll` (Optional): Read-only telemetry sensor for the companion HS Offline Tracker tool.
- `yytoolkit-modified/`: AGPL-3.0 compliance source notices and patches for the modified YYToolkit library.
  - `NOTICE.md`: Detailed rationale, file listing, and rebuild instructions for modified YYToolkit files.
  - `Generic-RunnerInterfaceNew.cpp`: Adds a disk cache (`<exe>.yytkcache`) and density-sorted `.text` page pre-filter to make the RunnerInterface search instantaneous instead of a ~1 minute full `.text` disassembly.
  - `source/YYTK/Hooks.cpp`: Disables the buggy `ExecuteIt` hook (`EVENT_OBJECT_CALL`), eliminating Hero Siege Season 10 crash loops and instance lookup corruptions while preserving `EVENT_FRAME` via `HkPresent`.
- `tests/`: Automated Python contract test suites validating plugin source invariants and panel logic without requiring a running game instance.
  - `test_density_reentry_contract.py`: Validates spawner object name lookups, stable spatial identity keys, and density placement guards.
  - `test_enemy_speed_contract.py`: Verifies enemy speed multipliers, Chaos Tower scoping, and release command accessibility.
  - `test_mod_backup.py`: Tests clean PE backup verification, build mismatch detection, stale backup archival, and atomic restore rollbacks.
  - `test_necro_balance_contract.py`: Validates Necromancer balance formulas, fail-closed runtime contracts, and panel visibility.
  - `test_release_hook_contract.py`: Validates zero eager gameplay hooks in release builds, all-off pass-through behavior, and telemetry exclusions.
  - `test_repo_bounds_contract.py`: Tests boundary protection and index validation against Season 10 item/relic categories.
  - `test_relic_filter_contract.py`: Validates the relic drop pool filter, orb pickup radius mod, build-order packaging guard, player-resolution against `VALUE_REF`, the stall watchdog's presence/ordering, and the Map Reveal / Headhunter / Tyrant's Crown / Beacon panel relocation (29 tests; the largest suite, covering everything fixed 2026-09-09/10).
- `docs/`: Reverse-engineering research logs, memory audits, and drop rate analysis.
  - `S10-special-content-notes.md`: Detailed Season 10 reverse-engineering log for special content spawners, gate mechanisms, crash thresholds, and investigated workarounds.
  - `dungeon-key-research.md`: Documentation of the two-stage key/relic drop architecture (`LoadDrops` outer gate + `droprate.base` inner roll).
  - `angelic-drop-research.md`: Analysis of Angelic/Unholy drop rates and synthetic drop roll implementation.
  - `blood-pact-values-research.md`: Research findings on Blood Pact modifiers and stat calculations.
- `build_release.py`: Packaging script creating the frozen PyInstaller distribution at `dist/ForgePact/` with release guard checks.

---

## Component Architecture & IPC Pipeline

```text
+---------------------------------------------------------------------------------------+
|                                  FORGEPACT PANEL (Python)                             |
|                                                                                       |
|   +------------------------------------+      +-----------------------------------+   |
|   |          src/forgepact.py          |      |         Process Watcher           |   |
|   |  - HTTP Server (127.0.0.1:8766)    |      |  - Background thread (5s poll)    |   |
|   |  - Web UI / Sliders / Settings     |      |  - Auto-applies on game start     |   |
|   |  - Mod Installer & Exe Backup      |      |  - Detects new process boot count |   |
|   +-----------------+------------------+      +-----------------+-----------------+   |
|                     |                                           |                     |
|                     +---------------------+---------------------+                     |
|                                           |                                           |
|                                           v                                           |
|                           [ Writes lines to bp_ipc/cmd.txt ]                          |
+-------------------------------------------|-------------------------------------------+
                                            |
                                            v (File-based IPC in <game>/bin/bp_ipc/)
+---------------------------------------------------------------------------------------+
|                                    GAME PROCESS (Hero_Siege.exe)                      |
|                                                                                       |
|   +-------------------------------------------------------------------------------+   |
|   | AurieCore.dll  --->  YYToolkit.dll  --->  BloodPactPlugin.dll                 |   |
|   +-------------------------------------------------------------------------------+   |
|                                           |                                           |
|                                           v (FrameCallback / PollCommands)            |
|   - Reads & clears bp_ipc/cmd.txt every 6 frames                                      |
|   - Appends status/replies to bp_ipc/out.txt                                          |
|   - Writes current item stats to bp_ipc/itemstats.json (max once every 2 seconds)     |
|                                           |                                           |
|               +---------------------------+---------------------------+               |
|               |                           |                           |               |
|               v                           v                           v               |
|     [ Spawner Throttling ]       [ Drop Gating & Rates ]     [ Stat / Combat Hooks ]  |
|     - Multiplies markers         - Scales droprate.base      - Hooks GetBloodPactInfo |
|     - Clears Chaos Tower /       - Opens LoadDrops gates     - Enemy speed scaling    |
|       Shadow Realm run flags       (Keys, Relics only)       - Headhunter / Tyrant /  |
|     - Queued spawn per frame     - Gated synthetic rolls       Beacon mechanics       |
+---------------------------------------------------------------------------------------+
```

---

## Representative Change Workflow

To add or modify a gameplay modifier or runtime command:

1. **Update Plugin Implementation (`plugin/ModuleMain.cpp`):**
   - Add or modify the command parser inside `DoCommand(const std::string& line)`.
   - Implement the corresponding hook or memory override. Ensure hot-path diagnostic counters are wrapped in `#ifndef FORGEPACT_RELEASE` (`BP_DIAG_INCREMENT`).
   - If introducing a functional hook, ensure it is installed lazily only when non-vanilla values are requested (preserving zero-overhead all-off baseline).
2. **Compile Native Plugin (`plugin_build/build.bat`):**
   - Build the release DLL:
     ```cmd
     plugin_build\build.bat release
     ```
   - Copy the output binary to the shipped staging directory:
     ```powershell
     Copy-Item plugin_build\BloodPactPlugin_ship.dll modfiles_shipped\BloodPactPlugin.dll
     ```
3. **Update Panel UI & Command Builder (`src/forgepact.py`):**
   - If adding a new setting, define its metadata in `SPAWNERS`, `KEYS`, `STATS`, or `PERCENT_STATS`.
   - Update `build_cmds(cfg)` and any specialized command generators (e.g. `build_key_cmds`).
   - Update the HTML/JavaScript UI templates within `src/forgepact.py`.
4. **Execute Contract Tests:**
   - Run the full Python test suite to verify contract adherence:
     ```powershell
     py -m unittest discover -s tests -v
     ```
5. **Package Release Distribution (`build_release.py`):**
   - Run the packaging script:
     ```powershell
     py build_release.py
     ```
  - The packaging guard requires `plugin_build\BloodPactPlugin_ship.dll` to exist and confirms that `modfiles_shipped\BloodPactPlugin.dll` matches it. A missing or stale staged plugin stops packaging so the Install button cannot ship an older DLL.
6. **Write Release Notes (REQUIRED — `release-notes-vX.Y.Z.md`):**
   - Every version that ships gets a `release-notes-vX.Y.Z.md` file at the ForgePact repo root (`v1.3.1` through `v1.3.15` are the existing precedent — do not skip this for a version bump, however small). Not optional: a version with player-visible changes and no release notes file is an incomplete change.
   - Player-facing only, in plain language — what was broken and what changed *for the player*, not internal refactors, build-script fixes, or debugging history (that belongs in this instructions.md, e.g. Known Limitations, not in release notes). Match the tone of the existing files: name the symptom before the fix ("Tyrant's Crown and Monster Rarity did nothing in 1.3.14" before explaining why), and give a measured before/after number when one exists.
   - Standard sections, in order: `## New`, `## Fixed` (either may be omitted if empty, but at least one must be present), then `## How to update` with the standard boilerplate (see any existing file). A `## Changed` section is used for reorganizations (e.g. a control moving to a different panel tab) that are neither strictly new nor a bug fix.
   - Never claim something is "Fixed" that is not actually resolved. If an investigation concluded the *reported* symptom is not this project's bug (e.g. a freeze traced to a display driver / GPU stall with a control run proving the plugin was not involved), that finding belongs in this instructions.md's Known Limitations, not in release notes as a fix — release notes are read by players deciding whether to update, and an overclaimed fix erodes trust in every note that follows it.

---

## Supported Platforms & Prerequisites

### Supported Platforms
- **Operating System:** Windows 10 / Windows 11 (64-bit x86_64).
- **Target Game:** Hero Siege Season 10 (offline, EAC disabled / single-player copy).

### Build & Development Prerequisites
- **Python:** Python 3.10+ (standard `py` launcher on Windows).
- **C++ Compiler:** Microsoft Visual C++ (MSVC) from Visual Studio 2022 / Build Tools supporting `/std:c++20`.
- **YYToolkit Headers:** YYToolkit C++ headers (`YYToolkit/`, `Aurie/`, `FunctionWrapper/`, and `YYTK_Shared_Types.cpp`) located in `plugin_build/include/`.
- **Python Packages (Optional / Packaging):**
  - `pyinstaller` (required for running `build_release.py`).
  - `pywebview` (optional; if installed, panel launches in a native desktop window, otherwise falls back to the default web browser).

---

## Command Reference

| Command | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
| --- | --- | --- | --- | --- | --- | --- |
| `py src/forgepact.py` | `ForgePact/` | PowerShell / CMD | Python 3.10+ | Launches local control panel HTTP server (`http://127.0.0.1:8766`). | Opens web browser / desktop window; watches for game process | Verified |
| `py -m unittest discover -s tests -v` | `ForgePact/` | PowerShell / CMD | Python 3.10+ | Executes all 76 Python contract tests. | Read-only test execution; all tests pass | Verified |
| `plugin_build\build.bat release` | `ForgePact/` | CMD / PowerShell (Windows x64) | MSVC v143+ (VS 2022), YYToolkit headers in `plugin_build\include\` | Compiles `BloodPactPlugin_ship.dll` with `/DFORGEPACT_RELEASE`. | Generates `plugin_build\BloodPactPlugin_ship.dll` and `obj_ship\` | Inspected |
| `plugin_build\build.bat` | `ForgePact/` | CMD / PowerShell (Windows x64) | MSVC v143+ (VS 2022), YYToolkit headers in `plugin_build\include\` | Compiles `BloodPactPlugin_rel.dll` (research build with inspection commands). | Generates `plugin_build\BloodPactPlugin_rel.dll` and `obj_dev\` | Inspected |
| `py build_release.py` | `ForgePact/` | PowerShell / CMD | PyInstaller installed, matching `BloodPactPlugin_ship.dll` | Builds complete release bundle in `dist/ForgePact/`. | Terminates existing `ForgePact.exe` processes; generates onefile executable | Inspected |

*Status notes:* Commands marked **Verified** have been executed and validated in the current environment. Commands marked **Inspected** have been audited against build script source declarations and compiler flags.

---

## Data Formats, Persistence & IPC Contracts

### 1. Panel Configuration (`%LOCALAPPDATA%\Hero_Siege\forgepact.json`)
Stores user UI settings in JSON format:
```json
{
  "exe": "C:\\Games\\HeroSiege\\Hero_Siege.exe",
  "auto_apply": true,
  "density": 1.5,
  "rift": 5,
  "battlefield": 1,
  "dungeon": 2,
  "relic": 1,
  "exp": 2.0,
  "magicfind": 1.5,
  "movespeed": 1.0,
  "damage": 25,
  "map_reveal": true,
  "headhunter": true,
  "tyrant": false,
  "beacon": false,
  "mod_filter_max_relics": false,
  "mod_orb_pickup_radius": false
}
```

### 2. IPC Command File (`<game>\bin\bp_ipc\cmd.txt`)
UTF-8 / ASCII plain-text command queue. The panel appends lines to `cmd.txt`; the plugin reads, executes, and clears the file every few frames.
- Example commands:
  - `ping`: Keepalive and synchronization probe.
  - `density 2.0`: Sets enemy density multiplier to 2.0x.
  - `specialrate rift 5`: Sets Rift Portal spawner multiplier to 5x.
  - `droprate group dungeon 2`: Sets Dungeon Key drop multiplier.
  - `dungeonkey add 12 1` / `dungeonkey on`: Opens outer LoadDrops gate for Dungeon Keys (type 12).
  - `stat damage 1.25`: Sets damage multiplier to 1.25 (+25%).
  - `mapreveal on` / `mapreveal off`: Toggles full map fog of war clearing.
  - `relicfilter 1` / `relicfilter 0`: Toggles the relic drop pool filter to exclude relics already at maximum level (10/10) in player equipped slots, backpack, or inventory. Safe to send at launch: it arms the mod and the `DropRelic` hook is installed later, once a player instance exists.
  - `orbpickup 10` / `orbpickup 0` / `orbpickup stat`: Widens the experience / magic-find globe pickup radius by 10x. `FrameCallback` enumerates the globe instances each frame and pulls any inside the widened radius toward the player at a constant `kGlobePullSpeed` (6 px/frame — tuned down from an accelerating ramp per user feedback 2026-09-10, which snapped the last stretch in a single frame and read as an unnatural teleport); the game's own pickup logic then fires normally once close enough. `orbpickup stat` (also printed when the mod is switched off) reads as a decision tree: `seen=0` while standing next to globes means the object indices are wrong, `noplayer>0` means the player never resolved, and only `outofreach` means the radius is too small — `nearest=` says by how much.
  - `headhunter force` / `tyrant force` / `beacon force`: Activates custom forge mechanic overrides.
  - `enemyspeed 1.5 ct`: Scales enemy movement speed by 1.5x scoped exclusively to Chaos Tower.

### 3. IPC Log & Output File (`<game>\bin\bp_ipc\out.txt`)
Append-only log containing plugin startup notifications, command responses, and runtime diagnostic dumps. The panel reads `out.txt` to track process boot counts (`BloodPact plugin loaded`).

### 4. Live Item Stats File (`<game>\bin\bp_ipc\itemstats.json`)
Exported by the plugin at most once every 2 seconds when custom forge hooks are active. Contains actual rolled `itemStatStruct` records keyed by `itemTimeStamp`. Consumed by the companion `hero-siege-item-editor` to display live rolled base stats.

---

## Safety, Installation & Backup Lifecycle

### Safe Mod Installation & Restoration
1. **Pre-flight Check:** Verifies `Hero_Siege.exe` exists, the game is not currently running, and required binaries (`AurieCore.dll`, `YYToolkit.dll`, `BloodPactPlugin.dll`, `AuriePatcher.exe`) exist.
2. **Clean Backup Archival & Refresh:**
   - If `Hero_Siege.exe.aurie_backup` does not exist, a byte-for-byte copy of the unpatched game executable is created.
   - If an existing backup belongs to an older game build (detected via PE headers and `.text` section hash comparisons), the old backup is archived as `Hero_Siege.exe.aurie_backup.stale-<timestamp>` and a fresh clean backup is created from the new executable.
3. **PE Import Patching:** `AuriePatcher.exe` injects a `.aurie` section into `Hero_Siege.exe` so the game automatically bootstraps `AurieCore.dll` on startup.
4. **Atomic Rollback:** If `AuriePatcher` fails or produces an invalid binary, the clean verified backup is immediately restored atomically over `Hero_Siege.exe`.
5. **Safe Mod Removal:** `op_remove_mod` validates that `Hero_Siege.exe.aurie_backup` matches the base build before restoring it over `Hero_Siege.exe`, and unlinks mod DLLs (`AurieCore.dll`, `mods/aurie/YYToolkit.dll`, `mods/aurie/BloodPactPlugin.dll`, `mods/aurie/HSOfflineTrackerProducer.dll`).

### Anti-Cheat & Offline Enforcement
- ForgePact is strictly designed for **single-player / offline** play with Easy Anti-Cheat (EAC) disabled.
- It does not contain EAC bypass mechanisms; launching an EAC-enabled online client will bounce back to the clean executable and fail to load Aurie mods.

---

## Packaging Hazards & Guardrails

The packaging script (`build_release.py`) includes explicit fail-closed safety checks:
1. **Plugin Sync Guard:** Compares `modfiles_shipped\BloodPactPlugin.dll` against `plugin_build\BloodPactPlugin_ship.dll`. If they differ or the ship DLL is missing, packaging is aborted to prevent shipping stale plugin binaries.
2. **Process Lock Prevention:** Uses `taskkill /F /IM ForgePact.exe` before rebuilding to avoid locked executable errors in `dist/ForgePact/`.
3. **Module Exclusion:** Excludes heavy or unused packages (`tkinter`, `PIL`, `numpy`, `pandas`, `PyQt5`, `IPython`, `pytest`) from the PyInstaller onefile package, keeping bundle size small.

---

## Known Limitations & Gaps

1. **The Abyss Spawner (`Spawn_Abyss_obj`):**
   - `Spawn_Abyss_obj` is currently **unsupported**. It sets `discoverable = true`, placing it behind an unmapped two-stage discover-then-activate gate that fails to place objects even when forced. Full investigation details are documented in `docs/S10-special-content-notes.md`.
2. **Equipped Item Detection for Custom Mechanics:**
   - Active mechanics like Headhunter, Beacon, and Tyrant's Crown currently rely on `force` commands sent by the panel rather than dynamically reading equipped inventory slots in C++.
3. **Release vs. Research Command Separation:**
   - Diagnostic commands (`readmem`, `census`, `enemylog`, `probestruct`, `structdump`) are excluded from release builds (`/DFORGEPACT_RELEASE`) to protect performance and stability.
4. **Build order matters since `build_release.py`'s plugin-sync guard:**
   - `build_release.py` now refuses to package unless `plugin_build\BloodPactPlugin_ship.dll` exists and byte-matches `modfiles_shipped\BloodPactPlugin.dll` (see `build_release.py`). Always run `plugin_build\build.bat release` (which now auto-stages the DLL into `modfiles_shipped\` and, if present, `dist\ForgePact\modfiles\`) *before* `py build_release.py`. Running the packaging script first, or after editing `plugin/ModuleMain.cpp` without rebuilding, is the most common "build keeps failing" report.
5. **Hot builtins must stay allocation-free (`distance_to_object`):**
   - `distance_to_object` is called by every spawner's periodic proximity check, so a hook body that calls into the runner (`asset_get_index`, `object_get_name`) or allocates per call collapses the frame rate. Any builtin hook should follow the shape of `HookICD`: cheap pass-through first, cached integer comparisons after.
   - Note the calling convention: for a builtin hook, `Args[N]` holds the GML arguments. `Other` is the GML `other` context, **not** the argument — matching an object argument against `Other` silently never fires.
   - **Measured 2026-09-09:** the globes do **not** reach the player through `distance_to_object` at all. With the player and all four globe objects resolved correctly, a full session shortened **0** distance checks.
   - **Measured 2026-09-10:** hooking the globe step scripts does not work either. `HookOneScript` reported both `ExpGlobeStepMain` and `MFGlobeStepMain` installed, and the hook body ran **0** times — the globes' step logic is not dispatched through those script-table entries. After two failed interception points, `orbpickup` is driven from `FrameCallback` instead (`OrbPickupTick`), enumerating globe instances with `instance_number` / `instance_find`. The frame callback is known to run because the stall watchdog heartbeats from it. **Prefer this pattern when an interception point is unproven:** drive from the frame callback, which cannot silently not-fire.
6. **Diagnose stalls with the built-in watchdog, do not guess:**
   - A multi-second freeze on the character screen was attributed twice to the wrong cause. `ModuleMain.cpp` now carries a stall watchdog in **every** build: a background thread notices when `FrameCallback` stops ticking for 3 s, suspends the frame thread just long enough to read its instruction pointer, and appends `STALL <ms> - frame thread at <module>+<rva>` to `bp_ipc/out.txt`, plus disk bytes and free RAM across the stall. That separates "stuck in BloodPactPlugin" from "stuck in the game" and "machine-wide thrashing" from "waiting on the GPU", with no debugger.
   - The frame thread is resumed **before** any allocation or formatting. Suspending a thread and then allocating is how this class of tool deadlocks on a heap/CRT/loader lock the stalled thread is holding; keep that ordering if you touch it.
   - **Measured 2026-09-09, character-select freeze (up to 85 s):** no sample landed in `BloodPactPlugin.dll`. The frame thread was blocked in `ZwWaitForSingleObject`, `ZwQuerySystemInformation`, `NtDxgkSubmitPresentToHwQueue` and `NtGdiDdDDIGetDeviceState` — kernel waits and GPU present/device-state calls — with no display-driver timeout (event 4101) logged. Resolve such addresses by parsing the export table of the named DLL and taking the nearest preceding export; the consistent `+0x14` offset is the syscall stub's return address.
   - **CONCLUDED — the freeze is not ForgePact.** A control run with `BloodPactPlugin.dll` removed from `mods/aurie/` (Aurie module list: YYToolkit + the game only) froze on the same screen. The same run also still produced the four `Unable to find any instance for object index ...` entries in `YYToolkit.log`, with identical indices and stacks, confirming those are game/YYToolkit noise and not caused by the plugin. Removing the plugin also removes the watchdog, so further measurement needs the out-of-process probe below.
   - **Measured 2026-09-10, with I/O and memory attached to each stall:** two stalls of ~65 s each showed **0 KB and 1 KB** of game disk reads, and free RAM *rising* by 1.1 GB and 843 MB respectively; sampled instruction pointers were `ZwWaitForSingleObject`, `NtGdiDdDDIGetDeviceState` and `ZwFreeVirtualMemory`. So the freeze is neither disk/anti-virus nor memory pressure — the process is blocked releasing memory and querying GPU device state. Remaining suspects are the display driver / GPU resource teardown, not the game's asset loading and not ForgePact.
   - **Probe for freezes that are not ours:** `tools/freeze_probe.ps1` (repo root) samples the game from outside — `Process.Responding` to bracket the freeze exactly, plus the game's disk I/O, free RAM and machine-wide CPU busy/idle, ranking processes once per freeze. Its verdict line separates *the machine is thrashing* (AV / paging / disk) from *the game is waiting on the GPU*. Note that per-process CPU via `TotalProcessorTime` or `Get-Counter` costs 1.5–6 s per sweep on a normal machine, which is why the continuous loop uses `GetSystemTimes`/`GetProcessIoCounters` instead.
7. **`instance_find` returns a REFERENCE, not a number (this broke every player-gated feature):**
   - **Measured 2026-09-10.** `HhResolveLocalPlayer`'s fallback accepted only `VALUE_REAL`/`VALUE_INT32`/`VALUE_INT64` from `instance_find(Player_obj, 0)`. This runner returns `VALUE_REF` (kind 15), so the fallback **always** failed, and with `GetMyPlayer` also returning a non-`VALUE_OBJECT` value the whole resolver returned false on every call. Everything gated on the local player then silently did nothing: `orbpickup` logged `seen=176993 noplayer=176993`, the relic filter never armed, and the Headhunter head labels reported "local player not found".
   - An instance reference is passed straight through — `variable_instance_get` accepts it. `HhUsableInstance()` now validates a candidate by *reading a variable from it*, which is what every caller does next, rather than trusting a kind tag. `orbpickup stat` reports `player via GetMyPlayer` / `instance_find(Player_obj)` / `none` so this can never fail silently again.
   - **Independently, upstream hit the same bug class** in the Headhunter kill/steal path (origin `v1.3.16`, commits `7743a99`/`4ae4e30`/`8005249`) and fixed it there with `HhResolveInstance()` — a `CInstance*` resolver that also accepts `VALUE_REF`, verifies the resolved instance still exists, and checks its `id` matches. The two fixes are complementary, not duplicates: `HhUsableInstance()` validates an `RValue` for callers that only need to read variables from it (orb pickup, the relic filter); `HhResolveInstance()` converts to an actual `CInstance*` for callers that need one (`HhSteal`). `HhSteal`'s own fallback now calls `HhResolveInstance()` on `HhResolveLocalPlayer`'s result rather than the old `p.ToInstance()` (which silently dropped a `VALUE_REF`, same failure mode) — see the merge commit on `release/v1.3.17` for the full reconciliation.
8. **Mods that install a hook must be armed, not hooked, at launch:**
   - Installing the `DropRelic` hook while character selection is still running stalls the runner. `relicfilter 1` therefore only sets `g_RelicFilterPending`; `FrameCallback` installs the hook once the `fc > 300` setup gate has passed and `HhResolveLocalPlayer` succeeds. This is what lets `build_cmds` emit the command at launch — an earlier workaround withheld it from `build_cmds` entirely, so the panel toggle stayed on but the mod silently did nothing after a game restart.
9. **`%f`-family `sprintf_s` on a game-memory-read `double` can abort the process (0xC0000409):**
   - A stale asset/script index or offset can make a `RValue::ToDouble()` read off game memory (e.g. an item's `droprate.base`) come back as `inf`/`NaN`/an astronomically large finite value. Formatting that with `%f`/`%.Nf` into a fixed `sprintf_s` buffer overruns it and the CRT fast-fails the whole game (`ucrtbase.dll`, exception `0xC0000409` / `STATUS_STACK_BUFFER_OVERRUN`) — this is what shows up as a hard "YYToolkit crash" with no other symptom. `ModuleMain.cpp` has a `SafeF()` helper (clamps non-finite doubles to `0.0`) used at every `droprate`/`dungeonkey` status-print call site and inside `VanilyaBase()` for exactly this reason; if a new command formats a game-read or `std::stod`-parsed double with `%f`, route it through `SafeF()` (or validate with `std::isfinite`) first.

---

## Maintenance Triggers

- **Game Executable Updates:** When a new Season 10 patch releases, verify that spawner object names, GML function names, and `LoadDrops` drop family indices remain valid.
- **YYToolkit Header / Binary Sync:** Any rebuild of `YYToolkit.dll` requires recompiling `BloodPactPlugin` against matching headers in `plugin_build\include\` to prevent vtable mismatch crashes.
- **Dependency Upgrades:** Check `yytoolkit-modified/NOTICE.md` if updating upstream YYToolkit; the custom disk cache and `ExecuteIt` hook disablement must be preserved.

---

## Source Documents & Evidence References
- Submodule Readme: `../../../ForgePact/README.md`
- Plugin Build Specifications: `../../../ForgePact/plugin/BUILD.md`
- Plugin Build Script: `../../../ForgePact/plugin_build/build.bat`
- Release Packaging Script: `../../../ForgePact/build_release.py`
- Modified YYToolkit Notice: `../../../ForgePact/yytoolkit-modified/NOTICE.md`
- Credits & License Notices: `../../../ForgePact/CREDITS.md`
- Season 10 Special Content Notes: `../../../ForgePact/docs/S10-special-content-notes.md`
- Dungeon Key & Drop Research: `../../../ForgePact/docs/dungeon-key-research.md`
- Angelic Drop Research: `../../../ForgePact/docs/angelic-drop-research.md`
- Out-of-Process Freeze Probe: `../../../tools/freeze_probe.ps1` (toolkit root, not ForgePact-specific)
- Release Notes: `../../../ForgePact/release-notes-v*.md` (one per shipped version, v1.3.1 onward; required for every version bump, see Representative Change Workflow)
