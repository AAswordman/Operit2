"""Prevent mixing V8 snapshots, bindings, platform configurations and cached archives."""
from pathlib import Path
import json
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import prepare_ios_v8 as v8


class IOSV8BuildTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="operit v8 ")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.config = v8.configuration("152.2.0", "aarch64-apple-ios-sim")
        self.gn_out = self.root / "gn_out"
        (self.gn_out / "obj").mkdir(parents=True)
        (self.gn_out / "obj/librusty_v8.a").write_bytes(b"matching archive and snapshot")
        (self.gn_out / "src_binding.rs").write_text("matching bindings")
        (self.gn_out / "args.gn").write_text("\n".join(
            f"{key} = {json.dumps(value)}" for key, value in self.config["gn"].items()))
        self.cache = self.root / "cache"
        self.directory = self.cache / "152.2.0/aarch64-apple-ios-sim"

    def store(self):
        v8.store_artifacts(self.gn_out, self.directory, self.config)

    def test_reuses_only_a_complete_matching_build(self):
        self.store()
        env_file = self.root / "output/v8.env"
        with patch.object(v8, "version_from_lock", return_value="152.2.0"), \
             patch.object(v8, "build", side_effect=AssertionError("must reuse cache")):
            v8.prepare(self.config["target"], env_file, self.cache)
        self.assertTrue(v8.cache_valid(self.directory, self.config))
        self.assertIn(str(self.directory / "librusty_v8.a"), env_file.read_text())
        self.assertIn(str(self.directory / "src_binding.rs"), env_file.read_text())
        self.assertIn("RUSTY_V8_ARCHIVE_SHA256", env_file.read_text())
        self.assertIn("unset V8_FROM_SOURCE DOCS_RS V8_FORCE_DEBUG", env_file.read_text())

    def test_changed_bindings_or_archive_invalidates_cache(self):
        for name in ("librusty_v8.a", "src_binding.rs"):
            with self.subTest(name=name):
                self.store()
                (self.directory / name).write_bytes(b"different build")
                self.assertFalse(v8.cache_valid(self.directory, self.config))

    def test_missing_snapshot_configuration_or_manifest_invalidates_cache(self):
        for name in ("args.gn", "manifest.json", "src_binding.rs", "librusty_v8.a"):
            with self.subTest(name=name):
                self.store()
                (self.directory / name).unlink()
                self.assertFalse(v8.cache_valid(self.directory, self.config))

    def test_device_build_cannot_reuse_simulator_artifacts(self):
        self.store()
        device = v8.configuration("152.2.0", "aarch64-apple-ios")
        self.assertFalse(v8.cache_valid(self.directory, device))
        with self.assertRaisesRegex(RuntimeError, "target_environment"):
            v8.store_artifacts(self.gn_out, self.root / "device", device)

    def test_rejects_jit_or_caged_heap_enabled_in_source_build(self):
        original = (self.gn_out / "args.gn").read_text()
        for name, old, new in (("v8_jitless", "true", "false"),
                               ("cppgc_enable_caged_heap", "false", "true")):
            with self.subTest(name=name):
                (self.gn_out / "args.gn").write_text(original.replace(f"{name} = {old}", f"{name} = {new}"))
                with self.assertRaisesRegex(RuntimeError, name):
                    self.store()
        self.assertFalse((self.directory / "manifest.json").exists())

    def test_build_failure_emits_no_environment_and_never_falls_back_to_download(self):
        env_file = self.root / "v8.env"
        with patch.object(v8, "version_from_lock", return_value="152.2.0"), \
             patch.object(v8, "build", side_effect=RuntimeError("source build failed")):
            with self.assertRaisesRegex(RuntimeError, "source build failed"):
                v8.prepare(self.config["target"], env_file, self.cache)
        self.assertFalse(env_file.exists())

    def test_version_is_taken_from_the_app_lockfile(self):
        lock = self.root / "Cargo.lock"
        lock.write_text('[[package]]\nname="v8"\nversion="152.2.0"\n')
        self.assertEqual(v8.version_from_lock(lock), "152.2.0")
        lock.write_text(lock.read_text() + '[[package]]\nname="v8"\nversion="153.0.0"\n')
        with self.assertRaisesRegex(RuntimeError, "exactly one"):
            v8.version_from_lock(lock)


if __name__ == "__main__":
    unittest.main()
