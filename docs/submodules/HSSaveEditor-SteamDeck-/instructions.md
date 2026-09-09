# HS Steam Deck Save Editor Module Development Guide

## Module Overview & Metadata
- **Module Name:** HS Steam Deck Save Editor
- **Submodule Path:** `HSSaveEditor-SteamDeck-`
- **Reviewed Git Revision:** `65c6539255c33fb930750cf041b80ce102d48b2a` (Tag: `v1.0.0-1-g65c6539`, Commit: `Update README.txt`)
- **Revision Date:** `2026-05-09 16:52:24 +0300`
- **Source Availability:** Full application source is present as a standalone client-side HTML5/JavaScript application (`HSSaveEditor.html`), accompanied by documentation (`README.txt`) and `.gitattributes`.
- **CI / Pipeline Availability:** **Not available** (no GitHub Actions, CI configurations, or remote build pipelines exist in the repository).
- **License & Provenance:** Community distribution by falorfrozen-cmd (`falorfrozen@gmail.com`). Formal open-source license file is **not available** in the submodule; usage boundaries and offline disclaimer are documented in `README.txt` and embedded in `HSSaveEditor.html`.
- **Purpose & Scope:** Zero-dependency, client-side web application designed to decode, inspect, edit, and re-encode Hero Siege offline character save files (`herosiege<N>.hss`) directly within web browsers on Steam Deck (SteamOS) and desktop operating systems without requiring Windows executables, local runtime installations, or remote server processing.
- **Research & Context7 Status:** Context7 MCP server was not accessible in the authoring environment; all architecture, cryptographic routines, data contracts, and operational guidelines are derived from direct static analysis of `HSSaveEditor.html`, `README.txt`, and sibling repository integration sources.

---

## Architecture & Repository Map

### Repository Layout
```text
HSSaveEditor-SteamDeck-/
├── .gitattributes      # Git attributes tracking configuration
├── HSSaveEditor.html   # Complete single-file HTML5/CSS3/ES6+ save editor application
└── README.txt          # Module overview, feature summary, and safe usage guidelines
```

### Component Architecture & Data Flow
`HSSaveEditor.html` is an entirely self-contained single-page application (SPA). It requires no external stylesheets, fonts, web workers, bundlers, or remote CDN dependencies.

```text
+--------------------------------------------------------------------------------------------------+
|                            HS STEAM DECK SAVE EDITOR (Browser Runtime)                           |
|                                                                                                  |
|   +------------------------------------------------------------------------------------------+   |
|   |                              User Interface & DOM Views                                  |   |
|   |  - Save Files Sidebar (File Input Picker, Drag-and-Drop Dropzone, Character Slot List)   |   |
|   |  - Character Fields Form (Name, Class dropdown [24 classes], Level, Hero Level, XP, WH)   |   |
|   |  - Action Buttons: Raw Decoded Text, Inventory Inspector, Download .hss, Direct Save,     |   |
|   |                    Unlock All Waypoints, Unlock Inferno Difficulty                       |   |
|   |  - Modals (<dialog>): Raw Decoded Text Editor / Search, Inventory Inspector & Details     |   |
|   +-----------------------------+------------------------------+-----------------------------+   |
|                                 |                              |                                 |
|                                 v                              v                                 |
|                [ Decode Pipeline (decodeHssFile) ]   [ Encode Pipeline (encodeHssText) ]         |
|                1. Read ArrayBuffer                   1. Encode UTF-8 (TextEncoder)               |
|                2. Plain text INI check               2. Interleave null bytes (Uint8Array x2)    |
|                3. Base64 decode (atob)               3. XOR obfuscate (HSS_XOR_KEY)              |
|                4. Decompress (DecompressionStream)   4. Compress (CompressionStream 'deflate')   |
|                5. XOR de-obfuscate (HSS_XOR_KEY)     5. Base64 encode (btoa)                     |
|                6. Extract even bytes -> UTF-8 Text   6. Append trailing \0 byte                  |
|                                 |                              |                                 |
|                                 +--------------+---------------+                                 |
|                                                |                                                 |
|                                                v                                                 |
|                             [ Storage / Export Handlers ]                                        |
|                             - Direct Save: window.showSaveFilePicker (File System Access API)    |
|                             - Fallback: Blob URL download (<a download>)                         |
+------------------------------------------------+-------------------------------------------------+
                                                 |
                                                 v (User-selected file path)
+--------------------------------------------------------------------------------------------------+
| STEAM DECK / LOCAL STORAGE (e.g. Proton prefix or local directory)                              |
|   Path: ~/.local/share/Steam/steamapps/compatdata/269210/pfx/drive_c/users/steamuser/             |
|         AppData/Local/Hero_Siege/ (or hs2saves/)                                                 |
|   Files: herosiege<N>.hss, shop.ini (optional)                                                   |
+--------------------------------------------------------------------------------------------------+
```

