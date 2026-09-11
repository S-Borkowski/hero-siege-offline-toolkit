# Toolkit Hub — design

One window that installs, launches and updates the ten tools in the Hero Siege
Offline Toolkit. Tauri 2 + Svelte 5, in [`hub/`](../../hub).

The reasoning behind the decisions here is in
[`docs/toolkit-hub-plan.md`](../toolkit-hub-plan.md); the topology question has
its own record in [`docs/adr/0001-repo-topology.md`](../adr/0001-repo-topology.md).
This document describes what was built.

---

## Shape

```
falorfrozen-cmd/<tool>  ──release──►  GitHub Releases (unchanged, ten repos)
         │ repository_dispatch: release-published
         ▼
  .github/workflows/catalog.yml → tools/build_catalog.py
         │        catalog.json + catalog.json.minisig, on the `catalog` release tag
         ▼
  Toolkit Hub (hub/)
    ├─ catalog.rs   fetch → minisign-verify → cache; embedded copy as the floor
    ├─ verify.rs    SHA-256 over artifacts, minisign over the catalog
    ├─ install.rs   download → hash → extract → versioned dir → activate → roll back
    ├─ launch.rs    spawn (elevated when declared) → health probe → track the PID
    ├─ game.rs      is Hero Siege up, is EAC up
    ├─ state.rs     installed set, settings, staged installs
    └─ lib.rs       the Tauri commands, and the worker threads they start
```

## It is a library, not a browser

Five of the ten tools are hardened loopback web applications that refuse to be
framed — `hero-siege-item-editor/hs_item_editor_gui.py:7808` sets
`frame-ancestors 'none'` and `X-Frame-Options: DENY`, and
`HS-Offline-Launcher/src/hs_offline_launcher.py:846` does the same plus an HMAC
API token bound per process. Two more (`HSSaveEditor`, `hs-stat-forge`) are
Tkinter and cannot be embedded in a webview at all.

A proxy in front of them would break the `Host`/`Origin` checks and the
per-process token — deliberate anti-DNS-rebinding defences dismantled to get a
cosmetic tab bar. So the hub owns processes and windows, not documents. Uniform
chrome across heterogeneous tools is not achievable and is not attempted.

---

## What is on disk

```
%LOCALAPPDATA%\Hero Siege Toolkit\
  tools\<id>\<version>\      extracted artifact
  tools\<id>\current.json    which version is live, and what to roll back to
  cache\downloads\           hash-verified before anything uses them
  cache\catalog.json         the last catalog that verified, with its signature
  state.json                 installed set, settings, last check
  logs\hub.log
  bundle\                    optional offline-bundle payload, consulted first
```

**Program files only.** `%LOCALAPPDATA%\Hero_Siege\forgepact.json`,
`%LOCALAPPDATA%\HSCraftSim\session.json`, `%LOCALAPPDATA%\HS Offline
Tracker\events.ndjson` and every tool's saves and backups stay exactly where the
tool puts them. Uninstalling the hub never costs a player their settings, and a
tool started outside the hub behaves identically.

`current.json` lives beside the version directories it arbitrates between,
separately from `state.json`, so a hub whose state file is lost can still tell
which of two version directories is live.

---

## Installing

1. **Download** into `cache\downloads\<name>.part`, streaming, with progress.
2. **Verify** against the SHA-256 the catalog pins. A mismatch deletes the file
   and stops — leaving it invites a later run to find it, skip the download and
   trust it.
3. **Extract** into `tools\<id>\.staging-<version>`, stripping the archive's
   wrapping directory. Entries that would write outside the destination are
   refused.
4. **Check the entry point exists** in what was extracted. A tool that installs
   but cannot launch is worse than one that refuses to install.
5. **Write a manifest** of every file's hash into the staging tree, so Verify
   can answer offline later.
6. **Activate** by renaming staging into `tools\<id>\<version>`, then updating
   `current.json`.

The rename is the commit point. An install interrupted anywhere before it leaves
rubbish in a staging directory and a working installation untouched — and the
next attempt clears that staging directory rather than building on it.

Only the live version and the one behind it are kept. Rollback is then a pointer
change rather than a re-download, and rolling back twice goes forward again.

### The verification chain

None of the ten tools is code-signed. SmartScreen will warn about every one of
them and Authenticode has nothing to say. What stands in for it:

```
embedded public key  (compiled into the hub; rotating it is a hub release)
        │ verifies
catalog.json.minisig ──covers── catalog.json
                                    │ pins
                              sha256 per artifact
                                    │ checked before
                              anything is extracted
```

