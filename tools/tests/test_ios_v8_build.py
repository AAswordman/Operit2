"""Prevent mixing V8 snapshots, bindings, platform configurations and cached archives."""
from pathlib import Path
import json
import re
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

    def test_wasm_interpreter_does_not_reenable_jit_or_address_space_cages(self):
        for target in v8.TARGETS:
            config = v8.configuration("152.2.0", target)
            self.assertTrue(config["gn"]["v8_enable_webassembly"])
            self.assertTrue(config["gn"]["v8_enable_drumbrake"])
            self.assertTrue(config["gn"]["v8_drumbrake_bounds_checks"])
            self.assertTrue(config["gn"]["v8_jitless"])
            for key in ("v8_enable_pointer_compression", "cppgc_enable_caged_heap", "v8_enable_sandbox"):
                self.assertFalse(config["gn"][key])
            self.assertEqual(config["interpreter_patch_sha256"], v8.digest(v8.interpreter_patch("152.2.0")))
        with self.assertRaisesRegex(RuntimeError, "No reviewed"):
            v8.configuration("153.0.0", "aarch64-apple-ios")

    def test_source_patch_digest_is_part_of_artifact_identity(self):
        self.store()
        changed = {**self.config, "interpreter_patch_sha256": "0" * 64}
        self.assertFalse(v8.cache_valid(self.directory, changed))

    def test_reviewed_source_patch_is_idempotent_and_rejects_partial_changes(self):
        source = self.root / "source"
        # Reconstruct the original hunk lines at their real offsets, without
        # copying the complete V8 source tree into a unit-test fixture.
        files = {}
        current = None
        for line in v8.interpreter_patch("152.2.0").read_text().splitlines(keepends=True):
            if line.startswith("--- a/"):
                current = line[len("--- a/"):].strip()
                files[current] = []
            elif line.startswith("@@"):
                start = int(re.match(r"@@ -(\d+)", line).group(1))
                files[current].extend(["\n"] * (start - 1 - len(files[current])))
            elif line.startswith((" ", "-")) and current:
                files[current].append(line[1:])
        for name, lines in files.items():
            path = source / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text("".join(lines))
        patch = v8.interpreter_patch("152.2.0")
        v8.apply_interpreter_patch(source, patch)
        expected = {name: (source / name).read_bytes() for name in files}
        v8.apply_interpreter_patch(source, patch)
        self.assertEqual(expected, {name: (source / name).read_bytes() for name in files})
        implementation = source / "v8/src/wasm/interpreter/wasm-interpreter.cc"
        self.assertIn("ReadUnalignedValue<Tagged_t>", implementation.read_text())
        self.assertNotIn("static_assert(COMPRESS_POINTERS_BOOL)", implementation.read_text())
        self.assertIn("__ SmiTag(x1);", (source / "v8/src/wasm/interpreter/arm64/interpreter-builtins-arm64.cc").read_text())
        self.assertIn("__ SmiTag(rbx);", (source / "v8/src/wasm/interpreter/x64/interpreter-builtins-x64.cc").read_text())
        implementation.write_text(implementation.read_text().replace("value.ptr()", "unexpected_source_change()"))
        with self.assertRaisesRegex(RuntimeError, "differs from the reviewed patch"):
            v8.apply_interpreter_patch(source, patch)

    def test_version_is_taken_from_the_app_lockfile(self):
        lock = self.root / "Cargo.lock"
        lock.write_text('[[package]]\nname="v8"\nversion="152.2.0"\n')
        self.assertEqual(v8.version_from_lock(lock), "152.2.0")
        lock.write_text(lock.read_text() + '[[package]]\nname="v8"\nversion="153.0.0"\n')
        with self.assertRaisesRegex(RuntimeError, "exactly one"):
            v8.version_from_lock(lock)


if __name__ == "__main__":
    unittest.main()