---

## Data Contracts & Cryptographic Pipeline

### 1. The `.hss` Save File Format
Hero Siege character save files (`herosiege<N>.hss`) employ an interleaved, XOR-masked, Deflate-compressed, and Base64-encoded representation.

#### Binary XOR Key
The static 32-byte repeating XOR key constant defined in `HSSaveEditor.html`:
```javascript
const HSS_XOR_KEY = Uint8Array.from([
  0xE3,0x95,0x3D,0xB1,0x01,0x6B,0xB6,0x58,0x54,0x38,0x3F,0x46,0xA1,0x74,0x29,0xCC,
  0x45,0x45,0x51,0xF2,0xA7,0xF7,0xAB,0xB7,0x26,0xF1,0x37,0xA8,0x81,0x91,0xE6,0x7E
]);
```

#### Decoding Pipeline (`decodeHssFile(file)`)
1. **Empty Slot Detection:** Verifies that the file buffer contains non-whitespace bytes (`![0,9,10,13,32]`). If empty, raises `"This save slot is empty."`
2. **Plaintext Probe:** Tests if the file is already unencoded plaintext INI (`looksLikePlainCharacterIni`). If matching `^\[[^\]]+\]` and standard character keys, returns text directly.
3. **Base64 Decode:** Converts ASCII string to raw byte buffer (`base64ToBytes` / `atob`).
4. **Decompression:** Passes compressed bytes through the browser's native `DecompressionStream("deflate")` via a `ReadableStream` pipeline.
5. **XOR De-obfuscation:** XORs each byte with `HSS_XOR_KEY[i % 32]`.
6. **Byte De-interleaving:** GameMaker/Hero Siege stores characters in an interleaved two-byte pattern where even indices (`payload[j] = decoded[i]`, where `i = j * 2`) contain the actual ASCII/UTF-8 character data and odd indices contain `0x00`. The function extracts even-indexed bytes.
7. **Text Decoding:** Decodes the extracted byte array into a normalized UTF-8 string (`\n` line endings).

#### Encoding Pipeline (`encodeHssText(text)`)
1. **UTF-8 Encoding:** Encodes normalized INI text to UTF-8 bytes using `TextEncoder`.
2. **Byte Interleaving:** Allocates a buffer of size `payload.length * 2` and assigns `utf16ish[i * 2] = payload[i]`, leaving odd bytes as `0x00`.
3. **XOR Obfuscation:** XORs each byte with `HSS_XOR_KEY[i % 32]`.
4. **Compression:** Compresses the obfuscated buffer using native `CompressionStream("deflate")`.
5. **Base64 Encoding:** Converts compressed binary data to Base64 text chunk-by-chunk (`bytesToBase64` / `btoa`).
6. **Output Blob:** Constructs an `application/octet-stream` Blob with the Base64 string and a trailing null terminator `\0`.