A release asset swapped after the catalog was built fails at the hash check. A
catalog edited after it was signed fails at the signature. The signature check
covers **both** of minisign's signatures — the one over the content and the one
over `signature || trusted comment` — so the trusted comment shown in About has
actually been signed.

---

## Launching

Each tool is spawned as its own process, with the working directory set to its
install directory: several resolve data relative to it (ForgePact looks for
`modfiles/` next to the executable), so that is load-bearing.

**Elevation.** `hs-stat-forge`, `HS-ValueEditor` and `Hs-Offline-Loot-Forge` need
`SeDebugPrivilege`. The hub itself runs non-elevated and elevates per launch
through `ShellExecuteExW` with the `runas` verb — `CreateProcess` cannot elevate,
and a non-elevated parent asking for an elevated child gets
`ERROR_ELEVATION_REQUIRED`. `SEE_MASK_NOCLOSEPROCESS` is what makes
`ShellExecuteExW` hand back a process handle, so an elevated tool can still show
as *Running* and still be stopped. A declined UAC prompt (`ERROR_CANCELLED`,
1223) is reported as a decision, not a failure.

**Health.** The hub reuses each tool's existing endpoint rather than inventing a
protocol that would need a commit to all ten repositories:
`hero-siege-item-editor` answers `/api/instance` with `{version, pid, port}`,
`HSCraftSim` answers `/_health` with `{application, version}`. HS Offline
Launcher gets `any_status`, because its per-process HMAC token answers an
unauthenticated probe with 401 — and "something is listening" is the only
question being asked. Tools with no HTTP surface fall back to "the process is
still alive", which is all a Tkinter app can offer.

**Running outside the hub.** If one of a tool's declared ports is already
answering, the card says *Running (outside the hub)* rather than offering Launch
and then failing on the tool's own single-instance lock.

---

## Updating

Policy: check on launch and on a button; a setting for auto-download; and, nested
under it, auto-install. `Settings::effective_auto_install()` enforces the nesting
in Rust as well as in the UI, so a hand-edited `state.json` cannot get past it.

### The two interlocks

Auto-install only. Both produce a *staged* install — downloaded, verified, and
waiting — with the reason shown on the Updates screen.

- **Never over a running tool.**
- **Never while `Hero_Siege.exe` is running.** ForgePact patches the game's PE
  and holds file IPC through `bp_ipc/cmd.txt`; HSSaveEditor documents a
  game-closed requirement. Writing over either mid-session is how a player loses
  a character.

Staged installs are applied when the blocking condition clears: on Stop, and at
startup before the network is touched — a pending install the user already agreed
to should not wait on a check succeeding.

### Work offline

A master switch that disables every outbound request including the launch check.
`HS-Offline-Tracker/src/About.svelte:6` states the project's value plainly: the
check is "never something the app does on its own... the only request the app
ever makes". A hub whose *job* is distribution cannot keep that literally, but it
keeps the part that matters — nothing is contacted before the first-run screen is
answered, and Work offline is one click away on that screen.

`Settings::may_reach_network()` is `!work_offline && first_run_done`, and every
command that would fetch checks it.

### The hub itself

Tauri's updater plugin, with `download()` and `install()` kept apart so the
auto-download and auto-install settings map onto it directly. `latest.json` is
published by `tauri-action` with `includeUpdaterJson: true` and signed with
`TAURI_SIGNING_PRIVATE_KEY`; the matching public key is in `tauri.conf.json`.

---

## The interface

1180×760, `decorations: false`, custom title bar. `theme.css` is copied verbatim
from HS-Offline-Tracker and the three skins (`obsidian`, `ember`, `void`) come
with it.

| Screen | What it is for |
| --- | --- |
| Library | The grid. Card per tool: name, one line, state chip, one primary button, overflow menu. |
| Updates | Everything with a newer release, *Update all*, and the staged queue with its reasons. |
| Game | Whether Hero Siege and EAC are running — so the interlocks are legible rather than mysterious — and a way to start the game through HS Offline Launcher. |
| Settings | Work offline, check on launch, auto-download, auto-install (nested), skin, developer mode. |
| About | Versions, the catalog's signature, the log, and the hub's own updater. |
| Downloads drawer | Per-file progress with *Verifying* as a step of its own. |
| First run | What the hub will contact, before it contacts it. |

### Two things about the sprites

`skin.svelte.js` is HS-Offline-Tracker's, with two deliberate differences and
two bugs fixed that are worth knowing about because both fail *silently*:

- The Tracker imports two PNGs for its backdrop and app mark; the hub draws both
  as SVG. The root `.gitignore` bans `*.png` (it exists to keep extracted game
  sprites out of the repository), and a hub whose art is entirely generated needs
  no exception to it. The six icon squares Tauri demands are the only PNGs, and
  `hub/.gitignore` re-includes exactly those.
- `encodeURIComponent` does not encode `(` or `)`, and every sprite refers to its
  own gradient as `url(#p)`. Unquoted inside a CSS `url(...)`, those parentheses
  close the token early and the sprite does not paint — while working perfectly
  in an `<img src>`, which is what makes it easy to miss.
- These are nine-slice marks. Painted as `background-image` at
  `background-size: 100% 100%` they stretch, so a 620px panel gets a 100px corner
  radius and its gold corner ticks become bars. `skin.css` uses `border-image`
  with a slice instead; the sprite comes in as `--skin-src` so a hover state can
  swap it without a second class.

### Developer mode

`source_launch` in the catalog is what makes the hub exercisable before any
release exists: with the submodules checked out, a tool can be run from source.
Eight of the ten have one; `HS-ValueEditor` and `Hs-Offline-Loot-Forge` ship a
built executable and nothing else.

`bridge.js` does the same thing one level up — with Tauri absent it answers from
the embedded catalog, so `npm run dev` draws the real ten-tool library in a plain
browser and the whole frontend is workable without the Rust side running.

---

## CI

| Workflow | Trigger | What it does |
| --- | --- | --- |
| `.github/workflows/catalog.yml` | `repository_dispatch: release-published`, nightly, manual | Rebuilds and re-signs the catalog, **opens a pull request**, and publishes to the `catalog` release tag once main agrees. |
| `.github/workflows/hub-release.yml` | `hub-v*` tag, manual dry run | Tests, builds, signs, and publishes the hub plus `latest.json`. |
| `.github/workflow-templates/notify-hub-release.example.yml` | — | The sending half, to copy into a tool repository. |

The catalog workflow opens a pull request rather than pushing, matching the rule
`submodule-dispatch.yml` already set: an event anyone can fire should not move
the default branch.

### Repository secrets

| Secret | Used by | What happens without it |
| --- | --- | --- |
| `HUB_MINISIGN_SECRET_KEY` | `catalog.yml` | The job fails deliberately. An unsigned catalog is one the hub refuses, so publishing one would ship a hub that cannot update. |
| `TAURI_SIGNING_PRIVATE_KEY` | `hub-release.yml` | The installer builds but its updates can never be verified. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | `hub-release.yml` | Only if the key has one. |
| `HUB_DISPATCH_TOKEN` | each tool repository | The notification is skipped and the nightly schedule picks the release up instead. Latency, not correctness. |

### Still to do, and why it is not done here

`.github/workflows/submodule-dispatch.yml` and
`.github/workflow-templates/notify-hub.example.yml` exist on the unmerged branch
`feature/hs-game-sdk-and-agent-guidelines`, not on `main`. The plan asks for two
edits to them:

1. Add `release-published` to `submodule-dispatch.yml`'s event-type allowlist.
2. Add a second job to `notify-hub.example.yml` firing on `release: [published]`.

Neither is made here, because creating those files on this branch would conflict
with that one when it merges. `notify-hub-release.example.yml` is a standalone
template instead, so a tool repository can install it today; when the other
branch lands, fold its job into `notify-hub.yml` and delete the duplicate.

---

## Verification

```bash
py -3 -m unittest discover -s tests   # catalog generation and signing
cd hub && npm test                    # cargo test: install, rollback, interlocks
cd hub && npm run build               # the frontend
cd hub && npm start                   # the app
```

Things worth checking by hand, because they are the ones that matter and the
ones a unit test cannot reach:

- Install ForgePact from its real release; confirm the extracted tree is
  `ForgePact.exe` + `modfiles/`, matching `build_release.py`'s documented layout.
- Launch it; confirm `http://127.0.0.1:8766/` answers and the card says
  *Running*; Stop it.
- Corrupt one `sha256` in `catalog/catalog.json`, re-sign, and confirm the
  install **refuses** rather than falling through.
- Start `Hero_Siege.exe`, trigger an auto-install, confirm it stages rather than
  applying, and applies on next launch once the game has closed.
- Toggle Work offline and confirm zero outbound requests on launch.
- `tools/freeze_probe.ps1` if a hub-launched tool is suspected of stalling the
  game — the existing instrument, not a new one.

## Out of scope

No changes to any tool's own UI or behaviour: v1 requires zero commits to the ten
submodules. No monorepo migration (ADR 0001). No code-signing certificate. No
Linux or Steam Deck hub build — the Steam Deck editor stays a browser page the
hub can open.
