"""Exercise real delta generation, corruption rejection and release validation."""
import gzip
import json
from pathlib import Path
import random
import struct
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/updates"))
sys.path.insert(0, str(ROOT / "tools/build_scripts"))
import patch as delta
import prepare
import release_artifacts


class IncrementalUpdateTests(unittest.TestCase):
    # Gives every scenario private input and output paths.
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.base, self.target, self.patch, self.output = [self.root / name for name in ("base", "target", "delta", "output")]
        self.data = random.Random(42).randbytes(512 * 1024)
        self.base.write_bytes(self.data)

    # COPY/ADD deltas must preserve arbitrary binary data after a shifted insertion.
    def test_round_trip_after_insertion_removal_and_reorder(self):
        expected = self.data[100000:300000] + b"new signed metadata" + self.data[:100000] + self.data[320000:]
        self.target.write_bytes(expected)
        delta.build(self.base, self.target, self.patch)
        delta.apply(self.base, self.patch, self.output)
        self.assertEqual(self.output.read_bytes(), expected)
        self.assertLess(self.patch.stat().st_size, len(expected) // 3)

    # The same inputs must produce the same compressed asset digest.
    def test_patch_is_reproducible(self):
        self.target.write_bytes(self.data + b"new")
        second = self.root / "second"
        delta.build(self.base, self.target, self.patch)
        delta.build(self.base, self.target, second)
        self.assertEqual(self.patch.read_bytes(), second.read_bytes())

    # No source bytes may be overwritten through an output path alias.
    def test_rejects_output_alias_and_existing_output(self):
        self.target.write_bytes(b"target")
        with self.assertRaises(ValueError):
            delta.build(self.base, self.target, self.base)
        self.output.write_bytes(b"keep")
        with self.assertRaises(ValueError):
            delta.apply(self.base, self.target, self.output)
        self.assertEqual(self.output.read_bytes(), b"keep")

    # A stale or modified installed base cannot create an output package.
    def test_wrong_base_is_rejected(self):
        self.target.write_bytes(self.data + b"changed")
        delta.build(self.base, self.target, self.patch)
        self.base.write_bytes(b"wrong base")
        with self.assertRaisesRegex(ValueError, "Base package"):
            delta.apply(self.base, self.patch, self.output)
        self.assertFalse(self.output.exists())

    # Truncated compressed streams must remove any partially reconstructed file.
    def test_truncation_cleans_output(self):
        self.target.write_bytes(self.data + b"new")
        delta.build(self.base, self.target, self.patch)
        self.patch.write_bytes(self.patch.read_bytes()[:-7])
        with self.assertRaises((EOFError, OSError, ValueError)):
            delta.apply(self.base, self.patch, self.output)
        self.assertFalse(self.output.exists())

    # An out-of-bounds COPY must fail before reading beyond the retained package.
    def test_rejects_copy_outside_base(self):
        self.target.write_bytes(self.data)
        delta.build(self.base, self.target, self.patch)
        stream = gzip.decompress(self.patch.read_bytes())[:89]
        self.patch.write_bytes(gzip.compress(stream + b"\x01" + struct.pack("<QQ", len(self.data), 1) + b"\0"))
        with self.assertRaisesRegex(ValueError, "COPY exceeds"):
            delta.apply(self.base, self.patch, self.output)
        self.assertFalse(self.output.exists())

    # Target and base hashes protect signed bytes independently of gzip checksums.
    def test_rejects_wrong_target_hash_and_trailing_instructions(self):
        self.target.write_bytes(self.data)
        delta.build(self.base, self.target, self.patch)
        original = gzip.decompress(self.patch.read_bytes())
        for stream in (original[:57] + b"\0" * 32 + original[89:], original + b"extra"):
            self.patch.write_bytes(gzip.compress(stream))
            with self.assertRaises(ValueError):
                delta.apply(self.base, self.patch, self.output)
            self.assertFalse(self.output.exists())

    # Different signed platform container types use the identical byte-exact protocol.
    def test_every_platform_package_format_round_trips(self):
        for header in (b"PK\x03\x04", b"xar!", b"\x1f\x8b\0\0", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"):
            self.base.write_bytes(header + self.data)
            self.target.write_bytes(header + self.data + b"signed target trailer")
            delta.build(self.base, self.target, self.patch)
            delta.apply(self.base, self.patch, self.output)
            self.assertEqual(self.output.read_bytes(), self.target.read_bytes())
            self.patch.unlink()
            self.output.unlink()

    # Prepared releases accept only target-bound manifests and the declared patch set.
    def test_prepare_and_publisher_validate_the_complete_asset_set(self):
        version = release_artifacts.release_version()
        name = f"operit2-app-windows-x86_64-{version}.zip"
        dist, old = self.root / "dist", self.root / "old"
        dist.mkdir(); old.mkdir()
        (old / "operit2-app-windows-x86_64-1.0.0.zip").write_bytes(b"PK\x03\x04" + self.data)
        (dist / name).write_bytes(b"PK\x03\x04" + self.data + b"new signed target")
        prepare.prepare(dist, [old])
        assets = list(dist.iterdir())
        release_artifacts.validate_release_assets(assets)
        manifest_path = dist / (name + ".update.json")
        manifest = json.loads(manifest_path.read_text())
        self.assertEqual(len(manifest["patches"]), 1)
        (dist / manifest["patches"][0]["assetName"]).write_bytes(b"corrupt")
        with self.assertRaisesRegex(RuntimeError, "missing or corrupt"):
            release_artifacts.validate_release_assets(assets)

    # The first release needs no historical package to produce authenticated metadata.
    def test_first_release_has_an_empty_patch_set(self):
        version = release_artifacts.release_version()
        dist = self.root / "dist"; dist.mkdir()
        name = f"operit2-app-windows-x86_64-{version}.zip"
        (dist / name).write_bytes(b"PK\x03\x04" + self.data)
        prepare.prepare(dist, [])
        release_artifacts.validate_release_assets(list(dist.iterdir()))
        self.assertEqual(json.loads((dist / (name + ".update.json")).read_text())["patches"], [])

    # Incompressible unrelated package bytes must not publish a delta larger than the full transfer.
    def test_larger_delta_is_omitted_from_the_release(self):
        version = release_artifacts.release_version()
        dist, old = self.root / "dist", self.root / "old"
        dist.mkdir(); old.mkdir()
        name = f"operit2-app-windows-x86_64-{version}.zip"
        (old / "operit2-app-windows-x86_64-1.0.0.zip").write_bytes(b"PK\x03\x04" + self.data)
        (dist / name).write_bytes(b"PK\x03\x04" + random.Random(7).randbytes(len(self.data)))
        prepare.prepare(dist, [old])
        release_artifacts.validate_release_assets(list(dist.iterdir()))
        self.assertEqual(json.loads((dist / (name + ".update.json")).read_text())["patches"], [])
        self.assertEqual(list(dist.glob("*.patch.gz")), [])

    # Both channels build byte-exact patches for matching Android, Windows and macOS distributions.
    def test_release_and_preview_generate_platform_deltas(self):
        families = (
            ("operit2-app-windows-x86_64", "zip", b"PK\x03\x04"),
            ("operit2-app-windows-x86_64", "msi", b"\xd0\xcf\x11\xe0\xa1\xb1\x1a\xe1"),
            ("operit2-app-android-arm64-v8a", "apk", b"PK\x03\x04"),
            ("operit2-app-macos-universal-unsandbox", "pkg", b"xar!"),
        )
        for base_version, version in (("1.9.0", "2.0.0"), ("2.0.0-preview.9", "2.0.0-preview.10"), ("2.0.0-preview.10", "2.0.0")):
            with self.subTest(base=base_version, target=version), tempfile.TemporaryDirectory() as temporary, patch.object(release_artifacts, "release_version", return_value=version), patch.object(prepare, "release_version", return_value=version):
                dist, old = Path(temporary) / "dist", Path(temporary) / "base"
                dist.mkdir(); old.mkdir()
                for family, extension, header in families:
                    (old / f"{family}-{base_version}.{extension}").write_bytes(header + self.data)
                    (dist / f"{family}-{version}.{extension}").write_bytes(header + self.data + b"signed target")
                prepare.prepare(dist, [old])
                release_artifacts.validate_release_assets(list(dist.iterdir()))
                for family, extension, _ in families:
                    manifest = json.loads((dist / f"{family}-{version}.{extension}.update.json").read_text())
                    self.assertEqual(len(manifest["patches"]), 1)
                    self.assertEqual(manifest["patches"][0]["baseVersion"], base_version)

    # Orphaned patches and metadata cannot be uploaded alongside unrelated packages.
    def test_rejects_orphaned_sidecars(self):
        for name in ("other.update.json", "other.patch.gz"):
            orphan = self.root / name; orphan.write_bytes(b"invalid")
            with self.assertRaises(RuntimeError):
                release_artifacts.validate_release_assets([orphan])


if __name__ == "__main__":
    unittest.main()
