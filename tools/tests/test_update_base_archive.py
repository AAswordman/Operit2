"""Verify private installer history and delta production without public Releases."""
import json
from pathlib import Path
import random
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/updates"))
sys.path.insert(0, str(ROOT / "tools/release"))
import archive
import prepare
from release_artifacts import release_version


class UpdateBaseArchiveTests(unittest.TestCase):
    # Uses a private history and distribution directory for every test.
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.history = self.root / "history"
        self.dist = self.root / "dist"
        self.dist.mkdir()
        self.version = release_version()
        self.name = f"operit2-app-windows-x86_64-{self.version}.zip"
        self.payload = b"PK\x03\x04" + random.Random(17).randbytes(128 * 1024)
        (self.dist / self.name).write_bytes(self.payload)

    # Clearing build intermediates cannot remove retained original installer bytes.
    def test_retains_exact_original_outside_work_and_is_repeatable(self):
        directory = archive.retain(self.dist, self.history)
        self.assertEqual((directory / self.name).read_bytes(), self.payload)
        self.assertEqual(archive.retain(self.dist, self.history), directory)
        (self.dist / self.name).unlink()
        self.assertEqual(archive.inventory(directory)["packages"][self.name]["size"], len(self.payload))

    # A rebuilt same-version package may not replace a base already sent to testers.
    def test_same_version_cannot_be_replaced_by_different_bytes(self):
        directory = archive.retain(self.dist, self.history)
        (self.dist / self.name).write_bytes(self.payload + b"different signature")
        with self.assertRaisesRegex(ValueError, "immutable"):
            archive.retain(self.dist, self.history)
        self.assertEqual((directory / self.name).read_bytes(), self.payload)

    # Corrupt disk history must fail verification before becoming a patch source.
    def test_corrupt_history_is_rejected(self):
        directory = archive.retain(self.dist, self.history)
        (directory / self.name).write_bytes(b"corrupt")
        with self.assertRaisesRegex(ValueError, "digest mismatch"):
            archive.history(self.history, "3.0.0")

    # Historical manually distributed packages can seed patches with no GitHub calls.
    def test_manual_old_package_generates_verified_delta_without_releases(self):
        old = self.root / "manually-sent"
        old.mkdir()
        old_name = "operit2-app-windows-x86_64-1.0.0.zip"
        (old / old_name).write_bytes(self.payload)
        directory = archive.retain(old, self.history, "1.0.0")
        (self.dist / self.name).write_bytes(self.payload + b"new signed bytes")
        with patch("gh_publish.gh", side_effect=AssertionError("No GitHub access allowed")):
            bases = archive.history(self.history, self.version)
            self.assertEqual(bases, [directory])
            prepare.prepare(self.dist, bases)
        manifest = json.loads((self.dist / (self.name + ".update.json")).read_text())
        self.assertEqual(manifest["patches"][0]["baseVersion"], "1.0.0")

    # An empty or missing local history is valid for a first manually distributed build.
    def test_missing_history_is_an_empty_base_set(self):
        self.assertEqual(archive.history(self.history, self.version), [])

    # Local preview churn must not evict stable bases or hash every historical installer on each build.
    def test_local_history_selects_recent_versions_per_channel(self):
        for version in ("1.8.0", "1.9.0", "2.0.0-preview.9", "2.0.0-preview.10", "2.0.0-preview.11", "3.0.0"):
            directory = self.root / version
            directory.mkdir()
            (directory / f"operit2-app-windows-x86_64-{version}.zip").write_bytes(b"PK\x03\x04old signed bytes")
            archive.retain(directory, self.history, version)
        selected = archive.history(self.history, "2.0.0-preview.12", count=1)
        self.assertEqual([directory.name for directory in selected], ["2.0.0-preview.11", "1.9.0"])


if __name__ == "__main__":
    unittest.main()
