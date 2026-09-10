import json
import sys
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SDK_ROOT = ROOT / "hs-game-sdk"


class TestSdkArtifacts(unittest.TestCase):
    def test_json_data_artifacts(self):
        data_dir = SDK_ROOT / "data"
        self.assertTrue(data_dir.exists())

        for name in ["manifest.json", "objects.json", "scripts.json", "sprites.json", "rooms.json", "sounds.json"]:
            path = data_dir / name
            self.assertTrue(path.exists(), f"Missing {name}")
            content = json.loads(path.read_text(encoding="utf-8"))
            if name != "manifest.json":
                self.assertGreater(len(content), 0, f"Empty list in {name}")

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


if __name__ == "__main__":
    unittest.main()
