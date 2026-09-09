# HS-Offline-Launcher Module Development Guide

## Module Overview & Metadata
- **Module Name:** HS Offline Launcher
- **Submodule Path:** `HS-Offline-Launcher`
- **Reviewed Git Revision:** `d0a02c5e25d4ff24e58c9445a973682e26e8b642` (Tag: `v1.0.1`, Branch: `main`)
- **Revision Date:** `Wed Sep 2 09:42:06 2026 +0300`
- **Commit Message:** `Release HS Offline Launcher v1.0.1`
- **Source Availability:** Full application source is present (`src/hs_offline_launcher.py`, `tests/test_launcher.py`, `build.ps1`, `requirements-build.txt`, `version_info.txt`, `SHA256SUMS.txt`, `README.md`, `LICENSE`).
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or remote CI configurations exist; validation is conducted locally via Python unit tests and PowerShell build automation).
- **License & Provenance:** MIT License (`Copyright (c) 2026 falorfrozen-cmd`).
- **Purpose & Scope:** Standalone, zero-modification offline game launcher for the Steam edition of Hero Siege (`Hero_Siege.exe`, Steam AppID `269210`). Discovers local Steam installations and game libraries, validates 64-bit Windows PE metadata and runtime dependencies, continuously monitors Easy Anti-Cheat (EAC) service and process states, launches the game directly in single-player offline mode with the required Steam environment variables, and embeds an authenticated local webview UI.

---

## Architecture & Repository Map

### Repository Layout
- `src/`: Core Python application source.
  - `hs_offline_launcher.py`: Standalone single-file application combining the discovery engine, PE validator, SCM/process security monitors, loopback HTTP server (`http://127.0.0.1:8861–8961`), single-page HTML5/CSS3 frontend, and `pywebview` desktop host integration.
- `tests/`: Automated unit test suite.
  - `test_launcher.py`: 30 automated unit tests covering PE validation, Steam and library discovery, EAC status detection, process scan fail-closed safety, double-launch locking, Explorer folder opening guards, and loopback HTTP server authorization/origin security.
- `build.ps1`: Automated packaging script managing an isolated clean virtual environment (`build/packaging-venv/`), dependency validation (`pip check`), test suite gating (`unittest discover`), PyInstaller single-file compilation, and SHA256 checksum generation.
- `requirements-build.txt`: Pinned build-time dependencies for packaging on Python 3.13/3.14 (including `pyinstaller==6.20.0`, `pywebview==6.2.1`, `pythonnet==3.1.0`, `pefile==2024.8.26`, `cffi==2.1.1`, etc.).
- `version_info.txt`: Windows PE version resource definition embedding product name (`HS Offline Launcher`), version (`1.0.1.0`), company (`falorfrozen-cmd`), and copyright details into the compiled executable.
- `SHA256SUMS.txt`: Authoritative release checksum catalog for `HS-Offline-Launcher.exe`.
- `README.md`: User documentation outlining operational steps, development execution, build prerequisites, and safety boundaries.
- `LICENSE`: MIT open-source license.

### Discovery & Validation Engine
The launcher implements multi-step discovery and binary inspection without invoking external tools or game modifications:
1. **Steam Installation Discovery (`registry_steam_roots`, `steam_libraries`):**
   - Queries Windows Registry across three keys: `HKCU\Software\Valve\Steam`, `HKLM\SOFTWARE\Valve\Steam`, and `HKLM\SOFTWARE\WOW6432Node\Valve\Steam` for `SteamPath` and `InstallPath`.
   - Fallbacks to `%ProgramFiles(x86)%\Steam`.
   - Reads `steamapps/libraryfolders.vdf` to discover all configured Steam library drives and paths, parsing both modern `"path"` definitions and legacy numbered indices without mistaking numeric AppIDs for library paths.
2. **Game Executable Resolution (`discover_game_exe`):**
   - Scans library folders for `steamapps/appmanifest_269210.acf` to extract the custom `installdir` path (`steamapps/common/<installdir>/bin/Hero_Siege.exe`).
   - Checks the default directory structure (`steamapps/common/HeroSiege/bin/Hero_Siege.exe`).
   - Verifies the presence of `steam_api64.dll` via `find_steam_runtime` (supporting both modern `bin/`-local placement and root-directory layouts).
