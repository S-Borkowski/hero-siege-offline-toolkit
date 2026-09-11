import { mount } from 'svelte';
import './theme.css';
import './skin.css';
import { wearSkin } from './skin.svelte.js';
import { chrome, invoke, listen, native, recall, remember } from './bridge.js';
import App from './App.svelte';

// The hub draws its own chrome; a WebView2 context menu over it is only ever a
// way to reload the page or open a devtools window a player did not ask for.
window.addEventListener('contextmenu', (e) => e.preventDefault());

// A panel that throws while rendering goes blank and says nothing, which has
// already cost the Tracker an evening. Everything the web side throws goes to
// the hub's log instead of a console nobody can see in a released build.
const told = new Set();
function tell(what) {
  if (told.has(what) || told.size > 40) return;
  told.add(what);
  invoke('report', { level: 'error', message: what }).catch(() => {});
}
window.addEventListener('error', (e) => {
  const where = e.filename ? ` (${e.filename}:${e.lineno}:${e.colno})` : '';
  tell(`${e.message}${where}\n${e.error?.stack ?? ''}`.trim());
});
window.addEventListener('unhandledrejection', (e) => {
  const reason = e.reason;
  tell(`unhandled rejection: ${reason?.stack ?? reason?.message ?? String(reason)}`);
});

// The skin is chosen before anything is drawn, so the window never flashes in
// the wrong colours. Asking the backend is a round trip -- long enough for one
// frame -- so the last answer is worn immediately and corrected a moment later.
function wearTheme(name) {
  const theme = ['obsidian', 'ember', 'void'].includes(name) ? name : 'obsidian';
  document.documentElement.setAttribute('data-theme', theme);
  remember('theme', theme);
  wearSkin(theme);
}
wearTheme(recall('theme', 'obsidian'));
invoke('get_settings')
  .then((s) => wearTheme(s?.theme))
  .catch(() => {});
listen('settings-changed', (e) => wearTheme(e.payload?.theme));

// Nothing drew.
//
// The hub's window is `decorations: false`, so a page that throws on its way up
// leaves a rectangle with no title bar, no close button and no drag region --
// nothing on screen to tell it from a working window, and no way out but the
// task manager. So if the interface cannot start, it says so and puts a button
// on it. Copied from the Tracker, which got this report from a real user.
function lastResort(err) {
  const said = `${err?.stack || err?.message || err}`;
  try {
    invoke('report', { level: 'error', message: `the interface did not start: ${said}` });
  } catch {
    // The bridge itself is what failed; the panel below is all that is left.
  }
  const root = document.getElementById('app') ?? document.body;
  root.innerHTML = '';
  const panel = document.createElement('div');
  panel.style.cssText =
    'font:13px/1.5 system-ui,sans-serif;box-sizing:border-box;height:100vh;padding:16px;' +
    'display:flex;flex-direction:column;gap:10px;background:#0b1320;color:#e3ebf2';
  const head = document.createElement('b');
  head.textContent = 'Hero Siege Toolkit could not start its interface.';
  const why = document.createElement('pre');
  why.style.cssText = 'flex:1;margin:0;overflow:auto;white-space:pre-wrap;font-size:11px;opacity:.75';
  why.textContent = said;
  const shut = document.createElement('button');
  shut.textContent = 'Close';
  shut.style.cssText = 'align-self:flex-start;padding:6px 14px;cursor:pointer';
  // `destroy` answers with a promise, so a refusal arrives after this function
  // has returned and a `try` around it would never see one. The only way out of
  // a window with nothing drawn in it was a button that did nothing at all.
  shut.onclick = () => {
    Promise.resolve(chrome.destroy()).catch(() => window.close());
  };
  panel.append(head, why, shut);
  root.append(panel);
  document.documentElement.style.background = '#0b1320';
}

let app;
try {
  app = mount(App, { target: document.getElementById('app') });
} catch (e) {
  lastResort(e);
}

if (native) {
  requestAnimationFrame(() => requestAnimationFrame(() => invoke('report', {
    level: 'info',
    message: 'the hub window painted',
  }).catch(() => {})));
}

export default app;
