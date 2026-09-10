# HS Game SDK Module Development Instructions

## Module Overview & Metadata

`hs-game-sdk` is the centralized, multi-language SDK and metadata library providing GameMaker objects, scripts, assets, stat IDs, and runtime struct definitions extracted directly from `Hero_Siege.exe` and `data.win`.

It serves as the unified source of truth for:
* **C++ plugins** (`ForgePact/plugin`, `HS-Offline-Tracker/aurie-producer`, `hs-stat-forge`)
* **Python tools** (`hero-siege-item-editor`, `HSSaveEditor`, `HS-Offline-Launcher`)
* **TypeScript / Web interfaces** (`HSCraftSim`, `HS-Offline-Tracker/src`)

| Item | Value |
| --- | --- |
| **Directory** | `hs-game-sdk/` |
| **Languages** | Python 3.10+, C++20, TypeScript / JavaScript |
| **Output Formats** | Python package (`hs_game_sdk`), C++ headers (`include/hs_game_sdk/`), TypeScript package (`@hero-siege/sdk`), JSON dumps |
| **Supported Game Build** | Hero Siege Season 10 (Steam / Offline) |

---

## Architecture & Directory Map

```text
hs-game-sdk/
├── data/                       # Extracted JSON databases (ignored by git for clean distribution)
│   ├── manifest.json           # Binary metadata and hashes
│   ├── objects.json            # 6,016 GameMaker Object definitions, indexes, parent hierarchy
│   ├── scripts.json            # 6,254 GML Script names and asset indexes
│   ├── sprites.json            # 32,270 Sprite indexes and names
│   ├── rooms.json              # 306 Room indexes and names
│   └── sounds.json             # 2,718 Sound indexes and names
├── curated/                    # Hand-verified game knowledge (NOT gitignored - tracked)
│   └── satanic_zone.json       # Satanic Zone buff/debuff ids/names/descriptions + Controller_obj var names
├── python/                     # Python SDK package
│   ├── hs_game_sdk/
│   │   ├── objects.py          # GameObject enum & index maps
│   │   ├── scripts.py          # GameScript enum & index maps
│   │   ├── rooms.py            # GameRoom enum & index maps
│   │   ├── sprites.py          # GameSprite enum & index maps
│   │   ├── sounds.py           # GameSound enum & index maps
│   │   ├── stats.py            # StatId enum, proc bundles (116/117/118), buff IDs (332)
│   │   ├── structs.py          # Dataclasses: ItemDefinitionStruct, ItemStatStruct, etc.
│   │   ├── player.py           # EquipmentSlot enums, PlayerEquipment, container scanners
│   │   ├── mod_registry.py     # ModDefinition & ModRegistry for declarative mods
│   │   └── satanic_zone.py     # SATANIC_BUFFS/SATANIC_DEBUFFS tuples, generated from curated/satanic_zone.json
│   ├── pyproject.toml
│   └── setup.py
├── cpp/                        # C++ Header SDK for YYToolkit / Aurie Plugins
│   └── include/hs_game_sdk/
│       ├── objects.hpp         # enum class GameObject & GetObjectName()
│       ├── scripts.hpp         # constexpr string_view script names & indexes
│       ├── rooms.hpp           # enum class GameRoom
│       ├── stats.hpp           # Stat constants & proc families
│       ├── yytk_helpers.hpp    # Typed helper wrappers for YYTKInterface
│       ├── hooks.hpp           # Declarative script hook macros (HS_INSTALL_SCRIPT_HOOK)
│       ├── player.hpp          # Player discovery, inventory & maxed relic scanners
│       ├── satanic_zone.hpp    # HeroSiege::SatanicZone::kBuffs/kDebuffs, generated from curated/satanic_zone.json
│       └── hs_game_sdk.hpp     # Main aggregate header
├── ts/                         # TypeScript / ESM SDK for web and UI modules
│   ├── src/
│   │   ├── objects.ts
│   │   ├── scripts.ts
│   │   ├── rooms.ts
│   │   ├── stats.ts
│   │   ├── player.ts
│   │   ├── satanic_zone.ts     # SATANIC_BUFFS/SATANIC_DEBUFFS, generated from curated/satanic_zone.json
│   │   └── index.ts
│   └── package.json
└── (Configured via repository root .gitignore & README.md)
```