3. **PE Binary & Section Validation (`pe_section_names`, `validate_game`, `game_details`):**
   - Directly parses the executable's DOS (`MZ`) and PE headers without external dependencies.
   - Validates that the file is 64-bit AMD64 (`0x8664`), uses PE32+ optional header (`0x020B`), contains between 1 and 96 sections, has the executable flag (`0x0002`), and possesses non-empty section raw data.
   - Computes SHA256 digests and matches them against known Hero Siege Season 10 releases:
     - `438bf4848688c5be52ac15f26f02b46da620d90587c28e766a9cea190f3a7de4`: `Season 10 · 2026.08.30`
     - `0766aa8bfc6eb5679df46f78546644e34fae333adc22474f96903c1d68f251f5`: `Season 10 · 2026.08.24`
     - `ba72b95ac10785d0ecdcc2b3d1925d6cb3439efaf4cef9de2ea1f67d6cfdd4df`: `Season 10 · 2026.08.22 legacy`
   - Detects modified/hooked executables: if a `.aurie` section is present in the PE section table, the launcher identifies the binary as a `"Compatible Aurie/ForgePact build — offline use only"`, allowing safe launch while protection is inactive.

### Runtime Hosting, Security & UI Architecture
```text
+-----------------------------------------------------------------------------------------+
|                                HS-Offline-Launcher Process                              |
|                                                                                         |
|   +-------------------------------------+      +------------------------------------+   |
|   |         pywebview Desktop UI        |      |      Embedded ThreadingHTTPServer  |   |
|   |  - Microsoft Edge WebView2 backend  |      |  - Bound strictly to 127.0.0.1     |   |
|   |  - Dark theme SPA (inline HTML/CSS) |      |  - Ports: 8861, 8862, 8863, 8961   |   |
|   |  - 2-second status polling loop     |      |  - Token check (X-HS-Launcher-Token|   |
|   |  - Native Win32 browse integration  |      |  - Strict Host & Origin validation |   |
|   +------------------+------------------+      +------------------+-----------------+   |
|                      |                                            |                     |
|                      +--------------------+-----------------------+                     |
|                                           |                                             |
|                                           v                                             |
|                             [ Security & SCM Monitor ]                                  |
|                             - Service: advapi32.dll (EasyAntiCheat_EOS)                 |
|                             - Processes: kernel32.dll (Toolhelp32)                      |
|                             - Mutex: LAUNCH_LOCK (prevents double launch)               |
+-------------------------------------------|---------------------------------------------+
                                            |
                                            v (Subprocess Popen with SteamAppId=269210)
+-----------------------------------------------------------------------------------------+
|                                    Child Game Process                                   |
|                                                                                         |
|   +---------------------------------------------------------------------------------+   |
|   | HeroSiege\bin\Hero_Siege.exe                                                    |   |
|   | - Clean direct spawn (no EAC service attached, no memory injection)             |   |
|   | - Steam runtime library path injected into PATH if external                     |   |
|   | - Post-launch background verification thread (checks PID & EAC state after 5s)  |   |
|   +---------------------------------------------------------------------------------+   |
+-----------------------------------------------------------------------------------------+
```

1. **Loopback Server Security:**
   - Binds exclusively to `127.0.0.1` on the first available port in `(8861, 8862, 8863, 8961)`.
   - Generates a cryptographically strong ephemeral session token (`secrets.token_urlsafe(32)`) on boot.
   - Enforces authorization via `X-HS-Launcher-Token` on all API endpoints using constant-time `hmac.compare_digest`.
   - Rejects non-loopback `Host` headers and foreign `Origin` preflight/POST requests with HTTP 403 Forbidden.
   - Employs strict Content Security Policy (`default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; connect-src 'self'`).
2. **EAC & Process Safety Monitoring:**
   - **Service Inspection:** Queries Windows Service Control Manager (`advapi32.dll!QueryServiceStatusEx`) for `EasyAntiCheat_EOS`. Maps numeric states (`SERVICE_STOPPED` = 1, `SERVICE_RUNNING` = 4, `ERROR_SERVICE_DOES_NOT_EXIST` = 1060) in a locale-independent manner. Only `stopped` and `not-installed` are considered safe.
   - **Process Inspection:** Takes process snapshots via `kernel32.dll!CreateToolhelp32Snapshot` to verify that neither `Hero_Siege.exe` nor any EAC binaries (`easyanticheat.exe`, `easyanticheat_eos.exe`, `easyanticheat_eossys.exe`, `start_protected_game.exe`) are currently running.
   - **Fail-Closed Safety:** If process snapshot creation fails or the EAC service state returns `unknown` or `transitioning`, launching is immediately blocked before any process side effects can occur.
