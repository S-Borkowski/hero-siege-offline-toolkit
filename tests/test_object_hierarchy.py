"""Regression tests for the OBJT parent/mask fields.

The extractor originally read the OBJT record with the pre-2022.5 GameMaker
layout, which is off by one 4-byte field for this build: the runtime inserts a
`managed` flag after `visible`. That made `parent_index` read the `persistent`
flag (only ever 0/1) and `mask_index` read the parent object index. These tests
pin the corrected meaning so the shift cannot silently come back.

Measured anchors (see tools/extract_and_generate_sdk.py for the full layout):
  * Quest_Act_01_Coffee_Beans_obj -> Quest_Object_Parent_obj -> Pickup_Parent_obj
    (a root), a three-level chain that only resolves with the right offset.
  * Parents are always -100 (no parent) or a valid *object* index.
  * Masks are always -1 or a valid *sprite* index, and resolve to sprites whose
    names carry the game's own "_Mask_spr" convention.
"""

import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SDK_PY_PATH = ROOT / "hs-game-sdk" / "python"
if str(SDK_PY_PATH) not in sys.path:
    sys.path.insert(0, str(SDK_PY_PATH))

from hs_game_sdk import (  # noqa: E402
    NO_MASK,
    NO_PARENT,
    OBJECT_INDEX_TO_NAME,
    OBJECT_MASK_SPRITE_INDEX,
    OBJECT_NAME_TO_INDEX,
    OBJECT_PARENT_INDEX,
    SPRITE_INDEX_TO_NAME,
    get_ancestor_indices,
    get_child_indices,
    get_descendant_indices,
    get_parent_index,
    is_descendant_of,
)

OBJECTS_JSON = ROOT / "hs-game-sdk" / "data" / "objects.json"

# The known-good chain, by name so a failure reads as a game concept.
COFFEE_BEANS = "Quest_Act_01_Coffee_Beans_obj"
QUEST_OBJECT_PARENT = "Quest_Object_Parent_obj"
PICKUP_PARENT = "Pickup_Parent_obj"


