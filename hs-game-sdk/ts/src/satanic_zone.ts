/**
 * Satanic Zone buff/debuff tables and Controller_obj variable names.
 *
 * Hand-verified game knowledge (not mechanically extracted). Regenerate this
 * file from hs-game-sdk/curated/satanic_zone.json with
 * tools/generate_satanic_zone_sdk.py -- do not edit by hand.
 */

export interface SatanicMod {
  readonly id: number;
  readonly name: string;
  readonly description: string;
}

export const SATANIC_BUFFS: readonly SatanicMod[] = [
  { id: 1, name: 'Loot Goblin I', description: '+1 Maximum Loot from Enemy Killed' },
  { id: 2, name: 'Loot Goblin II', description: '+2 Maximum Loot from Enemy Killed' },
  { id: 3, name: 'Rune Master', description: 'Rune Drop Chance increased by 15% + (5% per sub difficulty level)' },
  { id: 4, name: 'Gold Hunger', description: 'Gold from monster kills increased by 40% + (8.75% per sub difficulty level)' },
  { id: 5, name: 'Heroic Windfall', description: 'Heroic Item drop chances increased by 3% + (3% per sub difficulty level)' },
  { id: 6, name: 'Angelic Fortune', description: 'Angelic Item drop chances increased by 4% + (6% per sub difficulty level)' },
  { id: 7, name: 'Zephy’s Grace', description: 'Movement Speed increased by 50%' },
  { id: 8, name: 'Fury of Tempest', description: 'Attack Speed increased by 60%' },
  { id: 9, name: 'Rapid Casting', description: 'Faster Cast Rate increased by 60%' },
  { id: 10, name: 'Onslaught', description: 'Attack Damage increased by 100%' },
  { id: 11, name: 'Nether Surge', description: 'Magic Skill Damage increased by 40%' },
  { id: 12, name: 'Relic Keepers', description: 'Ancient monsters have a chance to drop a relic on death' },
  { id: 13, name: 'Goblin’s Greed', description: 'Champion+ monsters have a 0.5% chance to summon a Treasure Goblin on death' },
  { id: 14, name: 'Artifact Digger', description: 'Magic Find increased by 155% + (5% per sub difficulty level)' },
  { id: 15, name: 'Artifact Seeker', description: 'Magic Find increased by 210% + (10% per sub difficulty level)' },
  { id: 16, name: 'Artifact Excavator', description: 'Magic Find increased by 370% + (20% per sub difficulty level)' },
  { id: 17, name: 'Recruit', description: '+10% Experience Gain + (2.5% per sub difficulty level)' },
  { id: 18, name: 'Combat Training', description: '+15% Experience Gain + (3.75% per sub difficulty level)' },
  { id: 19, name: 'Battle Scarred', description: '+20% Experience Gain + (5% per sub difficulty level)' },
  { id: 20, name: 'Clairvoyance', description: 'All recovery increased by 100% (Includes: Mana per hit, Life per hit, Mana and Life Replenish etc)' },
  { id: 21, name: 'Aftermath', description: 'Monsters have a 3% chance to summon a Legion version of them on death' },
  { id: 22, name: 'Deep Cuts', description: 'Critical Strike damage increased by 200%' },
  { id: 23, name: 'Old Town', description: '+15% chance for Ancient Packs' },
  { id: 24, name: 'Terror Zone', description: '+25% chance for Ancient Packs' },
  { id: 25, name: 'Fields of Carnage', description: '+30% chance for Ancient Packs' },
];

export const SATANIC_DEBUFFS: readonly SatanicMod[] = [
  { id: 1, name: 'Dusk’s Shroud', description: 'Light Radius decreased' },
  { id: 2, name: 'Elemental Erosion', description: 'All Resistances decreased' },
  { id: 3, name: 'Sundered Armor', description: 'Damage Taken increased' },
  { id: 4, name: 'Vitality Drain', description: 'Life decreased' },
  { id: 5, name: 'Essence Drain', description: 'Mana decreased' },
  { id: 6, name: 'Abyssal Gloom', description: 'Darkness increased' },
  { id: 7, name: 'Skill Debilitation', description: 'All Skills decreased' },
  { id: 8, name: 'Weakening Essence', description: 'All Attributes decreased' },
  { id: 9, name: 'Lifeflow Starvation', description: 'Life and Mana replenish decreased' },
  { id: 10, name: 'Sanguine Impairment', description: 'Life Steal decreased' },
  { id: 11, name: 'Arcane Impairment', description: 'Mana Steal decreased' },
  { id: 12, name: 'Consumed Time', description: 'Cooldown Recovery reduced' },
  { id: 13, name: 'Absolute Limbo', description: 'Cooldown Recovery reduced (the stronger tier)' },
  { id: 14, name: 'Boulder Fall', description: 'Monsters have a chance to drop a boulder from the sky when killed' },
  { id: 15, name: 'Lingering Evil', description: 'Movement Speed decreased' },
  { id: 16, name: 'Fatal Wounds', description: 'Monster Critical Strike Damage increased' },
  { id: 17, name: 'Bloated Veins', description: 'Monster life increased' },
  { id: 18, name: 'Abnormal Dwelling', description: 'Monster life increased (the stronger tier)' },
  { id: 19, name: 'Colossal Bloating', description: 'Monster life increased (the strongest tier)' },
  { id: 20, name: 'Necrosis', description: 'Life drained every second' },
  { id: 21, name: 'Venomous Presence', description: 'Poison Length increased' },
  { id: 22, name: 'Flaming Agony', description: 'Monsters unleash a Fire Nova when killed' },
  { id: 23, name: 'Unholy Agility', description: 'Monster Movement Speed and Attack Speed increased' },
  { id: 24, name: 'Broken Armor', description: 'Block Rating reduced' },
  { id: 25, name: 'Hemorrhage', description: 'Monster attacks inflict a bleed that stacks 20 times' },
  { id: 26, name: 'Crippling Slow', description: 'Monster attacks inflict a slow for 2 seconds' },
];

export const SATANIC_ZONE_VAR = 'satanicZone';
export const SATANIC_ZONE_BUFF_VAR = 'satanicZoneBuff';
export const SATANIC_ZONE_DEBUFF_VAR = 'satanicZoneDebuff';
