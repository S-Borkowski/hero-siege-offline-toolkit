"""Data structures for Hero Siege runtime items, players, and crafting."""

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
