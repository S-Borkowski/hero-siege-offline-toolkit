# Agent Guidelines

## Submodule & Directory Development Instructions

When developing, modifying, testing, or investigating code within any submodule or specific directory (e.g., `HSCraftSim/`, `ForgePact/`, `HS-Offline-Tracker/`, etc.), always consult and follow the corresponding development instructions guide:
- Check the centralized index at [`docs/submodules/README.md`](docs/submodules/README.md) for available module guides.
- Check for submodule-specific development guides located at `docs/submodules/<submodule-name>/instructions.md` (or `<submodule-name>/instructions.md` if present within the directory).
- Adhere to the documented architecture, entry points, workflows, testing procedures, dependencies, and command conventions outlined in the relevant `instructions.md`.

## Legal: Decompiled Output Never Reaches Any Origin

This toolkit reverse-engineers Hero Siege's runtime (memory layout, hooked functions,
GameMaker object/script indices) to build offline tools. **The constraint is entirely
about output, not technique**: decompiling or disassembling the game locally — reading
a script body in Ghidra/IDA/UndertaleModTool/dnSpy to understand what a mechanism does —
is not restricted and is a legitimate research step when static name/hierarchy search
and live measurement (hooking, tracing, before/after diffing) run out, same as the rest
of this toolkit's reverse-engineering. What must never happen is for that output to
land in a repository that gets pushed to its origin (this includes every submodule's
own remote, not just this hub) — see the hard rule below.

**Hard rule: never commit, paste, or embed decompiled or disassembled Hero Siege
source (GML script bodies, decompiled bytecode, IDA/Ghidra/UndertaleModTool
listings or exports, disassembly dumps) into any tracked file.** This applies to
production code, `docs/` research notes, commit messages, and comments alike — write
up *what was learned*, in your own words, never the decompiled text itself.

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

**This was not respected closely enough during Pet Quest Collector's Phase 0
research (2026-09-10)**: candidate interaction hooks were added and tested
one small batch at a time - a named script, then five more named scripts,
then seven anonymous closures on one object, then two builtins, then nine
more anonymous closures on a second object - each round costing its own
rebuild, DLL swap, full game relaunch, and a live collect from the tester.
Several of those rounds could have been one round: `hs-game-sdk`'s static
name/hierarchy search (`grep` over `scripts.hpp`/`objects.hpp`, or the
Python/C++ bindings) can enumerate *every* plausibly-relevant script or
object *before* touching the game at all, and costs nothing to run
repeatedly. When a live research session's goal is "find which of several
unknown candidates does X" (not "verify one already-suspected mechanism"):
- Exhaust the static search first: every name matching the concept (by
  substring, by shared object/parent, by shared event) across
  `scripts.hpp`/`objects.hpp`, not just the one name the plan or a prior
  guess assumed. Read `hs-game-sdk`'s existing research docs
  (`ForgePact/docs/*-research.md`) for the technique already proven there -
  e.g. "every script-table entry inside `<object>`'s own Create event" found
  every anonymous closure GameMaker split out of that object, cheaply, with
  no live session.
- Hook every candidate that search turns up in the *same* build, gated
  together behind one research command, before asking for a single relaunch.
  A hook that turns out irrelevant costs one `HookOneScript`/`HookBuiltin`
  call and a few log lines - far cheaper than a round trip that could have
  included it.
- Only fall back to a narrower, more expensive technique (e.g. hooking hot
  builtins instead of named scripts) after the broad static-search round has
  been exhausted and come back empty, and even then, hook every plausible
  builtin candidate at once rather than one per relaunch.

## Prove the Instrument Before Trusting a Negative Result

The batching advice above is necessary but was not sufficient, and the reason
is worth its own rule. The same Pet Quest Collector research went on to spend
several more sessions on a *false negative*: 34 hooked call sites reporting
**0 calls** across multiple confirmed, observed collects. The conclusion drawn
- "the game does not call any of these" - was wrong. `HookOneScript` installs
by swapping a pointer inside the script-table entry, and this game's compiled
GML calls another script with a direct `call rel32` bound at compile time,
which never reads that table. **Every one of those zeros measured the
instrument, not the game** (`ForgePact/docs/pet-quest-collector-c-research.md`,
"The hooks were blind"). Two whole mechanisms were abandoned on that evidence.

So, before a "0 calls" / "no effect" / "never fires" result is allowed to
close a line of investigation:
- **Run a positive control through the same instrument.** Point it at
  something you already know fires - in this case any hook that had ever
  logged a call - in the same build, in the same session. An instrument that
  cannot produce a non-zero anywhere has told you nothing about your target.
