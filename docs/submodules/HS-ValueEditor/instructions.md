# HS-ValueEditor Module Development Guide

## Module Overview & Metadata
- **Module Name:** HS Value Scanner
- **Submodule Path:** `HS-ValueEditor`
- **Reviewed Git Revision:** `fe2834db2e1783c0ab25228a3dc1fc00aa1bea0f` (Tag: `v1.0.1`, Commit: `Update README.txt`)
- **Revision Date:** `Sat May 9 16:50:02 2026 +0300`
- **Source Availability:** **Binary-only distribution**. The submodule contains the standalone compiled Windows x64 executable (`HSValueScanner.exe`), user and operational instructions (`HS_Value_Scanner_Instructions.txt`), overview notes (`README.txt`), release notes (`RELEASE_NOTES.md`), visual pointer-path workflow guide screenshots (`Step1.png`, `Step2.png`, `Step3.png`), and `.gitattributes`. Source code, dependency manifests, packaging scripts, and automated test suites are **not available** within this submodule checkout.
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or continuous integration / build configurations exist in the repository).
- **License & Provenance:** Community distribution by falorfrozen-cmd (`falorfrozen@gmail.com`). A formal open-source license file is **not available** in the submodule.
- **Purpose & Scope:** Unofficial, standalone runtime memory scanner and value editor for Hero Siege single-player offline sessions. Enables attaching to the running `Hero_Siege.exe` process to search for memory addresses (supporting `Float`, `Double`, and `4 Bytes`), modify stat and stack values, freeze values in memory, calculate persistent pointer paths across game restarts, and organize pointer configurations into shareable JSON presets (`hs_favorites.json`).
- **Research & Context7 Status:** Context7 MCP server was not accessible in the authoring environment; all findings and behavioral descriptions are derived from direct static inspection of submodule artifacts, PE headers, and parent repository integration references.

---

## Architecture & Documented Runtime Behavior

### Repository Layout
```text
HS-ValueEditor/
├── .gitattributes                      # Git LFS / attribute tracking configuration
├── HSValueScanner.exe                  # Standalone Windows x64 PyInstaller binary (12,971,254 bytes)
├── HS_Value_Scanner_Instructions.txt   # Step-by-step scanner, pointer-path, and preset guide
├── README.txt                          # Quick start overview and offline usage boundaries
├── RELEASE_NOTES.md                    # v1.0.1 release notes and changelog
├── Step1.png                           # Visual guide: Adding target scan address to Favorites
├── Step2.png                           # Visual guide: Finding persistent pointer path (Log monitor)
└── Step3.png                           # Visual guide: Saving persistent pointer path into Presets
```

### Documented Runtime Model & Memory Inspection
Based on static binary analysis and documented procedures in `README.txt`, `HS_Value_Scanner_Instructions.txt`, and `RELEASE_NOTES.md`:
1. **Packaging Architecture:**
   - `HSValueScanner.exe` is a 64-bit AMD64 PE executable (`0x8664`, PE32+ optional header `0x020B`, 7 PE sections) packaged with PyInstaller (evidenced by embedded `PyInstaller` bootloader and `_MEI` decompression markers).
2. **Process Attachment:**
   - Attaches to `Hero_Siege.exe` in single-player offline mode via Windows process discovery and memory inspection APIs (`OpenProcess`, `VirtualQueryEx`, `ReadProcessMemory`, `WriteProcessMemory`).
   - Requires Administrator privileges (`SeDebugPrivilege` / `PROCESS_ALL_ACCESS`) to access protected process memory.
3. **Scanning Engine & Data Types:**
   - Supported practical value types in the UI: `Float` (32-bit single-precision float), `Double` (64-bit double-precision float), and `4 Bytes` (32-bit signed integer).
   - Multi-pass filtering workflow: **First Scan** performs an initial memory region scan; **Next Scan** refines matching addresses after in-game value changes until the definitive address is isolated.
4. **Value Modification & Freezing:**
   - **Write:** Writes the specified numeric value directly into the resolved game memory address.
   - **Freeze / Unfreeze:** Continuously overwrites the target address in a background loop to lock the desired value against in-game decrement/decay.
5. **Persistent Pointer Paths & Preset Storage:**
   - Dynamic memory allocation in Hero Siege alters raw memory addresses between game sessions and area loads.
   - **Find Pointer Path:** Performs multi-level pointer backtracking from the isolated static base module offset to the dynamic target address (typically completing in 10–15 seconds).
   - **Auto-Resolve:** When attached to `Hero_Siege.exe`, saved pointer paths automatically recalculate dynamic offsets and restore target addresses.
   - **Preset Persistence (`hs_favorites.json`):** Presets categorize groups of favorite pointer definitions (e.g., Magic Find setups, movement speed, crafting resources) and are serialized to a local JSON file residing next to `HSValueScanner.exe`.