3. **Execution & Lifecycle Management:**
   - Single-launch lock (`LAUNCH_LOCK`) prevents concurrent launch attempts.
   - Spawns Steam automatically in silent mode (`steam.exe -silent`) if Steam is not currently running.
   - Re-checks EAC and process state after waiting for Steam startup before initiating the game process.
   - Spawns `Hero_Siege.exe` with environment variables `SteamAppId=269210` and `SteamGameId=269210`. If `steam_api64.dll` is located in the parent directory, prepends its path to `PATH`.
   - Spawns a background thread (`verify_launch`) that re-verifies process health and EAC inactivity after 5 seconds.
4. **State Persistence & Diagnostics:**
   - Settings directory: `%LOCALAPPDATA%\HSOfflineLauncher\` (`STATE_DIR`).
   - Configuration file: `%LOCALAPPDATA%\HSOfflineLauncher\config.json` stores the user-selected `game_exe` path.
   - Rolling diagnostic log: `%LOCALAPPDATA%\HSOfflineLauncher\launcher.log` tracks UI actions (capped at 128 KB).
   - Error log: `%LOCALAPPDATA%\HSOfflineLauncher\launcher-error.log` captures runtime crashes if the WebView host fails to initialize.

### Integration Boundaries & Safety
- **No Game Modification:** The launcher never alters `Hero_Siege.exe`, replaces game DLLs, stops the EAC service, or writes to game directories.
- **Offline / Single-Player Restriction:** Intended strictly for single-player offline sessions. If protected multiplayer services or EAC are active, the launcher fails closed and refuses to start the game.
- **Path Traversal & Shell Injection Guards:** Game folder opening (`/api/folder`) validates that the target path is not a Windows reserved device name, is not a UNC/network share path, and strictly resolves to an existing local directory before invoking `explorer.exe`.

---

## Representative Change Workflow

To modify launcher behavior (for example, adding a new recognized Season build hash or adjusting discovery heuristics):

1. **Update Core Implementation (`src/hs_offline_launcher.py`):**
   - Add new SHA256 hashes and version labels to `KNOWN_BUILDS`.
   - Or adjust PE metadata parsing, library discovery rules, or UI status indicators.
2. **Execute Automated Unit Tests:**
   - Run the unit test suite across all discovery, validation, and security test cases:
     ```powershell
     py -m unittest discover -s tests -v
     ```
   - Confirm all 30 tests pass.
3. **Test Launcher in Development Mode:**
   - Run the Python script directly on Windows:
     ```powershell
     py .\src\hs_offline_launcher.py
     ```
   - Verify that the desktop WebView window opens, detects Steam and the local game installation, displays the correct status badge, and respects safety gates.
4. **Package Windows Release Executable:**
   - Execute the packaging automation script:
     ```powershell
     .\build.ps1
     ```
   - The script creates an isolated virtual environment, checks dependency integrity, runs the test suite, invokes PyInstaller, and writes the signed executable and checksums to `dist\HS-Offline-Launcher.exe` and `SHA256SUMS.txt`.

---

## Supported Platforms & Prerequisites

### Supported Platforms
- **Primary / Runtime Platform:** Windows 10 / 11 (64-bit AMD64). Windows APIs (`advapi32.dll`, `kernel32.dll`, `comdlg32.dll`, `winreg`) and Microsoft Edge WebView2 are required for native execution.
- **Cross-Platform Behavior:** Non-Windows operating systems (Linux, macOS) are detected on startup; `main()` prints an informative error message and exits cleanly (`HS Offline Launcher currently supports Windows only.`). Unit tests mock platform APIs and can run on any OS.

### Prerequisites & Dependencies
- **Python Runtime:** Python 3.11, 3.12, 3.13, or 3.14 (64-bit).
- **GUI Backend:** Microsoft Edge WebView2 Runtime (pre-installed on modern Windows 10/11; falls back to default web browser if unavailable).
- **Game & Platform Dependencies:** Steam client with Hero Siege (AppID `269210`) installed.

### Pinned Build Dependencies (`requirements-build.txt`)
| Dependency | Version | Purpose |
| :--- | :--- | :--- |
| `pyinstaller` | `6.20.0` | Windows PE single-file executable bundler |
| `pyinstaller-hooks-contrib` | `2026.7` | PyInstaller community hooks |
| `pywebview` | `6.2.1` | Lightweight cross-platform native webview desktop window |
| `pythonnet` | `3.1.0` | .NET CLR bridge for Windows Forms / Edge WebView2 integration |
| `clr_loader` | `0.3.1` | Underlying runtime loader for `pythonnet` |
| `pefile` | `2024.8.26` | PE metadata inspection library |
| `cffi` | `2.1.1` | C foreign function interface |
| `pycparser` | `3.0` | C parser for CFFI |
| `pywin32-ctypes` | `0.2.3` | Win32 ctypes reimplementation for PyInstaller resource embedding |
| `setuptools` | `84.0.0` | Packaging toolset |
| `packaging` | `26.3` | Core packaging utilities |
| `typing_extensions` | `4.16.0` | Python type hinting backports |
| `bottle` | `0.13.4` | Micro web-framework dependency |
| `proxy_tools` | `0.1.0` | Proxy tools dependency |
| `altgraph` | `0.17.5` | Graph computation library for PyInstaller module analysis |

---

## Setup, Development & Build Commands

All commands below assume execution from the `HS-Offline-Launcher` submodule root directory unless specified otherwise.

| Command | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `py .\src\hs_offline_launcher.py` | `HS-Offline-Launcher\` | PowerShell / Windows | Python 3.11+, optional `pywebview` | Launches local HTTP server and opens desktop WebView UI (or default browser fallback) | Creates `%LOCALAPPDATA%\HSOfflineLauncher\` config and log files | **Verified** |
| `py -m unittest discover -s tests -v` | `HS-Offline-Launcher\` | PowerShell / Cross-platform | Python 3.11+ | Runs all 30 unit test cases and outputs verbose test report | None (uses temp directories and mocks) | **Verified** |
| `.\build.ps1` | `HS-Offline-Launcher\` | PowerShell (Admin not required) / Windows | Python 3.11+ on PATH | Creates clean venv in `build/`, installs pinned packages, validates `pip check`, runs unittests, builds `dist\HS-Offline-Launcher.exe`, and updates `SHA256SUMS.txt` | Creates/overwrites `build/`, `dist/`, and `SHA256SUMS.txt` | **Verified** |
| `Get-FileHash -Algorithm SHA256 .\dist\HS-Offline-Launcher.exe` | `HS-Offline-Launcher\` | PowerShell / Windows | Compiled release executable in `dist\` | Displays SHA256 hash matching `SHA256SUMS.txt` | Read-only | **Verified** |

---

## Coding Conventions & Implementation Standards

- **Zero Game Modification:** The launcher must remain purely orchestrational. Never add features that modify game memory, inject DLLs, patch disk files, or tamper with system services.
- **Fail-Closed Security Posture:** Any ambiguous or unverified security condition (e.g., failed process snapshot, unknown SCM status, active EAC processes) must unconditionally block the launch action.
- **Strict Typing & Modern Python:** Uses Python 3.10+ annotations (`from __future__ import annotations`, `Path | None`, `tuple[int, str]`).
- **Standard Library Win32 ctypes:** Direct Win32 API calls (`advapi32`, `kernel32`, `comdlg32`) are implemented via `ctypes` and `ctypes.wintypes` with explicit `argtypes` and `restype` definitions.
- **Secure Local IPC:** All HTTP handlers must validate `X-HS-Launcher-Token`, verify `Host` and `Origin`, enforce non-blocking locking on state transitions, and employ CSP headers on HTML delivery.

---

## Test Suite & Verification Fixtures

The test suite in `tests/test_launcher.py` contains 30 unit tests organized into 6 test classes. All tests run in complete isolation using synthetic fixtures and mock patches without requiring Steam, the live game, or Windows administrator privileges.

### Test Classes & Mock Boundaries
| Test Class | Tests | Focus Area | Mock & Fixture Strategy |
| :--- | :--- | :--- | :--- |
| `GameValidationTests` | 7 | PE header validation, 64-bit check, section table structure, `.aurie` detection, runtime resolution | Generates synthetic binary PE files in temporary directory via `write_test_pe` with controlled headers, section counts, machine types, and section payloads. |
| `DiscoveryTests` | 3 | Steam registry, `libraryfolders.vdf`, `appmanifest_269210.acf` parsing, auto-recovery | Uses temporary directories with mocked `registry_steam_roots`, `steam_libraries`, and temporary `config.json` state. |
| `StatusTests` | 8 | Overall readiness calculation, blocker string generation, missing Steam/game handling, UI template binding | Mocks `load_config`, `processes`, `eac_service_status`, and `steam_exe` to test permutations of game/protection/Steam readiness. |
| `EacServiceTests` | 3 | Win32 SCM status code mapping and locale independence | Patches `windows_service_state` to return numeric states (`SERVICE_RUNNING`, `SERVICE_STOPPED`, `ERROR_SERVICE_DOES_NOT_EXIST`, arbitrary errors). |
| `LaunchSafetyTests` | 3 | Mutex double-launch rejection, process snapshot failure blocking, EAC re-verification after Steam boot | Mocks `LAUNCH_LOCK`, `processes`, `validate_game`, and `start_steam_if_needed`, ensuring `subprocess.Popen` is never called when safety checks fail. |
| `FolderOpeningTests` | 2 | Explorer folder opening path validation | Patches `subprocess.Popen` to verify correct arguments and tests rejection of unvalidated paths. |
| `LocalServerSecurityTests` | 4 | HTTP security, token injection, token enforcement, Host header validation, Origin/CORS rejection | Starts an ephemeral `LauncherServer` on `127.0.0.1` and issues real HTTP requests via `http.client.HTTPConnection`. |

---

## Packaging & Distribution Pipeline

The release packaging workflow is defined in `build.ps1` and produces an isolated, reproducible Windows executable:

1. **Environment Isolation:**
   - Sets `$env:PYTHONNOUSERSITE = '1'` and clears `$env:PYTHONPATH` to ensure no ambient user packages contaminate the build.
   - Clears existing build artifacts in `build/` and `dist/`.
   - Creates a fresh virtual environment at `build/packaging-venv/`.
2. **Dependency Installation & Verification:**
   - Installs pinned dependencies from `requirements-build.txt` with `--no-cache-dir`.
   - Executes `pip check` to guarantee dependency tree consistency without version conflicts.
3. **Automated Test Gate:**
   - Executes `python -m unittest discover -s tests -v` within the isolated environment. If any test fails, packaging terminates immediately.
4. **PyInstaller Compilation:**
   - Compiles `src/hs_offline_launcher.py` with the following flags:
     - `--noconfirm --clean --onefile --windowed --noupx`
     - `--name HS-Offline-Launcher`
     - `--version-file version_info.txt`
     - `--exclude-module webview.platforms.android`
     - `--exclude-module webview.platforms.cef`
     - `--exclude-module webview.platforms.cocoa`
     - `--exclude-module webview.platforms.gtk`
     - `--exclude-module webview.platforms.qt`
5. **Checksum Generation & Staging:**
   - Computes lowercase SHA256 hash of `dist\HS-Offline-Launcher.exe`.
   - Writes UTF-8 without BOM checksum file to `SHA256SUMS.txt` in the root repository and copies it to `dist\SHA256SUMS.txt`.
   - If any step fails, removes incomplete artifacts to prevent stale builds.

---

## Troubleshooting & Common Failure Modes

- **"Setup Needed" / "steam_api64.dll is missing":**
  - **Cause:** The game executable was found, but `steam_api64.dll` is missing from `bin/` and the game root directory.
  - **Resolution:** In Steam, right-click Hero Siege -> Properties -> Installed Files -> **Verify integrity of game files**, then restart the launcher.
- **"EAC is active" / "Hero Siege and EAC are active":**
  - **Cause:** A live multiplayer session is running or an EAC service/process (`EasyAntiCheat_EOS`, `easyanticheat.exe`, `start_protected_game.exe`) is active.
  - **Resolution:** Close the protected game and wait for the EAC service to stop before launching in offline mode.
- **"Steam was not found":**
  - **Cause:** Steam registry keys are absent and Steam is not running.
  - **Resolution:** Start Steam before launching, or install Steam if running on a new system.
- **"The selected file is not a valid Hero Siege Windows executable":**
  - **Cause:** The user selected a 32-bit binary, a non-PE file, or an executable named other than `Hero_Siege.exe`.
  - **Resolution:** Browse to `steamapps\common\HeroSiege\bin\Hero_Siege.exe`.
- **WebView Host Initialization Failure:**
  - **Cause:** Edge WebView2 runtime is missing or corrupted.
  - **Resolution:** Check `%LOCALAPPDATA%\HSOfflineLauncher\launcher-error.log`. The launcher will log the exception and fallback to opening the UI in the default browser.

---

## Gaps, Traceability & Maintenance Triggers

- **Season Update Hashes:** When Hero Siege releases a new game update or season patch, compute its SHA256 digest via `file_sha256` and add the hash and release label to `KNOWN_BUILDS` in `src/hs_offline_launcher.py`.
- **PyInstaller & WebView2 Updates:** When bumping dependencies in `requirements-build.txt`, re-run `build.ps1` and verify that `pip check`, the 30 unit tests, and the packaged executable pass sanity checks.
- **YYToolkit & Mod Framework Boundary:** HS-Offline-Launcher does not link or embed YYToolkit directly; it recognizes `.aurie`-modified binaries produced by ForgePact/Aurie as compatible offline targets. No code changes in the launcher are required when YYToolkit updates unless the section header structure changes.
- **CI Automation:** Continuous integration workflows are **not available** in the upstream repository. If automated builds are introduced, port the isolated venv and test-gated PyInstaller steps from `build.ps1` into a GitHub Actions Windows runner.
