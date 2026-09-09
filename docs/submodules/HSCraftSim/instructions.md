# HSCraftSim Module Development Guide

## Module Overview & Metadata
- **Module Name:** HSCraftSim (Cube Workshop)
- **Submodule Path:** `HSCraftSim`
- **Reviewed Git Revision:** `70e193b52e2e2a83af421cd97f8175ec921d8dc7` (Tag: `v1.0.2`)
- **Revision Date:** `Tue Sep 8 21:38:13 2026 +0300`
- **Commit Message:** `Add Falor and Graxy_TV credits in 1.0.2`
- **Source Availability:** Full application source is present (JavaScript ES modules, Python desktop host, Python/Node.js build tools, tests, and research artifacts).
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or external CI configurations exist in the repository; validation is performed locally via npm and Python test scripts).
- **Purpose & Scope:** Standalone offline crafting simulator reproducing Hero Siege's Cube crafting mechanics (recipes, rerolls, socketing, corruption, Runewords, Infernal Codex, and probability distributions). Runs both as an interactive static web application and as a packaged native Windows desktop application via Microsoft Edge WebView2.

---

## Architecture & Repository Map

### Repository Layout
- `engine/`: Core simulation logic written in pure ES6 JavaScript.
  - `cpr.js`, `cpr.py`: Pseudo-random number generator implementation matching game logic.
  - `items.js`, `item_setup.js`, `item_display.js`, `item_modifiers.js`: Item model, stat definitions, display formatting, and modification pipelines.
  - `recipes.js`, `recipe_context.js`, `craft_action.js`, `mechanics.js`: Recipe matching, validation, execution, and consumption rules for 301 recipes (146 cube, 48 prospecting, runewords, codex, etc.).
  - `crystal.js`, `crystal_rules.js`: Satanic Crystal slam, star upgrades, and corruption logic.
  - `runewords.js`, `runeword_rules.js`, `codex.js`, `codex_effects.js`: Runeword matching, socket validation, and Codex progression.
  - `socket_levels.js`, `socketable_rules.js`, `socket_stats.js`: Sockets, runes, gems, and jewel insertion mechanics.
  - `session.js`, `session_random.js`, `history.js`: Workspace session state, repeatable seed management, undo stack, and craft history journaling.
  - `analysis-worker.js`: Web Worker executing 1,000–100,000 probability simulation trials off the main UI thread.
- `ui/`: User interface components and styles.
  - `index.html`: Web application shell and layout.
  - `app.js`, `startup.js`: UI bootstrapping, module loading, event routing, and keyboard shortcut binding.
  - `desktop-storage.js`: Storage bridge routing persistence between browser `localStorage` and the native desktop host.
  - `item-setup.js`, `item-tooltip.js`, `recipe-card.js`, `codex-workshop.js`, `runeword-workshop.js`, `last-craft.js`, `history-view.js`: Modular UI views and components.
  - `*.css`: Viewport styles, game theme skinning, tooltip styling, and responsive layout definitions.
- `data/`: Game data catalogs, metadata, translations, and static assets.
  - `items_catalog.json`: 2,097 catalog records (1,932 named items, 65 hidden records).
  - `recipes.json`, `recipes_craft_raw.json`, `recipes_prospect_raw.json`: Structured recipe definitions and raw extracted combo tables.
  - `item_profiles.json`, `current_item_text.json`, `stat_names.json`, `stat_pools.json`: Stat models, descriptions, and roll ranges.
  - `codex_rules.json`, `current_craft_rules.json`, `current_crystal_rules.json`, `current_modifier_rules.json`, `current_socketable_rules.json`: Current game rule extracts.
  - `icons/`: 1,609 item sprite icons (`<spr>.png`).
  - `game/`: Shipped GameMaker sprites, cube backgrounds, borders, and asset import manifest (`manifest.json`).
  - `translations/`: Multi-language localisation extracts (`craft_en.json`, `attributes.json`, etc.).
