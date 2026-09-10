"""Player instance structures, equipment slot definitions, and container scanning helpers."""

from dataclasses import dataclass, field
from enum import IntEnum
from typing import Dict, List, Optional, Set, Any
from .structs import ItemDefinitionStruct


class EquipmentSlot(IntEnum):
    HELM = 0
    CHESTPLATE = 1
    BOOTS = 2
    GLOVES = 3
    BELT = 4
    AMULET = 5
    RING_1 = 6
    RING_2 = 7
    MAIN_HAND = 8
    OFF_HAND = 9
    RELIC_0 = 10
    RELIC_1 = 11
    RELIC_2 = 12
    RELIC_3 = 13
    RELIC_4 = 14
    CHARM_0 = 15
    CHARM_1 = 16
    CHARM_2 = 17
    CHARM_3 = 18


@dataclass
class PlayerEquipment:
    helm: Optional[ItemDefinitionStruct] = None
    chestplate: Optional[ItemDefinitionStruct] = None
    boots: Optional[ItemDefinitionStruct] = None
    gloves: Optional[ItemDefinitionStruct] = None
    belt: Optional[ItemDefinitionStruct] = None
    amulet: Optional[ItemDefinitionStruct] = None
    ring1: Optional[ItemDefinitionStruct] = None
    ring2: Optional[ItemDefinitionStruct] = None
    main_hand: Optional[ItemDefinitionStruct] = None
    off_hand: Optional[ItemDefinitionStruct] = None
    relics: List[ItemDefinitionStruct] = field(default_factory=list)
    charms: List[ItemDefinitionStruct] = field(default_factory=list)


def scan_relic_levels(container: Any, out_levels: Any = None, depth: int = 0) -> Dict[int, int]:
    """Recursively parses raw python dict/json dumps of player containers to extract relic levels."""
    if out_levels is None:
        out_levels = {}
    if depth > 5:
        return out_levels

    if isinstance(container, dict):
        # Check if dict represents an item
        relic_id = container.get("b", container.get("relicId", -1))
        rarity = container.get("c", container.get("itemType", -1))
        level = int(container.get("o", container.get("level", container.get("relicLevel", 0))))
        
        if (rarity == 16 or "relicLevel" in container) and relic_id >= 0:
            if relic_id not in out_levels or level > out_levels[relic_id]:
                out_levels[int(relic_id)] = level

        for k, v in container.items():
            scan_relic_levels(v, out_levels, depth + 1)
    elif isinstance(container, list):
        for elem in container:
            scan_relic_levels(elem, out_levels, depth + 1)

    return out_levels
