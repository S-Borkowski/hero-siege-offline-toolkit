"""Pins the OBJT record field offsets using a synthetic data.win.

The real extraction needs a Hero Siege installation, and its output
(`hs-game-sdk/data/`) is deliberately gitignored, so neither can be used to
check the extractor in a clean checkout. This builds a tiny GameMaker IFF file
by hand instead - a few objects with known field values at known offsets - so
the parsing itself is verifiable from the repository alone.

That matters because the offsets are the thing that was wrong: this runtime
inserts a `managed` flag at +12, which shifted five later fields. A fixture that
writes each field at its documented offset and reads back the expected value
fails immediately if the extractor drifts back.
"""

import struct
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TOOLS = ROOT / "tools"
if str(TOOLS) not in sys.path:
    sys.path.insert(0, str(TOOLS))

from extract_and_generate_sdk import GameDataExtractor  # noqa: E402

#: Bytes per synthetic OBJT record. The extractor reads up to +32; the rest of a
#: real record (physics block, then the event lists) is padding here.
RECORD_SIZE = 96


class _StringPool:
    """GameMaker strings are length-prefixed, and referenced by a pointer to the
    character data - so `length` sits at `pointer - 4`."""

    def __init__(self) -> None:
        self._blob = bytearray()
        self._offsets: dict[str, int] = {}

    def add(self, text: str) -> int:
        if text not in self._offsets:
            encoded = text.encode("utf-8")
            self._blob += struct.pack("<I", len(encoded))
            self._offsets[text] = len(self._blob)
            self._blob += encoded + b"\x00"
        return self._offsets[text]

    @property
    def blob(self) -> bytes:
        return bytes(self._blob)


def build_data_win(objects: list[dict], simple_chunks: dict[str, list[str]]) -> bytes:
    """Assembles a minimal FORM file holding one OBJT chunk plus name-only chunks.

    `objects` entries accept: name, sprite_index, visible, managed, solid, depth,
    persistent, parent_index, mask_index.
    """
    pool = _StringPool()
    for obj in objects:
        pool.add(obj["name"])
    for names in simple_chunks.values():
        for name in names:
            pool.add(name)
    pool_blob = pool.blob

    # Absolute offsets have to be known before payloads can be written, so walk
    # the chunk layout first. Every chunk is 8 bytes of header plus payload.
    pos = 8  # "FORM" + total size
    strg_base = pos + 8
    pos += 8 + len(pool_blob)

    objt_base = pos + 8
    objt_payload_size = 4 + 4 * len(objects) + len(objects) * RECORD_SIZE
    pos += 8 + objt_payload_size

    simple_bases: dict[str, int] = {}
    for tag, names in simple_chunks.items():
        simple_bases[tag] = pos + 8
        pos += 8 + 4 + 4 * len(names) + 4 * len(names)

    def string_ptr(text: str) -> int:
        return strg_base + pool.add(text)

    # OBJT payload
    record_base = objt_base + 4 + 4 * len(objects)
    objt = bytearray()
    objt += struct.pack("<I", len(objects))
    for index in range(len(objects)):
        objt += struct.pack("<I", record_base + index * RECORD_SIZE)
    for obj in objects:
        record = bytearray(b"\x00" * RECORD_SIZE)
        struct.pack_into("<I", record, 0, string_ptr(obj["name"]))
        struct.pack_into("<i", record, 4, obj.get("sprite_index", -1))
        struct.pack_into("<I", record, 8, 1 if obj.get("visible", True) else 0)
        struct.pack_into("<I", record, 12, 1 if obj.get("managed", True) else 0)
        struct.pack_into("<I", record, 16, 1 if obj.get("solid", False) else 0)
        struct.pack_into("<i", record, 20, obj.get("depth", 0))
        struct.pack_into("<I", record, 24, 1 if obj.get("persistent", False) else 0)
        struct.pack_into("<i", record, 28, obj.get("parent_index", -100))
        struct.pack_into("<i", record, 32, obj.get("mask_index", -1))
        objt += record

    # Name-only chunks (SCPT/SPRT/ROOM/SOND): a pointer list into 4-byte records
    # that each hold one name pointer.
    simple_payloads: dict[str, bytes] = {}
    for tag, names in simple_chunks.items():
        base = simple_bases[tag]
        rec_base = base + 4 + 4 * len(names)
        payload = bytearray()
        payload += struct.pack("<I", len(names))
        for index in range(len(names)):
            payload += struct.pack("<I", rec_base + index * 4)
        for name in names:
            payload += struct.pack("<I", string_ptr(name))
        simple_payloads[tag] = bytes(payload)

    body = bytearray()
    body += b"STRG" + struct.pack("<I", len(pool_blob)) + pool_blob
    body += b"OBJT" + struct.pack("<I", len(objt)) + bytes(objt)
    for tag in simple_chunks:
        payload = simple_payloads[tag]
        body += tag.encode("ascii") + struct.pack("<I", len(payload)) + payload

    return b"FORM" + struct.pack("<I", len(body)) + bytes(body)


