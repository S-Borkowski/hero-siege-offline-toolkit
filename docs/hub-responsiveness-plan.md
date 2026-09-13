# Hub responsiveness — the UI thread is blocked on loopback network probes

**Status:** implemented 2026-09-13, against hub 0.1.3. Stages 1-4 are done and
measured; the results are at the bottom. Written 2026-09-13.

## The complaint

The hub "feels a little unresponsive". Two things in particular: opening the Downloads
drawer while on the Updates, Settings or About tabs, and changing a setting — the
Appearance skin buttons were called out by name.

Both are the same defect seen from two angles, and it is not a rendering problem. The
window is genuinely frozen for seconds at a time.

---

## What was measured

Release build, this machine, 365 processes running. The numbers come from a throwaway
integration test against `hero_siege_toolkit_hub_lib`; Stage 4 makes a trimmed version of
it permanent.

| Measurement | Result |
| --- | --- |
| `procs::Snapshot::take()` | **25.5 ms** (stable across five runs) |
| `launch::already_running("forgepact")` | **715.6 ms** |
| `launch::already_running("hero-siege-item-editor")` | **712.9 ms** |
| `launch::already_running("hs-offline-launcher")` | **702.3 ms** |
| `launch::already_running("hscraftsim")` | **701.1 ms** |
| `already_running` for the other six tools | 0.0 ms |
| **One `build_view`** | **≈ 2860 ms** |

The six free ones are free because they declare neither a health endpoint nor a port, so
`already_running` returns without doing anything. The four expensive ones each cost
almost exactly `probe`'s `timeout_connect(700ms)` — the full timeout, every time.

That is the part worth explaining, because it is not what loopback is supposed to do:

| Measurement | Result |
| --- | --- |
| `port_in_use(8766)` — 120 ms timeout | **121.6 ms**, its whole timeout |
| `TcpStream::connect(("127.0.0.1", 8766))` — no timeout | **2028.0 ms**, then `ConnectionRefused` |
| `TcpStream::connect(("127.0.0.1", 17870))` — no timeout | **2037.9 ms**, then `ConnectionRefused` |

A connect to a closed loopback port takes **two seconds** to be refused on this machine.
Something in the network stack — a filtering driver, most likely — is swallowing the RST
that should come back instantly. So no probe in the hub ever gets a fast refusal: each
one burns whatever timeout it was given, in full.

This is machine-specific, and that is precisely why it must not be load-bearing. The
design currently assumes a fast refusal, does not say so anywhere, and degrades to
multi-second freezes on any machine that does not provide one.

### Why that reaches the UI at all

Every command in `hub/src-tauri/src/lib.rs` is a synchronous `fn` under a plain
`#[tauri::command]`. Tauri's macro compiles those through `body_blocking`, which calls the
function inline in the IPC handler — and that handler runs on the main thread. On Windows
the main thread is the one pumping WebView2 messages, so for as long as a command runs the
window processes no clicks and paints no frames.

`build_view` (`lib.rs:155`) is reached from the `library` command. Its cost is the 25 ms
snapshot plus a health probe per tool the process table did not already account for:

```rust
// lib.rs:204
let answering = pid.is_none() && launch::already_running(tool);
```

`pid` is `None` for every tool that is not currently running — which is the ordinary
state, and is unconditionally true for every tool that is not even installed. So the
ordinary case is the expensive one: four blocking HTTP probes, ~2.83 s, on the UI thread.

### Where that lands

| Trigger | Cost | Effect |
| --- | --- | --- |
| `setInterval(refresh, 10_000)` in `App.svelte:50` | 2.9 s every 10 s | The window is dead **~29% of the time**, on every tab, with the hub idle |
| `saveSettings` — `set_settings` calls `announce` (a `build_view`), then the frontend does `await refresh()` (another) | **≈ 5.7 s** | One Appearance click freezes the window for almost six seconds |
| `act(command, args)` — same double-charge for launch / stop / uninstall / rollback | ≈ 5.7 s | Every action costs twice what it needs to |
| `Game.svelte:54` — `act('launch_tool', …).then(refresh)` | ≈ 8.6 s | Three `build_view`s for one click |

The double-charge is pure waste, not a trade-off: `set_settings`, `uninstall_tool`,
`rollback_tool`, `launch_tool` and `stop_tool` all already call `announce`, which emits
`library-changed` with a freshly built view. The frontend's follow-up `refresh()` asks for
a view the backend has already pushed.

### Why those three tabs

