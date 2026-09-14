// Installing the hub's own update.
//
// Neither the check nor the install is here. Both live in Rust, because
// `work_offline` has to be enforced somewhere a stale frontend cannot get past
// -- the same reasoning as `check_for_updates`.
//
// The install used to be here, on the argument that the updater hands back a
// handle carrying the download URL and the verified signature, and that the
// handle cannot cross the Rust boundary. True, and beside the point: the handle
// never had to cross it, because the side that obtains it can also be the side
// that installs. Doing that from the frontend meant Work offline covered every
// request the hub makes except the largest one it makes about itself -- turn it
// on with an update pending and "Download and install" still downloaded and
// installed. `updater:default` is no longer in the window's capability, so this
// is not merely the polite route, it is the only one.
//
// Shared rather than written twice: About and the Updates screen both offer
// this, and two copies would be two chances to disagree about what "done"
// means.

import { native, invoke } from './bridge.js';

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
    // Every refusal this can meet -- Work offline, a release that has stopped
    // being offered, a signature that does not verify -- comes back as the
    // error text from Rust, so there is one place that decides what each of
    // them says.
    await invoke('install_hub_update');
    phase = 'done';
    message = 'Installed. Restart the hub to use it.';
  } catch (e) {
    phase = 'error';
    message = String(e?.message ?? e);
  }
}