- **Know how your instrument attaches, and whether the code under test can
  reach it.** Script-table swaps only see calls routed through the table;
  address-patching hooks (`MmCreateHook`) see the call itself. Prefer the
  latter whenever a table-based hook reports zero, before concluding anything
  about the game.

**The same blindness is a shipping bug, not only a research one.** A
table-only hook prints "HOOK INSTALLED" and then silently changes nothing on
the paths compiled GML actually uses - a feature that reports armed and does
nothing. Origin's review of ForgePact PR #2 found direct native callers for
`StatMovementSpeed`, `StatAttackSpeed`, `DropRelic`, `DropMonsterGold` and
`DropGold`, so stat scaling, drop multipliers and the max-level relic filter
were all in that state. `ForgePact`'s `HookOneScript` therefore installs
**both** - the table swap and an inline detour at the function's own address -
and hands the hook body the trampoline. `HookOneScriptTable` still exists for
exactly one purpose: `citrace nativetrace` needs a deliberately table-only
hook to compare against, and that comparison is what proved the problem.

The general rule: **put the interception in the installer, not in whichever
call sites a review happened to verify.** Fixing the five named functions
would have left every other gameplay hook, and every future one, blind.
- **Write the negative down as "not observed", not "does not happen"**, until
  a control backs it. Research docs in this repo are read later as settled
  fact; a mislabeled negative costs more sessions than the one that produced
  it.

## Never Call an Address You Resolved by Hand

Reading a function's address in Ghidra is a legitimate research step (see the
legal section above). **Shipping that address as a constant is not.** A game
build's RVAs are not an interface: the next recompile moves every one of them,
and the constant then names whatever bytes happen to sit at that offset. A
*read* through a stale address returns garbage; a *call* through one transfers
control into arbitrary code on a player's machine.

This toolkit has now hit that defect twice:

- **`relicgate`** used a fixed RVA inside `DropItem`. On the current build that
  address is not inside `DropItem` at all. The feature had been silently dead
  for an unknown number of releases; it is now a no-op that says so
  (`SetRelicGate`).
- **Pet Quest Collector** shipped `PetQuestCollectOne` calling
  `GetModuleHandleA(nullptr) + 0xB489070` - the runtime's call-a-method-value
  dispatcher, measured live and correct for exactly that build - validating
  neither the module, nor the bytes, nor the build. It survived one release
  before review caught it. Removing it took three attempts, and the second one
  (reading the callable off the value's own `CScriptRef`) failed for the same
  underlying reason as the first, which is why the struct bullet below exists.
  What works is `script_execute` through `CallBuiltinEx` - name-resolved,
  layout-free, confirmed live by the quest counter advancing.

So, before a pointer is called or dereferenced in code that reaches a player:

- **Resolve by name.** `GetNamedRoutinePointer` / `HookOneScript` /
  `HookBuiltin` for scripts and builtins, `asset_get_index` for assets,
  `CallBuiltin` for anything the runtime exposes. This is the default and it
  covers nearly everything.
- **Otherwise let the runtime do the work, still by name.** `CallBuiltinEx`
  supplies `self` and `other` to any builtin, so the runtime's own dispatcher
  can be handed a value whose internals you never inspect. This is how the Pet
  Quest Collector ends up invoking an anonymous method value
  (`script_execute`, `self` = the item, `other` = `Loot_Manager_obj`): no
  address, and no struct layout either.
- **Only then resolve off a runtime struct YYToolkit defines** - `CScriptRef`,
  `CScript`, `CInstance`, `RValue` - and treat that as an assumption to be
  measured, not a fact. **A struct layout is the quiet version of a hardcoded
  address.** Both are "a layout someone wrote down"; the address fails loudly
  and the field silently returns a plausible zero. This is not hypothetical:
  the fix for the Pet Quest Collector's hardcoded address was *itself* a
  `CScriptRef` read, and it shipped broken, because on this game's runner
  `m_Questpickup` is not a `CScriptRef` at all (`m_ObjectKind = 0`, both
  callable fields zero, `method_get_index` returns nothing - while a sibling
  variable on the same instance resolves fine). If you do read a struct,
  **find a positive control on the same target first**: something the plugin
  already proves works on this runtime, whose value you can compare against.
- **A measured address is a research finding, not an implementation.** Keep it
  in `docs/` and behind `#ifndef FORGEPACT_RELEASE`, where a wrong value costs
  a session. `citrace collect`'s `native` path is the pattern: the old shape
  stays runnable as an A/B check against the shipped one, and nothing else
  uses it.
- **If an address genuinely cannot be avoided, validate it and refuse.**
  Confirm the target is committed, executable, and inside the intended
  module's image (`AddrIsExecutableInModule` - `VirtualQuery`,
  `AllocationBase`, `PAGE_EXECUTE*`) before calling, and on failure disable
  the feature with a message the way `SetRelicGate` does. Never fall through
  to the call. Count the refusal so it surfaces in a `stat` command instead
  of as silence.

