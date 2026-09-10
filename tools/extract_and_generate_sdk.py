"""Hero Siege Game SDK Generator and Asset Extractor.

Extracts all GameMaker objects, scripts, sprites, rooms, sounds, and strings
from data.win and Hero_Siege.exe, exports them to structured JSON, and generates
reusable Python, C++, and TypeScript bindings for use across all submodules.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import re
import struct
import sys
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple


def sanitize_identifier(name: str) -> str:
    """Sanitize a name to make it a valid C++/Python/TS identifier."""
    # Replace non-alphanumeric chars with underscore
    clean = re.sub(r"[^0-9a-zA-Z_]", "_", name)
    if clean and clean[0].isdigit():
        clean = "_" + clean
    if not clean:
        clean = "_unnamed"
    return clean


class GameDataExtractor:
    def __init__(self, game_bin_dir: Path):
        self.game_bin = game_bin_dir.resolve()
        self.data_win_path = self.game_bin / "data.win"
        self.exe_path = self.game_bin / "Hero_Siege.exe"

        if not self.data_win_path.exists():
            raise FileNotFoundError(f"data.win not found at {self.data_win_path}")

        self.raw = self.data_win_path.read_bytes()
        if self.raw[:4] != b"FORM":
            raise ValueError("data.win is not a valid GameMaker IFF FORM file")

        self.chunks: Dict[str, Tuple[int, int]] = {}
        self._parse_chunks()
        self._strings: Dict[int, str] = {}

    def _parse_chunks(self) -> None:
        pos = 8
        while pos < len(self.raw):
            tag = self.raw[pos:pos + 4].decode("ascii", "replace")
            size = struct.unpack_from("<I", self.raw, pos + 4)[0]
            self.chunks[tag] = (pos + 8, size)
            pos += 8 + size

    def u32(self, p: int) -> int:
        return struct.unpack_from("<I", self.raw, p)[0]

    def i32(self, p: int) -> int:
        return struct.unpack_from("<i", self.raw, p)[0]

    def get_string(self, ptr: int) -> str:
        if ptr not in self._strings:
            n = self.u32(ptr - 4)
            self._strings[ptr] = self.raw[ptr:ptr + n].decode("utf-8", "replace")
        return self._strings[ptr]

    def ptr_list(self, base: int) -> List[int]:
        n = self.u32(base)
        return list(struct.unpack_from(f"<{n}I", self.raw, base + 4))

    def extract_all(self) -> Dict[str, Any]:
        data: Dict[str, Any] = {
            "metadata": {
                "game_bin": str(self.game_bin),
                "data_win_sha256": hashlib.sha256(self.raw).hexdigest(),
                "data_win_size": len(self.raw),
            },
            "objects": self.extract_objects(),
            "scripts": self.extract_scripts(),
            "sprites": self.extract_sprites(),
            "rooms": self.extract_rooms(),
            "sounds": self.extract_sounds(),
        }

        if self.exe_path.exists():
            with self.exe_path.open("rb") as f:
                data["metadata"]["exe_sha256"] = hashlib.file_digest(f, "sha256").hexdigest()
                data["metadata"]["exe_size"] = self.exe_path.stat().st_size

        return data

    def extract_objects(self) -> List[Dict[str, Any]]:
        if "OBJT" not in self.chunks:
            return []
        objt_base, _ = self.chunks["OBJT"]
        obj_ptrs = self.ptr_list(objt_base)
        objects = []
        for idx, ptr in enumerate(obj_ptrs):
            name = self.get_string(self.u32(ptr))
            sprite_idx = self.i32(ptr + 4)
            visible = self.u32(ptr + 8)
            solid = self.u32(ptr + 12)
            depth = self.i32(ptr + 16)
            persistent = self.u32(ptr + 20)
            parent_idx = self.i32(ptr + 24)
            mask_idx = self.i32(ptr + 28)
            objects.append({
                "index": idx,
                "name": name,
                "sprite_index": sprite_idx,
                "parent_index": parent_idx,
                "depth": depth,
                "visible": bool(visible),
                "solid": bool(solid),
                "persistent": bool(persistent),
                "mask_index": mask_idx
            })
        return objects

    def extract_scripts(self) -> List[Dict[str, Any]]:
        if "SCPT" not in self.chunks:
            return []
        scpt_base, _ = self.chunks["SCPT"]
        scpt_ptrs = self.ptr_list(scpt_base)
        scripts = []
        for idx, ptr in enumerate(scpt_ptrs):
            name = self.get_string(self.u32(ptr))
            scripts.append({
                "index": idx,
                "name": name
            })
        return scripts

    def extract_sprites(self) -> List[Dict[str, Any]]:
        if "SPRT" not in self.chunks:
            return []
        sprt_base, _ = self.chunks["SPRT"]
        sprt_ptrs = self.ptr_list(sprt_base)
        sprites = []
        for idx, ptr in enumerate(sprt_ptrs):
            name = self.get_string(self.u32(ptr))
            sprites.append({
                "index": idx,
                "name": name
            })
        return sprites

    def extract_rooms(self) -> List[Dict[str, Any]]:
        if "ROOM" not in self.chunks:
            return []
        room_base, _ = self.chunks["ROOM"]
        room_ptrs = self.ptr_list(room_base)
        rooms = []
        for idx, ptr in enumerate(room_ptrs):
            name = self.get_string(self.u32(ptr))
            rooms.append({
                "index": idx,
                "name": name
            })
        return rooms

    def extract_sounds(self) -> List[Dict[str, Any]]:
        if "SOND" not in self.chunks:
            return []
        sond_base, _ = self.chunks["SOND"]
        sond_ptrs = self.ptr_list(sond_base)
        sounds = []
        for idx, ptr in enumerate(sond_ptrs):
            name = self.get_string(self.u32(ptr))
            sounds.append({
                "index": idx,
                "name": name
            })
        return sounds


def generate_python_bindings(data: Dict[str, Any], output_dir: Path) -> None:
    py_dir = output_dir / "python" / "hs_game_sdk"
    py_dir.mkdir(parents=True, exist_ok=True)

    # 1. __init__.py
    init_content = '''"""Hero Siege Game SDK.

