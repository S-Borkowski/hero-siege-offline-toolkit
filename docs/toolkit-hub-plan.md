# Toolkit Hub — a single app that holds all the tools

**Status:** planned, not implemented. Written 2026-09-11.

## Context

`hero-siege-offline-toolkit` is a hub only on paper: a README table of ten links and ten
git submodules. A player who wants the toolkit today visits ten GitHub repos, downloads
ten differently-named archives, unpacks them wherever, and has no way to learn that any
of them shipped a fix. A maintainer regenerating `hs-game-sdk` after a game patch has to
open ten PRs and cannot test the result as one unit.

This plan builds the missing layer: **Toolkit Hub**, a Tauri 2 + Svelte 5 desktop app that
installs, launches, and updates every tool from one window, plus the signed catalog and
release pipeline that feeds it. It also answers the repo-topology question (monorepo vs
submodules), with the reasoning recorded rather than just the verdict.

---

## What the codebase already decides for us

These are measured facts, not assumptions. Each one closes off a design option.

| Finding | Evidence | Consequence |
| --- | --- | --- |
| Five tools are hardened loopback web apps that **refuse to be framed** | `hero-siege-item-editor/hs_item_editor_gui.py:7808` sets `frame-ancestors 'none'` + `X-Frame-Options: DENY`; `HS-Offline-Launcher/src/hs_offline_launcher.py:846` the same, plus an HMAC API token bound per process | The hub **cannot** be a tabbed shell that iframes tools. It must own windows/processes, not documents. See D2. |
| Release assets are named inconsistently across the ten repos | `ForgePact-1.3.16.zip` + `.sha256`; `HeroSiegeItemEditor-v2.15.4-s10.exe`; `hs-offline-tracker-0.1.2-portable.zip` + `SHA256SUMS-0.1.2.txt`; `HSCraftSim` ships 7 assets incl. a bare `.exe`; `HS-ValueEditor` tag `v1.0.2` ships an asset named `…-v1.0.1.zip` | No convention can be inferred. Asset choice must be a **declared per-tool rule**, and the tag is the authoritative version. See D3. |
| Four tools have **no LICENSE file**, one is AGPL-3.0, two ship only an EXE with no source | `ForgePact/LICENSE` = AGPL-3.0; `HS-Offline-Launcher`, `HS-Offline-Tracker` = MIT; `HSCraftSim`, `hero-siege-item-editor`, `hs-stat-forge`, `HSSaveEditor` = none; `HS-ValueEditor` and `Hs-Offline-Loot-Forge` contain only a built `.exe` | Download-on-demand redistributes nothing and ships now. The offline bundle is gated on a licensing prerequisite (Phase 6). |
| `hs-game-sdk` is consumed by **path traversal that silently dies when frozen** | `ForgePact/src/forgepact.py:37` falls back to `parents[2]/"hs-game-sdk"/"python"`, and `:58` swallows the failure into `GameObject = None`. `ForgePact/build_release.py` never bundles the SDK | Every shipped ForgePact runs with the SDK absent. This is the real coupling defect — and it is the one a monorepo was supposed to fix. See D1. |
| The SDK is already packageable but never published | `hs-game-sdk/python/pyproject.toml` (`hs-game-sdk` 1.0.0) exists; `hs-game-sdk/ts/package.json` declares `main: dist/index.js` but there is **no tsconfig and no build script**, so `@hero-siege/sdk` cannot actually be consumed as declared | Fixing the coupling is a small job, not a migration. |
| Tools need different privilege levels | `hs-stat-forge`, `HS-ValueEditor`, `Hs-Offline-Loot-Forge` need Administrator (`SeDebugPrivilege`); the rest do not | The hub must run **non-elevated** and elevate per-launch. |
| The visual language already exists | `HS-Offline-Tracker/src/theme.css` (full token set, `obsidian`/`ember`/`void` skins) and `src/skin.svelte.js` (9-slice SVG panel/chip/button, window chrome icons), with `decorations: false` windows in `src-tauri/tauri.conf.json` | The hub adopts this verbatim. No new design language. |
| Cross-repo notification machinery already exists | `.github/workflows/submodule-dispatch.yml` receives `repository_dispatch`, validates an allowlisted payload, and **opens a PR rather than pushing** | The catalog pipeline extends this, it does not invent a parallel one. |
| Tools already expose identity/health endpoints | `hero-siege-item-editor/hs_item_editor_gui.py:10271` returns `{version, pid, port}`; `HSCraftSim/server.py:12` `/_health` returns `{application, version}` | Reuse these for "already running" detection instead of inventing a hub protocol. |

