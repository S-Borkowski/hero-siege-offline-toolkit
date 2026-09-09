# HS Offline Stat Forge Module Development Guide

## Module Overview & Metadata
- **Module Name:** HS Offline Stat Forge (`hs-stat-forge`)
- **Submodule Path:** `hs-stat-forge`
- **Reviewed Git Revision:** `4d40636a7dd676b4893ab9ce19f1acc5e0791205` (Tag: `v2.5.0`, Branch: `main`)
- **Revision Date:** `Sat Sep 5 11:40:09 2026 +0300`
- **Commit Message:** `Release Stat Forge v2.5.0 additive All Skills`
- **Source Availability:** Full application source is present (Tkinter UI and memory manager `hs_statforge.py`, multiplier engine `hs_statforge_multiplier.py`, scanner utility `hs_valuescanner.py`, stat configuration `hs_statforge_stats.json`, C++20 native density injection runtime `native_density/HSStatForgeDensity.cpp`, MinHook third-party library `native_density/third_party/minhook/`, build script `native_density/build.ps1`, PyInstaller specification `HSStatForge.spec`, and Python test/probe suite `scripts/`).
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions or remote CI configurations exist; verification is conducted locally via Python unittest/verification scripts and MSVC native builds).
- **Purpose & Scope:** Standalone offline desktop control panel and runtime memory modification tool for Hero Siege Season 10. Adjusts runtime character statistics (Magic Find, Movement Speed, All Skills additive bonus, All Skills absolute override, EXP multiplier, Total Damage bonus, Attack Speed bonus, Faster Cast Rate, Skill Haste, Defense, Critical Strike Chance/Damage, Spell Critical Chance/Damage) and controls monster density scaling (1.0x to 5.0x in 0.5x increments) via dynamically allocated x64 code caves and native DLL injection into `Hero_Siege.exe` without permanently altering game binaries or local save files.

---

## Architecture & Repository Map

### Repository Layout
- `hs_statforge.py`: Main desktop application frontend (Tkinter) and memory engine. Manages process discovery, Toolhelp/PEB main-module fallback, GameMaker function resolution, code-cave allocation, thread suspension during writes, config lifecycle (`hs_statforge_stats.json`), heartbeat monitoring, and density runtime injection.
- `hs_statforge_multiplier.py`: Specialized multiplier and proxy engine. Implements RValue dynamic array unwrapping, call-stack compensation for RSP-relative epilogues, return-site validation, and UI copy formatting.
- `hs_valuescanner.py`: Memory scanning, pointer path resolution, PE header parsing, thread/module enumeration, and Windows API wrappers (`ctypes`/`pymem`).
- `hs_statforge_stats.json`: Stat configuration catalog defining keys, display names, UI button colors, slider limits, and resolver metadata (Version 9, Season 10).
- `HSStatForge.spec`: PyInstaller freeze specification for building `HSStatForge.exe` with UAC administrator execution level (`uac_admin=True`) and bundled `HSStatForgeDensity.dll`.
- `native_density/`: Native C++20 dynamic library for standalone monster density multiplication.
  - `HSStatForgeDensity.cpp`: C++20 source implementing `instance_create_depth` and `instance_create_layer` hooks via MinHook, protected variable pool monitoring, per-second rate limiting, GameMaker `FREE_RValue` memory reclamation, and shared memory IPC (`HSFD` magic, struct version 3).
  - `NativeGameTypes.hpp`: C++ type definitions for GameMaker internal structures (`RValue`, `YYGMLRoutine`, etc.).
  - `build.ps1`: MSVC build script compiling `HSStatForgeDensity.dll` using `vcvars64.bat` and `cl.exe`.
  - `bin/HSStatForgeDensity.dll`: Pre-compiled release 64-bit DLL binary.
  - `third_party/minhook/`: MinHook x86/x64 API hooking library (3-Clause BSD License).
