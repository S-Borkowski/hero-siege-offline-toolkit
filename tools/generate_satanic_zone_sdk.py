#!/usr/bin/env python3
"""Generate hs-game-sdk Python/C++/TypeScript bindings for Satanic Zone data.

Unlike tools/extract_and_generate_sdk.py, this does NOT read Hero_Siege.exe or
data.win. hs-game-sdk/curated/satanic_zone.json is hand-verified game knowledge
(buff/debuff ids, names, descriptions; Controller_obj variable names) that no
mechanical extractor can derive -- see that file's "$schema_note" and
ForgePact/docs/satanic-zone-mods-research.md for provenance.

It lives under hs-game-sdk/curated/, NOT hs-game-sdk/data/: the latter is
gitignored repo-wide ("Extracted Raw Game Data Dumps" -- raw dumps pulled
straight from the game binary, kept off the repo). This file is hand-curated,
small, and meant to be a tracked, shared source -- putting it in data/ would
silently make it local-only and defeat that purpose.

Run after editing hs-game-sdk/curated/satanic_zone.json:

    py -3 tools/generate_satanic_zone_sdk.py
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Any, Dict, List

ROOT = Path(__file__).resolve().parent.parent
SDK_ROOT = ROOT / "hs-game-sdk"
DATA_FILE = SDK_ROOT / "curated" / "satanic_zone.json"


def load_data() -> Dict[str, Any]:
    return json.loads(DATA_FILE.read_text(encoding="utf-8"))


def _escape_py(text: str) -> str:
    return text.replace("\\", "\\\\").replace('"', '\\"')


def generate_python(data: Dict[str, Any]) -> None:
    out = SDK_ROOT / "python" / "hs_game_sdk" / "satanic_zone.py"
    buffs: List[Dict[str, Any]] = data["buffs"]
    debuffs: List[Dict[str, Any]] = data["debuffs"]
    cvars = data["controller_vars"]

    lines = [
        '"""Satanic Zone buff/debuff tables and Controller_obj variable names.',
        "",
        "Hand-verified game knowledge (not mechanically extracted). Regenerate this",
        "file from hs-game-sdk/curated/satanic_zone.json with",
        "tools/generate_satanic_zone_sdk.py -- do not edit by hand.",
        '"""',
        "from __future__ import annotations",
        "from typing import NamedTuple, Tuple",
        "",
        "",
        "class SatanicMod(NamedTuple):",
        "    id: int",
        "    name: str",
        "    description: str",
        "",
        "",
        "SATANIC_BUFFS: Tuple[SatanicMod, ...] = (",
    ]
    for b in buffs:
        lines.append(f'    SatanicMod({b["id"]}, "{_escape_py(b["name"])}", "{_escape_py(b["description"])}"),')
    lines.append(")")
    lines.append("")
    lines.append("SATANIC_DEBUFFS: Tuple[SatanicMod, ...] = (")
    for d in debuffs:
        lines.append(f'    SatanicMod({d["id"]}, "{_escape_py(d["name"])}", "{_escape_py(d["description"])}"),')
    lines.append(")")
    lines.append("")
    lines.append("# Controller_obj instance variable names (old-build fallback: global namespace).")
    lines.append(f'SATANIC_ZONE_VAR = "{cvars["zone"]}"')
    lines.append(f'SATANIC_ZONE_BUFF_VAR = "{cvars["buffs"]}"')
    lines.append(f'SATANIC_ZONE_DEBUFF_VAR = "{cvars["debuffs"]}"')
    lines.append("")

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)}")


def generate_cpp(data: Dict[str, Any]) -> None:
    out = SDK_ROOT / "cpp" / "include" / "hs_game_sdk" / "satanic_zone.hpp"
    buffs: List[Dict[str, Any]] = data["buffs"]
    debuffs: List[Dict[str, Any]] = data["debuffs"]
    cvars = data["controller_vars"]

    def esc(text: str) -> str:
        return text.replace("\\", "\\\\").replace('"', '\\"')

    lines = [
        "#pragma once",
        "// Satanic Zone buff/debuff tables and Controller_obj variable names.",
        "//",
        "// Hand-verified game knowledge (not mechanically extracted). Regenerate this",
        "// file from hs-game-sdk/curated/satanic_zone.json with",
        "// tools/generate_satanic_zone_sdk.py -- do not edit by hand.",
        "#include <cstdint>",
        "#include <string_view>",
        "#include <array>",
        "",
        "namespace HeroSiege::SatanicZone {",
        "",
        "struct Mod {",
        "    int32_t id;",
        "    std::string_view name;",
        "    std::string_view description;",
        "};",
        "",
        f"inline constexpr std::array<Mod, {len(buffs)}> kBuffs = {{{{",
    ]
    for b in buffs:
        lines.append(f'    Mod{{{b["id"]}, "{esc(b["name"])}", "{esc(b["description"])}"}},')
    lines.append("}};")
    lines.append("")
    lines.append(f"inline constexpr std::array<Mod, {len(debuffs)}> kDebuffs = {{{{")
    for d in debuffs:
        lines.append(f'    Mod{{{d["id"]}, "{esc(d["name"])}", "{esc(d["description"])}"}},')
    lines.append("}};")
    lines.append("")
    lines.append("// Controller_obj instance variable names (old-build fallback: global namespace).")
    lines.append(f'inline constexpr std::string_view kZoneVar = "{cvars["zone"]}";')
    lines.append(f'inline constexpr std::string_view kZoneBuffVar = "{cvars["buffs"]}";')
    lines.append(f'inline constexpr std::string_view kZoneDebuffVar = "{cvars["debuffs"]}";')
    lines.append("")
    lines.append("inline constexpr const Mod* FindBuff(int32_t id) noexcept {")
    lines.append("    for (const auto& m : kBuffs) if (m.id == id) return &m;")
    lines.append("    return nullptr;")
    lines.append("}")
    lines.append("")
    lines.append("inline constexpr const Mod* FindDebuff(int32_t id) noexcept {")
    lines.append("    for (const auto& m : kDebuffs) if (m.id == id) return &m;")
    lines.append("    return nullptr;")
    lines.append("}")
    lines.append("")
    lines.append("}  // namespace HeroSiege::SatanicZone")
    lines.append("")

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)}")


