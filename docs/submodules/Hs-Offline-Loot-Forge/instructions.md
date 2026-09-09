# HS-Offline-Loot-Forge Module Development Guide

## Module Overview & Metadata
- **Module Name:** HS Offline Loot Forge
- **Submodule Path:** `Hs-Offline-Loot-Forge`
- **Reviewed Git Revision:** `c0223075847a4513878c0bff6dbc4d1746e66ed3` (Commit: `Release Loot Forge v2.7.9 for Hero Siege 7.0.5.0`)
- **Revision Date:** `Sat Aug 29 10:00:05 2026 +0300`
- **Source Availability:** **Binary-only distribution**. The submodule repository contains the compiled Windows executable (`HSOfflineLootForge.exe`), user and release documentation (`README.md`, `HSOfflineLootForge_INSTRUCTIONS_EN.txt`, `RELEASE_NOTES_v2.6.0.md` through `RELEASE_NOTES_v2.7.9.md`), UI screenshot (`docs/obsidian-forge-v2.6.0.png`), and checksum catalogs (`SHA256SUMS.txt`). Raw application source code, Python scripts, UI definitions, and compilation manifests are **not available** in the repository.
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or automated build/test workflows are present).
- **License & Provenance:** Closed-source community distribution by falorfrozen-cmd (`falorfrozen@gmail.com`). Formal open-source license file is **not available**.
- **Purpose & Scope:** Runtime target-farming utility for Hero Siege Season 10 (Hero Siege build 7.0.5.0) single-player offline play. Intercepts and reroutes monster drop logic by temporarily writing to the running `Hero_Siege.exe` process memory (using dynamic memory scanning and a private proxy page) without modifying disk game files.

---

## Architecture & Documented Runtime Behavior

### Repository Layout
```text
Hs-Offline-Loot-Forge/
├── docs/
│   └── obsidian-forge-v2.6.0.png         # UI interface screenshot (Obsidian dark theme)
├── .gitattributes                        # Git LFS / attribute settings
├── HSOfflineLootForge.exe                # Standalone Windows x64 PyInstaller binary (50,624,500 bytes)
├── HSOfflineLootForge_INSTRUCTIONS_EN.txt# Comprehensive English user operations and troubleshooting guide
├── README.md                             # Overview, verified features, safety rules, and build notes
├── RELEASE_NOTES_v2.6.0.md               # v2.6.0 release changelog
├── RELEASE_NOTES_v2.6.2.md               # v2.6.2 release changelog
├── RELEASE_NOTES_v2.6.3.md               # v2.6.3 release changelog
├── RELEASE_NOTES_v2.7.8.md               # v2.7.8 release changelog (Maledict & Mythic Jewels update)
├── RELEASE_NOTES_v2.7.9.md               # v2.7.9 release changelog (Hero Siege 7.0.5.0 compatibility)
└── SHA256SUMS.txt                        # Release SHA-256 digest catalog
```

### Documented Runtime Model & Memory Interception
Based on the documented features in `README.md`, `HSOfflineLootForge_INSTRUCTIONS_EN.txt`, and release notes:
1. **Dynamic Memory Resolution ("Adaptive Engine"):**
   - The runtime engine identifies itself as `v2.7.9-s10-705-adaptive`.
   - Instead of static hardcoded memory offsets, Loot Forge executes dynamic pattern scans across the running `Hero_Siege.exe` address space to locate drop dispatch tables and loot routing routines.
   - An explicit "Auto Resolve Build" function scans for updated function signatures across different minor Hero Siege Season 10 revisions.
2. **Private Proxy Page & Temporary Patching:**
   - Loot Forge allocates and manages a private memory proxy page in the target `Hero_Siege.exe` process to redirect loot dispatch calls.
   - Restoring a route or executing **Restore All** rewrites original instruction bytes and cleans up active routes before deallocating proxy pages.
3. **Single-Route Constraint:**
   - The runtime architecture strictly enforces that only **one target route** is active at a time to prevent game state corruption, dispatch table collision, or process crashes.
4. **Zero Disk Modification:**
   - No disk files (`Hero_Siege.exe`, data files, save files) are ever altered on disk.
   - Terminating Loot Forge, invoking **Restore All**, or restarting Hero Siege completely eliminates all runtime modifications.

