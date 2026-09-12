import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SDK_ROOT = ROOT / "hs-game-sdk"
DATA_DIR = SDK_ROOT / "data"

DATA_FILES = [
    "manifest.json", "objects.json", "scripts.json",
    "sprites.json", "rooms.json", "sounds.json",
]


class TestSdkArtifacts(unittest.TestCase):
    """The tracked SDK: these run from a clean checkout with no game install."""

    def test_cpp_headers_generated(self):
        cpp_inc = SDK_ROOT / "cpp" / "include" / "hs_game_sdk"
        self.assertTrue(cpp_inc.exists())

        for header in ["objects.hpp", "scripts.hpp", "rooms.hpp", "yytk_helpers.hpp", "hs_game_sdk.hpp"]:
            path = cpp_inc / header
            self.assertTrue(path.exists(), f"Missing C++ header {header}")
            text = path.read_text(encoding="utf-8")
            self.assertIn("#pragma once", text)
            self.assertIn("HeroSiege", text)

    def test_ts_bindings_generated(self):
        ts_src = SDK_ROOT / "ts" / "src"
        self.assertTrue(ts_src.exists())

        for ts_file in ["index.ts", "objects.ts", "scripts.ts", "rooms.ts", "stats.ts"]:
            path = ts_src / ts_file
            self.assertTrue(path.exists(), f"Missing TS file {ts_file}")
            text = path.read_text(encoding="utf-8")
            self.assertGreater(len(text), 10)

    def test_curated_data_is_tracked(self):
        """`curated/` is hand-verified and tracked, unlike the extracted `data/`."""
        curated = SDK_ROOT / "curated" / "satanic_zone.json"
        self.assertTrue(curated.exists(), "Missing curated/satanic_zone.json")
        content = json.loads(curated.read_text(encoding="utf-8"))
        self.assertGreater(len(content), 0)


@unittest.skipUnless(
    DATA_DIR.exists(),
    "hs-game-sdk/data/ is gitignored extraction output; run "
    "tools/extract_and_generate_sdk.py against a game install to populate it",
)
class TestExtractedDataArtifacts(unittest.TestCase):
    """Local-extraction integration checks.

    `hs-game-sdk/data/` is deliberately gitignored (it is bulk output derived
    from a contributor's own game install), so these cannot run from the
    repository alone and skip instead of failing. The extractor's own parsing is
    covered without a game install by tests/test_extractor_layout.py, which
    builds a synthetic data.win.
    """

    def test_json_data_artifacts(self):
        for name in DATA_FILES:
            path = DATA_DIR / name
            self.assertTrue(path.exists(), f"Missing {name}")
            content = json.loads(path.read_text(encoding="utf-8"))
            if name != "manifest.json":
                self.assertGreater(len(content), 0, f"Empty list in {name}")

    def test_manifest_records_the_binaries_it_came_from(self):
        manifest = json.loads((DATA_DIR / "manifest.json").read_text(encoding="utf-8"))
        for key in ["game_bin", "data_win_sha256", "data_win_size"]:
            self.assertIn(key, manifest)


if __name__ == "__main__":
    unittest.main()