- `scripts/`: Automated unit tests, in-process execution tests, and live-game research probes.
  - `test_s10_multiplier.py`: Pure Python/in-memory contract test exercising mock harnesses and Capstone disassembly validation.
  - `test_s10_adaptive_builds.py`: Pure Python test validating Season 10 adaptive resolver patterns across synthetic byte streams.
  - `test_total_damage_epilogue_resolver.py`: Pure Python test checking Total Damage epilogue resolution and adjacent helper code rejection.
  - `test_dynamic_proxy_allocation.py`: Platform-native test validating dynamic memory allocation within the local Python process (`os.getpid()`).
  - `test_module_resolution_fallback.py`: Platform-native test verifying PEB main-module resolution fallback when Toolhelp/Pymem fails.
  - `test_native_result_hook_execution.py`: Platform-native in-process execution test for native result multiplier code caves.
  - `test_native_result_epilogue_hook_execution.py`: Platform-native in-process execution test for epilogue multiplier hooks.
  - `test_set_variable_hook_execution.py`: Platform-native in-process execution test for variable injection hooks.
  - `test_extended_result_hook_execution.py`: Platform-native in-process execution test for extended result resolvers.
  - `test_density_runtime.py`: Live-game test script injecting `HSStatForgeDensity.dll` into running `Hero_Siege.exe` and polling shared state.
  - `probe_attack_speed_root.py`: Live-game probe inspecting `gml_Script_StatAttackSpeed` scalar results.
  - `probe_density_runtime.py`: Live-game memory probe scanning GameMaker object tables and `Enemy_Creator_*` definitions.
  - `probe_extended_result_calls.py`: Live-game probe tracing real call execution counts for extended stat hooks.
  - `probe_live_multiplier.py`: Live-game probe testing active multiplier hooks on `Hero_Siege.exe`.
  - `probe_named_result.py`: Live-game probe verifying GameMaker named function resolution.
  - `verify_extended_result_resolvers.py`: Live-game verification utility testing all configured stat resolvers against live memory.
- `STATFORGE_S10_REAL_STAT_HOOK_NOTES.md`: Empirical research document detailing Season 10 YYC 64-bit RValue array layouts, epilogue patterns, and verification metrics.

---

## Component Architecture & Runtime Hooking Pipeline

```text
+-----------------------------------------------------------------------------------------+
|                                HS STAT FORGE (Python / Tkinter)                         |
|                                                                                         |
|   +-------------------------------------+      +------------------------------------+   |
|   |         hs_statforge.py             |      |    hs_statforge_multiplier.py      |   |
|   | - Tkinter GUI & Slider Controls     |      | - RValue dynamic array unwrapper   |   |
|   | - Process Watcher & Heartbeat Loop  |      | - CALL-stack epilogue compensator  |   |
|   | - Config Loader (Stats JSON v9)     |      | - Multiplier payload compiler      |   |
|   +------------------+------------------+      +-----------------+------------------+   |
|                      |                                           |                      |
|                      +---------------------+---------------------+                      |
|                                            |                                            |
|                                            v (Pymem / Win32 API)                        |
|                     +---------------------------------------------+                     |
|                     |             hs_valuescanner.py              |                     |
|                     | - Main module PEB / Toolhelp discovery      |                     |
|                     | - Code cave allocator (VirtualAllocEx)      |                     |
|                     | - Thread suspender & RIP safety checker     |                     |
|                     +----------------------+----------------------+                     |
+--------------------------------------------|--------------------------------------------+
                                             |
                  +--------------------------+--------------------------+
                  |                                                     |
                  v (Direct Memory Injection)                           v (DLL Injection via Pymem)
+---------------------------------------------------+ +-----------------------------------+
|            GAME PROCESS: Hero_Siege.exe           | |      HSStatForgeDensity.dll       |
|                                                   | |                                   |
|  +---------------------------------------------+  | |  +-----------------------------+  |
|  | GameMaker YYC Named Functions:              |  | |  | MinHook Engine:             |  |
|  | - gml_Script_StatMagicFind                  |  | |  | - instance_create_depth     |  |
|  | - gml_Script_StatMovementSpeed              |  | |  | - instance_create_layer     |  |
|  | - gml_Script_StatAllSkills                  |  | |  +--------------+--------------+  |
|  | - gml_Script_EnemyCalculateExperience       |  | |                 |                 |
|  | - gml_Script_CalculateEndDamage             |  | |                 v                 |
|  | - gml_Script_StatAttackSpeed                |  | |  - Enemy_Creator_* filtering   |  |
|  | - gml_Script_StatFasterCastRate             |  | |  - Protected pool usage guard  |  |
|  | - gml_Script_StatSpellHaste                 |  | |    (stops creating if >200k)   |  |
|  | - gml_Script_StatDefense                    |  | |  - Burst rate limit:           |  |
|  | - gml_Script_StatCritRate / Damage          |  | |    <= 800 extra creators/sec   |  |
|  | - gml_Script_StatSpellCritRate / Damage     |  | |  - FREE_RValue cleanup         |  |
|  +----------------------+----------------------+  | |  - Shared Memory IPC (HSFD v3) |  |
|                         |                         | |  - Heartbeat watchdog          |  |
|                         v                         | +-----------------+-----------------+
|  [ Dynamic Code Caves / Rel32 JMP Epilogues ]     |                   |
|  - Validates full instruction boundaries          |                   |
|  - Replays original epilogue instructions         |                   |
|  - Scales RValue Array element[0] or scalar float |                   |
|  - Reentrancy / pointer dedup protection          |<------------------+
+---------------------------------------------------+
```