### State Transitions & Button Lifecycle
The UI displays discrete button states reflecting runtime memory conditions:
- **STANDBY:** Game process is not attached, or the native drop dispatch table has not yet been resolved.
- **OFF:** Target loot route is resolved and ready in memory, but inactive (standard game drop tables active).
- **ON:** Runtime hook is active; monster drop dispatch is routed to the selected item category/rarity.
- **PARTIAL:** Memory hook is partially written or state is inconsistent. User must invoke **Restore All** before proceeding.
- **MISMATCH:** Memory signature at the target location does not match expected opcode patterns (prevents blind patching). User must restore, restart the game, and invoke **Auto Resolve Build**.

---

## Documented Feature Catalog & Farming Profiles

### Quick Farm Profiles
One-click presets that configure runtime drop tables for specific farming targets:
- **Angelic Farm:** Routes drops to Season 10 Angelic / SS rarity items.
- **Unholy Farm:** Directs drop dispatch to the Unholy item focus pool.
- **Heroic Farm:** Directs drop dispatch to native Heroic items.
- **Uncut Jewel:** Directs normal monster drops to native Jewel-category Uncut Jewels (automatically toggles Mythic purple rarity).
- **Materials Farm:** Routes drops to Ore Materials.
- **Rare Material Gates:** Routes drops to Random Orbs.
- **Tarot Farm:** Routes drops to standard Tarot Cards.
- **Divine Tarot:** Routes drops to Season 10 native Divine Tarot cards.
- **Reset Farm:** Restores all active runtime hooks to stock game dispatch.

### Boost Vault Routing Options
Granular individual routing categories available in the UI:
- **Rarity & Focus:** Angelic / SS Drops, Angelic Focus, Heroic Focus, Unholy Focus, Satanic Items, Unique Flasks, Relics.
- **Cards & Keys:** Tarot Cards, Divine Tarot, Codex, Angelic Keys, Satanic Dice, Keys, Dungeon Keys.
- **Charms & Gems:** Normal Charms, Angelic Charms, Unholy Charms, Satanic Charms (includes working Set Charm pool), Uncut Jewels, Incarnation Gems (green Gem category), Boss Gems (native Season 10 pool).
- **Materials & Crafting:** Ore Materials, Essence of Chaos, Random Orbs, Life / Mana Flasks, Satanic Crystal, Blacksmith's Mallet, Destiny Shard, Gypsy's Prophecy, Prophet's Wisdom, Runes, Reflection.
- **Jewel Specific Modifiers (v2.7.8+):**
  - *Maledict Uncut Jewel:* Requests correct Maledict prefix and assigns Maledict stat tables (granting jewel skills).
  - *Uncut Jewel -> Mythic (purple):* Changes base jewel item rarity to purple/Mythic.
  - *Suffix Tier -> S:* Forces maximum tier (Tier S) roll on jewel suffixes.

### Documented Safety Exclusions
The following routes are explicitly excluded or disabled in the UI to prevent game instability:
- **Boss Parts:** Excluded (produced no visible drops from normal monsters).
- **Standalone Set Charm Focus:** Excluded (standalone hook caused game freezes; Satanic Charms route safely drops both Satanic and Set charms).
- **Heroic Charm Focus:** Excluded (Season 10 internal memory mapping unverified).

---

## Operating Environment, Prerequisites & Commands

### System Requirements
- **Operating System:** Windows 10 / 11 (64-bit x86_64 / AMD64).
- **Privilege Level:** Administrator rights required (`HSOfflineLootForge.exe` requires `SeDebugPrivilege` / `PROCESS_ALL_ACCESS` to inspect and write to `Hero_Siege.exe` memory).
- **Target Game:** Hero Siege (Steam AppID `269210`), Season 10 (Hero Siege build 7.0.5.0 verified).
- **EAC State:** Must be run without Easy Anti-Cheat (`Launch Without EAC` via Steam or direct launch via `HS-Offline-Launcher`).

### Operational Command Reference