### 2. INI Formatting & Field Mutation
- **Section Parsing (`getSections`, `getIniFromBody`, `setIniValue`):** Parses INI sections (`[0]`, `[4]`, `[inventory]`, etc.) using regex without destroying unrecognized keys or comments.
- **Value Formatting:** Floating-point numeric values are formatted with 6 decimal places (e.g., `"1.000000"`).
- **Class Mapping (`CLASS_ID_TO_NAME` / `CLASS_NAME_TO_ID`):** Maps class IDs 1 through 24:
  1: Viking, 2: Pyromancer, 3: Marksman, 4: Pirate, 5: Nomad, 6: Redneck, 7: Necromancer, 8: Samurai, 9: Paladin, 10: Amazon, 11: Demon Slayer, 12: Demonspawn, 13: Shaman, 14: White Mage, 15: Marauder, 16: Plague Doctor, 17: Shield Lancer, 18: Illusionist, 19: Jotunn, 20: Exo, 21: Butcher, 22: Stormweaver, 23: Bard, 24: Prophet.

### 3. Inventory Inspector & Base64 JSON Payloads
- **Inventory Inspection (`iterInventoryPayloads`, `openInspector`):** Reads the `inventory=` keys in sections `[inventory]` and `[0]`.
- **Payload Decoding (`decodeBase64Json`):** Pads and decodes Base64-encoded JSON item definitions.
- **Stat Slot Inspection (`decodeStatSlot`, `statSlots`):** Inspects item properties `s1` through `s6` (containing nested Base64 JSON stat descriptors `{a, b, n}`) and displays decoded item structures in a read-only table and modal view.

### 4. Progression & Difficulty Helpers
- **Unlock All Waypoints (`unlockAllWaypoints`):** Sets `act_1` through `act_8` and all associated `zone<Act>,<0..4>` keys in section `[0]` to `"4.000000"`.
- **Unlock Inferno Difficulty (`unlockInferno`):** Unlocks all waypoints, sets `difficulty` to `"3.000000"`, `hell_subdifficulty` to `"6.000000"`, and configures quest completion flags in section `[4]` (`questlog_chain1`, `questlog_chain8`, `questlog_chain9`, `questlog_chain12`, `questlog_chain13`, `questlog_chain17`, and difficulty clears `questlog_diff5` through `questlog_diff9`).

---

## Steam Deck & Browser Environment Specifications

### Supported Platform & Environment Constraints
- **Primary Target:** Steam Deck / SteamOS (Desktop Mode & Game Mode browser overlay).
- **Supported Browsers:**
  - Chromium / Google Chrome / Brave / Microsoft Edge (Full support: Compression Streams API + File System Access API `showSaveFilePicker`).
  - Mozilla Firefox (Supported with download fallback: Compression Streams API supported since Firefox 113; `showSaveFilePicker` is not supported, triggering automatic fallback to standard file download).
  - WebKit / Safari (Supported with download fallback on modern versions supporting Compression Streams).
- **Prerequisites & Web APIs:**
  - `window.CompressionStream` / `window.DecompressionStream` (`"deflate"` format) — **Mandatory**.
  - `window.showSaveFilePicker` — **Optional** (progressive enhancement for direct save overwriting).
  - `Blob`, `FileReader` / `ArrayBuffer`, `TextEncoder`, `TextDecoder` — **Mandatory**.

### Steam Deck File System & Proton Path Navigation
When running Hero Siege under Proton on Steam Deck, character saves reside in the Wine/Proton prefix rather than native Linux paths.

#### Proton Save Path:
```text
~/.local/share/Steam/steamapps/compatdata/269210/pfx/drive_c/users/steamuser/AppData/Local/Hero_Siege/
```
*(Or the subfolder `hs2saves/` if Season 2+ folder layout is enabled).*

#### Steam Deck File Picker Navigation Tips:
1. **Hidden Files:** In the browser file picker dialog, press `Ctrl + H` or right-click and check **Show Hidden Files** to reveal `.local`.
2. **Flatpak Sandbox Isolation:** If using Firefox or Chrome installed via Flatpak from the Discover Store, the browser runs in a sandbox and may require filesystem permissions (configurable via Flatseal: filesystem permission `xdg-data/Steam:ro` or user home access) to navigate directly into `~/.local/share/Steam`.
3. **Download Fallback Workflow:** If browser sandboxing prevents saving directly into the Proton prefix:
   - Click **Download Edited .hss**.
   - The file saves to the standard `~/Downloads/` directory.
   - Use the Steam Deck Dolphin file manager to copy the edited file into the Hero Siege save folder, replacing the original file after taking a backup.