---

## Decision records

Recorded so the reasoning is not re-derived later, per the practice already set by
[`docs/ownership-and-offline-integrity-plan.md`](ownership-and-offline-integrity-plan.md).

### D1 — Keep submodules; fix the coupling instead of moving the repos

**Both were researched.** The general literature favours the monorepo for exactly this
shape: shared SDK, atomic cross-cutting changes, one CI run. The usual objection
(per-project release cycles) is solved — `release-please` with `monorepo-tags: true` gives
every tool its own tag, changelog and GitHub Release inside one repo. So the standard
argument does not decide it. Three project-specific facts do:

1. **Ownership.** The canonical repos are `falorfrozen-cmd/*`; `origin` here is
   `S-Borkowski/hero-siege-offline-toolkit` with `falorfrozen-cmd` as `upstream`, and
   submodules carry `origin = falorfrozen-cmd`, `fork = S-Borkowski`. A migration is not
   ours to perform — it is a proposal to upstream. Blocking the hub on it would mean
   shipping nothing.
2. **Two members cannot join.** `HS-ValueEditor` and `Hs-Offline-Loot-Forge` have no
   source at all. In a monorepo they would be committed binaries, which the root
   `.gitignore` bans outright (`*.exe`). They would stay external regardless, so the
   monorepo never actually becomes whole.
3. **The migration would not fix the defect it is prescribed for.** The SDK breakage is
   not "ten PRs are annoying" — it is that `ForgePact.exe` ships *without the SDK* and
   fails open to `None`. A monorepo does not bundle a wheel into a PyInstaller build. The
   fix is publishing and pinning the SDK, and that fix is needed in either topology.

**Decision:** keep submodules, and do the three things a monorepo was wanted for, directly:

- **Publish `hs-game-sdk` as a versioned artifact** — build a wheel from the existing
  `hs-game-sdk/python/pyproject.toml`, add the missing `tsconfig.json` + build script so
  `@hero-siege/sdk` matches its own manifest, and zip the C++ headers. Attach all three to
  an `sdk-vX.Y.Z` release. Each submodule pins a version and **vendors it into its frozen
  build** (a `--add-data`/`collect` step in `ForgePact/build_release.py` and friends),
  which deletes the `parents[2]` path hack and the silent `None`.
- **Make the hub indifferent to topology.** The hub consumes *released artifacts* via the
  catalog, never source. Whether the source lives in one repo or ten is invisible to it.
- **Keep the door open.** If upstream later wants the monorepo, the step above has already
  removed most of its motivation and nothing in the hub or catalog changes. The migration
  path is recorded in `docs/adr/0001-repo-topology.md` so it is not re-derived.

**What this gives up:** a cross-cutting SDK regeneration still spans ten PRs, and CI still
cannot test the whole matrix in one run. The pinned-SDK model contains the blast radius —
consumers upgrade deliberately instead of being broken by a pointer bump — but it does not
make the change atomic. That is the accepted cost.

### D2 — Windows, not tabs

Two tools explicitly forbid framing (table above), and two more (`HSSaveEditor`,
`hs-stat-forge`) are Tkinter, which cannot be embedded in a webview at all. A proxy in
front of them would break the `Host`/`Origin` checks and the per-process HMAC token — that
is, deliberate anti-DNS-rebinding defences would be dismantled to get a cosmetic tab bar.

**Decision:** the hub is a **library and process manager** — Steam-shaped, not
browser-shaped. Each tool runs as its own process in its own window. The hub tracks it,
shows it as *Running*, and can stop it. Uniform chrome across heterogeneous tools is not
achievable and is not pursued.

### D3 — A generated, signed catalog; not live GitHub API calls

Querying ten `releases/latest` endpoints at launch costs 10 of the 60/hr unauthenticated
GitHub rate limit per user, gives no integrity guarantee, and cannot express the per-tool
asset rules the inconsistent naming demands.

**Decision:** a build-time generator resolves releases, picks assets by declared rule, and
pins SHA-256 into one `catalog.json`, signed with minisign. The hub makes **one** request
per check. The catalog is the trust root: a swapped release asset fails hash verification
even though the tools themselves are unsigned EXEs.

### D4 — Engine in Rust, generator in Python

