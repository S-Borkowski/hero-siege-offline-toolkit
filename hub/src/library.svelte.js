// The one copy of what the hub currently knows.
//
// Every view reads this; nothing recomputes it. In particular `update_available`
// is decided in Rust and carried here as a field, so the `0.9.10` against
// `0.9.8` comparison has exactly one implementation rather than one per screen.

import { invoke, listen } from './bridge.js';

let view = $state(null);
let loading = $state(true);
let error = $state('');
let checking = $state(false);
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
  return { loading, error, checking };
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
    error = '';
  } catch (e) {
    error = String(e?.message ?? e);
  } finally {
    loading = false;
  }
}

export async function checkForUpdates() {
  checking = true;
  error = '';
  try {
    view = await invoke('check_for_updates');
  } catch (e) {
    error = String(e?.message ?? e);
  } finally {
    checking = false;
  }
}

export async function saveSettings(next) {
  try {
    await invoke('set_settings', { settings: next });
    await refresh();
    error = '';
  } catch (e) {
    error = String(e?.message ?? e);
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
    error = '';
    if (command !== 'install_tool') await refresh();
    return result;
  } catch (e) {
    error = String(e?.message ?? e);
    throw e;
  }
}

export function dismissError() {
  error = '';
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