The freeze is global and time-based rather than tab-specific — but the reported tabs are
where a click is most likely to arrive while a command holds the thread:

- **Settings** — every toggle and every skin button costs 5.7 s, so the *next* click,
  whatever it is, lands in a dead window. Reaching for Downloads right after changing the
  skin is the worst case in the whole app.
- **About** — `onMount` fires `hub_info`, which queues behind any in-flight `library`.
- **Updates** — "Check now" and "Update all" both run network work on the same thread.

On Library there is nothing that triggers a `build_view` by itself, so only the 10 s poll
freezes it, and the drawer usually opens.

---

## Plan

Four stages, then a release. Stages 1 and 3 are small and end the freezing; Stage 2 is the
structural fix that stops the work being expensive in the first place.

### Stage 1 — get blocking work off the UI thread

Change `#[tauri::command]` to `#[tauri::command(async)]` on every command that touches the
disk, the network or the process table:

`library`, `set_settings`, `check_for_updates`, `check_hub_update`, `install_tool`,
`uninstall_tool`, `rollback_tool`, `verify_tool`, `launch_tool`, `stop_tool`, `game_status`,
`open_path`, `open_url`.

No signature changes are needed: applied to a sync `fn`, the attribute selects the
`sync_threadpool` kind and the body is spawned on the async runtime instead of called
inline. `State<'_, Arc<Hub>>` is fine there because every one of these returns an owned
value.

Leave `hub_info`, `get_settings` and `report` synchronous — they read memory and nothing
else, and a thread hop would cost more than the work.

**Effect on its own:** the window stops freezing. The work still takes ~2.9 s, so the view
still arrives late, but nothing blocks input while it does.

### Stage 2 — make `build_view` cheap

Take the network out of the view-building path entirely.

- Add a probe cache to `Hub`: tool id → (last answer, when it was taken).
- `build_view` reads only the cache. It never probes, and never blocks on a socket.
- A background ticker owns the cache: it refreshes on an interval, runs the at-most-four
  probes **concurrently** rather than in series, and calls `announce` only when an answer
  actually changed.
- Skip probing any tool that is neither installed nor tracked. That is the case that costs
  the most and answers the least — a tool the hub has never installed cannot be launched,
  stopped or updated from its card, so "is it answering somewhere" changes nothing the
  reader can act on.
- Drop `probe`'s `timeout_connect` to ~250 ms and `port_in_use`'s to ~100 ms, and say in
  the comment why the number is what it is. A local server that cannot accept a loopback
  connection in 250 ms is not usefully "up".

**Effect:** `build_view` becomes ~25 ms, all of it process enumeration. Worst case for the
whole probe set becomes one timeout rather than four, and it happens off the view path.

### Stage 3 — stop asking for the same work twice

- Drop the `await refresh()` from `saveSettings` in `library.svelte.js`. `set_settings`
  already announces.
- In `act`, refresh only for commands that do **not** announce. Today the exception list
  names `install_tool`; it should be inverted — announce is the rule, and the frontend
  refresh is the exception.
- Drop the `.then(refresh)` in `Game.svelte:54`.
- Replace `setInterval(refresh, 10_000)` in `App.svelte` with the Stage 2 ticker, which
  takes one snapshot and announces only when something changed — the game came up or went
  down, a tracked PID died, a probe flipped. An idle hub then costs one 25 ms snapshot per
  tick and no IPC at all, where today it costs a full view build, a serialization and a
  round trip every ten seconds regardless.

Keep the comment's intent from `App.svelte:47` — Hero Siege starting or stopping still has
to be noticed without anything pushing it — but put the polling where the data is.

### Stage 4 — verify

- Keep a trimmed version of the measurement as a permanent regression test, asserting a
  `build_view` budget (200 ms is generous against a 25 ms target and will not flake on a
  loaded runner). The test is the only thing that would catch a future probe creeping back
  into the view path, which is exactly how this got here.
- `cargo test` and `npm run check` in `hub/`.
- Run the real app and click through: skin buttons, the Downloads drawer from each of the
  four tabs, a tool launch, a tool stop.
- **`launch_tool` needs a real trigger, not reasoning.** It calls `ShellExecuteExW` with
  the `runas` verb for the three elevated tools. Moving it off the main thread should be
  fine, but a UAC prompt raised from a threadpool thread is the one change here I would
  not sign off on without seeing it work.

### Stage 5 — build and release on the fork

