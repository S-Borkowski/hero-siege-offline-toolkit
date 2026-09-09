# ForgePact Module Development Guide

## Module Overview & Metadata
- **Module Name:** ForgePact (Hero Siege Season 10 Offline Mod Panel & BloodPactPlugin)
- **Submodule Path:** `ForgePact`
- **Reviewed Git Revision:** `69b65cbda16975277f993bbebb337e0cef89e3c1` (Tag: `v1.3.15`, Branch: `main`)
- **Revision Date:** `Tue Sep 8 02:14:28 2026 +0300`
- **Commit Message:** `v1.3.15 notes in plain language: what was broken for players and what is fixed`
- **Source Availability:** Full application source is present (Python control panel `src/forgepact.py`, C++20 native mod plugin `plugin/ModuleMain.cpp`, modified YYToolkit patches `yytoolkit-modified/`, build scripts `plugin_build/build.bat` and `build_release.py`, Python contract tests `tests/`, and reverse-engineering research notes `docs/`).
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or remote CI configurations exist; verification is conducted locally via Python unittest test suites and static build audits).
- **Purpose & Scope:** Standalone offline control panel and native runtime hook plugin providing runtime modifiers for Hero Siege single-player sessions. Controls monster density, special content spawns (Rift Portals, Battlefields, Cursed Orbs, Chaos Tower, Shadow Realm, etc.), drop rate multipliers and gated drop families (Keys, Relics, Angelic/Unholy uniques), player/combat stat scaling, full map reveal, and custom forge mechanics (Headhunter, Tyrant's Crown, Beacon, Item Editor base stat export) without permanently altering save files or the base game executable.

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
   - Verify that the packaging guard confirms `modfiles_shipped\BloodPactPlugin.dll` matches `plugin_build\BloodPactPlugin_ship.dll`.

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
| `py -m unittest discover -s tests -v` | `ForgePact/` | PowerShell / CMD | Python 3.10+ | Executes all 47 Python contract tests. | Read-only test execution; all tests pass | Verified |
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
  "beacon": false
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