---

## Stat Resolvers & Hooking Mechanics

Season 10 uses 64-bit GameMaker YYC compilation. Character statistics are computed dynamically through internal script functions rather than static memory addresses. Stat Forge classifies hooks into distinct resolver categories:

### 1. `s10_array_result_multiplier`
- **Applied to:** Magic Find (`gml_Script_StatMagicFind`), Movement Speed (`gml_Script_StatMovementSpeed`), Defense (`gml_Script_StatDefense`), Critical Strike Chance/Damage (`gml_Script_StatCritRate`, `gml_Script_StatCritDamage`), Spell Critical Chance/Damage (`gml_Script_StatSpellCritRate`, `gml_Script_StatSpellCritDamage`).
- **Mechanism:** The function returns a GameMaker `RValue` of type `VALUE_ARRAY` (kind = 2). The dynamic array pointer at `rvalue + 0x00` contains an element pointer at `+0x08`. The double value consumed by gameplay is located in `element[0] + 0x00`.
- **Cave Logic:** Intercepts the epilogue before `RET`, verifies the array pointer, extracts `element[0]`, applies multiplier `value × factor` (or percentage bonus `value × (1 + bonus / 100)`), and writes the modified float back.
- **Deduplication:** The code cave tracks the last processed element pointer and scaled value to prevent repeated compounding over subsequent frames.

### 2. `s10_array_result_additive`
- **Applied to:** All Skills Bonus (`gml_Script_StatAllSkills`), Faster Cast Rate (`gml_Script_StatFasterCastRate`), Skill Haste (`gml_Script_StatSpellHaste`).
- **Mechanism:** Similar to `s10_array_result_multiplier`, but applies an additive delta (`value + bonus`) directly to `element[0]`. This allows gear bonuses, elemental skills, and buffs to continue stacking on top of the modified base.

### 3. `s10_scalar_result_multiplier`
- **Applied to:** EXP Multiplier (`gml_Script_EnemyCalculateExperience`), Total Damage Bonus (`gml_Script_CalculateEndDamage`), Attack Speed Bonus (`gml_Script_StatAttackSpeed`).
- **Mechanism:** Targets functions returning scalar double values. For EXP, the result pointer resides in `R15` at the epilogue (`49 8B C7 ...`). For Total Damage and Attack Speed, the epilogue is analyzed via Capstone disassembly, ensuring patch placement does not truncate multi-byte instructions or hook adjacent helper routines.

### 4. `s10_stat_return_proxy`
- **Applied to:** All Skills Exact (`gml_Script_StatAllSkills`).
- **Mechanism:** Replaces the entire function entry with a direct proxy cave that creates and returns a static RValue array containing the specified exact skill level.

### 5. `s10_standalone_density`
- **Applied to:** Monster Density Multiplier.
- **Mechanism:** Injects `HSStatForgeDensity.dll` into `Hero_Siege.exe`. MinHook intercepts GameMaker engine instance creation functions (`instance_create_depth` and `instance_create_layer`), identifies creator objects (`Enemy_Creator_*`), and safely spawns additional instances proportionally to the multiplier.

---

