#pragma once
#include <vector>
#include <unordered_set>
#include <unordered_map>
#include <string>
#include "yytk_helpers.hpp"
#include "objects.hpp"

namespace HeroSiege::Player {

#ifdef HS_SDK_HAS_YYTK

using ::YYTK::YYTKInterface;
using ::YYTK::RValue;
using ::YYTK::CInstance;

/**
 * Resolves the primary local player CInstance or RValue.
 */
inline bool ResolveLocalPlayer(YYTKInterface* yytk, RValue& outPlayer) {
    if (!yytk) return false;
    try {
        CInstance* globalInst = nullptr;
        yytk->GetGlobalInstance(&globalInst);
        if (globalInst) {
            for (const char* varName : { "player", "local_player", "oPlayer", "player_obj" }) {
                if (YYTK::GlobalHasVariable(yytk, varName)) {
                    RValue cand = YYTK::GetGlobalVariable(yytk, varName);
                    if (cand.m_Kind == ::YYTK::VALUE_OBJECT && cand.m_Object) {
                        outPlayer = cand;
                        return true;
                    }
                }
            }
        }
    } catch (...) {}
    return false;
}

/**
 * Fast, non-recursive inspection of item structs, inventory arrays, and dedicated relic containers.
 * Populates outRelicLevels with relic IDs (type `b`) and their highest level/count.
 */
inline void ScanContainerForRelics(
    YYTKInterface* yytk,
    const RValue& container,
    std::unordered_map<int, int>& outRelicLevels,
    int depth = 0
) {
    if (!yytk || depth > 2) return;
    try {
        if (container.m_Kind == ::YYTK::VALUE_OBJECT && container.m_Object) {
            // 1. If this is a slot container wrapping an inner 'data' struct:
            if (YYTK::StructHasVariable(yytk, container, "data")) {
                RValue innerData = YYTK::GetStructVariable(yytk, container, "data");
                ScanContainerForRelics(yytk, innerData, outRelicLevels, depth + 1);
            }

            // 2. Direct item inspection
            int b = -1;
            int level = 0;
            bool isRelic = false;

            if (YYTK::StructHasVariable(yytk, container, "b")) {
                b = static_cast<int>(YYTK::GetStructVariable(yytk, container, "b").ToDouble());
            } else if (YYTK::StructHasVariable(yytk, container, "relicId")) {
                b = static_cast<int>(YYTK::GetStructVariable(yytk, container, "relicId").ToDouble());
            }

            if (YYTK::StructHasVariable(yytk, container, "cls")) {
                int cls = static_cast<int>(YYTK::GetStructVariable(yytk, container, "cls").ToDouble());
                if (cls == 16) isRelic = true;
            }
            if (YYTK::StructHasVariable(yytk, container, "c")) {
                int c = static_cast<int>(YYTK::GetStructVariable(yytk, container, "c").ToDouble());
                if (c == 16) isRelic = true;
            }
            if (YYTK::StructHasVariable(yytk, container, "itemType")) {
                int it = static_cast<int>(YYTK::GetStructVariable(yytk, container, "itemType").ToDouble());
                if (it == 16) isRelic = true;
            }
            if (YYTK::StructHasVariable(yytk, container, "g")) {
                int g = static_cast<int>(YYTK::GetStructVariable(yytk, container, "g").ToDouble());
                if (g >= 10 && g <= 14) isRelic = true;
            }

            // Check level / amount fields
            for (const char* lField : { "o", "level", "relicLevel", "amount", "count", "stack", "qty", "p" }) {
                if (YYTK::StructHasVariable(yytk, container, lField)) {
                    int val = static_cast<int>(YYTK::GetStructVariable(yytk, container, lField).ToDouble());
                    if (val > level) level = val;
                    if (strcmp(lField, "relicLevel") == 0) isRelic = true;
                }
            }

            // Valid relic ID range in Season 10 is 0..155
            if (b >= 0 && b < 160) {
                if (isRelic || level > 0) {
                    if (level == 0) level = 1;
                    if (outRelicLevels.find(b) == outRelicLevels.end() || level > outRelicLevels[b]) {
                        outRelicLevels[b] = level;
                    }
                }
            }
        } else if (container.m_Kind == ::YYTK::VALUE_ARRAY) {
            int len = YYTK::GetArrayLength(yytk, container);
            // Cap array length to avoid pathological tables
            if (len > 512) len = 512;
            for (int i = 0; i < len; ++i) {
                RValue child = YYTK::GetArrayElement(yytk, container, i);
                if (child.m_Kind == ::YYTK::VALUE_REAL || child.m_Kind == ::YYTK::VALUE_INT32 || child.m_Kind == ::YYTK::VALUE_INT64) {
                    // Direct numeric array indexed by relic ID
                    if (i < 160) {
                        int numVal = static_cast<int>(child.ToDouble());
                        if (numVal > 0 && (outRelicLevels.find(i) == outRelicLevels.end() || numVal > outRelicLevels[i])) {
                            outRelicLevels[i] = numVal;
                        }
                    }
                } else if (child.m_Kind == ::YYTK::VALUE_OBJECT || child.m_Kind == ::YYTK::VALUE_ARRAY) {
                    ScanContainerForRelics(yytk, child, outRelicLevels, depth + 1);
                }
            }
        }
    } catch (...) {}
}

/**
 * Returns a map of all owned relic IDs and their highest recorded level across equipped slots & inventory.
 */
inline std::unordered_map<int, int> GetOwnedRelicLevels(YYTKInterface* yytk, const RValue& player) {
    std::unordered_map<int, int> relicMap;
    if (!yytk || player.m_Kind != ::YYTK::VALUE_OBJECT) return relicMap;

    try {
        // 1. Equipped items array
        if (YYTK::InstanceHasVariable(yytk, player, "equippedItems")) {
            RValue eq = YYTK::GetInstanceVariable(yytk, player, "equippedItems");
            ScanContainerForRelics(yytk, eq, relicMap, 0);
        }

        // 2. Inventory / Bags / Dedicated Relic Tabs
        for (const char* varName : {
            "inventory", "bags", "inventory_relic_tab", "relics", "relic_array", "pRelics",
            "relic_tab", "relic_inventory", "relics_collected", "relic_levels", "relicPage"
        }) {
            if (YYTK::InstanceHasVariable(yytk, player, varName)) {
                RValue val = YYTK::GetInstanceVariable(yytk, player, varName);
                ScanContainerForRelics(yytk, val, relicMap, 0);
            }
        }
    } catch (...) {}

    return relicMap;
}

/**
 * Returns set of relic IDs that are at maximum level (>= 10).
 */
inline std::unordered_set<int> GetMaxedRelicIds(YYTKInterface* yytk, const RValue& player) {
    std::unordered_set<int> maxed;
    auto relicMap = GetOwnedRelicLevels(yytk, player);
    for (const auto& [id, lvl] : relicMap) {
        if (lvl >= 10) {
            maxed.insert(id);
        }
    }
    return maxed;
}

#endif

} // namespace HeroSiege::Player