The fork is `S-Borkowski/hero-siege-offline-toolkit`. `TAURI_SIGNING_PRIVATE_KEY` and its
(empty) password are already set there, so `hub-release.yml` will sign the bundle and emit
`latest.json`.

1. Land the change on `main` (fork is writable; upstream `falorfrozen-cmd` is not, so
   anything going there is a separate PR).
2. Bump all six version fields at once — 0.1.3 → **0.1.4**:

   ```
   py -3 tools/cut_release.py 0.1.4
   py -3 tools/cut_release.py --check --expect 0.1.4
   ```

   `cut_release.py` deliberately does not touch git; commit the six edits by hand with a
   message that says why the release exists.
3. Optionally dry-run first: `workflow_dispatch` on Hub release with `dry_run: true`. It
   builds and tests without publishing and keeps the installer as an artifact.
4. Tag and push:

   ```
   git tag hub-v0.1.4
   git push origin hub-v0.1.4
   ```

5. The workflow points the build at whichever repository is publishing it — `HUB_REPO` and
   the updater endpoint are both derived from `github.repository` — so the fork's release
   checks the fork, with no source edit.
6. It creates a **draft**. A draft's assets are not served, so
   `releases/latest/download/latest.json` stays a 404 and no installed hub can see the
   release until a human publishes it at
   `https://github.com/S-Borkowski/hero-siege-offline-toolkit/releases`.
7. Install the built `.exe` over the existing hub and confirm the freeze is gone against
   the thing that was actually shipped, not against `npm start`.

---

## Risks

| Risk | Handling |
| --- | --- |
| A UAC prompt raised from a threadpool thread behaves differently | Stage 4 triggers it for real; if it misbehaves, `launch_tool` stays synchronous — it is a deliberate user action, and a brief freeze on a click that is about to raise a system modal is acceptable in a way that a background poll is not |
| The probe cache reports a tool as up shortly after it exited | Bounded by the ticker interval, and the process table — which is authoritative and cheap — is still consulted first on every view build. The cache only answers where the process table had nothing |
| Shortened timeouts miss a slow-starting local server | `wait_until_healthy` keeps its own generous per-tool `timeout_s` for the launch path. Only the "is it already up" question gets the short timeout, and that question is asked repeatedly rather than once |
| The background ticker and an install worker both call `announce` | Already the case today; `build_view` takes its locks in a fixed order and Stage 2 does not change that |

## What this does not change

The catalog, the signature checking, the install and rollback paths, and every interlock
around running tools and the running game. This is a threading and caching change to how
the view is produced, not a change to what the view says.

---

## What it measured afterwards

Same machine, same day, against the dev build driven through the MCP bridge. Each figure
is the round trip as the webview saw it, so it includes the IPC both ways.

| Measurement | Before | After |
| --- | --- | --- |
| `library` | ≈ 2860 ms | **34 ms** |
| `set_settings` (one Appearance click) | ≈ 5700 ms | **36 ms** |
| `launch_tool` (ForgePact) | ≈ 8600 ms for the click | **139 ms** |
| Skin button, then Downloads on the next click | the second click landed in a dead window | **1 ms apart, both took effect** |

The `library` figure is the whole of Stage 2: `build_view` no longer probes, so what is
left is the 25 ms process snapshot plus serialization.

### That the UI thread is actually free

The point of Stage 1 is not that the work got faster — some of it cannot. It is that the
window keeps answering while it runs. Measured by holding a slow command open and timing
`hub_info`, which is still synchronous and so still runs on the thread in question:

| While this ran | For | `hub_info` round trips | Median | Worst |
| --- | --- | --- | --- | --- |
| `verify_tool` (ForgePact, hashing ~20 MB) | 1064 ms | 8 | 1 ms | 1 ms |
| `stop_tool` (killing the process tree) | 1094 ms | 40 | 1 ms | 2 ms |

Before Stage 1 every one of those would have queued behind the command and returned only
once it finished.

### That the frontend no longer asks for what it is already sent

`launch_tool` was driven with the frontend's `refresh` removed, and the card still went
from Launch to Running: three `library-changed` events arrived carrying the new PID -- the
command's own `announce`, the health-wait thread's, and the ticker's. Nothing the interface
did produced any of them.

### Still owed

**The UAC prompt.** `launch_tool` raises one through `ShellExecuteExW` with the `runas`
verb for the three elevated tools, and Stage 1 moved that call to a threadpool thread. It
has not been triggered since. It needs a person at the machine to answer the prompt, so it
is the one line of this that is signed off by a human or not at all.