### Visual Workflow Guide
- **Step 1 (`Step1.png`):** Isolate the target stat address using First Scan / Next Scan, right-click the confirmed address in the scan results list, and select **Add to Favorites**.
- **Step 2 (`Step2.png`):** Right-click the newly added entry in the Favorites list and select **Find Pointer Path**. Monitor the status log at the bottom for the confirmation message: `Pointer path found`.
- **Step 3 (`Step3.png`):** Assign or select a preset category from the preset bar and click **Save** to persist the pointer path into `hs_favorites.json`.

---

## Cross-Module Provenance & Disambiguation Warning

> **CRITICAL ARCHITECTURAL WARNING:**
> Do NOT confuse the binary distribution `HS-ValueEditor/HSValueScanner.exe` with the Python script `hs_valuescanner.py` located in the sibling submodule `hs-stat-forge/` (`../../../hs-stat-forge/hs_valuescanner.py`).
>
> While `hs-stat-forge/hs_valuescanner.py` provides a standalone Python scanning utility used internally by HS Offline Stat Forge, `HS-ValueEditor/HSValueScanner.exe` is an independently packaged binary release in a separate repository. Source equivalence, exact compiler flags, internal struct layouts, and patch level parity cannot be assumed without an authoritative source commit in `HS-ValueEditor`.

---

## Documented Feature Catalog & Operations

### Value Scanning & Data Types
- **Double (Recommended for Stats):** Default type for character attributes, Magic Find (MF), movement speed, and scaling attributes.
- **Float:** Used for single-precision coordinates, timers, and specific floating-point multipliers.
- **4 Bytes (Int32):** Used for integer counts, item stack sizes, gold, crystals, and discrete counters.

### Pointer Path & Preset Operations
- **Add to Favorites:** Moves an active address into the persistent favorites pool.
- **Find Pointer Path:** Initiates deep pointer traversal to establish a multi-level pointer chain.
- **Resolve All / Reattach:** Recomputes active addresses from saved pointer paths after game restarts.
- **Preset Management:** Allows creating named profiles (e.g., `MF`, `Speed`, `Runes`) saved in `hs_favorites.json`.
- **Move to Results:** Transfers a resolved favorite back to the active editing table.

### Magic Find (MF) Scanning Strategy
1. Hero Siege frequently applies difficulty-based or aura-based Magic Find bonuses to the UI display. Scan using the clean base value by removing bonus gear or taking difficulty offsets into account.
2. Search using `Double` value format on initial scan.
3. Modify gear or allocate stat points, then trigger `Next Scan` to filter out non-stat memory addresses.

---

## Operating Environment, Prerequisites & Commands

### System Requirements
- **Operating System:** Windows 10 / 11 (64-bit AMD64 / x86_64).
- **Target Process:** `Hero_Siege.exe` (Steam AppID `269210`).
- **Privilege Level:** Administrator rights required (`SeDebugPrivilege` / `PROCESS_ALL_ACCESS` for memory scanning and pointer resolution).
- **Anti-Cheat Boundary:** Must be launched in single-player offline mode without Easy Anti-Cheat (EAC). Never run alongside protected multiplayer sessions.

### Operational Command Reference

