// The one copy of what the hub currently knows.
//
// Every view reads this; nothing recomputes it. In particular `update_available`
// is decided in Rust and carried here as a field, so the `0.9.10` against
// `0.9.8` comparison has exactly one implementation rather than one per screen.

import { invoke, listen } from './bridge.js';

let view = $state(null);
let loading = $state(true);
let checking = $state(false);

/**
 * Things that went wrong, shown as toasts until dismissed.
 *
 * This was a single `error` string on a banner at the top of the scrolling
 * content, and it was wrong twice over. The banner sat above the tool grid, so
 * an error raised by the eighth card appeared somewhere the reader had scrolled
 * past -- and `refresh`, which runs on a ten-second poll, cleared it on success,
 * so anything not read within ten seconds was gone. An error nobody can see is
 * the same as no error at all.
 *
 * So: viewport-anchored, and it stays until someone closes it. Only an action
 * the user took clears anything, and only its own entry.
 */
let notices = $state([]);
let nextNoticeId = 1;
/** Tool id -> the most recent install-progress event for it. */
let progress = $state({});
let health = $state({});

export function library() {
  return view;
}

export function tools() {
  return view?.tools ?? [];
}

export function tool(id) {
  return tools().find((t) => t.id === id) ?? null;
}

export function settings() {
  return view?.settings ?? null;
}

export function status() {
  return { loading, checking };
}

export function allNotices() {
  return notices;
}

/**
 * Add a notice, or bring an existing identical one forward.
 *
 * The dedupe matters because `refresh` polls: a backend that has stopped
 * answering would otherwise stack six copies of the same sentence a minute.
 */
export function notify(kind, text) {
  const message = String(text ?? '').trim();
  if (!message) return;
  const existing = notices.find((n) => n.text === message && n.kind === kind);
  if (existing) {
    existing.count += 1;
    existing.at = Date.now();
    return;
  }
  notices = [...notices, { id: nextNoticeId++, kind, text: message, at: Date.now(), count: 1 }];
}

export function dismissNotice(id) {
  notices = notices.filter((n) => n.id !== id);
}

export function dismissAllNotices() {
  notices = [];
}

export function progressFor(id) {
  return progress[id] ?? null;
}

export function healthFor(id) {
  return health[id] ?? null;
}

/** Everything with a newer release than the copy on disk. */
export function updates() {
  return tools().filter((t) => t.update_available);
}

/** Downloads that finished but are waiting on the game or the tool to close. */
export function staged() {
  return tools().filter((t) => t.staged);
}

/** Anything the hub is mid-way through fetching or writing. */
export function busy() {
  return Object.values(progress).filter(
    (p) => p && !['done', 'failed'].includes(p.phase),
  );
}

export async function refresh() {
  try {
    view = await invoke('library');
  } catch (e) {
    notify('error', e?.message ?? e);
  } finally {
    loading = false;
  }
}

export async function checkForUpdates() {
  checking = true;
  try {
    view = await invoke('check_for_updates');
  } catch (e) {
    notify('error', e?.message ?? e);
  } finally {
    checking = false;
  }
}

export async function saveSettings(next) {
  try {
    await invoke('set_settings', { settings: next });
    await refresh();
  } catch (e) {
    notify('error', e?.message ?? e);
  }
}

/**
 * Run a command that changes what is installed, then re-read.
 *
 * `install_tool` is the exception: it returns as soon as the worker thread is
 * spawned, and the view is refreshed by the `library-changed` event when the
 * install actually finishes.
 */
export async function act(command, args) {
  try {
    const result = await invoke(command, args);
    if (command !== 'install_tool') await refresh();
    return result;
  } catch (e) {
    notify('error', e?.message ?? e);
    throw e;
  }
}



/** Subscribe to the backend's events. Called once, from App. */
export function connect() {
  const unsubscribes = [];
  listen('library-changed', (e) => {
    view = e.payload;
    loading = false;
  }).then((off) => unsubscribes.push(off));

  listen('install-progress', (e) => {
    const payload = e.payload;
    if (!payload?.id) return;
    progress = { ...progress, [payload.id]: payload };
    if (payload.phase === 'failed') {
      // This happened on a worker thread, long after the click returned, so
      // there is no rejected promise anywhere for it to surface through.
      notify('error', `${payload.id}: ${payload.error}`);
    }
    if (payload.phase === 'done') {
      // Leave the finished row up briefly so the drawer does not blink an
      // install out of existence the instant it lands.
      const id = payload.id;
      setTimeout(() => {
        const { [id]: _gone, ...rest } = progress;
        progress = rest;
      }, 2500);
    }
  }).then((off) => unsubscribes.push(off));

  listen('tool-health', (e) => {
    const payload = e.payload;
    if (payload?.id) health = { ...health, [payload.id]: payload };
  }).then((off) => unsubscribes.push(off));

  refresh();
  return () => unsubscribes.forEach((off) => off?.());
}

/** "3m ago" for the status bar. */
export function ago(iso) {
  if (!iso) return 'never';
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return iso;
  const seconds = Math.max(0, Math.round((Date.now() - then) / 1000));
  if (seconds < 60) return 'just now';
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.round(hours / 24)}d ago`;
}

export function bytes(value) {
  if (!value && value !== 0) return '';
  if (value < 1024) return `${value} B`;
  if (value < 1024 * 1024) return `${(value / 1024).toFixed(0)} KB`;
  return `${(value / 1024 / 1024).toFixed(1)} MB`;
}