- `tools/`: Build and reverse-engineering utilities.
  - `build-web.mjs`: Node.js script bundling web assets, compressing runtime data with gzip into `dist/data/runtime.bin`, and writing `dist/build-report.json`.
  - `build-desktop.py`: Python script orchestrating the web build, unit tests, PyInstaller Windows packaging, isolated self-tests, checksums, and release ZIP creation.
  - `extract_cube_assets.py`, `build_recipes.py`, `build_item_profiles.py`, `import_item_editor.py`, `extract_translations.py`: Data extraction and migration tools.
- `tests/`: Automated test suites and verification fixtures.
  - 39 Node.js `.mjs` test suites covering mechanics, workshop UI logic, codex, runewords, sockets, item generation, and web build output.
  - Python tests: `tests/test_desktop.py` (server and session storage unit tests), `tests/test_cpr.py` (RNG verification), `tests/test_desktop_release.py` (isolated WebView2 executable launch test).
  - Native verification fixtures (`current_*_native.json`, `editor_golden.json`) and Markdown verification logs.
- `research/`: Reverse-engineering documentation, decompiler outputs, Ghidra annotations, and current native verification audits.
  - `current/VERIFIED_RULES.md`: Authoritative record of verified native game mechanics vs. simulation approximations.
  - `current/`: Extraction scripts and native probe data.

### Entry Points & Hosting Models
1. **Local Web Development Server (`server.py` / `Start.bat`):**
   - Lightweight, zero-dependency Python HTTP server (`http.server.ThreadingHTTPServer`) serving the repository root.
   - Listens exclusively on loopback (`127.0.0.1:17870` or next free port up to `17879`).
   - Serves `/_health` health check endpoint and automatically launches the default web browser to `http://127.0.0.1:17870/ui/`.
2. **Static Web Production Bundle (`dist/`):**
   - Built via `node tools/build-web.mjs`.
   - Bundles all CSS into `ui/app.css`, packages static assets under `assets/<revision>/`, and compresses all core data into a single `data/runtime.bin` gzip payload (reducing data size by ~92%).
   - Ready for deployment to any static HTTPS web server (no backend, API, or database required).
   - Previewed locally via `npm run preview` on `http://127.0.0.1:17880/`.
3. **Desktop Native Application (`HSCraftSim.py` / `HSCraftSim.exe` / `Start-Desktop.bat`):**
   - Native desktop wrapper built on `pywebview` with Microsoft Edge WebView2 backend (`edgechromium`).
   - Launches an embedded ephemeral loopback HTTP server (`desktop_runtime.start_server`) serving `dist/`.
   - Injects desktop markers (`<meta name="hscraftsim-desktop" content="1">`) and exposes the Python `SessionStore` API to JavaScript via `window.pywebview.api`.
   - Manages single-instance execution via `ProfileLock` (`desktop.lock` with non-blocking Windows `msvcrt` locking).
   - Persists user sessions atomically to `%LOCALAPPDATA%\HSCraftSim\session.json`.

### Integration Boundaries & Safety
- **No Game/Save Modification:** HSCraftSim is strictly an offline, standalone simulation tool. It does not attach to live game processes, inject code, or modify Hero Siege save files.
- **Client-Side Storage:** Browser session state lives in `localStorage` (`hscraftsim.workshop.v2`); desktop sessions live in `%LOCALAPPDATA%\HSCraftSim\session.json`. Sessions can be transferred between platforms using the built-in JSON Export/Import features.
- **Loopback Enforcement:** Local development and desktop servers bind strictly to `127.0.0.1`, verify request `Host` headers, and disable directory listings.

---

## Representative Change Workflow

To modify or extend a simulation mechanic (for example, updating a recipe calculation or star outcome):

1. **Update Engine Logic:**
   - Modify the calculation in `engine/` (e.g., `engine/crystal.js` or `engine/craft_action.js`).
   - Ensure the CPR PRNG is used for all probabilistic choices via `engine/session_random.js`.
2. **Update UI / View Layer (if applicable):**
   - Adjust presentation or inspection logic in `ui/` (e.g., `ui/last-craft.js`, `ui/item-tooltip.js`).