| Command | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `.\HSValueScanner.exe` | `HS-ValueEditor\` | Windows GUI / PowerShell | Windows x64, Administrator privileges, `Hero_Siege.exe` running offline without EAC | Launches the graphical scanner interface | Attaches to target process memory; loads `hs_favorites.json` if present | **Inspected** (Runtime execution omitted during static docs authoring) |
| `Get-FileHash -Algorithm SHA256 .\HSValueScanner.exe` | `HS-ValueEditor\` | PowerShell / Windows | Windows PowerShell / PowerShell 7+ | Computes SHA256 digest `5E02E484FBAF498C2D9B9F01EBE40ED74883F7E39773906C6916F609F9D6BDB3` | Read-only | **Verified** |
| `certutil -hashfile .\HSValueScanner.exe SHA256` | `HS-ValueEditor\` | CMD / Windows | Windows standard command toolset | Outputs SHA256 checksum matching release catalog | Read-only | **Verified** |

### Submodule Distribution Artifact Checksums

| File | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :--- |
| `HSValueScanner.exe` | 12,971,254 | `5E02E484FBAF498C2D9B9F01EBE40ED74883F7E39773906C6916F609F9D6BDB3` |
| `HS_Value_Scanner_Instructions.txt` | 3,416 | `3C02C80F4C090BDB4DD02185F79481BBBA6F42B2A7CEA5C3468C9FA566C517DE` |
| `README.txt` | 1,679 | `5813E5939D8B83ADC66AF4766C2345CB9C079BA4F4361914FD0B6BC7E0231672` |
| `RELEASE_NOTES.md` | 1,363 | `2B6710581B5513AC39102F926DBC077F58ED33966A701B89062DDE620BF604B7` |
| `Step1.png` | 1,248,543 | `1A265E84A00D80A357A36C13317333D9B81BE575A5ACA958ACF85697CCE904D2` |
| `Step2.png` | 1,381,333 | `056CAA81773AFB2B539C79C7C03446EC9F6CC4D9DAF5AABBF4DCD95D9B5C8035` |
| `Step3.png` | 1,371,295 | `6290A6687F6138B6A9A76CA3178C383B5C85D444EE8E3A113EB1FAD228EF6CDF` |
| `.gitattributes` | 68 | `D8FB0DE4792538F93822B2C0D235604921299D5E54A3D6EC7A6CB34536E8BF1E` |

---

## Reproducible Development Gap Analysis

Because `HS-ValueEditor` is distributed as a compiled binary release without source code, reproducible source development, automated testing, and compilation cannot be executed directly from this submodule repository.

### Identified Artifact & Source Gaps
1. **Source Code (`*.py` / `*.pyw` / `*.cpp`):** **Not available**. No UI layout definitions, process scanning algorithms, pointer backtracking routines, or memory management source files are included.
2. **Dependency Manifests & Lockfiles:** **Not available**. No `requirements.txt`, `Pipfile`, `poetry.lock`, or `pyproject.toml` is provided. The packaging toolchain is identified as PyInstaller, but exact Python minor versions and library dependencies (e.g., `pymem`, `ctypes`, `tkinter`/`customtkinter`) remain unpinned.
3. **Build & Packaging Scripts:** **Not available**. No `build.bat`, `build.ps1`, or PyInstaller `.spec` build configuration exists in the submodule.
4. **Automated Test Suites & Fixtures:** **Not available**. No automated unit tests, synthetic memory dumps, or process mocking fixtures exist.
5. **Code Signing Certificate:** **Not available**. The release binary is unsigned; Windows Defender SmartScreen warnings may appear upon first launch.

### Reproducibility Requirements Checklist
To enable full source development and deterministic compilation for this submodule, the following upstream deliverables are required:
- [ ] Complete application source code (UI view, scan worker threads, pointer resolver engine).
- [ ] Pinned dependency manifest (e.g., `requirements-build.txt` with specific PyInstaller, GUI toolkit, and memory reading libraries).
- [ ] Deterministic PyInstaller build script or `.spec` recipe (`HSValueScanner.spec`).
- [ ] Synthetic memory snapshot fixtures for unit testing scan algorithms without requiring a running game client.
- [ ] Automated regression test suite for pointer backtracking and JSON preset serialization.

---

## Troubleshooting & Failure Modes

- **Address Displays `?` in Favorites / Results:**
  - *Cause:* Target memory page is not currently committed or pointer chain failed to resolve after game restart.
  - *Resolution:* Click **Resolve All** in the Favorites bar, or reattach to `Hero_Siege.exe` via **Select Process**.
- **Log Reports "No pointer path found":**
  - *Cause:* Memory address is deeply nested, transient, or located on a dynamically allocated heap beyond maximum search depth.
  - *Resolution:* Return to a cleaner in-game state (e.g., town hub), perform a fresh scan to find the base address, and retry **Find Pointer Path**.
- **Access Denied / Memory Open Failure:**
  - *Cause:* Insufficient Windows user permissions, or Easy Anti-Cheat service is running and blocking handle creation.
  - *Resolution:* Ensure Hero Siege is running in single-player offline mode without EAC. Relaunch `HSValueScanner.exe` with Administrator privileges (**Run as administrator**).
- **Pointer Path Breaks After Patch:**
  - *Cause:* Hero Siege game update shifted static base offsets or data structure layouts.
  - *Resolution:* Delete obsolete pointer entries in `hs_favorites.json`, scan the new value, and generate updated pointer paths.
- **Portability of `hs_favorites.json` Across Machines:**
  - *Cause:* Different game builds, OS memory layouts, or DLC configurations may alter pointer stability.
  - *Resolution:* Share `hs_favorites.json` alongside `HSValueScanner.exe` in the same directory; recreate pointer paths if foreign resolution fails.

---

## Gaps, Traceability & Maintenance Triggers

- **Source Code Availability:** If application source code is published upstream, transition this guide from a binary distribution manual to a full Python development guide modeled after [`../HS-Offline-Launcher/instructions.md`](../HS-Offline-Launcher/instructions.md).
- **Game Engine & Offset Maintenance:** When Hero Siege receives updates that shift memory layouts, existing pointer paths in `hs_favorites.json` must be re-verified or regenerated.
- **Submodule Pointer Synchronization:** When upstream updates `HSValueScanner.exe` or documentation, update the parent repository submodule reference and refresh SHA-256 checksums in this guide.
- **Cross-Submodule References:**
  - Parent Submodule Index: [`../README.md`](../README.md)
  - HS Offline Stat Forge (Companion Memory Tool): [`../hs-stat-forge/instructions.md`](../hs-stat-forge/instructions.md)
  - HS Offline Launcher (Offline Direct Launch): [`../HS-Offline-Launcher/instructions.md`](../HS-Offline-Launcher/instructions.md)
  - ForgePact (Native Aurie Gameplay Plugin): [`../ForgePact/instructions.md`](../ForgePact/instructions.md)
