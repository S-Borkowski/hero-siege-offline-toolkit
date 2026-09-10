"""Declarative Mod Registry for Toolkit Modules (ForgePact, ValueEditor, LootForge)."""

from dataclasses import dataclass
from typing import Callable, Dict, List, Optional, Any


@dataclass
class ModDefinition:
    key: str
    title: str
    description: str
    tab: str = "mods"
    default: Any = False
    control_type: str = "switch"  # "switch", "slider", "select"
    ipc_command_template: Optional[str] = None  # e.g. "relicfilter {value}"
    min_val: Optional[float] = None
    max_val: Optional[float] = None
    step: Optional[float] = None

    def format_ipc(self, val: Any) -> Optional[str]:
        if not self.ipc_command_template:
            return None
        if isinstance(val, bool):
            formatted_val = "1" if val else "0"
        else:
            formatted_val = str(val)
        return self.ipc_command_template.replace("{value}", formatted_val)


class ModRegistry:
    def __init__(self):
        self._mods: Dict[str, ModDefinition] = {}

    def register(self, mod: ModDefinition) -> ModDefinition:
        self._mods[mod.key] = mod
        return mod

    def get(self, key: str) -> Optional[ModDefinition]:
        return self._mods.get(key)

    def all_mods(self) -> List[ModDefinition]:
        return list(self._mods.values())

    def mods_for_tab(self, tab: str) -> List[ModDefinition]:
        return [m for m in self._mods.values() if m.tab == tab]


GLOBAL_MOD_REGISTRY = ModRegistry()

# Standard Built-in Mod Definitions
GLOBAL_MOD_REGISTRY.register(
    ModDefinition(
        key="mod_filter_max_relics",
        title="Remove owned relics from drop pool",
        description="When a relic is dropped, prevents relics already at maximum level (10 out of 10) in your equipped slots, backpack, or inventory from dropping.",
        tab="mods",
        default=False,
        control_type="switch",
        ipc_command_template="relicfilter {value}",
    )
)