3. **Validate Syntax & Test Suites:**
   - Run syntax checking across all engine and UI files:
     ```powershell
     npm run check
     ```
   - Run the full Node.js unit test suite:
     ```powershell
     npm test
     ```
4. **Build Static Distribution:**
   - Compile and bundle the web runtime distribution:
     ```powershell
     npm run build
     ```
5. **Validate Web Build & Distribution:**
   - Run the production distribution verification suite:
     ```powershell
     npm run test:web
     ```
   - Alternatively run the aggregate verification command:
     ```powershell
     npm run verify
     ```
6. **Validate Desktop Runtime & Host:**
   - Execute the Python desktop test suite:
     ```powershell
     python -m unittest tests.test_desktop tests.test_cpr
     ```
7. **(Optional) Build and Verify Desktop Executable:**
   - If releasing a desktop package, run:
     ```powershell
     python tools/build-desktop.py
     ```
   - This automatically verifies web assets, executes `tests.test_desktop`, compiles `release/HSCraftSim-1.0.2-Windows-x64/HSCraftSim.exe`, runs an isolated WebView2 self-test, and generates the final ZIP with SHA256 checksums.

---

## Supported Platforms, Prerequisites & Dependencies

### Supported Platforms
- **Web:** Modern Evergreen web browsers supporting ES2020 modules, Web Workers, CSS Grid/Flexbox, and the Compression Streams API (Google Chrome, Mozilla Firefox, Microsoft Edge, Safari).
- **Desktop:** Windows 10 / Windows 11 x64 with Microsoft Edge WebView2 Runtime (Evergreen).

### Prerequisites & Tools
- **Node.js:** >= 18.0.0 LTS (required for ES module syntax checks, build scripts, and test runners).
- **Python:** 3.10+ (tested on Python 3.13 x64; required for local server, desktop host, and PyInstaller packaging).
- **Microsoft Edge WebView2 Runtime:** Required for running the desktop application from source or packaged binary.
- **Hero Siege Game Assets / Binaries:** **Not required** for running or developing the simulator. Extracted assets are committed to the repository. Extraction tools (`tools/extract_cube_assets.py`) require original game files and Python `Pillow` only when regenerating base assets.

### Dependency Manifests
- **Web / Engine:** Zero runtime npm dependencies (`package.json` contains only build/test scripts and `"type": "module"`).
- **Desktop Runtime (`requirements-desktop.txt`):**
  - `pywebview==6.2.1` (transitive: `pythonnet`, `clr_loader`, `bottle`, `proxy_tools`, `typing_extensions`).
- **Desktop Packaging (`requirements-build.txt`):**
  - `-r requirements-desktop.txt`
  - `pyinstaller==6.20.0`
- **Asset Tools (Optional / Development):**
  - `Pillow` (for sprite sheet slicing in `tools/extract_cube_assets.py`).

---

## Command Reference