class TestObjectParentChain(unittest.TestCase):
    """Hierarchy assertions against the generated Python bindings."""

    def test_coffee_beans_parent_chain(self):
        self.assertEqual(
            get_parent_index(COFFEE_BEANS),
            OBJECT_NAME_TO_INDEX[QUEST_OBJECT_PARENT],
        )
        self.assertEqual(
            get_parent_index(QUEST_OBJECT_PARENT),
            OBJECT_NAME_TO_INDEX[PICKUP_PARENT],
        )
        self.assertIsNone(
            get_parent_index(PICKUP_PARENT),
            f"{PICKUP_PARENT} should be a root object",
        )

        chain = [OBJECT_INDEX_TO_NAME[i] for i in get_ancestor_indices(COFFEE_BEANS)]
        self.assertEqual(chain, [QUEST_OBJECT_PARENT, PICKUP_PARENT])

        self.assertTrue(is_descendant_of(COFFEE_BEANS, PICKUP_PARENT))
        self.assertFalse(is_descendant_of(PICKUP_PARENT, COFFEE_BEANS))

    def test_parent_indices_are_valid_object_indices(self):
        max_index = max(OBJECT_INDEX_TO_NAME)
        for child, parent in OBJECT_PARENT_INDEX.items():
            self.assertLessEqual(
                parent,
                max_index,
                f"{OBJECT_INDEX_TO_NAME[child]} has parent {parent} > max object index {max_index}",
            )
            self.assertGreaterEqual(parent, 0)
            self.assertIn(parent, OBJECT_INDEX_TO_NAME)

    def test_parent_field_is_not_a_boolean(self):
        """Guards the exact regression: the old offset read a 0/1 flag."""
        distinct = set(OBJECT_PARENT_INDEX.values())
        self.assertGreater(
            len(distinct),
            2,
            "parent_index looks like a boolean flag - the OBJT field offset has shifted back",
        )
        self.assertFalse(distinct <= {0, 1})

    def test_no_parent_cycles(self):
        for index in OBJECT_PARENT_INDEX:
            seen = {index}
            current = OBJECT_PARENT_INDEX.get(index)
            while current is not None:
                self.assertNotIn(
                    current,
                    seen,
                    f"parent cycle reached from {OBJECT_INDEX_TO_NAME[index]}",
                )
                seen.add(current)
                current = OBJECT_PARENT_INDEX.get(current)

    def test_known_parent_families_resolve(self):
        """Grouping by parent reproduces the families implied by the names."""
        for family, minimum in [
            ("Collision_Prop_obj", 1000),
            ("Visual_Parent_obj", 500),
            ("Player_Damage_Parent_obj", 100),
            ("Enemy_Child_Basic_obj", 100),
            ("UI_Parent_obj", 100),
        ]:
            with self.subTest(family=family):
                self.assertGreaterEqual(len(get_child_indices(family)), minimum)

        # Every child of a "*_Parent_obj" family really points back at it.
        pickup_parent = OBJECT_NAME_TO_INDEX[PICKUP_PARENT]
        for descendant in get_descendant_indices(PICKUP_PARENT):
            self.assertTrue(is_descendant_of(descendant, pickup_parent))

    def test_mask_indices_are_valid_sprite_indices(self):
        max_sprite = max(SPRITE_INDEX_TO_NAME)
        for obj, mask in OBJECT_MASK_SPRITE_INDEX.items():
            self.assertNotEqual(mask, NO_MASK)
            self.assertIn(
                mask,
                SPRITE_INDEX_TO_NAME,
                f"{OBJECT_INDEX_TO_NAME[obj]} mask {mask} is not a sprite index (max {max_sprite})",
            )

    def test_masks_reach_beyond_the_object_table(self):
        """Masks index sprites, so some must exceed the highest object index."""
        max_object = max(OBJECT_INDEX_TO_NAME)
        self.assertTrue(
            any(mask > max_object for mask in OBJECT_MASK_SPRITE_INDEX.values()),
            "no mask exceeds the object count - mask_index may be reading the parent field",
        )

    def test_sentinels(self):
        self.assertEqual(NO_PARENT, -100)
        self.assertEqual(NO_MASK, -1)
        self.assertNotIn(NO_PARENT, OBJECT_PARENT_INDEX.values())


@unittest.skipUnless(
    OBJECTS_JSON.exists(),
    "hs-game-sdk/data is gitignored; run tools/extract_and_generate_sdk.py first",
)
class TestObjectsJsonMatchesBindings(unittest.TestCase):
    """Cross-check the extracted JSON against the generated bindings."""

    @classmethod
    def setUpClass(cls):
        cls.objects = json.loads(OBJECTS_JSON.read_text(encoding="utf-8"))

    def test_json_parent_and_mask_match_bindings(self):
        for obj in self.objects:
            index = obj["index"]
            expected_parent = OBJECT_PARENT_INDEX.get(index, NO_PARENT)
            expected_mask = OBJECT_MASK_SPRITE_INDEX.get(index, NO_MASK)
            self.assertEqual(obj["parent_index"], expected_parent, obj["name"])
            self.assertEqual(obj["mask_index"], expected_mask, obj["name"])

    def test_no_parent_index_exceeds_max_object_index(self):
        max_index = max(o["index"] for o in self.objects)
        for obj in self.objects:
            parent = obj["parent_index"]
            if parent == NO_PARENT:
                continue
            self.assertTrue(
                0 <= parent <= max_index,
                f"{obj['name']} has out-of-range parent_index {parent} (max {max_index})",
            )

    def test_mask_resolves_to_a_mask_named_sprite(self):
        """The game names dedicated collision sprites "*_Mask_spr"."""
        by_name = {o["name"]: o for o in self.objects}
        entrance = by_name["Abandoned_Mine_Entrance_obj"]
        self.assertEqual(
            SPRITE_INDEX_TO_NAME[entrance["mask_index"]],
            "Abandoned_Mine_Mask_spr",
        )
        self.assertNotEqual(entrance["mask_index"], entrance["sprite_index"])

    def test_managed_flag_is_recorded(self):
        """The inserted flag that shifted the layout is now named, not silently eaten."""
        self.assertTrue(all(obj["managed"] for obj in self.objects))


if __name__ == "__main__":
    unittest.main()