Install/verify/launch/rollback live in the hub's Rust side (`src-tauri`), tested with
`cargo test` exactly as `HS-Offline-Tracker` already does via `npm test`. The catalog
generator is Python in `tools/`, matching `tools/extract_and_generate_sdk.py` and testable
with the root `tests/` unittest style. No sidecar, no duplicated engine.

### D5 — Update policy, and the interlocks it needs

The chosen behaviour: **check on launch and on a manual button**; a setting for
**auto-download**; and, only when auto-download is on, a sub-setting for **auto-install**.
Tauri v2's updater exposes `download()` and `install()` separately, so this maps onto the
plugin directly rather than needing custom staging.

Two interlocks are added because the toolkit's own risk posture demands them, and they
apply to auto-install only:

- **Never install over a running tool, and never while `Hero_Siege.exe` is running.**
  ForgePact patches the game's PE and holds file IPC; `HSSaveEditor` documents a
  game-closed requirement. A staged install waits for next launch and says so.
- **A "Work offline" master switch** disables the launch check entirely.
  `HS-Offline-Tracker/src/About.svelte:6` states the project value plainly — the check is
  "never something the app does on its own... the only request the app ever makes". A
  launch check is a defensible default for a hub whose *job* is distribution, but the
  toolkit's offline-first promise needs an honest way out, and first run says what the
  default does before it does it.

### D6 — The hub never relocates a tool's data

`%LOCALAPPDATA%\Hero_Siege\forgepact.json`, `%LOCALAPPDATA%\HSCraftSim\session.json`,
`%LOCALAPPDATA%\HS Offline Tracker\events.ndjson` and each tool's saves/backups stay
exactly where they are. The hub manages *program files only*, so uninstalling the hub
never touches a player's settings, and a tool launched outside the hub behaves identically.

---

## Architecture

```
falorfrozen-cmd/<tool>  ──release──►  GitHub Releases (unchanged, ten repos)
         │ repository_dispatch: release-published
         ▼
  hub repo CI: tools/build_catalog.py  ──►  catalog.json + catalog.json.minisig
         │                                    (published to the `catalog` release tag)
         ▼
  Toolkit Hub (Tauri 2 + Svelte 5)
    ├─ catalog.rs   fetch + minisign-verify + cache (embedded copy as fallback)
    ├─ install.rs   download → SHA-256 → extract → versioned dir → activate → rollback
    ├─ launch.rs    spawn (elevated when declared) → health probe → track PID
    ├─ updater.rs   Tauri updater plugin for the hub's own binary
    └─ state.rs     installed versions, pins, settings, offline mode
```

### Install layout (per-user, no admin)

```
%LOCALAPPDATA%\Hero Siege Toolkit\
  tools\<id>\<version>\      extracted artifact
  tools\<id>\current.json    which version is active (previous kept for rollback)
  cache\downloads\           resumable, hash-verified before use
  state.json                 installed set, settings, last check
  logs\hub.log
  bundle\                    optional: offline-bundle payload, used before network
```

### Catalog schema (`catalog/catalog.json`, documented in `docs/hub/catalog-schema.md`)

```jsonc
{
  "schema": 1,
  "generated": "2026-09-11T20:00:00Z",
  "tools": [{
    "id": "forgepact",
    "name": "ForgePact",
    "summary": "Offline gameplay modifiers: density, spawns, drop rates, map reveal.",
    "repo": "falorfrozen-cmd/ForgePact",
    "submodule": "ForgePact",
    "version": "1.3.18",
    "published": "2026-09-11T...",
    "license": "AGPL-3.0",
    "requires": { "admin": false, "game_closed": false, "windows_only": true },
    "artifact": { "kind": "zip", "url": "...", "size": 20975420,
                  "sha256": "...", "strip_prefix": "ForgePact/" },
    "launch": { "exe": "ForgePact.exe", "args": [], "elevate": false,
                "health": { "url": "http://127.0.0.1:8766/", "timeout_s": 20 },
                "ports": [8766, 8780, 8801, 8899, 9133, 9777] },
    "source_launch": { "cmd": "py", "args": ["src/forgepact.py"] },
    "notes_url": "https://github.com/.../releases/tag/v1.3.18",
    "guide": "docs/submodules/ForgePact/instructions.md"
  }]
}
```