**`data/` vs `curated/`:** `data/` is mechanically extracted straight from `Hero_Siege.exe`/`data.win`
by `tools/extract_and_generate_sdk.py` and is gitignored (see Safety section below) - it never leaves
a contributor's machine. `curated/` is hand-verified game knowledge that no extractor can derive (item
names/effect text that requires playing the game and cross-checking, not just walking a symbol table)
and IS tracked in git, generated into the same three language targets by a small sibling script,
`tools/generate_satanic_zone_sdk.py` (run it after editing a `curated/*.json` file; it is not part of
`extract_and_generate_sdk.py`'s pipeline since it has nothing to extract from a binary). `curated/`
is the pattern to extend for any future hand-verified, non-mechanically-extracted domain knowledge a
submodule needs to share - see `ForgePact/docs/satanic-zone-mods-research.md` for how `satanic_zone.json`
came to exist.

---

## OBJT Record Layout & the Object Parent Hierarchy

`objects.json` is extracted by walking the `OBJT` chunk's pointer list. This build's
runtime inserts a `managed` flag right after `visible`, which pushes every later field
4 bytes further than the pre-2022.5 layout most references describe. Offsets relative
to an object's record pointer, as measured against `data.win`
(`2fc37b1b…`, GEN8 bytecode version 17, `UILR`/`PSEM`/`PSYS`/`FEAT` chunks present):

| Offset | Field | Notes |
| --- | --- | --- |
| `+0` | name | string pointer |
| `+4` | `sprite_index` | `-1` = no sprite |
| `+8` | `visible` | bool32 |
| `+12` | `managed` | bool32 — **the inserted field**; `true` for all 6,016 objects |
| `+16` | `solid` | bool32 |
| `+20` | `depth` | i32 — `0` for every object in this build (depth is layer-driven) |
| `+24` | `persistent` | bool32 |
| `+28` | `parent_index` | i32 **object** index, `-100` = root object |
| `+32` | `mask_index` | i32 **sprite** index, `-1` = collide using `sprite_index` |
| `+36` | `uses_physics` | bool32, followed by the physics block and the 15 event lists |

Anchors that pin this layout, in case it has to be re-derived for a future game build:

* `+48`/`+52`/`+64`/`+72` hold the GameMaker physics defaults `0.5`, `0.1`, `0.1`, `0.2`.
* Parsing the tail from `+68` (physics vertex count) yields exactly 15 event lists whose
  pointers are in-chunk and ascending for all 6,016 records, and no record's parsed end
  overruns the next record's start.
* `+28` is never anything but `-100` or a valid object index, and grouping by it produces
  the families the names imply (`Collision_Prop_obj` 1,495 children, `Visual_Parent_obj` 960,
  `Player_Damage_Parent_obj` 582, …), with 840 roots and no cycles.
* `+32` reaches past the object table into the sprite table and resolves to the game's own
  mask sprites (`Abandoned_Mine_Entrance_obj` → `Abandoned_Mine_Mask_spr`).

`tests/test_object_hierarchy.py` asserts all of the above, including the three-level chain
`Quest_Act_01_Coffee_Beans_obj → Quest_Object_Parent_obj → Pickup_Parent_obj`. Before
2026-09-10 the extractor used the unshifted offsets, so `parent_index` carried the
`persistent` flag (`0`/`1` only) and `mask_index` carried the parent index; any consumer
written against a `data/objects.json` from before that date needs regenerating.

### Hierarchy lookups in the bindings

All three targets expose the parent/mask tables plus lookup helpers, so a hook can ask
"is this instance an enemy?" instead of enumerating indices (the GML `object_is_ancestor`
relation):

```python
from hs_game_sdk import get_parent_index, get_child_indices, is_descendant_of

is_descendant_of("Quest_Act_01_Coffee_Beans_obj", "Pickup_Parent_obj")  # True
len(get_child_indices("Collision_Prop_obj"))                            # 1495
```

```cpp
using namespace HeroSiege::Objects;
static_assert(IsDescendantOf(GameObject::Quest_Act_01_Coffee_Beans_obj,
                             GameObject::Pickup_Parent_obj));
std::vector<int32_t> props = GetChildObjects(static_cast<int32_t>(GameObject::Collision_Prop_obj));
```

```typescript
import { GameObject, isDescendantOf, getChildObjects } from '@hero-siege/sdk';
```

Python: `OBJECT_PARENT_INDEX`, `OBJECT_MASK_SPRITE_INDEX`, `NO_PARENT`, `NO_MASK`,
`get_parent_index`, `get_ancestor_indices`, `get_child_indices`, `get_descendant_indices`,
`is_descendant_of`, `get_mask_sprite_index`. C++: `kObjectParents`, `kObjectMasks`,
`kNoParent`, `kNoMask`, `GetParentObject`, `GetMaskSpriteIndex`, `IsDescendantOf`,
`GetChildObjects`, `GetDescendantObjects` (the first three are `constexpr`, so ancestry
checks can be `static_assert`ed). TypeScript mirrors the Python names in camelCase.

---

## Integration Workflow Across Submodules

### 1. Python Submodules (`hero-siege-item-editor`, `HSSaveEditor`, etc.)
Install in editable mode:
```powershell
py -3 -m pip install -e hs-game-sdk/python
```
Or import directly:
```python
from hs_game_sdk import GameObject, GameScript, StatId, PROC_FAMILIES, ItemDefinitionStruct
```

### 2. C++ Submodules (`ForgePact/plugin`, `HS-Offline-Tracker/aurie-producer`)
Add `hs-game-sdk/cpp/include` to the include search path and include the aggregate header:
```cpp
#include <hs_game_sdk/hs_game_sdk.hpp>

using namespace HeroSiege;

void ExampleHook() {
    auto obj = Objects::GameObject::Enemy_Parent_obj;
    std::string_view script = Scripts::gml_Script_DropItem;
}
```

### 3. TypeScript Submodules (`HSCraftSim`, `HS-Offline-Tracker` UI)
Import from the module:
```typescript
import { GameObject, GameScripts, StatId } from '@hero-siege/sdk';
```

---

## Setup, Extraction & Test Command Reference

| Command | Working Directory | Purpose | Verification Status |
| --- | --- | --- | --- |
| `py -3 tools/extract_and_generate_sdk.py --game-bin "<path-to-game-bin>"` | Workspace Root | Re-extract symbols from `data.win` and regenerate all SDK bindings | Verified |
| `py -3 tools/generate_satanic_zone_sdk.py` | Workspace Root | Regenerate `satanic_zone.py`/`.hpp`/`.ts` from `hs-game-sdk/curated/satanic_zone.json` (hand-edited, not extracted) | Verified 2026-09-10 |
| `py -3 -m unittest discover tests` | Workspace Root | Run full SDK verification test suite | Verified |
| `py -3 -m pip install -e hs-game-sdk/python` | Workspace Root | Install Python SDK in development mode | Verified |

### Never hand-edit a generated file

`tools/extract_and_generate_sdk.py` rewrites **every** file under `python/hs_game_sdk/`,
`cpp/include/hs_game_sdk/` and `ts/src/` on each run — including the ones with no extracted
content in them (`__init__.py`, `hs_game_sdk.hpp`, `yytk_helpers.hpp`, `index.ts`), which come
from string templates inside the generator. Editing those files in place works right up until
the next extraction silently reverts them. Change the template in the generator instead, then
re-run it; regeneration is idempotent, so a second run must produce no diff.

`satanic_zone.py`/`.hpp`/`.ts` are the exception: they belong to
`tools/generate_satanic_zone_sdk.py` and are regenerated from `curated/satanic_zone.json`.

---

## Safety, Git & Intellectual Property Boundaries

* **No Game Binaries / Bytecode in Git**: Root `.gitignore` excludes `data.win`, `.exe`, `.dll`, audio groups, texture pages, and raw dump folders (`hs-game-sdk/data/`, `raw/`, `extracted/`). `hs-game-sdk/curated/` is the deliberate exception to this rule: it holds hand-verified data (not extracted bytecode/assets) and is meant to be shared, so it is tracked normally.
* **Interoperability Definitions**: Distributes typed symbol names, enum IDs, and data structures necessary for interoperability and modding.
* **Idempotent Regeneration**: Extraction tooling is deterministic and can be rerun against any updated game binary to regenerate SDK bindings.
