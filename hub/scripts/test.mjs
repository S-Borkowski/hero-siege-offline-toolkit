// `npm test` -> `cargo test`, on whichever machine this is. Windows uses Rustup's stable cargo
// directly so project paths containing spaces never pass through cmd parsing.

import { execFileSync } from 'node:child_process';
import { readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const manifest = join('src-tauri', 'Cargo.toml');

// The frontend's own tests first: they are a second of Node against a twelve-
// minute Windows build, and the bugs they cover are the ones nothing else can
// reach. Anything under `src/` matching `*.test.js` runs, and those modules are
// deliberately free of Svelte runes so the plain runner can import them.
//
// Pass explicit file paths: supported Node 20 does not expand test globs,
// and directory arguments are not portable across Node versions. Discover
// top-level src/*.test.js files here without invoking a shell.
//
// Skipped when arguments are passed, which means someone is running one Rust
// test by name and does not want the whole frontend suite in the way.
if (args.length === 0) {
  const testFiles = readdirSync(join(root, 'src'), { withFileTypes: true })
    .filter((entry) => entry.isFile() && entry.name.endsWith('.test.js'))
    .map((entry) => join('src', entry.name))
    .sort();
  if (testFiles.length === 0) throw new Error('No frontend test files found in src/.');
  execFileSync(process.execPath, ['--test', ...testFiles], { cwd: root, stdio: 'inherit' });
}

// Calling a batch file through `cmd /c` made a checkout whose path contained
// spaces stop at the first word (for example `...\\Hero Siege\\...`). Rustup's
// cargo executable can be launched directly and does not need shell quoting.
const file = process.platform === 'win32' ? join(homedir(), '.cargo', 'bin', 'cargo.exe') : 'cargo';
const argv = ['test', '--manifest-path', manifest, ...args];

execFileSync(file, argv, { cwd: root, stdio: 'inherit' });