`kind` covers the four real shapes found: `zip` (ForgePact, Tracker portable, HSCraftSim,
ValueEditor, SteamDeck), `exe` (item editor, Launcher, StatForge — a bare executable),
`nsis` (Tracker setup), `html` (SteamDeck editor, opened rather than run).
`source_launch` is what makes the hub useful in *this* workspace: with the submodules
checked out it can run a tool from source, so the hub is exercisable before any release
exists — the "build the fast loop first" rule from [`agents.md`](../agents.md).

---

## The app

**Window** 1180×760, `decorations: false`, custom title bar reusing
`HS-Offline-Tracker/src/skin.svelte.js` chrome icons; `theme.css` tokens and the three
skins adopted unchanged (`--ground-*`, `--edge-*`, `--gold-1 #d6a64c`, `--arcane #59d6c0`).

```
┌───────────────────────────────────────────────────────────────────────┐
│ ◈ Hero Siege Toolkit                                      — ✕         │
├──────────┬────────────────────────────────────────────────────────────┤
│ Library  │  Library                       [ Check for updates ]       │
│ Updates 2│  ┌──────────────┐ ┌──────────────┐ ┌──────────────┐        │
│ Game     │  │ ▣ ForgePact  │ │ ▣ Item Editor│ │ ▣ Tracker    │        │
│ Settings │  │ Offline mods │ │ Items, stash │ │ Loot journal │        │
│ About    │  │ v1.3.16      │ │ v2.15.4      │ │ not installed│        │
│          │  │ ● Running    │ │ ▲ 2.15.4→…   │ │              │        │
│          │  │ [  Stop  ]   │ │ [ Update ]   │ │ [ Install ]  │        │
│          │  └──────────────┘ └──────────────┘ └──────────────┘        │
├──────────┴────────────────────────────────────────────────────────────┤
│ ◆ Hero Siege: not running   ◆ EAC: disabled   ◆ catalog 3m ago  ⚑ off │
└───────────────────────────────────────────────────────────────────────┘
```

- **Tool card** — mark, name, one-line summary, state chip (*Not installed / v1.3.16 /
  Update available / Running / Staged for next launch*), one primary button, overflow menu
  (Release notes, Developer guide → `docs/submodules/<id>/instructions.md`, Open folder,
  Verify, Roll back, Uninstall).
- **Detail view** — changelog from the release body, requirement chips (*Administrator*,
  *Close the game first*, *Windows only*), install path, pinned SHA-256, license, links.
- **Updates** — everything with a newer version, per-tool *Update* plus *Update all*, and
  any staged install waiting on the game closing.
- **Game** — surfaces `Hero_Siege.exe` and EAC state so the interlocks are legible, and
  launches the game through `HS-Offline-Launcher` when it is installed.
- **Settings** — Work offline; Check on launch; Auto-download; Auto-install (nested,
  disabled unless auto-download is on); skin; install root; developer mode (run from
  submodule source).
- **Downloads drawer** — per-file progress with an explicit *Verifying* step, so hash
  checking is visible rather than implied.
- **First run** — one screen naming exactly what the hub will contact
  (`raw`/`objects.githubusercontent.com` + `api.github.com`) with the defaults preselected
  and Work offline one click away.

---

## Work plan

### Phase 0 — Catalog engine (no GUI; lands and is testable on its own)
- `catalog/sources.toml` — the ten hand-maintained per-tool rules (repo, asset-selector
  regex, kind, launch spec, requirements, guide path).
- `tools/build_catalog.py` — resolve `releases/latest`, select the asset, prefer a
  published `.sha256`/`SHA256SUMS*` sidecar and otherwise download and hash, emit
  `catalog/catalog.json`. Same CLI shape as `tools/extract_and_generate_sdk.py`.
- `tests/test_build_catalog.py` — unittest, fixtures for all four `kind`s and for the
  `HS-ValueEditor` tag/asset version mismatch; no network.

### Phase 1 — Hub shell
- `hub/` Tauri 2 + Svelte 5 scaffold mirroring `HS-Offline-Tracker`'s layout
  (`package.json`, `vite.config.js`, `src-tauri/tauri.conf.json`, `capabilities/`);
  `theme.css` + `skin.svelte.js` adopted; window chrome, sidebar, Library grid reading a
  **local** catalog with everything stubbed as not-installed.

