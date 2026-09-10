#pragma once
// Satanic Zone buff/debuff tables and Controller_obj variable names.
//
// Hand-verified game knowledge (not mechanically extracted). Regenerate this
// file from hs-game-sdk/curated/satanic_zone.json with
// tools/generate_satanic_zone_sdk.py -- do not edit by hand.
#include <cstdint>
#include <string_view>
#include <array>

namespace HeroSiege::SatanicZone {

struct Mod {
    int32_t id;
    std::string_view name;
    std::string_view description;
};

inline constexpr std::array<Mod, 25> kBuffs = {{
    Mod{1, "Loot Goblin I", "+1 Maximum Loot from Enemy Killed"},
    Mod{2, "Loot Goblin II", "+2 Maximum Loot from Enemy Killed"},
    Mod{3, "Rune Master", "Rune Drop Chance increased by 15% + (5% per sub difficulty level)"},
    Mod{4, "Gold Hunger", "Gold from monster kills increased by 40% + (8.75% per sub difficulty level)"},
    Mod{5, "Heroic Windfall", "Heroic Item drop chances increased by 3% + (3% per sub difficulty level)"},
    Mod{6, "Angelic Fortune", "Angelic Item drop chances increased by 4% + (6% per sub difficulty level)"},
    Mod{7, "Zephy’s Grace", "Movement Speed increased by 50%"},
    Mod{8, "Fury of Tempest", "Attack Speed increased by 60%"},
    Mod{9, "Rapid Casting", "Faster Cast Rate increased by 60%"},
    Mod{10, "Onslaught", "Attack Damage increased by 100%"},
    Mod{11, "Nether Surge", "Magic Skill Damage increased by 40%"},
    Mod{12, "Relic Keepers", "Ancient monsters have a chance to drop a relic on death"},
    Mod{13, "Goblin’s Greed", "Champion+ monsters have a 0.5% chance to summon a Treasure Goblin on death"},
    Mod{14, "Artifact Digger", "Magic Find increased by 155% + (5% per sub difficulty level)"},
    Mod{15, "Artifact Seeker", "Magic Find increased by 210% + (10% per sub difficulty level)"},
    Mod{16, "Artifact Excavator", "Magic Find increased by 370% + (20% per sub difficulty level)"},
    Mod{17, "Recruit", "+10% Experience Gain + (2.5% per sub difficulty level)"},
    Mod{18, "Combat Training", "+15% Experience Gain + (3.75% per sub difficulty level)"},
    Mod{19, "Battle Scarred", "+20% Experience Gain + (5% per sub difficulty level)"},
    Mod{20, "Clairvoyance", "All recovery increased by 100% (Includes: Mana per hit, Life per hit, Mana and Life Replenish etc)"},
    Mod{21, "Aftermath", "Monsters have a 3% chance to summon a Legion version of them on death"},
    Mod{22, "Deep Cuts", "Critical Strike damage increased by 200%"},
    Mod{23, "Old Town", "+15% chance for Ancient Packs"},
    Mod{24, "Terror Zone", "+25% chance for Ancient Packs"},
    Mod{25, "Fields of Carnage", "+30% chance for Ancient Packs"},
}};

inline constexpr std::array<Mod, 26> kDebuffs = {{
    Mod{1, "Dusk’s Shroud", "Light Radius decreased"},
    Mod{2, "Elemental Erosion", "All Resistances decreased"},
    Mod{3, "Sundered Armor", "Damage Taken increased"},
    Mod{4, "Vitality Drain", "Life decreased"},
    Mod{5, "Essence Drain", "Mana decreased"},
    Mod{6, "Abyssal Gloom", "Darkness increased"},
    Mod{7, "Skill Debilitation", "All Skills decreased"},
    Mod{8, "Weakening Essence", "All Attributes decreased"},
    Mod{9, "Lifeflow Starvation", "Life and Mana replenish decreased"},
    Mod{10, "Sanguine Impairment", "Life Steal decreased"},
    Mod{11, "Arcane Impairment", "Mana Steal decreased"},
    Mod{12, "Consumed Time", "Cooldown Recovery reduced"},
    Mod{13, "Absolute Limbo", "Cooldown Recovery reduced (the stronger tier)"},
    Mod{14, "Boulder Fall", "Monsters have a chance to drop a boulder from the sky when killed"},
    Mod{15, "Lingering Evil", "Movement Speed decreased"},
    Mod{16, "Fatal Wounds", "Monster Critical Strike Damage increased"},
    Mod{17, "Bloated Veins", "Monster life increased"},
    Mod{18, "Abnormal Dwelling", "Monster life increased (the stronger tier)"},
    Mod{19, "Colossal Bloating", "Monster life increased (the strongest tier)"},
    Mod{20, "Necrosis", "Life drained every second"},
    Mod{21, "Venomous Presence", "Poison Length increased"},
    Mod{22, "Flaming Agony", "Monsters unleash a Fire Nova when killed"},
    Mod{23, "Unholy Agility", "Monster Movement Speed and Attack Speed increased"},
    Mod{24, "Broken Armor", "Block Rating reduced"},
    Mod{25, "Hemorrhage", "Monster attacks inflict a bleed that stacks 20 times"},
    Mod{26, "Crippling Slow", "Monster attacks inflict a slow for 2 seconds"},
}};

// Controller_obj instance variable names (old-build fallback: global namespace).
inline constexpr std::string_view kZoneVar = "satanicZone";
inline constexpr std::string_view kZoneBuffVar = "satanicZoneBuff";
inline constexpr std::string_view kZoneDebuffVar = "satanicZoneDebuff";

inline constexpr const Mod* FindBuff(int32_t id) noexcept {
    for (const auto& m : kBuffs) if (m.id == id) return &m;
    return nullptr;
}

inline constexpr const Mod* FindDebuff(int32_t id) noexcept {
    for (const auto& m : kDebuffs) if (m.id == id) return &m;
    return nullptr;
}

}  // namespace HeroSiege::SatanicZone
