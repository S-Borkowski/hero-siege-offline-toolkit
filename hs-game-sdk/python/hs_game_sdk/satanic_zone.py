"""Satanic Zone buff/debuff tables and Controller_obj variable names.

Hand-verified game knowledge (not mechanically extracted). Regenerate this
file from hs-game-sdk/curated/satanic_zone.json with
tools/generate_satanic_zone_sdk.py -- do not edit by hand.
"""
from __future__ import annotations
from typing import NamedTuple, Tuple


class SatanicMod(NamedTuple):
    id: int
    name: str
    description: str


SATANIC_BUFFS: Tuple[SatanicMod, ...] = (
    SatanicMod(1, "Loot Goblin I", "+1 Maximum Loot from Enemy Killed"),
    SatanicMod(2, "Loot Goblin II", "+2 Maximum Loot from Enemy Killed"),
    SatanicMod(3, "Rune Master", "Rune Drop Chance increased by 15% + (5% per sub difficulty level)"),
    SatanicMod(4, "Gold Hunger", "Gold from monster kills increased by 40% + (8.75% per sub difficulty level)"),
    SatanicMod(5, "Heroic Windfall", "Heroic Item drop chances increased by 3% + (3% per sub difficulty level)"),
    SatanicMod(6, "Angelic Fortune", "Angelic Item drop chances increased by 4% + (6% per sub difficulty level)"),
    SatanicMod(7, "Zephy’s Grace", "Movement Speed increased by 50%"),
    SatanicMod(8, "Fury of Tempest", "Attack Speed increased by 60%"),
    SatanicMod(9, "Rapid Casting", "Faster Cast Rate increased by 60%"),
    SatanicMod(10, "Onslaught", "Attack Damage increased by 100%"),
    SatanicMod(11, "Nether Surge", "Magic Skill Damage increased by 40%"),
    SatanicMod(12, "Relic Keepers", "Ancient monsters have a chance to drop a relic on death"),
    SatanicMod(13, "Goblin’s Greed", "Champion+ monsters have a 0.5% chance to summon a Treasure Goblin on death"),
    SatanicMod(14, "Artifact Digger", "Magic Find increased by 155% + (5% per sub difficulty level)"),
    SatanicMod(15, "Artifact Seeker", "Magic Find increased by 210% + (10% per sub difficulty level)"),
    SatanicMod(16, "Artifact Excavator", "Magic Find increased by 370% + (20% per sub difficulty level)"),
    SatanicMod(17, "Recruit", "+10% Experience Gain + (2.5% per sub difficulty level)"),
    SatanicMod(18, "Combat Training", "+15% Experience Gain + (3.75% per sub difficulty level)"),
    SatanicMod(19, "Battle Scarred", "+20% Experience Gain + (5% per sub difficulty level)"),
    SatanicMod(20, "Clairvoyance", "All recovery increased by 100% (Includes: Mana per hit, Life per hit, Mana and Life Replenish etc)"),
    SatanicMod(21, "Aftermath", "Monsters have a 3% chance to summon a Legion version of them on death"),
    SatanicMod(22, "Deep Cuts", "Critical Strike damage increased by 200%"),
    SatanicMod(23, "Old Town", "+15% chance for Ancient Packs"),
    SatanicMod(24, "Terror Zone", "+25% chance for Ancient Packs"),
    SatanicMod(25, "Fields of Carnage", "+30% chance for Ancient Packs"),
)

SATANIC_DEBUFFS: Tuple[SatanicMod, ...] = (
    SatanicMod(1, "Dusk’s Shroud", "Light Radius decreased"),
    SatanicMod(2, "Elemental Erosion", "All Resistances decreased"),
    SatanicMod(3, "Sundered Armor", "Damage Taken increased"),
    SatanicMod(4, "Vitality Drain", "Life decreased"),
    SatanicMod(5, "Essence Drain", "Mana decreased"),
    SatanicMod(6, "Abyssal Gloom", "Darkness increased"),
    SatanicMod(7, "Skill Debilitation", "All Skills decreased"),
    SatanicMod(8, "Weakening Essence", "All Attributes decreased"),
    SatanicMod(9, "Lifeflow Starvation", "Life and Mana replenish decreased"),
    SatanicMod(10, "Sanguine Impairment", "Life Steal decreased"),
    SatanicMod(11, "Arcane Impairment", "Mana Steal decreased"),
    SatanicMod(12, "Consumed Time", "Cooldown Recovery reduced"),
    SatanicMod(13, "Absolute Limbo", "Cooldown Recovery reduced (the stronger tier)"),
    SatanicMod(14, "Boulder Fall", "Monsters have a chance to drop a boulder from the sky when killed"),
    SatanicMod(15, "Lingering Evil", "Movement Speed decreased"),
    SatanicMod(16, "Fatal Wounds", "Monster Critical Strike Damage increased"),
    SatanicMod(17, "Bloated Veins", "Monster life increased"),
    SatanicMod(18, "Abnormal Dwelling", "Monster life increased (the stronger tier)"),
    SatanicMod(19, "Colossal Bloating", "Monster life increased (the strongest tier)"),
    SatanicMod(20, "Necrosis", "Life drained every second"),
    SatanicMod(21, "Venomous Presence", "Poison Length increased"),
    SatanicMod(22, "Flaming Agony", "Monsters unleash a Fire Nova when killed"),
    SatanicMod(23, "Unholy Agility", "Monster Movement Speed and Attack Speed increased"),
    SatanicMod(24, "Broken Armor", "Block Rating reduced"),
    SatanicMod(25, "Hemorrhage", "Monster attacks inflict a bleed that stacks 20 times"),
    SatanicMod(26, "Crippling Slow", "Monster attacks inflict a slow for 2 seconds"),
)

# Controller_obj instance variable names (old-build fallback: global namespace).
SATANIC_ZONE_VAR = "satanicZone"
SATANIC_ZONE_BUFF_VAR = "satanicZoneBuff"
SATANIC_ZONE_DEBUFF_VAR = "satanicZoneDebuff"
