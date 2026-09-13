import sys
import unittest
from pathlib import Path

# Add hs-game-sdk/python to sys.path
SDK_PY_PATH = Path(__file__).resolve().parents[1] / "hs-game-sdk" / "python"
sys.path.insert(0, str(SDK_PY_PATH))

import hs_game_sdk
from hs_game_sdk import (
    GameObject,
    OBJECT_INDEX_TO_NAME,
    OBJECT_NAME_TO_INDEX,
    GameScript,
    SCRIPT_INDEX_TO_NAME,
    SCRIPT_NAME_TO_INDEX,
    GameRoom,
    GameSprite,
    GameSound,
    StatId,
    ProcBundle,
    PROC_FAMILIES,
    DECODED_STAT_NAMES,
    BUFF_ANGELIC_CHANCE,
    ItemDefinitionStruct,
    ItemStatStruct,
    CraftData,
    PlayerInstance,
)


class TestHsGameSdk(unittest.TestCase):
    def test_objects_exist_and_resolve(self):
        self.assertIn("Player_obj", OBJECT_NAME_TO_INDEX)
        self.assertIn("Enemy_Parent_obj", OBJECT_NAME_TO_INDEX)
        self.assertIn("Loot_Manager_obj", OBJECT_NAME_TO_INDEX)

        player_idx = OBJECT_NAME_TO_INDEX["Player_obj"]
        self.assertEqual(GameObject.Player_obj, player_idx)
        self.assertEqual(OBJECT_INDEX_TO_NAME[player_idx], "Player_obj")

        enemy_parent_idx = OBJECT_NAME_TO_INDEX["Enemy_Parent_obj"]
        self.assertEqual(GameObject.Enemy_Parent_obj, enemy_parent_idx)

    def test_scripts_exist_and_resolve(self):
        self.assertIn("gml_Script_DropItem", SCRIPT_NAME_TO_INDEX)
        self.assertIn("gml_Script_cpr_init", SCRIPT_NAME_TO_INDEX)

        drop_idx = SCRIPT_NAME_TO_INDEX["gml_Script_DropItem"]
        self.assertEqual(GameScript.gml_Script_DropItem, drop_idx)
        self.assertEqual(SCRIPT_INDEX_TO_NAME[drop_idx], "gml_Script_DropItem")

    def test_stats_and_proc_bundles(self):
        self.assertEqual(BUFF_ANGELIC_CHANCE, 332)
        self.assertEqual(StatId.JEWELCRAFTING_LEVEL, 157)
        self.assertEqual(DECODED_STAT_NAMES[0], "Strength")

        striking = PROC_FAMILIES["when_striking"]
        self.assertEqual(striking.skill_id_key, 116)
        self.assertEqual(striking.level_key, 117)
        self.assertEqual(striking.chance_key, 118)

    def test_structs(self):
        item_def = ItemDefinitionStruct(a=123456, b=2, c=1, j=13, r=0, p=5)
        self.assertEqual(item_def.a, 123456)
        self.assertEqual(item_def.b, 2)
        self.assertEqual(item_def.c, 1)
        self.assertEqual(item_def.p, 5)

        stats = ItemStatStruct()
        stats.set(116, 167)
        stats.set(117, 25)
        stats.set(118, 15)
        self.assertEqual(stats.get(116), 167)
        self.assertEqual(stats.get(999, -1), -1)


if __name__ == "__main__":
    unittest.main()
