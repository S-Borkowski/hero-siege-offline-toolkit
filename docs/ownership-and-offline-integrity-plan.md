# Plan: Steam ownership gating & offline integrity

> **Status: future consideration — not scheduled, and nothing here is implemented.**
>
> This is a design note kept for later, not a work item. It records what was
> investigated, what the constraints actually are, and what the options would be if the
> toolkit ever takes this on. Revisit it if the ownership question comes up again, if
> Hero Siege ever ships an official offline launch option, or before any change to how
> `HS-Offline-Launcher` decides whether it is allowed to start the game.
>
> The findings in §1–§2 were verified against a live Season 10 install on 2026-09-11 and
> can go stale with a game or Steam update — re-check them before acting on any of this.

**Question.** Can the toolkit run mods, force the game to run offline, *and* keep Easy
Anti-Cheat and Steam's anti-piracy protections — so that a non-owner can't use the
launcher to play a game they don't own?

**Answer.** Half of it. **EAC: no, and no design gets you there.** **Steam ownership:
yes, and the launcher can enforce it better than the bare game executable does today.**
Those two were never the same protection, and separating them is what makes this
tractable.

---

## 1. Why EAC and ForgePact are mutually exclusive

This is not a limitation of `HS-Offline-Launcher`; it is the definition of anti-cheat.
Every mechanism ForgePact depends on is on EAC's detection list:

| ForgePact does | EAC treats it as |
| --- | --- |
| `AuriePatcher` adds a `.aurie` section to `Hero_Siege.exe` on disk | executable integrity failure |
| `AurieCore.dll` / `YYToolkit.dll` / `BloodPactPlugin.dll` load into the game process | unauthorized module injection |
| YYToolkit hooks GameMaker runtime functions | code patching / inline hooks |
| `HS-ValueEditor` writes process memory from outside | external memory tampering |

Confirmed against this machine's install (`steamapps/common/HeroSiege/bin`):

- `Hero_Siege.exe` currently carries a `.aurie` section; `Hero_Siege.exe.aurie_backup`
  holds the clean PE with the original 8 sections. The patch is an **on-disk** change,
  so it is visible to a file-integrity scan before a single instruction runs.
- `start_protected_game.exe` is Authenticode-signed by `CN=EasyAntiCheat Oy`. That is
  the protected launch path, and it is the one ForgePact cannot use.

There is no client-side "allow this module" switch. EAC's only sanctioned path for
third-party code is **developer-side**: the game developer allowlists a specific signed
module in their EAC/EOS configuration. Only Panic Art Studios can do that.

**And "EAC on, but offline" is actively worse than EAC off.** EAC's integrity scan is
local and runs whether or not a session reaches a game server; the detection is then
reported to the EOS backend at the next opportunity. A user would collect an account
ban for a purely single-player session — all of EAC's risk, none of its benefit. The
launcher's current fail-closed posture (refuse to launch while the EAC service or any
EAC process is live) is the correct behaviour and should stay exactly as it is.

---

## 2. The reframe: EAC was never the anti-piracy layer

- **EAC = anti-cheat.** It protects *other players* in online play. Skipping it for an
  offline single-player session harms nobody and pirates nothing.
- **Steam ownership = anti-piracy.** Entirely separate machinery.

So the real concern — "someone who doesn't own the game could use this" — has to be
answered with an **ownership check**, not with EAC. And here is the uncomfortable part,
verified on this install:

| Check | Result | Meaning |
| --- | --- | --- |
| `Hero_Siege.exe` PE sections | no `.bind` section | **not Steam CEG/DRM-wrapped** |
| `Hero_Siege.exe` Authenticode | `NotSigned` | no publisher signature to anchor to |
| `steam_api64.dll` Authenticode | `Valid`, `CN=Valve Corp.` | usable trust anchor |
| `steam.exe` Authenticode | `Valid`, `CN=Valve Corp.` | usable trust anchor |

**The game binary enforces no ownership of its own.** Once the files exist on disk, it
runs. Steam's protection here is upstream — the client only serves the depot to owners —
plus whatever `SteamAPI_RestartAppIfNecessary` / `SteamAPI_Init` does in-game, which the
launcher currently sidesteps by setting `SteamAppId=269210` in the child environment.

That cuts both ways, and it is the honest framing for this whole plan:

- The launcher **removes nothing**. A non-owner with the files can double-click
  `Hero_Siege.exe` and get the same result without any of our tools.