### Phase 2 — Install / verify / launch
- `src-tauri/src/{catalog,verify,install,launch,state}.rs`: minisign verification via
  `minisign-verify` (already in the updater's dependency tree), download → hash → extract
  → versioned dir → activate, rollback to previous, elevate via `ShellExecuteW`/`runas`
  when `requires.admin`, health probe reusing each tool's existing `/_health` and
  `/api/instance` endpoints, PID tracking and Stop.
- `cargo test` over: version compare (`0.9.8` vs `0.9.10` — port the `newer()` logic from
  `About.svelte:29`), asset selection, hash mismatch refusal, interrupted-extract
  recovery, rollback, and the game-running interlock.
- An end-to-end install test against a fixture catalog served from a temp HTTP server, so
  the loop never needs a real release.

### Phase 3 — Updates
- Tauri updater plugin for the hub (`plugins.updater` with `pubkey` + `endpoints`;
  `latest.json` published by `tauri-action` with `includeUpdaterJson: true`; signed with
  `TAURI_SIGNING_PRIVATE_KEY` in repo secrets). Separate `download()` / `install()` so the
  auto-download / auto-install settings map straight onto the plugin.
- Catalog check on launch + manual button; auto-download and nested auto-install settings;
  staged-install queue honouring the D5 interlocks.

### Phase 4 — Release pipeline
- `.github/workflows/hub-release.yml` — build, sign, publish the hub + `latest.json`.
- `.github/workflows/catalog.yml` — regenerate the catalog on `repository_dispatch`
  (`release-published`) and on a schedule, **opening a PR**, matching
  `submodule-dispatch.yml`'s existing never-push-directly rule.
- Extend `.github/workflow-templates/notify-hub.example.yml` with a second job firing
  `release-published` on `release: [published]`, and add the event type to the allowlist in
  `submodule-dispatch.yml`.

### Phase 5 — SDK de-coupling (D1's substantive half)
- `hs-game-sdk/ts/tsconfig.json` + build script so `@hero-siege/sdk` matches its manifest.
- CI job publishing wheel + npm tarball + header zip to an `sdk-vX.Y.Z` release.
- Vendor the SDK into `ForgePact/build_release.py`'s PyInstaller invocation and remove the
  `parents[2]` fallback at `ForgePact/src/forgepact.py:37`; repeat per consumer. **Each of
  these is a PR to a submodule repo this repo does not own** — sequence them after the hub
  ships, so upstream review never blocks the hub.

### Phase 6 — Offline bundle (gated)
- **Prerequisite, not optional:** add LICENSE files to `HSCraftSim`,
  `hero-siege-item-editor`, `hs-stat-forge`, `HSSaveEditor`, and obtain redistribution
  permission for the two binary-only tools. Without a license grant there is no right to
  redistribute, and AGPL-3.0 aggregation additionally requires shipping the licence text
  and a source offer for ForgePact.
- Then: a CI job zipping the hub installer + every artifact + catalog into
  `hs-toolkit-offline-<ver>.zip`, which the hub installs from `bundle\` before touching
  the network.

### Docs (per `agents.md`'s maintenance rule)
`docs/hub/design.md`, `docs/hub/catalog-schema.md`, `docs/adr/0001-repo-topology.md`;
root `README.md` gains a Hub section; `docs/submodules/README.md` gains a Hub row.

---

## Verification

- `py -3 -m unittest discover -s tests` — catalog generation, all four artifact kinds, the
  tag/asset version mismatch.
- `cd hub && npm test` (→ `cargo test`) — verify, install, rollback, interlocks, version
  compare.
- `cd hub && npm start` — Library renders from `catalog/catalog.json`; install ForgePact
  from its real release; confirm the extracted tree matches `build_release.py`'s documented
  layout (`ForgePact.exe` + `modfiles/`); Launch; confirm `http://127.0.0.1:8766/` answers
  and the card shows *Running*; Stop.
- Corrupt one `sha256` in the catalog and confirm the install **refuses** rather than
  falling through.
- Launch `Hero_Siege.exe`, trigger an auto-install, confirm it stages instead of applying,
  and applies on next launch after the game closes.
- Toggle Work offline and confirm zero outbound requests on launch.
- `tools/freeze_probe.ps1` if any hub-launched tool is suspected of stalling the game —
  the existing instrument, not a new one.

## Out of scope

No changes to any tool's own UI or behaviour: **v1 requires zero commits to the ten
submodules.** No monorepo migration (D1). No code-signing certificate — SmartScreen
warnings remain, and the catalog's hash pinning is the integrity story instead. No Linux or
Steam Deck hub build; the SteamDeck editor stays a browser page the hub can open.
