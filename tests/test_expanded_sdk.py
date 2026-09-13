import sys
import unittest
from pathlib import Path

_sdk_path = Path(__file__).resolve().parents[1] / "hs-game-sdk" / "python"
if _sdk_path.exists() and str(_sdk_path) not in sys.path:
    sys.path.insert(0, str(_sdk_path))

from hs_game_sdk import (
    GameObject,
    GameScript,
    StatId,
    EquipmentSlot,
    PlayerEquipment,
    scan_relic_levels,
    ModDefinition,
    ModRegistry,
    GLOBAL_MOD_REGISTRY,
)


class TestExpandedSDK(unittest.TestCase):
    def test_equipment_slots(self):
        self.assertEqual(EquipmentSlot.HELM, 0)
        self.assertEqual(EquipmentSlot.RELIC_0, 10)
        self.assertEqual(EquipmentSlot.RELIC_4, 14)
        self.assertEqual(EquipmentSlot.CHARM_0, 15)

    def test_scan_relic_levels(self):
        container = {
            "equippedItems": [
                {"b": 42, "c": 16, "o": 10},
                {"b": 99, "c": 16, "level": 7},
                {"b": 15, "c": 8, "level": 100},  # Not a relic (c=8)
            ],
            "inventory": {
                "bag1": [
                    {"relicId": 42, "relicLevel": 8},  # 10 is higher, should keep 10
                    {"b": 99, "c": 16, "o": 10},       # Now 10
                ]
            }
        }
        levels = scan_relic_levels(container)
        self.assertEqual(levels.get(42), 10)
        self.assertEqual(levels.get(99), 10)
        self.assertNotIn(15, levels)

    def test_mod_registry(self):
        mod = GLOBAL_MOD_REGISTRY.get("mod_filter_max_relics")
        self.assertIsNotNone(mod)
        self.assertEqual(mod.tab, "mods")
        self.assertEqual(mod.format_ipc(True), "relicfilter 1")
        self.assertEqual(mod.format_ipc(False), "relicfilter 0")

        custom_reg = ModRegistry()
        custom_reg.register(
            ModDefinition(
                key="test_mod",
                title="Test",
                description="Test description",
                ipc_command_template="test {value}",
            )
        )
        self.assertEqual(custom_reg.get("test_mod").format_ipc(10), "test 10")


if __name__ == "__main__":
    unittest.main()
