"""Tests for the version bumper.

Two things are actually worth pinning here, and neither is "it can replace a
string". The first is that it rewrites the hub's own version and nothing else:
both lockfiles are full of other packages' `"version"` fields, and a loose
pattern would quietly bump a dependency. The second is that the files come back
with the line endings they went in with -- the worktree is CRLF, and a
text-mode rewrite would flip a whole lockfile to LF for a one-line change,
which `git diff` then reports as a file with no changes in it.
"""

import shutil
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "tools"))

import cut_release  # noqa: E402


REPO = Path(__file__).resolve().parents[1]

# Everything the bumper touches, plus the two lockfiles it must not damage.
COPIED = [
    "hub/package.json",
    "hub/package-lock.json",
    "hub/src-tauri/Cargo.toml",
    "hub/src-tauri/Cargo.lock",
    "hub/src-tauri/tauri.conf.json",
]


class CutRelease(unittest.TestCase):
    def setUp(self):
        self.dir = Path(tempfile.mkdtemp())
        self.addCleanup(shutil.rmtree, self.dir, ignore_errors=True)
        for name in COPIED:
            target = self.dir / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(REPO / name, target)
        self.before = {name: (self.dir / name).read_bytes() for name in COPIED}

    def bump(self, version):
        return cut_release.cut(self.dir, version)

    def test_every_site_moves_together(self):
        start = cut_release.current(self.dir)
        self.bump("9.9.9")

        self.assertEqual(cut_release.current(self.dir), "9.9.9")
        ok, lines = cut_release.check(self.dir, "9.9.9")
        self.assertTrue(ok, "\n".join(lines))
        self.assertNotEqual(start, "9.9.9", "the fixture must not start at 9.9.9")

    def test_the_tauri_config_moves_because_the_updater_reads_it(self):
        # The one that decides whether an installed hub sees a release at all.
        self.bump("9.9.9")
        conf = (self.dir / "hub/src-tauri/tauri.conf.json").read_text("utf-8")
        self.assertIn('"version": "9.9.9"', conf)

    def test_the_crate_manifest_moves_because_the_binary_reports_it(self):
        # CARGO_PKG_VERSION: what About shows and what every log line is
        # stamped with. Left behind, it disagrees with the updater's answer.
        self.bump("9.9.9")
        toml = (self.dir / "hub/src-tauri/Cargo.toml").read_text("utf-8")
        self.assertIn('version = "9.9.9"', toml)

    def test_no_dependency_version_is_touched(self):
        start = cut_release.current(self.dir).encode()
        self.bump("9.9.9")
        for name in COPIED:
            after = (self.dir / name).read_bytes()
            # Exactly the occurrences of the old version that were there
            # before, minus the ones this file legitimately owns.
            owned = self.before[name].count(start) - after.count(start)
            self.assertEqual(
                after.count(b"9.9.9"),
                owned,
                f"{name} gained a 9.9.9 somewhere it should not have",
            )

    def test_line_endings_survive(self):
        self.bump("9.9.9")
        for name in COPIED:
            before, after = self.before[name], (self.dir / name).read_bytes()
            self.assertEqual(
                before.count(b"\r\n"),
                after.count(b"\r\n"),
                f"{name} changed its CRLF count",
            )
            self.assertEqual(
                before.count(b"\n") - before.count(b"\r\n"),
                after.count(b"\n") - after.count(b"\r\n"),
                f"{name} changed its bare-LF count",
            )

    def test_a_half_bumped_tree_is_refused_rather_than_finished(self):
        # The failure this guards: the workflow checks some of the files, so a
        # tree where only the checked ones moved would pass CI and ship a hub
        # that reports one version and compares with another.
        conf = self.dir / "hub/src-tauri/tauri.conf.json"
        conf.write_bytes(conf.read_bytes().replace(b'"version": "0.', b'"version": "7.'))

        with self.assertRaises(SystemExit) as raised:
            self.bump("9.9.9")
        self.assertIn("does not agree", str(raised.exception))

    def test_check_reports_a_tag_that_does_not_match(self):
        ok, lines = cut_release.check(self.dir, "0.0.1")
        self.assertFalse(ok)
        self.assertTrue(any("MISMATCH" in line for line in lines))

    def test_a_version_that_is_not_three_numbers_is_refused(self):
        for bad in ["v1.2.3", "1.2", "1.2.3-rc1", "latest"]:
            with self.assertRaises(SystemExit):
                self.bump(bad)

    def test_bumping_to_the_version_already_there_says_so(self):
        with self.assertRaises(SystemExit) as raised:
            self.bump(cut_release.current(self.dir))
        self.assertIn("already at", str(raised.exception))


if __name__ == "__main__":
    unittest.main()
