"""Relic identification in the Python SDK, mirroring tests/cpp/test_sdk_player_hooks.cpp.

The two SDKs answer the same question - "which relics does this player own, and
at what level" - and a consumer that switches language must get the same answer.
REPORTED 2026-09-12 by origin's review of PR #3: the C++ scanner accepted any
item carrying a level field, so the ordinary item {b:15, c:8, level:100} came
back as maxed relic 15 while Python correctly ignored it.

These are the Python half of that pair. tests/test_cpp_sdk.py runs the C++ half
and asserts both agree on the shared fixture.
"""

import sys
import unittest
from pathlib import Path

SDK_PY_PATH = Path(__file__).resolve().parents[1] / "hs-game-sdk" / "python"
if str(SDK_PY_PATH) not in sys.path:
    sys.path.insert(0, str(SDK_PY_PATH))

from hs_game_sdk import scan_relic_levels  # noqa: E402

# Rarity tier 16 identifies a relic (docs/RUNTIME_DATA_MODELS.md).
ORDINARY_ITEM_WITH_LEVEL = {"b": 15, "c": 8, "level": 100}
REAL_MAXED_RELIC = {"b": 42, "c": 16, "o": 10}
STAR_UPGRADED_ORDINARY_ITEM = {"b": 7, "c": 6, "p": 12}
STACKED_LOW_LEVEL_RELIC = {"b": 50, "c": 16, "o": 3, "count": 99}

MAXED_RELIC_LEVEL = 10


def maxed_relic_ids(container) -> set:
    return {rid for rid, level in scan_relic_levels(container).items() if level >= MAXED_RELIC_LEVEL}


class TestRelicIdentification(unittest.TestCase):
    def test_ordinary_item_with_a_level_is_not_a_relic(self):
        """The reported case: a level field alone is not evidence of relic-ness."""
        levels = scan_relic_levels({"equippedItems": [ORDINARY_ITEM_WITH_LEVEL]})
        self.assertEqual(levels, {})
        self.assertEqual(maxed_relic_ids({"equippedItems": [ORDINARY_ITEM_WITH_LEVEL]}), set())

    def test_real_relic_is_found(self):
        levels = scan_relic_levels({"equippedItems": [REAL_MAXED_RELIC]})
        self.assertEqual(levels, {42: 10})
        self.assertEqual(maxed_relic_ids({"equippedItems": [REAL_MAXED_RELIC]}), {42})

    def test_ordinary_item_and_relic_together(self):
        container = {"equippedItems": [ORDINARY_ITEM_WITH_LEVEL, REAL_MAXED_RELIC]}
        self.assertEqual(scan_relic_levels(container), {42: 10})
        self.assertEqual(maxed_relic_ids(container), {42})

    def test_star_upgrade_count_is_not_a_relic_level(self):
        self.assertEqual(scan_relic_levels({"equippedItems": [STAR_UPGRADED_ORDINARY_ITEM]}), {})

    def test_stack_count_does_not_inflate_a_relic_level(self):
        container = {"equippedItems": [STACKED_LOW_LEVEL_RELIC]}
        self.assertEqual(scan_relic_levels(container), {50: 3})
        self.assertEqual(maxed_relic_ids(container), set())

    def test_bare_numbers_in_a_general_container_invent_nothing(self):
        self.assertEqual(scan_relic_levels({"inventory": [10, 10, 10]}), {})

    def test_relic_level_field_identifies_a_relic_without_a_rarity_tier(self):
        self.assertEqual(scan_relic_levels({"bag": [{"relicId": 88, "relicLevel": 6}]}), {88: 6})

    def test_highest_level_wins_across_containers(self):
        container = {
            "equippedItems": [{"b": 42, "c": 16, "o": 10}],
            "inventory": {"bag1": [{"relicId": 42, "relicLevel": 8}]},
        }
        self.assertEqual(scan_relic_levels(container)[42], 10)

    def test_nested_slot_wrapper_resolves(self):
        self.assertEqual(
            scan_relic_levels({"equippedItems": [{"data": REAL_MAXED_RELIC}]}),
            {42: 10},
        )


if __name__ == "__main__":
    unittest.main()
