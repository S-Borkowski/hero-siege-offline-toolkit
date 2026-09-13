// Installing the hub's own update.
//
// The *check* is not here. It lives in Rust, behind `check_hub_update`, because
// the launch check runs the same question and `work_offline` has to be enforced
// somewhere a stale frontend cannot get past -- the same reasoning as
// `check_for_updates`. What is left here is the install, and only because
// Tauri's updater hands back a handle carrying the download URL and the
// signature it just verified: that handle cannot cross the Rust boundary, so
// the side that holds it has to be the side that installs.
//
// Shared rather than written twice: About and the Updates screen both offer
// this, and two copies would be two chances to disagree about what "done"
// means.

import { native } from './bridge.js';

let phase = $state('idle'); // idle | downloading | done | error
let message = $state('');

export function hubInstall() {
  return { phase, message };
}

export function clearHubInstall() {
  phase = 'idle';
  message = '';
}

export async function installHubUpdate() {
  if (phase === 'downloading') return;
  if (!native) {
    phase = 'error';
    message = 'Only the desktop app can update itself.';
    return;
  }

  phase = 'downloading';
  message = 'Downloading…';
  try {
    const { check } = await import('@tauri-apps/plugin-updater');
    const handle = await check();
    if (!handle) {
      // The backend said there was one. Between then and now the release page
      // stopped offering it -- a draft re-drafted, a release deleted. Saying
      // so beats a silent no-op on a button that was just clicked.
      phase = 'error';
      message = 'The release is no longer being offered. Check again.';
      return;
    }
    await handle.downloadAndInstall();
    phase = 'done';
    message = 'Installed. Restart the hub to use it.';
  } catch (e) {
    phase = 'error';
    message = String(e?.message ?? e);
  }
}