| Command | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
|---|---|---|---|---|---|---|
| `npm test` | `HSCraftSim/` | PowerShell / Bash | Node.js >= 18 | Runs 39 `.mjs` test suites in `tests/` testing engine mechanics, codex, runewords, sockets, etc. All tests pass with exit code 0. | None (read-only execution) | Inspected |
| `npm run check` | `HSCraftSim/` | PowerShell / Bash | Node.js >= 18 | Executes `node --check` across 27 JavaScript files in `ui/`, `engine/`, and `tools/`. | None (syntax analysis only) | Inspected |
| `npm run build` | `HSCraftSim/` | PowerShell / Bash | Node.js >= 18 | Runs `tools/build-web.mjs`. Compiles all styles into `dist/assets/<rev>/ui/app.css`, packages images, compresses data to `runtime.bin`, writes `dist/build-report.json`. | Replaces `dist/` directory contents | Inspected |
| `npm run preview` | `HSCraftSim/` | PowerShell / Bash | Python 3, `dist/` built | Starts Python HTTP server on `127.0.0.1:17880` serving `dist/`. | Listens on local port 17880 | Inspected |
| `npm run test:web` | `HSCraftSim/` | PowerShell / Bash | Node.js >= 18 | Executes `npm run build` then runs `tests/test_web_build.mjs` verifying production URLs, assets, data loading, and catalog stat formatting. | Regenerates `dist/` | Inspected |
| `npm run verify` | `HSCraftSim/` | PowerShell / Bash | Node.js >= 18 | Runs `npm test`, `npm run check`, and `npm run test:web` sequentially. | Regenerates `dist/` | Inspected |
| `python server.py` | `HSCraftSim/` | PowerShell / Bash | Python 3 | Starts local dev server on `127.0.0.1:17870` (or next free port) and opens `http://127.0.0.1:17870/ui/` in default browser. | Listens on local port, launches browser | Inspected |
| `python server.py --no-browser --port 17870` | `HSCraftSim/` | PowerShell / Bash | Python 3 | Starts local dev server without launching a browser window. | Listens on specified local port | Inspected |
| `Start.bat` (or `Baslat.bat`, `run.bat`) | `HSCraftSim/` | Windows CMD / Explorer | Python 3 | Launches `pythonw server.py` in background and opens browser. | Spawns background `pythonw` process | Inspected |
| `python HSCraftSim.py` | `HSCraftSim/` | PowerShell / CMD | Python 3, `requirements-desktop.txt`, built `dist/` | Launches native desktop window using Microsoft Edge WebView2 and local `dist/` HTTP server. | Creates/locks `%LOCALAPPDATA%\HSCraftSim\desktop.lock`, saves `session.json` | Inspected |
| `python HSCraftSim.py --profile-dir <DIR> --self-test <REPORT.json>` | `HSCraftSim/` | PowerShell / CMD | Python 3, `requirements-desktop.txt`, built `dist/` | Runs headless WebView2 self-test verifying readiness, 301 recipes, bridge functions, layout, and credits, writing JSON report. | Writes JSON test report, cleans up window | Inspected |
| `Start-Desktop.bat` | `HSCraftSim/` | Windows CMD / Explorer | Python 3, `requirements-desktop.txt`, built `dist/` | Launches `pythonw HSCraftSim.py` without a console window. | Spawns background desktop window | Inspected |
| `python -m unittest tests.test_desktop tests.test_cpr` | `HSCraftSim/` | PowerShell / Bash | Python 3 | Executes Python unit tests covering server handlers, atomic session storage, profile locking, and CPR RNG math. | Creates temporary test files | Inspected |
| `python tools/build-desktop.py` | `HSCraftSim/` | PowerShell / CMD (Windows x64) | Python 3, Node.js, `requirements-build.txt` | Runs full web verify, desktop tests, PyInstaller packaging, isolated WebView2 test, generates `release/HSCraftSim-<ver>-Windows-x64.zip` and `SHA256SUMS.txt`. | Creates `build/desktop/` and `release/` artifacts | Inspected |
| `Build-Desktop.bat` | `HSCraftSim/` | Windows CMD / Explorer | Python 3, Node.js, `requirements-build.txt` | Executes `python tools/build-desktop.py`. | Creates build and release directories | Inspected |
| `Build-Web.bat` | `HSCraftSim/` | Windows CMD / Explorer | Node.js >= 18 | Executes `npm run verify`. | Regenerates `dist/` | Inspected |

*Status notes:* Commands marked **Inspected** have been statically verified against manifests, script definitions, and source files without triggering unneeded production packaging builds.

---

## Coding Conventions & Architecture Details

### JavaScript / ES Module Standards
- Written in modern standard ECMAScript modules (`import` / `export`) with `"type": "module"` declared in `package.json`.
- Strict separation of concerns:
  - **Simulation Engine (`engine/`):** Pure functions operating on item state objects, recipes, and PRNG seeds without DOM references. Suitable for execution in Node.js, Web Workers, and browser contexts.
  - **User Interface (`ui/`):** Handles DOM events, keyboard shortcuts, canvas rendering, modal dialogs, drag-and-drop, and localized string formatting.
- Deterministic random simulation via `engine/cpr.js` implementing Hero Siege's `cpr_init` and `cpr_irandom` linear congruential generator formulas.
- Off-thread heavy computations: Monte Carlo probability simulations run in `engine/analysis-worker.js` (1,000–100,000 trials) communicating via standard `postMessage` interfaces.