def generate_ts(data: Dict[str, Any]) -> None:
    out = SDK_ROOT / "ts" / "src" / "satanic_zone.ts"
    buffs: List[Dict[str, Any]] = data["buffs"]
    debuffs: List[Dict[str, Any]] = data["debuffs"]
    cvars = data["controller_vars"]

    def esc(text: str) -> str:
        return text.replace("\\", "\\\\").replace("'", "\\'")

    lines = [
        "/**",
        " * Satanic Zone buff/debuff tables and Controller_obj variable names.",
        " *",
        " * Hand-verified game knowledge (not mechanically extracted). Regenerate this",
        " * file from hs-game-sdk/curated/satanic_zone.json with",
        " * tools/generate_satanic_zone_sdk.py -- do not edit by hand.",
        " */",
        "",
        "export interface SatanicMod {",
        "  readonly id: number;",
        "  readonly name: string;",
        "  readonly description: string;",
        "}",
        "",
        "export const SATANIC_BUFFS: readonly SatanicMod[] = [",
    ]
    for b in buffs:
        lines.append(f"  {{ id: {b['id']}, name: '{esc(b['name'])}', description: '{esc(b['description'])}' }},")
    lines.append("];")
    lines.append("")
    lines.append("export const SATANIC_DEBUFFS: readonly SatanicMod[] = [")
    for d in debuffs:
        lines.append(f"  {{ id: {d['id']}, name: '{esc(d['name'])}', description: '{esc(d['description'])}' }},")
    lines.append("];")
    lines.append("")
    lines.append(f"export const SATANIC_ZONE_VAR = '{cvars['zone']}';")
    lines.append(f"export const SATANIC_ZONE_BUFF_VAR = '{cvars['buffs']}';")
    lines.append(f"export const SATANIC_ZONE_DEBUFF_VAR = '{cvars['debuffs']}';")
    lines.append("")

    out.write_text("\n".join(lines), encoding="utf-8")
    print(f"wrote {out.relative_to(ROOT)}")


def wire_python_init() -> None:
    init_path = SDK_ROOT / "python" / "hs_game_sdk" / "__init__.py"
    text = init_path.read_text(encoding="utf-8")
    if "satanic_zone" in text:
        return
    text = text.replace(
        "from .structs import (",
        "from .satanic_zone import (\n"
        "    SatanicMod,\n"
        "    SATANIC_BUFFS,\n"
        "    SATANIC_DEBUFFS,\n"
        "    SATANIC_ZONE_VAR,\n"
        "    SATANIC_ZONE_BUFF_VAR,\n"
        "    SATANIC_ZONE_DEBUFF_VAR,\n"
        ")\n"
        "from .structs import (",
        1,
    )
    text = text.replace(
        '    "PlayerInstance",\n]',
        '    "PlayerInstance",\n'
        '    "SatanicMod",\n'
        '    "SATANIC_BUFFS",\n'
        '    "SATANIC_DEBUFFS",\n'
        '    "SATANIC_ZONE_VAR",\n'
        '    "SATANIC_ZONE_BUFF_VAR",\n'
        '    "SATANIC_ZONE_DEBUFF_VAR",\n'
        "]",
        1,
    )
    init_path.write_text(text, encoding="utf-8")
    print(f"wired {init_path.relative_to(ROOT)}")


def wire_cpp_aggregator() -> None:
    hpp_path = SDK_ROOT / "cpp" / "include" / "hs_game_sdk" / "hs_game_sdk.hpp"
    text = hpp_path.read_text(encoding="utf-8")
    if "satanic_zone.hpp" in text:
        return
    text = text.replace('#include "player.hpp"', '#include "player.hpp"\n#include "satanic_zone.hpp"', 1)
    hpp_path.write_text(text, encoding="utf-8")
    print(f"wired {hpp_path.relative_to(ROOT)}")


def wire_ts_index() -> None:
    index_path = SDK_ROOT / "ts" / "src" / "index.ts"
    text = index_path.read_text(encoding="utf-8")
    if "satanic_zone" in text:
        return
    text = text.rstrip("\n") + "\nexport * from './satanic_zone';\n"
    index_path.write_text(text, encoding="utf-8")
    print(f"wired {index_path.relative_to(ROOT)}")


def main() -> None:
    data = load_data()
    generate_python(data)
    generate_cpp(data)
    generate_ts(data)
    wire_python_init()
    wire_cpp_aggregator()
    wire_ts_index()


if __name__ == "__main__":
    main()
