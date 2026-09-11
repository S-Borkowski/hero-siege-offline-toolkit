// The visual language is HS Offline Tracker's, adopted rather than reinvented:
// the same three skins, the same nine-slice panel/chip/button sprites, the same
// window-chrome icons. `theme.css` is copied verbatim; this file is the Tracker's
// `skin.svelte.js` with two changes.
//
// The changes: the Tracker imports two PNGs for its backdrop and app mark, and
// the hub draws both as SVG instead. That is not a style preference -- the root
// `.gitignore` bans `*.png` outright (it exists to keep extracted game sprites
// out of the repository), and a hub whose entire art is generated needs no
// exception to it. The six icon squares Tauri demands are the only PNGs here,
// and `hub/.gitignore` re-includes exactly those.

let skin = $state('obsidian');

export function wearSkin(name) {
  skin = name === 'ember' ? 'ember' : name === 'void' ? 'void' : 'obsidian';
  if (typeof document !== 'undefined') document.documentElement.dataset.theme = skin;
}

export function currentSkin() {
  return skin;
}

// `encodeURIComponent` leaves `(`, `)` and `'` alone, and every gradient here
// refers to its own def as `url(#p)`. Unquoted inside a CSS `url(...)` those
// parentheses close the token early, and the whole sprite silently fails to
// paint -- the panel and button backgrounds just are not there, with no error
// anywhere. They show up fine in an `<img src>`, which is what makes it easy to
// miss. Encoding all three means one string works in both places.
const svg = (body, view = '0 0 64 64') => {
  // The intrinsic width/height matters: `border-image-slice` counts image
  // pixels, and a sizeless SVG leaves the browser to guess what those are.
  const [, , w, h] = view.split(' ');
  const source =
    `<svg xmlns="http://www.w3.org/2000/svg" viewBox="${view}" width="${w}" height="${h}">${body}</svg>`;
  const encoded = encodeURIComponent(source)
    .replace(/\(/g, '%28')
    .replace(/\)/g, '%29')
    .replace(/'/g, '%27');
  return `data:image/svg+xml,${encoded}`;
};

const panel = svg(`
  <defs><linearGradient id="p" x2="0" y2="1"><stop stop-color="#1a2635"/><stop offset="1" stop-color="#080d15"/></linearGradient></defs>
  <rect x="2" y="2" width="60" height="60" rx="10" fill="url(#p)" stroke="#36506b" stroke-width="2"/>
  <path d="M3 18V8a5 5 0 0 1 5-5h10M46 3h10a5 5 0 0 1 5 5v10M61 46v10a5 5 0 0 1-5 5H46M18 61H8a5 5 0 0 1-5-5V46" fill="none" stroke="#d6a64c" stroke-width="1.4"/>
`);

const chip = svg(`
  <defs><linearGradient id="c" x2="1" y2="1"><stop stop-color="#142334"/><stop offset=".55" stop-color="#0b121d"/><stop offset="1" stop-color="#171323"/></linearGradient></defs>
  <rect x="1.5" y="1.5" width="61" height="61" rx="9" fill="url(#c)" stroke="#294157" stroke-width="2"/>
`);

const button = (hover = false, down = false) => svg(`
  <defs><linearGradient id="b" x2="0" y2="1"><stop stop-color="${down ? '#172231' : hover ? '#263e52' : '#1b2b3c'}"/><stop offset="1" stop-color="${down ? '#0b111b' : hover ? '#122235' : '#0d1623'}"/></linearGradient></defs>
  <rect x="2" y="7" width="60" height="50" rx="9" fill="url(#b)" stroke="${hover ? '#62d9c0' : '#36526c'}" stroke-width="2"/>
  <path d="M12 11h40M12 53h40" stroke="${hover ? '#d6a64c' : '#806735'}" opacity=".75"/>
`);

const icon = (path, hover = false) => svg(`
  <circle cx="32" cy="32" r="27" fill="${hover ? '#20394b' : '#101c29'}" stroke="${hover ? '#63ddc4' : '#40566c'}" stroke-width="2"/>
  <path d="${path}" fill="none" stroke="${hover ? '#f4d38a' : '#d9e3ed'}" stroke-width="5" stroke-linecap="round" stroke-linejoin="round"/>
`);

// The same diamond `scripts/make-icons.py` rasterises for the taskbar, so the
// window and the tab agree about what this application looks like.
const appMark = svg(`
  <circle cx="32" cy="32" r="30" fill="#09101a" stroke="#31516a" stroke-width="2.5"/>
  <path d="M32 13L51 32 32 51 13 32z" fill="#d6a64c"/>
  <path d="M32 24l8 8-8 8-8-8z" fill="#59d6c0"/>
`);

// A backdrop with no bitmap in it: two radial washes over the obsidian ground,
// stretched across whatever the window happens to be.
const backdrop = svg(`
  <defs>
    <radialGradient id="a" cx=".18" cy="0" r=".9"><stop stop-color="#152538"/><stop offset="1" stop-color="#05080e" stop-opacity="0"/></radialGradient>
    <radialGradient id="b" cx=".92" cy="1" r=".8"><stop stop-color="#1b1630"/><stop offset="1" stop-color="#05080e" stop-opacity="0"/></radialGradient>
  </defs>
  <rect width="160" height="100" fill="#070b12"/>
  <rect width="160" height="100" fill="url(#a)"/>
  <rect width="160" height="100" fill="url(#b)"/>
`, '0 0 160 100');

const transparent = svg('');

const assets = {
  app_mark: appMark,
  backdrop,
  panel,
  chip_dark: chip,
  button: button(),
  button_hover: button(true),
  button_down: button(false, true),
  header: svg('<rect width="64" height="64" rx="12" fill="#0b1522"/><path d="M7 50V22L20 9h24l13 13v28" fill="none" stroke="#d6a64c" stroke-width="2" opacity=".7"/>'),

  // Window chrome, verbatim from the Tracker so the two applications' title
  // bars are the same title bar.
  minimize: icon('M19 34h26'),
  minimize_hover: icon('M19 34h26', true),
  maximize: icon('M21 21h22v22H21z'),
  maximize_hover: icon('M21 21h22v22H21z', true),
  close: icon('M21 21l22 22M43 21L21 43'),
  close_hover: icon('M21 21l22 22M43 21L21 43', true),

  // Sidebar and card glyphs.
  library: icon('M19 19h10v10H19zM35 19h10v10H35zM19 35h10v10H19zM35 35h10v10H35z'),
  library_hover: icon('M19 19h10v10H19zM35 19h10v10H35zM19 35h10v10H35zM35 35h10v10H35z', true),
  updates: icon('M32 45V19m0 0l-9 9m9-9l9 9'),
  updates_hover: icon('M32 45V19m0 0l-9 9m9-9l9 9', true),
  game: icon('M20 40l4-14h16l4 14M27 33h4m-2-2v4'),
  game_hover: icon('M20 40l4-14h16l4 14M27 33h4m-2-2v4', true),
  settings: icon('M32 25a7 7 0 1 0 0 14 7 7 0 0 0 0-14M32 15v5m0 24v5m17-17h-5M20 32h-5'),
  settings_hover: icon('M32 25a7 7 0 1 0 0 14 7 7 0 0 0 0-14M32 15v5m0 24v5m17-17h-5M20 32h-5', true),
  about: icon('M32 29v14m0-20v.01'),
  about_hover: icon('M32 29v14m0-20v.01', true),

  install: icon('M32 19v20m0 0l-9-9m9 9l9-9M19 45h26'),
  install_hover: icon('M32 19v20m0 0l-9-9m9 9l9-9M19 45h26', true),
  play: icon('M25 19l22 13-22 13z'),
  play_hover: icon('M25 19l22 13-22 13z', true),
  stop: icon('M22 22h20v20H22z'),
  stop_hover: icon('M22 22h20v20H22z', true),
  check_on: icon('M18 32l9 9 20-21', true),
  check_off: icon('M21 21l22 22M43 21L21 43'),
  shield: icon('M32 14l16 6v12c0 10-7 16-16 20-9-4-16-10-16-20V20z'),
  shield_gold: icon('M32 14l16 6v12c0 10-7 16-16 20-9-4-16-10-16-20V20z', true),
  folder: icon('M14 22h14l4 5h18v19H14z'),
  folder_hover: icon('M14 22h14l4 5h18v19H14z', true),
  more: icon('M22 32h.01M32 32h.01M42 32h.01'),
  more_hover: icon('M22 32h.01M32 32h.01M42 32h.01', true),
};

export function art(name) {
  // Reading the rune makes every `art()` call re-run when the skin changes, so
  // the sprites follow the palette instead of lagging a frame behind it.
  void skin;
  return assets[name] ?? transparent;
}