### Data Model & Item State Format
Item instances in the simulation state adhere to the GameMaker save JSON structure and Item Editor specifications:
- `a`: Base numeric seed (`int64`).
- `b`: Base item ID (`itemId`).
- `c`: Unique / static item flag (`0` or `1`).
- `j`: Subtype / weapon class ID.
- `s`: Sockets seed.
- `w`: Bricked / corrupted status flag (`1` = corrupted, `0` = normal; corrected in 2026-09-07 research).
- `r`: Corruption state marker.
- `p`: Star upgrade count (incremented on successful star upgrade).
- `q`: Satanic Crystal slam count / status.
- `v`, `t`: Infernal Codex modifier flags.
- `l`: Augment level.
- `o`: Origin / stack amount flag.
- `g`: Equipped group index.
- `h`: Generated item flag.
- Derived values (`item.GetItemInfo(idx)`): Rarity (`27`), Tier (`32`), Weapon Subtype (`34`), Two-Handed (`21`), Max Stack (`31`), Value (`9`).

### Desktop Architecture & Persistence Pipeline
- **SessionStore (`desktop_runtime.py`):**
  - Session path: `%LOCALAPPDATA%\HSCraftSim\session.json`.
  - Max payload limit: 10 MB (`MAX_SESSION_BYTES = 10_000_000`).
  - Validation: Requires schema version 2 (`schema: 2`), `state: {}`, and non-negative integer `desktopRevision`.
  - Atomic writing: Writes payload to `session-<pid>-<rand>.tmp` inside the profile folder, flushes and calls `os.fsync`, then atomically replaces `session.json` via `os.replace`.
  - Monotonic revision check: Close-time flushes and queued saves reject out-of-order writes where `revision < self._revision`.
- **ProfileLock (`desktop_runtime.py`):**
  - Maintains `desktop.lock` in the user profile directory.
  - Employs Windows `msvcrt.locking(stream.fileno(), msvcrt.LK_NBLCK, 1)` to prevent concurrent instances from corrupting profile data.
- **Desktop Web Server (`DesktopHandler`):**
  - Subclasses `SimpleHTTPRequestHandler` with strict loopback host validation (`127.0.0.1:<port>`).
  - Prevents path traversal via `target.is_relative_to(root)` checks.
  - Automatically injects `<meta name="hscraftsim-desktop" content="1">` into `index.html`.
  - Configures caching: immutable cache (`max-age=31536000`) for `/assets/`, `no-cache` for HTML/data files, and `X-Content-Type-Options: nosniff`.

---

## Data Provenance & Simulation Boundaries

### Evidence & Extraction Sources
- **Historical Season 10 Build:** `Hero_Siege.exe.aurie_backup` (SHA-256 `2034fad4…`, image base `0x140000000`). Used for initial static recipe and switch table decompilation (`DoCraftResult` RVA `0x803010`).
- **Item Editor Reference:** Tied to Steam 7.0.5.0 (`438bf484…`). Supplies 1,444 stat profiles, 392 stat descriptions, 817 skill names, 22 special roll models, and 267 socket chains.
- **Clean Game Backup (`c6ecc069…`):** Authoritative source for current native modifier rules, 144 socketable item definitions, star/corruption odds, and base value ranges.
- **Asset Manifest:** UI sprite imports and file hashes are recorded in `data/game/manifest.json`.

### Verified Native Behaviors vs. Simulation Assumptions
- **Verified Native Outcomes (covered by golden test fixtures):**
  - Destiny / Gypsy star outcomes: 70% star increase (increments `p`), 22% star decrease (floor at 0), 8% corruption (resets stars to 0, sets corruption flag).
  - Satanic Dice: Rerolls item stats while preserving base seeds; checks stat 20.
  - Orbs: Rerolls select 1 of 18 Orb types and outputs 8 copies of that type (matching `DoCraftResult` case `0x16`).
  - Add Sockets Gate: Enforces rarity/type/zero-socket rules; excludes Satanic, Angelic, Runeword, Heroic, and Unholy items. 273 normal definitions verified against 1,911 native results.
  - Codex Progression: Infernal Codex tiers 1–20, 25 Dungeon Keys, 9 Relic uniques, and 7 Codex Runewords.
  - Random Unique Crafting: Selects 1 of 6 tiers equally, then draws 1 eligible item from that tier pool (pools contain 48, 61, 79, 152, 218, 203 items). Angelic/Unholy use 31 and 17 tier-SS items.