4. **`shop.ini` Visibility:** On Steam Deck, `shop.ini` is located in the same directory but is frequently inaccessible or omitted during single-file uploads. `HSSaveEditor.html` intentionally hides `shop.ini`-backed profession inputs (`<section class="card" hidden>`) by default to protect Steam Deck users from incomplete cross-file synchronization.

---

## Operational Command Reference

Because `HSSaveEditor-SteamDeck-` is a standalone, client-side web application, no compilation, packaging, or local Node/Python servers are required.

| Command / Action | Working Directory | Shell / Platform | Prerequisites | Expected Result | Side Effects | Status |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| `python -m http.server 8000` | `HSSaveEditor-SteamDeck-/` | PowerShell / Bash | Python 3.x installed (optional) | Starts a local development HTTP server at `http://localhost:8000/HSSaveEditor.html` | Opens local port 8000 for network access | **Verified** |
| `npx serve .` | `HSSaveEditor-SteamDeck-/` | Node.js / Terminal | Node.js 18+ (optional) | Serves the directory over local HTTP | Local static HTTP server | **Inspected** |
| `Invoke-Item .\HSSaveEditor.html` / double-click file | `HSSaveEditor-SteamDeck-/` | Windows Explorer / PowerShell | Any modern web browser | Opens `HSSaveEditor.html` directly via `file://` protocol | None (Runs purely in local browser memory) | **Verified** |
| `xdg-open HSSaveEditor.html` | `HSSaveEditor-SteamDeck-/` | SteamOS / Linux Desktop | Default Linux browser configured | Opens editor in default browser | None | **Inspected** |
| `Get-FileHash -Algorithm SHA256 .\*` | `HSSaveEditor-SteamDeck-/` | PowerShell / Windows | Windows PowerShell | Computes SHA-256 checksums for submodule files | Read-only | **Verified** |

### Submodule Distribution Artifact Checksums

| File | Size (Bytes) | SHA-256 Digest |
| :--- | :--- | :--- |
| `HSSaveEditor.html` | 34,652 | `4DAFFC66BEEE46E057BCF74A9E75E885513E8AC84033EF1B31530FED5581E883` |
| `README.txt` | 1,583 | `0B22753F32E52AA0D8C88E54B020EB71D3CD29AB0CCE08BCF7C0AAF39E4AA179` |
| `.gitattributes` | 68 | `D8FB0DE4792538F93822B2C0D235604921299D5E54A3D6EC7A6CB34536E8BF1E` |

---

## Development & Modification Workflow

When modifying or enhancing `HSSaveEditor.html`:

1. **Direct File Editing:** Edit `HSSaveEditor.html` directly in any text editor.
2. **Preserve Embedded Architecture:** Do not split CSS or JavaScript into external files without providing a build script that bundles them back into a single HTML file. Single-file portability is a core requirement for Steam Deck offline use.
3. **Safe Round-Trip Verification:**
   - Always verify changes against disposable synthetic test saves.
   - Never test modifications on live personal character saves.
   - Verify that unknown sections and custom keys are preserved when editing and re-encoding.
4. **Browser Capability Compatibility:**
   - Maintain fallback paths when using modern Web APIs.
   - If utilizing `CompressionStream` or `showSaveFilePicker`, ensure graceful error messages or alternative fallback mechanisms are triggered on unsupported browsers.

---

## Automated Test Harness & Validation Status

### Test Harness Availability
- **Automated Test Harness:** **Not available** within the `HSSaveEditor-SteamDeck-` submodule checkout. No Jest, Playwright, Mocha, or Cypress test configurations exist.
- **Cross-Submodule Reference:** The Python desktop editor in `HSSaveEditor/` (`test_hs_save_editor.py`) contains comprehensive automated unit tests for the same `.hss` XOR/Deflate/Base64 cryptographic algorithm and INI data layout.