`ForgePact/tests/test_release_hook_contract.py`'s
`test_player_binary_calls_no_hand_resolved_game_address` enforces this
mechanically: it strips the research blocks and fails if anything left can
reach a `*Rva*` constant or computes a call target from a module base plus a
literal. Extend that test rather than working around it.

A corollary, learned from the same investigation: when a call shape is
rejected, record *what was supplied* alongside the result. Nine name-resolved
invoke shapes were written off as "measured negative" before a later round
established that the callee wanted one argument and a specific `self`, neither
of which those nine had passed - the notes say so themselves
(`ForgePact/docs/pet-quest-collector-c-research.md`, "This explains every C0.2
access violation"). **One of those nine is now the shipped mechanism.** Three
rounds went into inventing call machinery while the working answer sat in a
table of already-implemented paths, mislabelled. That is the "not observed" vs
"does not happen" distinction from the section above, applied to call
signatures - and the cost of getting it wrong is not one wasted session, it is
every session that trusts the label afterwards.

Finally, make a refusal say why. `InvokeMethodValue` logs one line naming the
field that failed, and `petquest 0` reports which route actually ran plus a
`dispatched-but-item-remained` counter that separates "nothing ran" from "ran
and did nothing". Those two outputs turned a hypothesis that would have cost a
research session into two launches. A mod that fails silently is a mod nobody
can debug from a bug report.

## Don't Suspend the Game's Own Runtime

Every mod in this toolkit that works does one of two things: it **reads** game
state, or it **changes one value inside a call the game is already making** — a
`droprate.base` divided before the game's own die is rolled, a base speed scaled
for the duration of one `PathFindStartPath`, a marker object multiplied so the
game places and runs the mechanic itself. The game stays in charge of its own
loop throughout.

**Features that take the loop away from the game are not recommended**: pause,
time scaling or slow-motion, save-state/rewind, forced state restore, freezing
or wholesale deactivating instances — anything whose contract is "suspend the
world and give it back unchanged". They are a different risk class, and the
reasons are structural rather than a matter of implementation quality:

- **The failure mode inverts.** Ordinary mods fail by doing nothing (a dead
  `relicgate`, a collect that never fires). A suspension feature fails by
  leaving the player's session stuck, or by letting the game save while its own
  state is half-removed. When a design needs a panic hotkey, a watchdog and a
  fail-open path on every branch before it can ship, that machinery is the
  signal, not the mitigation.
- **The precise instruments are unavailable on this build.** YYToolkit's
  per-event hook (`EVENT_OBJECT_CALL`) is deliberately disabled in the
  YYToolkit this project ships — it crash-looped on Season 10 — and
  named-script hooks are structurally blind against this YYC build's direct
  calls (see the section above). What remains is blunt, whole-subtree
  instance deactivation, with the widest possible blast radius.
- **The claim cannot be verified.** "Everything stops" is a statement about
  every timer, DoT, cooldown and internal counter in the game, including the
  ones nobody has enumerated. Contract tests can pin the mod's own structure;
  they cannot establish that. A miss surfaces as a buff that quietly expired or
  a cooldown that quietly advanced — wrongness a player reports months later as
  "the mod broke my character".
- **It taxes every future game patch.** Anything that has to know the game's
  full object or UI surface (which windows count as a menu, which objects are
  actors) is upkeep on someone else's release schedule.

The worked example is `ForgePact/docs/menu-pause-plan.md` — a complete design
for "pause the world while a menu is open", researched to the point where the
mechanism was clear, and **not recommended for implementation** for exactly the
reasons above. Read its §0 before proposing anything in this class; if one is
built anyway, that is a deliberate decision to accept those risks, and it
belongs in the submodule's Known Limitations with the acceptance recorded.

Prefer the alternatives: read-only tooling outside the game, a change to one
value the game is about to use, or leaving the behaviour alone.

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