- But the launcher is a *convenient, documented, turn-key* path, and it currently
  asks no questions. That is the complicity problem worth fixing.

**Goal, stated precisely:** make launching through this toolkit *strictly harder* for a
non-owner than not using the toolkit at all, and make a modded session incapable of
touching live services or online saves.

---

## 3. Plan

### P0 — Steam ownership gate in `HS-Offline-Launcher` *(highest value, do first)*

A new `ownership.py` module, called from `launch_safety_blocker()` so it participates in
the existing fail-closed gate.

1. **Locate** `steam_api64.dll` via the existing `find_steam_runtime()`.
2. **Verify it before loading it.** Check the Authenticode signature chains to
   `CN=Valve Corp.` with status `Valid` (`WinVerifyTrust` via `ctypes`, or
   `Get-AuthenticodeSignature` semantics reimplemented). This is the load-bearing step:
   Goldberg / SmartSteamEmu / similar emulators are unsigned, so a swapped DLL fails
   here. Do **not** redistribute our own copy — verifying the user's copy avoids the
   Steamworks redistribution question entirely and is what was empirically confirmed to
   work above.
3. **Verify the Steam client too.** `steam.exe` from the discovered Steam root must be
   Valve-signed and running. An emulator has no real client behind it.
4. **Load and query**, with `SteamAppId=269210` set:
   - `SteamAPI_Init()` — fails outright with no logged-in client.
   - `ISteamApps::BIsSubscribedApp(269210)` — the ownership answer.
   - `ISteamApps::BIsSubscribedFromFamilySharing()` — **allow**; family sharing is
     legitimate ownership. Log it, don't block it.
   - `ISteamApps::BIsSubscribedFromFreeWeekend()` — policy call; recommend allow + warn.
   - `ISteamApps::GetAppInstallDir(269210)` — cross-check that the exe the launcher is
     about to spawn actually lives under the path **Steam itself** reports for app
     269210. This is what defeats "point the launcher at a repack in Downloads".
5. **Corroborate** with what the launcher already does: `appmanifest_269210.acf` exists
   with a fully-installed `StateFlags`, and the exe SHA256 is a `KNOWN_BUILDS` entry (or
   a clean-backup match, for the ForgePact-patched case).
6. **Fail closed**, consistent with the existing posture: any error, unknown state, or
   failed signature check blocks the launch with a specific message.
7. **Keep it local.** Log the authorizing SteamID64 to `launcher.log` for the user's own
   diagnostics; transmit nothing.

UI: a new status row next to the EAC badge — *Ownership: verified (Steam account …)* /
*not verified*. Tests mirror the existing 30-test style: mocked signature results,
mocked `BIsSubscribedApp` returns, install-dir mismatch, DLL-missing, client-not-running.

### P1 — Every tool checks, not just the launcher

ForgePact patches and injects on its own; the save and value editors act independently.
Gating only the launcher gates the least important door.

Ship the P0 check as a small shared module — `hs-game-sdk/python/hs_game_sdk/ownership.py`
(plus a thin C++ header for the plugin side if ever needed) — and call it at startup from
`ForgePact`, `hs-stat-forge`, `Hs-Offline-Loot-Forge`, `HS-ValueEditor`, `HSSaveEditor`
and `hero-siege-item-editor`.

Prefer this over a launcher→tool handshake token. A token would be local, user-controlled
and trivially forged; it would add real complexity for no additional resistance. Each tool
asking Steam directly is simpler and exactly as strong.

### P2 — Actually enforce "offline" *(highest value for community harm)*

"Offline" today means only "we didn't start EAC". The modded process is otherwise free to
reach the network. Make it structural:

- Around a modded session, add a Windows Firewall / WFP outbound block rule scoped to the
  patched `Hero_Siege.exe` path; remove it on exit, and on next start if it was orphaned.
- Requires elevation, so: opt-in, clearly explained, and the launcher must still work
  (with a visible warning) when declined.
- This is the measure that prevents the harm people actually care about — modded items
  and inflated characters reaching online play, leaderboards and the trade economy.

### P3 — Quarantine modded saves

Hero Siege saves to `%LOCALAPPDATA%\Hero_Siege`, shared between modded and online play.
The `Hero_Siege_backup_planC_*` directories in the research notes show this is already
being managed by hand.