# A three-level parent chain plus a root, mirroring the shape the real data has.
FIXTURE_OBJECTS = [
    {"name": "Pickup_Parent_obj", "sprite_index": -1, "parent_index": -100, "mask_index": -1},
    {"name": "Quest_Object_Parent_obj", "sprite_index": 5, "parent_index": 0, "mask_index": -1},
    {
        "name": "Quest_Act_01_Coffee_Beans_obj",
        "sprite_index": 7,
        "parent_index": 1,
        "mask_index": 9,
        "solid": True,
        "persistent": True,
        "visible": False,
        "depth": -42,
    },
]

FIXTURE_CHUNKS = {
    "SCPT": ["gml_Script_DropItem"],
    # Index 9 is the mask the Coffee Beans fixture points at.
    "SPRT": [f"spr_{i}" for i in range(9)] + ["Coffee_Mask_spr"],
    "ROOM": ["rm_init"],
    "SOND": ["snd_click"],
}


class TestObjtRecordLayout(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls._tmp = tempfile.TemporaryDirectory()
        game_bin = Path(cls._tmp.name)
        (game_bin / "data.win").write_bytes(build_data_win(FIXTURE_OBJECTS, FIXTURE_CHUNKS))
        cls.objects = GameDataExtractor(game_bin).extract_objects()

    @classmethod
    def tearDownClass(cls):
        cls._tmp.cleanup()

    def test_all_objects_round_trip(self):
        self.assertEqual(len(self.objects), len(FIXTURE_OBJECTS))
        self.assertEqual(
            [o["name"] for o in self.objects],
            [o["name"] for o in FIXTURE_OBJECTS],
        )

    def test_parent_index_is_read_from_plus_28(self):
        beans = self.objects[2]
        self.assertEqual(beans["parent_index"], 1)
        self.assertEqual(self.objects[1]["parent_index"], 0)
        self.assertEqual(self.objects[0]["parent_index"], GameDataExtractor.OBJECT_NO_PARENT)

    def test_mask_index_is_read_from_plus_32(self):
        self.assertEqual(self.objects[2]["mask_index"], 9)
        self.assertEqual(self.objects[0]["mask_index"], GameDataExtractor.OBJECT_NO_MASK)

    def test_the_shifted_flags_land_on_their_own_fields(self):
        """A one-field shift would swap these, so each is given a distinct value."""
        beans = self.objects[2]
        self.assertTrue(beans["managed"])          # +12
        self.assertTrue(beans["solid"])            # +16
        self.assertEqual(beans["depth"], -42)      # +20
        self.assertTrue(beans["persistent"])       # +24
        self.assertFalse(beans["visible"])         # +8
        self.assertEqual(beans["sprite_index"], 7)  # +4

        root = self.objects[0]
        self.assertFalse(root["solid"])
        self.assertFalse(root["persistent"])
        self.assertEqual(root["depth"], 0)
        self.assertTrue(root["visible"])

    def test_offsets_match_the_documented_constants(self):
        self.assertEqual(GameDataExtractor.OBJ_OFF_SPRITE, 4)
        self.assertEqual(GameDataExtractor.OBJ_OFF_VISIBLE, 8)
        self.assertEqual(GameDataExtractor.OBJ_OFF_MANAGED, 12)
        self.assertEqual(GameDataExtractor.OBJ_OFF_SOLID, 16)
        self.assertEqual(GameDataExtractor.OBJ_OFF_DEPTH, 20)
        self.assertEqual(GameDataExtractor.OBJ_OFF_PERSISTENT, 24)
        self.assertEqual(GameDataExtractor.OBJ_OFF_PARENT, 28)
        self.assertEqual(GameDataExtractor.OBJ_OFF_MASK, 32)

    def test_a_one_field_shift_is_detected(self):
        """The actual regression: reading parent at +24 and mask at +28."""
        game_bin = Path(self._tmp.name)
        extractor = GameDataExtractor(game_bin)
        try:
            GameDataExtractor.OBJ_OFF_PARENT = 24
            GameDataExtractor.OBJ_OFF_MASK = 28
            shifted = extractor.extract_objects()
        finally:
            GameDataExtractor.OBJ_OFF_PARENT = 28
            GameDataExtractor.OBJ_OFF_MASK = 32

        # With the old offsets the parent slot reads `persistent` - a flag.
        self.assertEqual(shifted[2]["parent_index"], 1)  # persistent == True
        self.assertEqual(shifted[0]["parent_index"], 0)  # persistent == False
        self.assertTrue(set(o["parent_index"] for o in shifted) <= {0, 1})

    def test_other_chunks_still_parse(self):
        game_bin = Path(self._tmp.name)
        extractor = GameDataExtractor(game_bin)
        self.assertEqual([s["name"] for s in extractor.extract_scripts()], FIXTURE_CHUNKS["SCPT"])
        self.assertEqual([s["name"] for s in extractor.extract_sprites()], FIXTURE_CHUNKS["SPRT"])
        self.assertEqual([r["name"] for r in extractor.extract_rooms()], FIXTURE_CHUNKS["ROOM"])
        self.assertEqual([s["name"] for s in extractor.extract_sounds()], FIXTURE_CHUNKS["SOND"])


if __name__ == "__main__":
    unittest.main()
