"""Hero Siege Game SDK.

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
from .satanic_zone import (
    SatanicMod,
    SATANIC_BUFFS,
    SATANIC_DEBUFFS,
    SATANIC_ZONE_VAR,
    SATANIC_ZONE_BUFF_VAR,
    SATANIC_ZONE_DEBUFF_VAR,
)
from .structs import (
    ItemDefinitionStruct,
    ItemStatStruct,
    CraftData,
    PlayerInstance,
)
from .player import EquipmentSlot, PlayerEquipment, scan_relic_levels
from .mod_registry import ModDefinition, ModRegistry, GLOBAL_MOD_REGISTRY

__version__ = "1.1.0"
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
    "EquipmentSlot",
    "PlayerEquipment",
    "scan_relic_levels",
    "ModDefinition",
    "ModRegistry",
    "GLOBAL_MOD_REGISTRY",
]