| Command | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `.\HSOfflineLootForge.exe` | `Hs-Offline-Loot-Forge\` | Windows GUI / PowerShell | Windows 64-bit, Administrator privileges, running `Hero_Siege.exe` without EAC | Launches Loot Forge graphical interface | Attaches to `Hero_Siege.exe` process memory | **Inspected** (Runtime execution not run during docs authoring) |
| `Get-FileHash -Algorithm SHA256 .\HSOfflineLootForge.exe` | `Hs-Offline-Loot-Forge\` | PowerShell / Windows | Windows PowerShell / PowerShell 7+ | Outputs SHA-256 hash `5A0970DEE180B2E19594BD469DD4E0592FDC48E76A0881B5158E441EB23A57A9` | Read-only | **Verified** |
| `certutil -hashfile .\HSOfflineLootForge.exe SHA256` | `Hs-Offline-Loot-Forge\` | CMD / Windows | Windows standard toolset | Displays SHA-256 hash matching `SHA256SUMS.txt` | Read-only | **Verified** |

---

## Reproducible Development Gap Analysis

Because `Hs-Offline-Loot-Forge` is distributed exclusively as a compiled standalone binary, source-level development and packaging cannot be performed directly from the repository in its current state.

### Identified Artifact & Source Gaps
1. **Source Code (`*.py` / `*.cpp`):** **Not available**. No Python source files, C/C++ memory hooking libraries, or UI view templates exist in the repository.
2. **Package Manifests & Dependencies:** **Not available**. No `requirements.txt`, `Pipfile`, `poetry.lock`, or `pyproject.toml` is provided. The binary packaging toolchain is documented as standalone PyInstaller, but exact dependency versions (e.g., UI framework, memory reading library) are unrecorded.
3. **Build & Packaging Scripts:** **Not available**. No `build.ps1`, `build.bat`, or PyInstaller `.spec` configuration file is present.
4. **Automated Test Suite & Test Harness:** **Not available**. No unit tests, memory mock fixtures, or automated integration test suites exist in the submodule.
5. **Code Signing:** **Not available**. Community release binaries are unsigned; Windows SmartScreen warnings are expected.

### Reproducibility Requirements Checklist
To establish a fully reproducible source development workflow for this module, the upstream repository would need to provide:
- [ ] Python / C++ source codebase implementing the GUI, pattern scanner, and process memory manager.
- [ ] Pinned dependency manifest (e.g., `requirements-build.txt` with PyInstaller, ctypes/win32 bindings, UI backend).
- [ ] PyInstaller build specification (`HSOfflineLootForge.spec`) or automated build script (`build.ps1`).
- [ ] Memory signature catalog and pattern definitions used to locate Season 10 dispatch routines.
- [ ] Synthetic memory fixtures or offline mock process tests for validating route patching without a live game.

---

## Troubleshooting & Failure Modes

- **UI Button stuck on "STANDBY":**
  - *Cause:* `Hero_Siege.exe` is not running, or Loot Forge lacks administrator permissions to access process memory.
  - *Resolution:* Ensure Hero Siege is running in single-player offline mode without EAC. Run `HSOfflineLootForge.exe` as Administrator, click **Attach / Select**, and click **Auto Resolve Build**.
- **UI Button shows "MISMATCH":**
  - *Cause:* Target function opcodes in memory do not match the expected pattern (e.g., game updated to a new build, or conflicting memory patch present).
  - *Resolution:* Click **Restore All**, restart both `Hero_Siege.exe` and `HSOfflineLootForge.exe`, and click **Auto Resolve Build**.
- **No Loot Drops from Monsters:**
  - *Cause:* Route is in `OFF` state or incompatible route was selected.
  - *Resolution:* Verify that the button indicates `ON`. Slay normal monsters in a combat zone. Test with a standard Quick Farm profile (e.g., `Angelic Farm`).
- **Game Crashes or Freezes:**
  - *Cause:* Multiple conflicting routes activated simultaneously, or excluded routes (such as standalone Set Charm) were forced.
  - *Resolution:* Close and restart Hero Siege. Strictly adhere to the single-route constraint and use **Restore All** before switching routes.
- **Access Denied / Memory Error:**
  - *Cause:* Easy Anti-Cheat driver is active, preventing memory read/write operations (`PROCESS_VM_WRITE`).
  - *Resolution:* Close Hero Siege completely and relaunch it using `Launch Without EAC` or `HS-Offline-Launcher`.

---

## Gaps, Traceability & Maintenance Triggers

- **Game Update & Signature Maintenance:** When Hero Siege releases patches (e.g., Season 10 point releases beyond 7.0.5.0), the pattern scanner must be updated with new function signatures if opcodes shift.
- **Source Code Recovery:** If open-source development is initiated, create the required Python project structure (`src/`, `tests/`, `build.ps1`, `requirements.txt`) following the standard architecture used in companion tools like `HS-Offline-Launcher`.
- **Submodule Reference Synchronization:** When upstream publishes new versions or source updates, update the Git submodule pointer in the parent repository and verify the binary SHA-256 digest against `SHA256SUMS.txt`.