Formalize it: the launcher swaps `%LOCALAPPDATA%\Hero_Siege` with a
`%LOCALAPPDATA%\Hero_Siege.forgepact` profile for the duration of a modded session and
restores it on exit (crash-safe — a marker file lets the next start finish an interrupted
swap). A modded character then cannot be carried into an online session by accident.

### P4 — Policy, docs, release hygiene

The written disclaimers in `ForgePact/README.md` ("⚠️ Offline only") and
`HS-Offline-Launcher/README.md` are already clear and honest; the gap is enforcement, not
documentation. Add only:

- A short "Ownership requirement" section in the root README and the launcher README,
  stating the gate exists and what it checks.
- An `agents.md` rule: **no feature may weaken or make optional the ownership gate, the
  EAC fail-closed gate, or the save quarantine** — matching the existing "Never Call an
  Address You Resolved by Hand" and decompiler rules in tone.
- Confirm release archives never contain game-derived binaries. The root `.gitignore`
  already excludes `*.exe`, `*.dll`, `*.win`, `*.yytex` and the extraction dumps;
  `ForgePact/modfiles_shipped/` legitimately contains only Aurie/YYToolkit binaries
  (AGPL-3.0, ours to ship) and the project's own plugin. Add a CI or
  `test_repo_bounds_contract.py` assertion so it stays that way.

---

## 4. Explicitly not doing

- **No backend ownership verification.** `ISteamUser/CheckAppOwnership` in the Steam Web
  API requires a *publisher* key for app 269210 — we are not the publisher, so no
  third-party service can authoritatively confirm ownership. `IPlayerService/GetOwnedGames`
  needs a public profile and is trivially spoofed. A backend would add privacy liability
  and buy nothing.
- **No collecting or transmitting SteamIDs.**
- **No attempt to re-enable EAC on a patched executable.** It would fail, and if it
  didn't it would ban the user.
- **No obfuscation or tamper-proofing of the gate.** It is a locked front door, not a
  vault; pretending otherwise wastes effort and invites an arms race.

---

## 5. Honest threat model

**What this stops:** a non-owner using the toolkit as a ready-made offline launcher; a
repack + Steam-emulator setup (the signature check rejects it); a user aiming the
launcher at a copy outside their Steam library; modded characters and network traffic
leaking into online play.

**What this does not stop:** anyone willing to patch our launcher or simply run
`Hero_Siege.exe` directly — which is already possible today and needs none of our code.

That residual gap is not closable from our side, because the game binary carries no DRM
of its own. What *is* achievable is that the toolkit stops being the easy path, refuses
the casual "I downloaded a repack, how do I mod it?" user, and holds a documented,
defensible position.

---

## 6. The only real route to "mods *and* EAC"

Not an engineering task — an ask to Panic Art Studios:

1. **A developer-provided offline launch option.** A Steam launch option or depot flag
   that starts `Hero_Siege.exe` without `start_protected_game.exe`, flags the resulting
   saves as offline-only, and blocks them from online play. Several games ship exactly
   this. It solves ownership (Steam launches it), anti-cheat (EAC isn't lied to), and
   save integrity (the game itself segregates them) in one move.
2. **EAC module allowlisting** for a signed ForgePact build — technically possible,
   but requires far more trust from the developer and is the weaker ask.

Worth sending, given the toolkit's existing reverse-engineering notes demonstrate
good-faith offline-only intent. Low odds, high payoff, near-zero cost.

---

## 7. Verification still outstanding

- Whether `Hero_Siege.exe` calls `SteamAPI_RestartAppIfNecessary(269210)` and what it
  does on failure. If it does hard-fail, the launcher's `SteamAppId` env var is
  suppressing a real check and dropping it would be the cheapest ownership gate of all.
  Test: launch with Steam signed out and observe.
- Whether `SteamAPI_Init()` failure is fatal in the GameMaker Steam extension
  (`PAS_Steam_GMS_x64.dll` / `Steamworks_x64.dll`) or merely disables achievements.
- Whether the EAC service state alone is a sufficient online gate, or the game can reach
  backend services (leaderboards, trade) without EAC running.

Answering the first two may collapse P0 into "stop suppressing the game's own check",
so run them before building.

---

## Suggested order

`P0` → `P3` → `P2` → `P1` → `P4`, with the §7 experiments first.

P0 answers the question that was asked. P3 is cheap and prevents the most common real
accident. P2 needs elevation UX and deserves its own pass. P1 is mechanical once P0
exists. P4 closes it out.