- **Simulation Assumptions & Gaps (Labeled / Unverified):**
  - **PRNG Execution:** HSCraftSim uses a client-side seeded simulation PRNG (`cpr.js`), not the live game's runtime RNG state. A simulated sequence will not reproduce the live game's random drops.
  - **Normal Item Affix Generation:** Normal item rarity/affix generation chains and natural socket generation are partially modeled and approximate.
  - **Angelic Augments & Combat Stats:** Angelic augment roll combinations and live combat stat evaluations (requiring player runtime state) are outside simulation scope.
  - **Player Testing vs. Automated Tests:** Observations in `PLAYER-TESTING.md` represent player-reported usability feedback and subjective testing guidelines, distinct from deterministic golden test assertions.

---

## Troubleshooting & Common Issues

1. **Desktop Window Does Not Open / WebView2 Error:**
   - *Cause:* Microsoft Edge WebView2 Evergreen Runtime is missing or damaged.
   - *Resolution:* Download and install the Evergreen Bootstrapper from Microsoft's official WebView2 portal: `https://developer.microsoft.com/en-us/microsoft-edge/webview2/`.
2. **"HSCraftSim is already running" Error:**
   - *Cause:* A lingering `desktop.lock` file or background `HSCraftSim.exe` / `pythonw.exe` process is holding the file lock in `%LOCALAPPDATA%\HSCraftSim\`.
   - *Resolution:* Close existing instances via Task Manager or delete `%LOCALAPPDATA%\HSCraftSim\desktop.lock` after terminating orphan processes.
3. **Blank Page or Network Errors When Opening `index.html` Directly:**
   - *Cause:* Modern browsers block ES module imports and fetch requests on `file://` URIs due to CORS restrictions.
   - *Resolution:* Do not open `index.html` via `file://`. Use `Start.bat`, `python server.py`, or `npm run preview`.
4. **Port In Use During Local Web Development:**
   - *Cause:* Another service is bound to port 17870.
   - *Resolution:* `server.py` automatically tries ports 17870 through 17879. If all are occupied, specify a custom port: `python server.py --port 18000`.
5. **Saved Session Corruption or Version Incompatibility:**
   - *Cause:* Stale or oversized session payload in `localStorage` or `%LOCALAPPDATA%\HSCraftSim\session.json`.
   - *Resolution:* Clear browser `localStorage` or delete `%LOCALAPPDATA%\HSCraftSim\session.json`. Use the in-app Export / Import JSON feature to migrate valid sessions.

---

## Source Documents & Evidence References
- Submodule Readme: `../../../HSCraftSim/README.md`
- Desktop Documentation: `../../../HSCraftSim/DESKTOP.md`
- Player Testing Guide: `../../../HSCraftSim/PLAYER-TESTING.md`
- Reverse-Engineering Research Record: `../../../HSCraftSim/RESEARCH.md`
- Authoritative Verified Native Rules: `../../../HSCraftSim/research/current/VERIFIED_RULES.md`
- Package Manifest & Scripts: `../../../HSCraftSim/package.json`
- Desktop Entry Point: `../../../HSCraftSim/HSCraftSim.py`
- Desktop Runtime & Storage: `../../../HSCraftSim/desktop_runtime.py`
- Local HTTP Launcher: `../../../HSCraftSim/server.py`
- Web Asset Bundler: `../../../HSCraftSim/tools/build-web.mjs`
- Desktop Packaging Tool: `../../../HSCraftSim/tools/build-desktop.py`
- Verification Fixtures: `../../../HSCraftSim/tests/`