## Memory Safety, Configuration & Incompatibilities

### Mutually Exclusive Stat Hooks
`hs_statforge.py` enforces exclusive activation groups (`STAT_EXCLUSIVE_KEYS`):
- `all_skills` (Additive `+N`) and `all_skills_set` (Absolute Set Exact) both target `gml_Script_StatAllSkills`.
- Because the entry proxy returns immediately before the epilogue executes, activating both simultaneously would starve the additive hook. Stat Forge automatically disables one when the other is engaged.

### Configuration Migration & Pruning
- Current schema version: `CONFIG_VERSION = 9`, `CONFIG_SEASON = 10`.
- Legacy Season 9 bindings (`angelic_drop_rate`, `angelic_ss_drops`) targeted code paths removed in Season 10. `_clean_config_stats()` automatically prunes these retired routes upon loading to prevent restoring invalid memory targets.

### Code Cave & Thread Safety
- **Thread Suspension:** All target process threads are suspended via `OpenThread` and `SuspendThread` before writing patches, and RIP registers are inspected to ensure no thread is paused inside the target byte range.
- **Leak vs. Crash Tradeoff:** Executed code caves allocated via `VirtualAllocEx` are deliberately **not freed** during the game's execution lifecycle when disabling boosts. Freeing executable memory while another thread might transition into it risks immediate access violations; the OS reclaims the allocated virtual pages upon game exit.
- **Restoration:** Restoring boosts disables the hook flag inside the code cave first, allows a clean pass-through, and then restores the original instructions byte-for-byte.

### Native Density Safety Guards
- **Protected Variable Pool Guard:** Monitors GameMaker's internal variable pool. If utilized pool slots exceed `kProtectedPoolSafeUsed` (200,000 out of 262,144 capacity), extra creator spawning is halted immediately to prevent fatal engine crashes.
- **Rate Throttling:** Spawning is limited to a maximum of 800 extra creators per second (`kMaxExtraCreatorsPerSecond`) to smooth out map loading bursts.
- **RValue Cleanup:** Every extra creator invocation cleans up its returned RValue via GameMaker's `FREE_RValue` routine, preventing reference count accumulation during zone transitions.
- **Heartbeat Watchdog:** The host Python application writes a timestamp to `gState->hostHeartbeat` every second. If Stat Forge closes or crashes, the DLL detects the lost heartbeat, uninstalls all MinHook hooks, and unloads itself.

---

## Representative Change Workflow

To add a new stat boost or update an existing resolver:

1. **Research & Inspect Function Layout:**
   - Use `scripts/probe_named_result.py` or disassembly analysis to locate the target `gml_Script_Stat*` function in `Hero_Siege.exe`.
   - Inspect the epilogue instruction boundaries and determine whether the result is returned as a scalar double or an RValue array.
   - Record findings in `STATFORGE_S10_REAL_STAT_HOOK_NOTES.md`.

2. **Add Configuration Definition (`hs_statforge_stats.json`):**
   - Define the new stat entry with unique `key`, display `name`, `slider_max`, `button_color`, and `resolver` configuration (`kind`, `function_name`, `input_mode`).

3. **Update Resolver Logic (if introducing a new pattern):**
   - Update `hs_statforge.py` (`_resolve_named_result_stat_target`) or `hs_statforge_multiplier.py` to support the required instruction pattern and cave assembly template.
   - Ensure mutually exclusive keys are added to `STAT_EXCLUSIVE_KEYS` if the new stat conflicts with existing entry/epilogue hooks.

4. **Verify via In-Process and Contract Tests:**
   - Execute pure tests (`test_s10_multiplier.py`, `test_s10_adaptive_builds.py`).
   - Execute platform-native in-process tests (`test_native_result_hook_execution.py`).
   ```powershell
   python scripts/test_s10_multiplier.py
   python scripts/test_native_result_hook_execution.py
   ```

5. **Build Native DLL (if density logic modified):**
   ```powershell
   powershell -ExecutionPolicy Bypass -File native_density\build.ps1
   ```

6. **Package Distribution:**
   ```powershell
   pyinstaller HSStatForge.spec --noconfirm
   ```

---

## Prerequisites & Environment Setup