### Manual Verification & Round-Trip Scenarios
To manually validate modifications to `HSSaveEditor.html`:
1. **Round-Trip Serialization Scenario:**
   - Load a valid synthetic `.hss` file.
   - Verify character metadata is parsed and displayed in the character list and input fields.
   - Click **Download Edited .hss** without changes.
   - Verify that the downloaded file decodes to identical INI text.
2. **Field Mutation Scenario:**
   - Modify Name, Class (dropdown), Level, Hero Level, and Experience.
   - Export save file, reload the exported file, and verify all values persist accurately formatted to 6 decimal places.
3. **Progression Helpers Scenario:**
   - Click **Unlock All Waypoints** -> Verify `act_1`..`act_8` and `zone*` keys in the Raw Decoded Text dialog.
   - Click **Unlock Inferno Difficulty** -> Verify difficulty flags in section `[0]` and quest flags in section `[4]`.
4. **Inventory Inspector Scenario:**
   - Open save with populated `[inventory]` / `[0]` Base64 JSON strings.
   - Open Inventory Inspector modal and verify item list, container names, and `s1`–`s6` stat slot previews render correctly.
5. **Direct Save vs. Download Fallback Scenario:**
   - Test in Chrome/Edge: Verify `Save Directly...` invokes `showSaveFilePicker`.
   - Test in Firefox: Verify `Save Directly...` displays the explanatory alert advising the user to use `Download Edited .hss`.

---

## Troubleshooting & Common Pitfalls

- **Error: "This browser does not support CompressionStream/DecompressionStream":**
  - *Cause:* Outdated browser version (e.g., Firefox < 113 or legacy mobile browsers).
  - *Resolution:* Update browser to the latest version on SteamOS / desktop.
- **Error: "This save slot is empty" or "This file is not a supported encoded Hero Siege character save":**
  - *Cause:* Selected file is 0 bytes, corrupted, or an unrelated binary file.
  - *Resolution:* Ensure a valid `herosiege<N>.hss` file is selected.
- **Direct Save Picker Fails or is Not Allowed:**
  - *Cause:* Browser security policies restrict `showSaveFilePicker` in non-secure (`file://`) contexts in certain browser configurations, or the browser lacks API support.
  - *Resolution:* Use the **Download Edited .hss** button instead.
- **Edits Do Not Appear In-Game:**
  - *Cause:* Hero Siege was running while the save file was replaced, causing the game to overwrite files on exit or keep previous state in memory. Steam Cloud Synchronization may also revert modified files if replaced while the game process is active.
  - *Resolution:* Fully exit Hero Siege. Replace `herosiege<N>.hss` in the Proton save directory. Restart the game offline.

---

## Gaps, Traceability & Maintenance Triggers

- **Submodule Pointer & Upstream Tracking:** When `falorfrozen-cmd/HSSaveEditor-SteamDeck-` publishes updates to `HSSaveEditor.html` or documentation, update the parent submodule commit pointer and refresh checksums in this guide.
- **Game Updates & New Classes:** If Hero Siege adds new character classes beyond ID 24, update `CLASS_ID_TO_NAME` and `CLASS_NAME_TO_ID` mappings in `HSSaveEditor.html`.
- **Cross-Submodule Documentation References:**
  - Parent Repository Submodule Index: [`../README.md`](../README.md)
  - HS Save Editor (Desktop Python/Tkinter Implementation): [`../HSSaveEditor/instructions.md`](../HSSaveEditor/instructions.md)
  - Hero Siege Item Editor: [`../hero-siege-item-editor/instructions.md`](../hero-siege-item-editor/instructions.md)
  - HSCraftSim (Crafting Simulator): [`../HSCraftSim/instructions.md`](../HSCraftSim/instructions.md)
  - HS Offline Launcher: [`../HS-Offline-Launcher/instructions.md`](../HS-Offline-Launcher/instructions.md)
  - HS Offline Tracker: [`../HS-Offline-Tracker/instructions.md`](../HS-Offline-Tracker/instructions.md)
