# Agent Guidelines

## Submodule & Directory Development Instructions

When developing, modifying, testing, or investigating code within any submodule or specific directory (e.g., `HSCraftSim/`, `ForgePact/`, `HS-Offline-Tracker/`, etc.), always consult and follow the corresponding development instructions guide:
- Check the centralized index at [`docs/submodules/README.md`](docs/submodules/README.md) for available module guides.
- Check for submodule-specific development guides located at `docs/submodules/<submodule-name>/instructions.md` (or `<submodule-name>/instructions.md` if present within the directory).
- Adhere to the documented architecture, entry points, workflows, testing procedures, dependencies, and command conventions outlined in the relevant `instructions.md`.

## Legal: No Decompiled Code in Any Origin

This toolkit reverse-engineers Hero Siege's runtime (memory layout, hooked functions,
GameMaker object/script indices) to build offline tools. That research must never
turn into copied source code landing in a repository that gets pushed to its origin
(this includes every submodule's own remote, not just this hub).

**Hard rule: never commit, paste, or embed decompiled or disassembled Hero Siege
source (GML script bodies, decompiled bytecode, IDA/Ghidra/UndertaleModTool
listings or exports, disassembly dumps) into any tracked file.** This applies to
production code, `docs/` research notes, commit messages, and comments alike.

What is fine to commit — because it documents *interoperability facts*, not the
game's copyrightable expression:
- Object/script/room/sprite/sound **names and their numeric indices** (e.g. the
  `hs-game-sdk` tables, or a hook installed by name via `HookOneScript`).
- **Measured runtime behavior**: what a function does when called, what it reads
  or writes, observed crashes and their signatures, before/after values.
- **Our own** C++/Python/TypeScript code that reacts to that behavior (hooks,
  panels, SDK bindings) — original work, not derived from the game's source text.
- Function/struct *offsets and calling conventions* needed to hook or read memory.

What must stay local and out of version control:
- Full or partial decompiled/disassembled script bodies, however they were
  produced (Ghidra, IDA, dnSpy, UndertaleModTool, manual transcription).
- Exported decompiler project files or listings (e.g. `.gpr`, `.i64`, `.idb`,
  UndertaleModTool `Export/` dumps) — kept on the researcher's machine only.
- Screenshots or copy-pasted excerpts of the game's own script source in issues,
  PRs, or docs.

If a research note needs to explain *why* something behaves a certain way,
paraphrase the mechanism ("the door script rolls the same die as case 11/31/40")
rather than quoting the script. `ForgePact/docs/*.md` is the existing example to
follow — behavior and our own hook code only, no game script text, and that
practice is what keeps the AGPL-3.0 "original work" claim in `ForgePact/CREDITS.md`
true. `.gitignore` also excludes common decompiler artifact paths as a mechanical
backstop; extend it rather than working around it if a new tool produces a new
artifact type.

## Mod Development Workflow: Test Before / After, Then Build to It

When developing a mod, hook, or any gameplay-affecting change (drop rates, stats,
spawns, combat, crafting, etc.), don't start by editing the hook and checking it
live in-game. Establish both ends of the behavior change as tests first, then
write the mod to close the gap:

1. **Baseline test** — capture how the game behaves *without* the mod (vanilla
   read, unmodified value, default probability/roll). This pins down the exact
   starting point so a regression in "no mod applied" behavior is caught too.
2. **Target test** — capture how it *should* behave once the mod is applied
   (the new value, rate, or code path the mod is meant to produce).
3. **Implement the mod** to turn the target test green without breaking the
   baseline test's assumptions about the unmodified path (e.g. a toggle/config
   that's off by default should still reproduce baseline behavior).

Place these alongside existing coverage — see `tests/` at the repo root for the
`hs-game-sdk` test style, or a submodule's own test directory per its
`instructions.md`. Where a real game process is required to observe the
behavior, prefer a fixture or mock built on `hs-game-sdk` structs over a live
game session for the baseline/target tests, and reserve actual in-game runs for
final verification.

## Limit Rebuilds & Reruns During Development

Full recompiles and relaunching the game for every change are slow and make the
edit-verify loop expensive. Before or alongside mod development:
- Look for an existing tool that lets you exercise the change without a full
  rebuild/relaunch (e.g. a standalone test harness, a script that replays
  captured game state, incremental/unity builds, hot-reloadable hook code).
- If nothing suitable exists for the submodule you're working in, build one
  (or extend `tools/`) and document it in that submodule's `instructions.md`
  so the shortcut is reusable rather than one-off.
- `tools/freeze_probe.ps1` is an example of this pattern applied to diagnostics
  (observe the running game from outside it instead of adding print-and-relaunch
  instrumentation); prefer the same "build a tool once, reuse it" approach for
  mod iteration.
- Reserve full rebuild + in-game relaunch cycles for final confirmation once
  the baseline/target tests above already pass against the faster loop.

## Documentation & Instructions Maintenance

Upon completing any task or making changes to features, workflows, architecture, or dependencies:
- Update documentation, instructions (such as submodule `instructions.md` files), and `README.md` files when and where relevant to reflect the changes.
- Ensure any new guides, updated links, or modified commands remain accurate and in sync across project and submodule documentation.

## YYToolkit Integration

When a prompt or task requires the use of `yytoolkit`, attempt to retrieve `yytoolkit` documentation and references from the `context7` MCP server if it is available.

## HS Game SDK Usage

When developing, modifying, testing, or reverse-engineering game logic, hooks, drops, and items across any submodules:
- Use `hs-game-sdk` (`hs-game-sdk/`) as the central source of truth for GameMaker object indices, script names, room indices, sprite indices, sound indices, stat IDs, proc bundles, and runtime item/stat structs.
- In **C++** plugins (`ForgePact/plugin`, `HS-Offline-Tracker/aurie-producer`, `hs-stat-forge`), `#include <hs_game_sdk/hs_game_sdk.hpp>` and use strongly-typed definitions from namespace `HeroSiege` (such as `HeroSiege::Objects::GameObject`, `HeroSiege::Scripts::gml_Script_*`, `HeroSiege::Stats::StatId`, and `HeroSiege::YYTK`).
- In **Python** submodules (`ForgePact/src`, `hero-siege-item-editor`, `HSSaveEditor`, `HS-Offline-Launcher`), import models and constants from `hs_game_sdk` (e.g. `from hs_game_sdk import GameObject, GameScript, StatId, PROC_FAMILIES, ItemDefinitionStruct, ItemStatStruct`).
- In **TypeScript / Web** submodules (`HSCraftSim`, `HS-Offline-Tracker/src`), import from `@hero-siege/sdk`.
- Avoid declaring raw string literals or magic numbers for game scripts, asset indices, object types, and stat keys when equivalent constants exist in `hs-game-sdk`.
- If game updates shift asset or script indices, regenerate the SDK bindings using `tools/extract_and_generate_sdk.py`.