### Required Tools & Runtimes
- **OS:** Windows 10/11 x64.
- **Python:** Python 3.10+ 64-bit (Tkinter included in standard Windows installer).
- **C++ Compiler:** Visual Studio 2022 (MSVC v143 toolset) with C++20 support (`vcvars64.bat`).
- **Python Packages:**
  - `pymem`: Process memory inspection and DLL injection.
  - `capstone`: Disassembly framework for epilogue validation.
  - `pyinstaller`: Binary packaging and resource bundling.

### Dependency Installation
```powershell
# In hs-stat-forge directory:
python -m pip install pymem capstone pyinstaller
```

---

## Command Reference

All commands must be executed from the `hs-stat-forge/` submodule directory using PowerShell unless otherwise specified.

### Application Launch & Build Commands

| Command | Platform / Shell | Working Dir | Prerequisites | Expected Result | Side Effects | Status |
|---|---|---|---|---|---|---|
| `python hs_statforge.py` | Windows / PowerShell | `hs-stat-forge` | Python 3.10+, `pymem` | Launches Tkinter UI desktop panel | Creates `hs_statforge.log` in working directory | Verified |
| `powershell -ExecutionPolicy Bypass -File native_density\build.ps1` | Windows / PowerShell | `hs-stat-forge` | VS 2022 C++ Tools (`vcvars64.bat`) | Compiles `native_density/bin/HSStatForgeDensity.dll` | Generates object files in `native_density/build/` | Verified |
| `pyinstaller HSStatForge.spec --noconfirm` | Windows / PowerShell | `hs-stat-forge` | `pyinstaller`, compiled `HSStatForgeDensity.dll` | Produces standalone executable `dist/HSStatForge.exe` | Creates `build/` and `dist/` directories | Verified |

### Test Suite: Pure In-Memory / Contract Tests
These tests run in standard Python environments without Windows API hooking or process attachment:

| Command | Platform / Shell | Working Dir | Prerequisites | Expected Result | Side Effects | Status |
|---|---|---|---|---|---|---|
| `python scripts/test_s10_multiplier.py` | Cross-Platform / Python | `hs-stat-forge` | `capstone` | Validates multiplier harnesses and Capstone instruction decoding | None (pure in-memory) | Verified |
| `python scripts/test_s10_adaptive_builds.py` | Cross-Platform / Python | `hs-stat-forge` | Python standard library | Validates Season 10 YYC adaptive pattern matchers on test buffers | None (pure in-memory) | Verified |
| `python scripts/test_total_damage_epilogue_resolver.py` | Cross-Platform / Python | `hs-stat-forge` | `capstone` | Tests Total Damage epilogue resolution and helper code rejection | None (pure in-memory) | Verified |

### Test Suite: Platform-Native In-Process Tests
These tests use `pymem` and `ctypes` on Windows to allocate and execute code caves **strictly within the running Python process itself (`os.getpid()`)**:

| Command | Platform / Shell | Working Dir | Prerequisites | Expected Result | Side Effects | Status |
|---|---|---|---|---|---|---|
| `python scripts/test_dynamic_proxy_allocation.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Verifies dynamic proxy allocation and memory protection in local process | Allocates virtual memory in Python PID | Verified |
| `python scripts/test_module_resolution_fallback.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Verifies native PEB fallback when Toolhelp/Pymem module lookup fails | Reads local process PEB | Verified |
| `python scripts/test_native_result_hook_execution.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Executes native result hook assembly cave inside local Python process | Executes JIT code cave in Python PID | Verified |
| `python scripts/test_native_result_epilogue_hook_execution.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Tests epilogue multiplier hook execution in local Python process | Executes JIT code cave in Python PID | Verified |
| `python scripts/test_set_variable_hook_execution.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Tests variable injection hook execution in local Python process | Executes JIT code cave in Python PID | Verified |
| `python scripts/test_extended_result_hook_execution.py` | Windows x64 / Python | `hs-stat-forge` | `pymem` | Tests extended result hook execution in local Python process | Executes JIT code cave in Python PID | Verified |

### Test Suite: Live-Game Verification & Research Probes
> ⚠️ **CRITICAL WARNING:** The following scripts require a running, offline instance of `Hero_Siege.exe` (without EAC). They attach to live process memory and must **never** be executed during automated static test runs or when connected to multiplayer.

| Command | Platform / Shell | Working Dir | Prerequisites | Expected Result | Side Effects | Status |
|---|---|---|---|---|---|---|
| `python scripts/test_density_runtime.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe`, `HSStatForgeDensity.dll` | Injects DLL and validates density shared state IPC | Modifies live game density state | Live-Process Only |
| `python scripts/verify_extended_result_resolvers.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Verifies all configured stat resolvers against live GameMaker functions | Inspects live process memory | Live-Process Only |
| `python scripts/probe_named_result.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Resolves GameMaker named script function RVAs | Reads live process memory | Live-Process Only |
| `python scripts/probe_density_runtime.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Scans GameMaker object tables for `Enemy_Creator_*` definitions | Reads live process memory | Live-Process Only |
| `python scripts/probe_attack_speed_root.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Inspects live `StatAttackSpeed` double return values | Reads live process memory | Live-Process Only |
| `python scripts/probe_extended_result_calls.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Validates live execution call counts for extended stat hooks | Injects temporary counters | Live-Process Only |
| `python scripts/probe_live_multiplier.py` | Windows x64 / Python | `hs-stat-forge` | Live `Hero_Siege.exe` | Exercises live multiplier hook application and teardown | Modifies live process memory | Live-Process Only |

---

## Packaging & Distribution

### PyInstaller Build Specification (`HSStatForge.spec`)
- **Target:** `HSStatForge.exe` single-file windowed application.
- **Embedded Binaries:** `native_density/bin/HSStatForgeDensity.dll` bundled into binary root (`.`).
- **Hidden Imports:** Dynamically collects all submodules of `pymem` via `collect_submodules("pymem")`.
- **UPX Compression:** Enabled (`upx=True`), explicitly excluding `HSStatForgeDensity.dll` (`upx_exclude=["HSStatForgeDensity.dll"]`) to avoid breaking native DLL exports and relocations.
- **UAC Privilege:** Configured with `uac_admin=True` to automatically request administrator elevation required for Windows memory access APIs (`OpenProcess`, `VirtualAllocEx`).

### Resource Resolution Model
Resource path discovery in `runtime_resource_path()` follows a three-tier hierarchy:
1. `sys._MEIPASS`: PyInstaller temporary extraction folder when running as a frozen binary.
2. `runtime_app_dir()`: Local directory containing the script/executable.
3. `native_density/bin/`: Relative development binary staging path.

---

## Safety, Provenance & Licensing

### Offline Safety Policy
- **Offline / Single-Player Only:** Stat Forge must only be used in offline single-player mode with Easy Anti-Cheat disabled.
- **Zero On-Disk Modifications:** `Hero_Siege.exe` and local character save files are never modified on disk. All modifications reside in volatile runtime process memory.
- **Anti-Corruption Verification:** Every target hook site undergoes strict byte-pattern matching and instruction length verification before injection. Ambiguous or unknown layouts are rejected with fail-closed safety.

### Third-Party Dependencies & Licensing
- **MinHook:** Used for C++ API interception in `native_density/`. Licensed under the 3-Clause BSD License (Copyright © 2009–2017 Tsuda Kageyu).
- **Pymem:** Python memory manipulation library.
- **Capstone:** Disassembly framework for x86/x64 instruction validation.

---

## Maintenance Triggers & Known Gaps

- **Game Updates / Patches:** If Hero Siege updates its executable, function RVAs and compiler-generated epilogue structures in GameMaker YYC output may shift. Stat Forge's runtime symbol resolver automatically accommodates layout shifts, but fundamental changes to RValue array structures (e.g., changes to `VALUE_ARRAY` layout) require updating `STATFORGE_S10_REAL_STAT_HOOK_NOTES.md` and resolver definitions.
- **New Stat Additions:** When adding new character statistics, developers must confirm whether the value is calculated as a scalar double or an RValue array using `scripts/probe_named_result.py` before authoring a hook.
- **CI / Build Automation:** No automated remote CI pipeline exists. Local execution of `scripts/test_s10_multiplier.py` and `native_density/build.ps1` must be performed prior to tagging releases.
