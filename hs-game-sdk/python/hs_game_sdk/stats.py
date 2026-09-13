"""Hero Siege Stat IDs, Proc Bundles, Buffs, and Roll Constants."""

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