Auto-generated bindings and models for Hero Siege GameMaker objects,
scripts, assets, and runtime structures.
"""

from .objects import GameObject, OBJECT_INDEX_TO_NAME, OBJECT_NAME_TO_INDEX
from .scripts import GameScript, SCRIPT_INDEX_TO_NAME, SCRIPT_NAME_TO_INDEX
from .rooms import GameRoom, ROOM_INDEX_TO_NAME, ROOM_NAME_TO_INDEX
from .sprites import GameSprite, SPRITE_INDEX_TO_NAME, SPRITE_NAME_TO_INDEX
from .sounds import GameSound, SOUND_INDEX_TO_NAME, SOUND_NAME_TO_INDEX
from .stats import (
    StatId,
    ProcBundle,
    PROC_FAMILIES,
    DECODED_STAT_NAMES,
    BUFF_ANGELIC_CHANCE,
)
from .structs import (
    ItemDefinitionStruct,
    ItemStatStruct,
    CraftData,
    PlayerInstance,
)

__version__ = "1.0.0"
__all__ = [
    "GameObject",
    "OBJECT_INDEX_TO_NAME",
    "OBJECT_NAME_TO_INDEX",
    "GameScript",
    "SCRIPT_INDEX_TO_NAME",
    "SCRIPT_NAME_TO_INDEX",
    "GameRoom",
    "ROOM_INDEX_TO_NAME",
    "ROOM_NAME_TO_INDEX",
    "GameSprite",
    "SPRITE_INDEX_TO_NAME",
    "SPRITE_NAME_TO_INDEX",
    "GameSound",
    "SOUND_INDEX_TO_NAME",
    "SOUND_NAME_TO_INDEX",
    "StatId",
    "ProcBundle",
    "PROC_FAMILIES",
    "DECODED_STAT_NAMES",
    "BUFF_ANGELIC_CHANCE",
    "ItemDefinitionStruct",
    "ItemStatStruct",
    "CraftData",
    "PlayerInstance",
]
'''
    (py_dir / "__init__.py").write_text(init_content, encoding="utf-8")

    # 2. objects.py
    lines = [
        '"""GameMaker GameObject enumeration and index lookup tables."""',
        "from __future__ import annotations",
        "from enum import IntEnum\n",
        "class GameObject(IntEnum):",
    ]
    name_map: Dict[str, int] = {}
    idx_map: Dict[int, str] = {}
    used_enum_names = set()

    for obj in data["objects"]:
        name = obj["name"]
        idx = obj["index"]
        enum_name = sanitize_identifier(name)
        if enum_name in used_enum_names:
            enum_name = f"{enum_name}_{idx}"
        used_enum_names.add(enum_name)
        lines.append(f"    {enum_name} = {idx}")
        name_map[name] = idx
        idx_map[idx] = name

    lines.append("\n")
    lines.append("OBJECT_NAME_TO_INDEX: dict[str, int] = " + repr(name_map))
    lines.append("OBJECT_INDEX_TO_NAME: dict[int, str] = " + repr(idx_map))
    (py_dir / "objects.py").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 3. scripts.py
    lines = [
        '"""GameMaker Script enumeration and index lookup tables."""',
        "from __future__ import annotations",
        "from enum import IntEnum\n",
        "class GameScript(IntEnum):",
    ]
    script_name_map: Dict[str, int] = {}
    script_idx_map: Dict[int, str] = {}
    used_script_enum_names = set()

    for sc in data["scripts"]:
        name = sc["name"]
        idx = sc["index"]
        enum_name = sanitize_identifier(name)
        if enum_name in used_script_enum_names:
            enum_name = f"{enum_name}_{idx}"
        used_script_enum_names.add(enum_name)
        lines.append(f"    {enum_name} = {idx}")
        script_name_map[name] = idx
        script_idx_map[idx] = name

    lines.append("\n")
    lines.append("SCRIPT_NAME_TO_INDEX: dict[str, int] = " + repr(script_name_map))
    lines.append("SCRIPT_INDEX_TO_NAME: dict[int, str] = " + repr(script_idx_map))
    (py_dir / "scripts.py").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 4. rooms.py
    lines = [
        '"""GameMaker Room enumeration and lookup tables."""',
        "from __future__ import annotations",
        "from enum import IntEnum\n",
        "class GameRoom(IntEnum):",
    ]
    room_name_map: Dict[str, int] = {}
    room_idx_map: Dict[int, str] = {}
    used_room_names = set()
    for rm in data["rooms"]:
        name = rm["name"]
        idx = rm["index"]
        enum_name = sanitize_identifier(name)
        if enum_name in used_room_names:
            enum_name = f"{enum_name}_{idx}"
        used_room_names.add(enum_name)
        lines.append(f"    {enum_name} = {idx}")
        room_name_map[name] = idx
        room_idx_map[idx] = name

    lines.append("\n")
    lines.append("ROOM_NAME_TO_INDEX: dict[str, int] = " + repr(room_name_map))
    lines.append("ROOM_INDEX_TO_NAME: dict[int, str] = " + repr(room_idx_map))
    (py_dir / "rooms.py").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 5. sprites.py
    lines = [
        '"""GameMaker Sprite enumeration and lookup tables."""',
        "from __future__ import annotations",
        "from enum import IntEnum\n",
        "class GameSprite(IntEnum):",
    ]
    spr_name_map: Dict[str, int] = {}
    spr_idx_map: Dict[int, str] = {}
    used_spr_names = set()
    for sp in data["sprites"]:
        name = sp["name"]
        idx = sp["index"]
        enum_name = sanitize_identifier(name)
        if enum_name in used_spr_names:
            enum_name = f"{enum_name}_{idx}"
        used_spr_names.add(enum_name)
        lines.append(f"    {enum_name} = {idx}")
        spr_name_map[name] = idx
        spr_idx_map[idx] = name

    lines.append("\n")
    lines.append("SPRITE_NAME_TO_INDEX: dict[str, int] = " + repr(spr_name_map))
    lines.append("SPRITE_INDEX_TO_NAME: dict[int, str] = " + repr(spr_idx_map))
    (py_dir / "sprites.py").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 6. sounds.py
    lines = [
        '"""GameMaker Sound enumeration and lookup tables."""',
        "from __future__ import annotations",
        "from enum import IntEnum\n",
        "class GameSound(IntEnum):",
    ]
    snd_name_map: Dict[str, int] = {}
    snd_idx_map: Dict[int, str] = {}
    used_snd_names = set()
    for snd in data["sounds"]:
        name = snd["name"]
        idx = snd["index"]
        enum_name = sanitize_identifier(name)
        if enum_name in used_snd_names:
            enum_name = f"{enum_name}_{idx}"
        used_snd_names.add(enum_name)
        lines.append(f"    {enum_name} = {idx}")
        snd_name_map[name] = idx
        snd_idx_map[idx] = name

    lines.append("\n")
    lines.append("SOUND_NAME_TO_INDEX: dict[str, int] = " + repr(snd_name_map))
    lines.append("SOUND_INDEX_TO_NAME: dict[int, str] = " + repr(snd_idx_map))
    (py_dir / "sounds.py").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 7. stats.py
    stats_content = '''"""Hero Siege Stat IDs, Proc Bundles, Buffs, and Roll Constants."""

from __future__ import annotations
from dataclasses import dataclass
from enum import IntEnum

BUFF_ANGELIC_CHANCE = 332

class StatId(IntEnum):
    STRENGTH = 0
    DEFENSE = 1
    SWIFTNESS = 2
    STAMINA = 3
    ENERGY = 4
    PHYSICAL_DAMAGE = 5
    FIRE_DAMAGE = 6
    ICE_DAMAGE = 7
    LIGHTNING_DAMAGE = 8
    POISON_DAMAGE = 9
    MAGIC_DAMAGE = 10
    WIND_DAMAGE = 11
    HOLY_DAMAGE = 12
    SHADOW_DAMAGE = 13
    CHAOS_DAMAGE = 14
    ATTACK_SPEED = 15
    CAST_RATE = 16
    CRITICAL_RATE = 17
    CRITICAL_DAMAGE = 18
    MOVEMENT_SPEED = 19
    MAGIC_FIND = 20
    EXTRA_GOLD = 21
    EXP_GAIN = 22
    ALL_STATS = 23
    MAX_HEALTH = 24
    MAX_MANA = 25
    HEALTH_REGEN = 26
    MANA_REGEN = 27
    MANA_PER_HIT = 28
    HEALTH_PER_HIT = 29
    LIFE_PER_KILL = 30
    MANA_PER_KILL = 31
    LIFE_LEECH = 32
    MANA_LEECH = 33
    COOLDOWN_REDUCTION = 34
    DODGE_CHANCE = 35
    BLOCK_CHANCE = 36
    DAMAGE_REDUCTION = 37
    DAMAGE_TO_BOSSES = 38
    DAMAGE_TO_ELITES = 39
    ARMOR_PENETRATION = 40
    MAGIC_PENETRATION = 41
    JEWELCRAFTING_LEVEL = 157

@dataclass(frozen=True)
class ProcBundle:
    trigger: str
    skill_id_key: int
    level_key: int
    chance_key: int

PROC_FAMILIES = {
    "when_striking": ProcBundle("When Striking", 116, 117, 118),
    "when_attacking": ProcBundle("When Attacking", 113, 114, 115),
    "after_kill": ProcBundle("After Kill", 122, 123, 124),
    "when_casting": ProcBundle("When Casting", 125, 126, 127),
    "when_struck": ProcBundle("When Struck", 185, 186, 187),
    "after_blocking": ProcBundle("After Blocking", 188, 189, 190),
}

DECODED_STAT_NAMES: dict[int, str] = {
    0: "Strength",
    1: "Defense",
    2: "Swiftness",
    3: "Stamina",
    4: "Energy",
    5: "Physical Damage",
    6: "Fire Damage",
    7: "Ice Damage",
    8: "Lightning Damage",
    9: "Poison Damage",
    10: "Magic Damage",
    11: "Wind Damage",
    12: "Holy Damage",
    13: "Shadow Damage",
    14: "Chaos Damage",
    15: "Attack Speed",
    16: "Cast Rate",
    17: "Critical Rate",
    18: "Critical Damage",
    19: "Movement Speed",
    20: "Magic Find",
    21: "Extra Gold",
    22: "Experience Gain",
    23: "All Attributes",
    24: "Max Health",
    25: "Max Mana",
    26: "Health Regen",
    27: "Mana Regen",
    28: "Mana Per Hit",
    29: "Health Per Hit",
    30: "Life Per Kill",
    31: "Mana Per Kill",
    32: "Life Leech",
    33: "Mana Leech",
    34: "Cooldown Reduction",
    35: "Dodge Chance",
    36: "Block Chance",
    37: "Damage Reduction",
    38: "Damage to Bosses",
    39: "Damage to Elites",
    40: "Armor Penetration",
    41: "Magic Penetration",
    157: "Jewelcrafting Level",
}
'''
    (py_dir / "stats.py").write_text(stats_content, encoding="utf-8")

    # 8. structs.py
    structs_content = '''"""Data structures for Hero Siege runtime items, players, and crafting."""

from __future__ import annotations
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional

@dataclass
class ItemDefinitionStruct:
    a: int = 0          # generation seed / serial
    b: int = 0          # base item ID in catalog
    c: int = 0          # isUnique (1 = unique repo, 0 = normal repo)
    j: int = 0          # subtype / weapon sub-class
    i: Optional[int] = None   # secondary seed (if applicable)
    s: Optional[int] = None   # socket generation seed
    r: int = 0          # corrupted flag
    p: int = 0          # star upgrade count
    extra: Dict[str, Any] = field(default_factory=dict)

@dataclass
class ItemStatStruct:
    stats: Dict[int, float] = field(default_factory=dict)

    def get(self, stat_id: int, default: float = 0.0) -> float:
        return self.stats.get(stat_id, default)

    def set(self, stat_id: int, value: float) -> None:
        self.stats[stat_id] = value

@dataclass
class CraftData:
    item_type: Any
    item_id: Any
    result_chance: float = 100.0
    is_unique: bool = False
    amount: int = 1
    result_type: int = 0
    tier_requirement: Optional[int] = None
    rarity_requirement: Optional[int] = None
    craft_name: Optional[str] = None
    craft_desc: Optional[str] = None
    allow_multi_craft: bool = False
    keep_item: bool = False

@dataclass
class PlayerInstance:
    instance_id: int
    object_index: int
    x: float = 0.0
    y: float = 0.0
    buffs: Dict[int, float] = field(default_factory=dict)
'''
    (py_dir / "structs.py").write_text(structs_content, encoding="utf-8")

    # 9. setup.py and pyproject.toml for standard packaging
    pyproject = '''[build-system]
requires = ["setuptools>=61.0"]
build-backend = "setuptools.build_meta"

[project]
name = "hs-game-sdk"
version = "1.0.0"
description = "Hero Siege Game SDK - GameMaker symbols, object schemas, and runtime SDK"
readme = "README.md"
requires-python = ">=3.8"
dependencies = []

[tool.setuptools.packages.find]
where = ["."]
'''
    (output_dir / "python" / "pyproject.toml").write_text(pyproject, encoding="utf-8")

    setup_py = '''from setuptools import setup, find_packages

setup(
    name="hs-game-sdk",
    version="1.0.0",
    packages=find_packages(),
)
'''
    (output_dir / "python" / "setup.py").write_text(setup_py, encoding="utf-8")
    (output_dir / "python" / "README.md").write_text("# Hero Siege Game SDK (Python)\n\nImportable SDK for Hero Siege GameMaker symbols, objects, scripts, and stat models.\n", encoding="utf-8")


def generate_cpp_bindings(data: Dict[str, Any], output_dir: Path) -> None:
    inc_dir = output_dir / "cpp" / "include" / "hs_game_sdk"
    inc_dir.mkdir(parents=True, exist_ok=True)

    # 1. objects.hpp
    lines = [
        "#pragma once",
        "#include <cstdint>",
        "#include <string_view>",
        "#include <string>",
        "#include <unordered_map>",
        "",
        "namespace HeroSiege::Objects {",
        "",
        "enum class GameObject : int32_t {",
    ]
    used_names = set()
    for obj in data["objects"]:
        name = obj["name"]
        idx = obj["index"]
        clean = sanitize_identifier(name)
        if clean in used_names:
            clean = f"{clean}_{idx}"
        used_names.add(clean)
        lines.append(f"    {clean} = {idx},")

    lines.extend([
        "};",
        "",
        "[[nodiscard]] inline std::string_view GetObjectName(GameObject obj) {",
        "    switch (obj) {",
    ])
    for obj in data["objects"]:
        name = obj["name"]
        idx = obj["index"]
        lines.append(f"        case GameObject({idx}): return \"{name}\";")
    lines.extend([
        "        default: return \"Unknown_Object\";",
        "    }",
        "}",
        "",
        "} // namespace HeroSiege::Objects",
    ])
    (inc_dir / "objects.hpp").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 2. scripts.hpp
    lines = [
        "#pragma once",
        "#include <cstdint>",
        "#include <string_view>",
        "",
        "namespace HeroSiege::Scripts {",
        "",
    ]
    used_script_names = set()
    for sc in data["scripts"]:
        name = sc["name"]
        idx = sc["index"]
        clean = sanitize_identifier(name)
        if clean in used_script_names:
            clean = f"{clean}_{idx}"
        used_script_names.add(clean)
        lines.append(f"inline constexpr std::string_view {clean} = \"{name}\";")
        lines.append(f"inline constexpr int32_t {clean}_Index = {idx};")

    lines.extend([
        "",
        "} // namespace HeroSiege::Scripts",
    ])
    (inc_dir / "scripts.hpp").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 3. rooms.hpp
    lines = [
        "#pragma once",
        "#include <cstdint>",
        "#include <string_view>",
        "",
        "namespace HeroSiege::Rooms {",
        "",
        "enum class GameRoom : int32_t {",
    ]
    used_room_names = set()
    for rm in data["rooms"]:
        name = rm["name"]
        idx = rm["index"]
        clean = sanitize_identifier(name)
        if clean in used_room_names:
            clean = f"{clean}_{idx}"
        used_room_names.add(clean)
        lines.append(f"    {clean} = {idx},")

    lines.extend([
        "};",
        "",
        "} // namespace HeroSiege::Rooms",
    ])
    (inc_dir / "rooms.hpp").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 4. yytk_helpers.hpp
    helpers = '''#pragma once
#include <string_view>
#include <vector>
#include "objects.hpp"
#include "scripts.hpp"

#ifdef __has_include
#if __has_include(<YYToolkit/YYTK_Shared.hpp>)
#include <YYToolkit/YYTK_Shared.hpp>
#define HS_SDK_HAS_YYTK 1
#endif
#endif

namespace HeroSiege::YYTK {

#ifdef HS_SDK_HAS_YYTK
using ::YYTK::RValue;
using ::YYTK::CInstance;
using ::YYTK::YYTKInterface;

inline RValue CallGameScript(YYTKInterface* yytk, std::string_view scriptName, const std::vector<RValue>& args = {}) {
    if (!yytk) return RValue();
    return yytk->CallGameScript(std::string(scriptName), args);
}

inline RValue GetInstanceVariable(YYTKInterface* yytk, const RValue& instance, std::string_view varName) {
    if (!yytk) return RValue();
    return yytk->CallBuiltin("variable_instance_get", { instance, RValue(std::string(varName)) });
}

inline void SetInstanceVariable(YYTKInterface* yytk, const RValue& instance, std::string_view varName, const RValue& value) {
    if (!yytk) return;
    yytk->CallBuiltin("variable_instance_set", { instance, RValue(std::string(varName)), value });
}

inline RValue GetGlobalVariable(YYTKInterface* yytk, std::string_view varName) {
    if (!yytk) return RValue();
    return yytk->CallBuiltin("variable_global_get", { RValue(std::string(varName)) });
}

inline void SetGlobalVariable(YYTKInterface* yytk, std::string_view varName, const RValue& value) {
    if (!yytk) return;
    yytk->CallBuiltin("variable_global_set", { RValue(std::string(varName)), value });
}
#endif

} // namespace HeroSiege::YYTK
'''
    (inc_dir / "yytk_helpers.hpp").write_text(helpers, encoding="utf-8")

    # 5. hs_game_sdk.hpp
    main_header = '''#pragma once

#include "objects.hpp"
#include "scripts.hpp"
#include "rooms.hpp"
#include "yytk_helpers.hpp"

namespace HeroSiege {
    inline constexpr int32_t BUFF_ANGELIC_CHANCE = 332;
    inline constexpr int32_t STAT_JEWELCRAFTING_LEVEL = 157;
}
'''
    (inc_dir / "hs_game_sdk.hpp").write_text(main_header, encoding="utf-8")


def generate_ts_bindings(data: Dict[str, Any], output_dir: Path) -> None:
    ts_dir = output_dir / "ts" / "src"
    ts_dir.mkdir(parents=True, exist_ok=True)

    # 1. index.ts
    index_content = '''/**
 * Hero Siege Game SDK (TypeScript / ESM)
 */

export * from './objects';
export * from './scripts';
export * from './rooms';
export * from './stats';
'''
    (ts_dir / "index.ts").write_text(index_content, encoding="utf-8")

    # 2. objects.ts
    lines = [
        "export enum GameObject {",
    ]
    used_names = set()
    for obj in data["objects"]:
        name = obj["name"]
        idx = obj["index"]
        clean = sanitize_identifier(name)
        if clean in used_names:
            clean = f"{clean}_{idx}"
        used_names.add(clean)
        lines.append(f"  {clean} = {idx},")
    lines.append("}\n")
    (ts_dir / "objects.ts").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 3. scripts.ts
    lines = [
        "export const GameScripts = {",
    ]
    used_script_names = set()
    for sc in data["scripts"]:
        name = sc["name"]
        idx = sc["index"]
        clean = sanitize_identifier(name)
        if clean in used_script_names:
            clean = f"{clean}_{idx}"
        used_script_names.add(clean)
        lines.append(f"  {clean}: '{name}',")
    lines.append("} as const;\n")
    (ts_dir / "scripts.ts").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 4. rooms.ts
    lines = [
        "export enum GameRoom {",
    ]
    used_room_names = set()
    for rm in data["rooms"]:
        name = rm["name"]
        idx = rm["index"]
        clean = sanitize_identifier(name)
        if clean in used_room_names:
            clean = f"{clean}_{idx}"
        used_room_names.add(clean)
        lines.append(f"  {clean} = {idx},")
    lines.append("}\n")
    (ts_dir / "rooms.ts").write_text("\n".join(lines) + "\n", encoding="utf-8")

    # 5. stats.ts
    stats_ts = '''export const BUFF_ANGELIC_CHANCE = 332;

export enum StatId {
  STRENGTH = 0,
  DEFENSE = 1,
  SWIFTNESS = 2,
  STAMINA = 3,
  ENERGY = 4,
  PHYSICAL_DAMAGE = 5,
  FIRE_DAMAGE = 6,
  ICE_DAMAGE = 7,
  LIGHTNING_DAMAGE = 8,
  POISON_DAMAGE = 9,
  MAGIC_DAMAGE = 10,
  WIND_DAMAGE = 11,
  HOLY_DAMAGE = 12,
  SHADOW_DAMAGE = 13,
  CHAOS_DAMAGE = 14,
  ATTACK_SPEED = 15,
  CAST_RATE = 16,
  CRITICAL_RATE = 17,
  CRITICAL_DAMAGE = 18,
  MOVEMENT_SPEED = 19,
  MAGIC_FIND = 20,
  EXTRA_GOLD = 21,
  EXP_GAIN = 22,
  ALL_STATS = 23,
  MAX_HEALTH = 24,
  MAX_MANA = 25,
  HEALTH_REGEN = 26,
  MANA_REGEN = 27,
  MANA_PER_HIT = 28,
  HEALTH_PER_HIT = 29,
  LIFE_PER_KILL = 30,
  MANA_PER_KILL = 31,
  LIFE_LEECH = 32,
  MANA_LEECH = 33,
  COOLDOWN_REDUCTION = 34,
  DODGE_CHANCE = 35,
  BLOCK_CHANCE = 36,
  DAMAGE_REDUCTION = 37,
  DAMAGE_TO_BOSSES = 38,
  DAMAGE_TO_ELITES = 39,
  ARMOR_PENETRATION = 40,
  MAGIC_PENETRATION = 41,
  JEWELCRAFTING_LEVEL = 157,
}
'''
    (ts_dir / "stats.ts").write_text(stats_ts, encoding="utf-8")

    # 6. package.json
    pkg_json = {
        "name": "@hero-siege/sdk",
        "version": "1.0.0",
        "description": "Hero Siege Game SDK TypeScript Bindings",
        "main": "dist/index.js",
        "types": "dist/index.d.ts",
        "type": "module"
    }
    (output_dir / "ts" / "package.json").write_text(json.dumps(pkg_json, indent=2) + "\n", encoding="utf-8")


def export_json_data(data: Dict[str, Any], output_dir: Path) -> None:
    data_dir = output_dir / "data"
    data_dir.mkdir(parents=True, exist_ok=True)

    (data_dir / "manifest.json").write_text(json.dumps(data["metadata"], indent=2) + "\n", encoding="utf-8")
    (data_dir / "objects.json").write_text(json.dumps(data["objects"], indent=2) + "\n", encoding="utf-8")
    (data_dir / "scripts.json").write_text(json.dumps(data["scripts"], indent=2) + "\n", encoding="utf-8")
    (data_dir / "sprites.json").write_text(json.dumps(data["sprites"], indent=2) + "\n", encoding="utf-8")
    (data_dir / "rooms.json").write_text(json.dumps(data["rooms"], indent=2) + "\n", encoding="utf-8")
    (data_dir / "sounds.json").write_text(json.dumps(data["sounds"], indent=2) + "\n", encoding="utf-8")


def main() -> None:
    parser = argparse.ArgumentParser(description="Extract game symbols and build SDK bindings.")
    parser.add_argument(
        "--game-bin",
        type=Path,
        default=Path(r"C:\Program Files (x86)\Steam\steamapps\common\HeroSiege\bin"),
        help="Path to Hero Siege game bin directory",
    )
    parser.add_argument(
        "--output-dir",
        type=Path,
        default=Path(__file__).resolve().parents[1] / "hs-game-sdk",
        help="Output directory for generated SDK and data",
    )
    args = parser.parse_args()

    print(f"Extracting game symbols from: {args.game_bin}")
    extractor = GameDataExtractor(args.game_bin)
    data = extractor.extract_all()

    print(f"Extracted:")
    print(f"  - Objects: {len(data['objects'])}")
    print(f"  - Scripts: {len(data['scripts'])}")
    print(f"  - Sprites: {len(data['sprites'])}")
    print(f"  - Rooms: {len(data['rooms'])}")
    print(f"  - Sounds: {len(data['sounds'])}")

    out_dir = args.output_dir
    print(f"Generating SDK at: {out_dir}")
    export_json_data(data, out_dir)
    generate_python_bindings(data, out_dir)
    generate_cpp_bindings(data, out_dir)
    generate_ts_bindings(data, out_dir)

    print("SDK successfully generated!")


if __name__ == "__main__":
    main()
