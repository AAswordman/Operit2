"""Verify ToolPkg identity depends on file contents rather than filesystem metadata."""

import importlib.util
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile


REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "sync_plugin_packages", REPO_ROOT / "plugins/tools/sync_plugin_packages.py"
)
SYNC = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = SYNC
SPEC.loader.exec_module(SYNC)


class PluginPackageSyncTests(unittest.TestCase):
    # Requires both runtime sources to pass the gate before publishing hot-reload notifications.
    def test_runtime_parse_failure_stops_sync_before_hot_reload(self):
        with patch.object(SYNC.sys, "argv", ["sync_plugin_packages.py", "--source", "runtime"]), \
                patch.object(SYNC, "_generate_plugin_sdk_types"), \
                patch.object(SYNC, "_sync", return_value=(0, 1, 0)) as sync, \
                patch.object(SYNC, "_validate_plugin_packages", side_effect=RuntimeError("registration rejected")) as gate, \
                patch.object(SYNC, "_maybe_hot_reload_output") as reload:
            with self.assertRaisesRegex(RuntimeError, "registration rejected"):
                SYNC.main()
            self.assertEqual(sync.call_count, 2)
            self.assertEqual([source.name for source, output in gate.call_args.args[1]], ["buildin", "external"])
            reload.assert_not_called()

    # Invokes the real loader test on every sync, including runs that reuse compiler caches.
    def test_parse_gate_has_explicit_outputs_and_no_activation_or_cache_skip(self):
        jobs = [(Path("plugins/packages/buildin"), Path("assets/buildin")),
                (Path("plugins/packages/external"), Path("assets/external"))]
        with patch.object(SYNC, "_host_cargo_environment", return_value={"CARGO_BUILD_TARGET": "aarch64-linux-android"}), \
                patch.object(SYNC, "_platform_command", return_value="cargo"), \
                patch.object(SYNC, "_run_checked_command") as run:
            for _ in range(2):
                SYNC._validate_plugin_packages(REPO_ROOT, jobs, dry_run=False)
            self.assertEqual(run.call_count, 2)
            command = run.call_args.args[0]
            self.assertIn("operit-js-bridge", command)
            self.assertIn("plugin_parse_gate", command)
            self.assertIn("--include-ignored", command)
            environment = run.call_args.kwargs["env"]
            self.assertNotIn("CARGO_BUILD_TARGET", environment)
            self.assertEqual(json.loads(environment["OPERIT_PLUGIN_PARSE_INPUTS"]),
                             {source.name: str(output.resolve()) for source, output in jobs})

    # Rebuilds cleaned, partial, or modified outputs while reusing an intact build.
    def test_child_prebuild_cache_requires_the_complete_generated_tree(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "plugins/packages/buildin"
            child = source / "daily_life"
            (child / "src").mkdir(parents=True)
            (child / "manifest.json").write_text('{"main":"dist/main.js"}', encoding="utf-8")
            (child / "tsconfig.json").write_text('{"compilerOptions":{"outDir":"dist"}}', encoding="utf-8")
            (child / "src/main.ts").write_text('export const main = 1;', encoding="utf-8")
            main = child / "dist/main.js"
            dependency = child / "dist/reminders.js"

            # Emulates the compiler emitting both the entry and its imported module.
            def compile_outputs(*args, **kwargs):
                if kwargs["dry_run"]:
                    return
                main.parent.mkdir(exist_ok=True)
                main.write_text('exports.main = 1;', encoding="utf-8")
                dependency.write_text('exports.reminder = 1;', encoding="utf-8")

            with patch.object(SYNC, "_plugins_root", return_value=root / "plugins"), \
                    patch.object(SYNC, "_typescript_command", return_value="tsc"), \
                    patch.object(SYNC, "_run_checked_command", side_effect=compile_outputs) as compiler:
                signature = SYNC._compute_paths_signature(root, SYNC._collect_prebuild_inputs(source, child))
                state_file = source / ".sync_state.json"
                state_file.write_text(json.dumps({"prebuild:daily_life": signature}), encoding="utf-8")
                plans = SYNC._collect_sync_plan(source)
                SYNC._prebuild_plans(root, source, plans, dry_run=False)
                self.assertEqual(compiler.call_count, 1, "Legacy input-only cache must rebuild missing outputs")
                SYNC._prebuild_plans(root, source, plans, dry_run=False)
                self.assertEqual(compiler.call_count, 1, "Intact outputs should reuse the build")
                dependency.unlink()
                SYNC._prebuild_plans(root, source, plans, dry_run=False)
                self.assertEqual(compiler.call_count, 2, "Missing imports must rebuild even with main.js present")
                dependency.write_text('damaged output', encoding="utf-8")
                SYNC._prebuild_plans(root, source, plans, dry_run=False)
                self.assertEqual(compiler.call_count, 3, "Modified outputs must rebuild")
                main.unlink()
                dependency.unlink()
                main.parent.rmdir()
                state_before = state_file.read_bytes()
                SYNC._prebuild_plans(root, source, plans, dry_run=True)
                self.assertEqual(state_file.read_bytes(), state_before, "Dry runs must not cache builds")
                compiler.reset_mock()
                SYNC._prebuild_plans(root, source, plans, dry_run=False)
                compiler.assert_called_once()
                self.assertTrue(main.is_file())
                self.assertTrue(dependency.is_file())

    # Rejects a successful compiler process that failed to produce runtime files.
    def test_child_prebuild_does_not_cache_an_empty_compiler_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "plugins/packages/buildin"
            child = source / "daily_life"
            child.mkdir(parents=True)
            (child / "manifest.json").write_text('{"main":"dist/main.js"}', encoding="utf-8")
            (child / "tsconfig.json").write_text('{}', encoding="utf-8")
            with patch.object(SYNC, "_plugins_root", return_value=root / "plugins"), \
                    patch.object(SYNC, "_typescript_command", return_value="tsc"), \
                    patch.object(SYNC, "_run_checked_command"):
                with self.assertRaisesRegex(RuntimeError, "produced no ToolPkg JavaScript"):
                    SYNC._prebuild_plans(root, source, SYNC._collect_sync_plan(source), dry_run=False)
            self.assertFalse((source / ".sync_state.json").exists())

    # Requires ignored compiler outputs in archives without bundling old packages.
    def test_repacking_includes_ignored_dist_outputs_without_nested_archives(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            (root / ".gitignore").write_text("**/dist/\n*.wasm\n*.local\n", encoding="utf-8")
            source = root / "plugin"
            (source / "dist/ui").mkdir(parents=True)
            (source / "resources/dsp").mkdir(parents=True)
            (source / "modules").mkdir()
            (source / "manifest.json").write_text(
                '{"main":"dist/main.js","subpackages":[{"entry":"dist/ui/screen.js"}]}',
                encoding="utf-8",
            )
            (source / "dist/main.js").write_bytes(b"compiled entry")
            (source / "dist/ui/screen.js").write_bytes(b"compiled screen")
            (source / "resources/dsp/runtime.wasm").write_bytes(b"wasm resource")
            (source / "modules/runtime.wasm").write_bytes(b"wasm module")
            (source / "dist/previous.toolpkg").write_bytes(b"previous archive")
            (source / "secrets.local").write_bytes(b"private data")
            output = root / "plugin.toolpkg"
            SYNC._pack_toolpkg_folder(root, source, output)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(set(archive.namelist()), {
                    "manifest.json", "dist/main.js", "dist/ui/screen.js",
                    "resources/dsp/runtime.wasm", "modules/runtime.wasm",
                })
                self.assertEqual(archive.read("dist/main.js"), b"compiled entry")
                self.assertEqual(archive.read("dist/ui/screen.js"), b"compiled screen")
            original = output.read_bytes()
            for path, field in [("dist/main.js", "manifest.main"),
                                ("dist/ui/screen.js", "manifest.subpackages[0].entry")]:
                entry = source / path
                contents = entry.read_bytes()
                entry.unlink()
                with self.assertRaises(ValueError) as error:
                    SYNC._pack_toolpkg_folder(root, source, output)
                self.assertIn(field, str(error.exception))
                self.assertIn(path, str(error.exception))
                self.assertEqual(output.read_bytes(), original)
                entry.write_bytes(contents)

    def test_macos_prebuild_adds_homebrew_without_overriding_caller_tools(self):
        with patch.object(SYNC.sys, "platform", "darwin"), patch.object(SYNC.os, "pathsep", ":"), patch.dict(os.environ, {"PATH": "/custom/bin:/usr/bin:/bin"}, clear=True):
            environment = SYNC._plugin_build_environment()
            self.assertEqual(environment["PATH"], "/custom/bin:/usr/bin:/bin:/opt/homebrew/bin:/usr/local/bin")
            self.assertEqual(os.environ["PATH"], "/custom/bin:/usr/bin:/bin")
        with patch.object(SYNC.sys, "platform", "darwin"), patch.object(SYNC.os, "pathsep", ":"), patch.dict(os.environ, {"PATH": "/opt/homebrew/bin:/usr/bin:/usr/local/bin"}, clear=True):
            self.assertEqual(SYNC._plugin_build_environment()["PATH"], os.environ["PATH"])

    def test_other_platforms_keep_the_existing_build_environment(self):
        for platform in ("linux", "win32"):
            with self.subTest(platform=platform), patch.object(SYNC.sys, "platform", platform), patch.dict(os.environ, {"PATH": "/custom/tools", "BUILD_FLAG": "1"}, clear=True):
                self.assertEqual(SYNC._plugin_build_environment(), dict(os.environ))

    def test_repacking_ignores_timestamps_and_permissions_but_keeps_content_changes(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            subprocess.run(["git", "init", "-q", str(root)], check=True)
            source = root / "plugin"
            source.mkdir()
            (source / "manifest.json").write_text('{"name":"demo"}', encoding="utf-8")
            script = source / "main.js"
            script.write_bytes(b"original code")
            output = root / "demo.toolpkg"
            SYNC._pack_toolpkg_folder(root, source, output)
            original = output.read_bytes()
            for file in source.iterdir():
                os.utime(file, (1900000000, 1900000000))
                file.chmod(0o755)
            SYNC._pack_toolpkg_folder(root, source, output)
            self.assertEqual(output.read_bytes(), original)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(archive.read("main.js"), b"original code")
                self.assertTrue(all(entry.date_time == (1980, 1, 1, 0, 0, 0)
                                    for entry in archive.infolist()))
            script.write_bytes(b"updated code")
            SYNC._pack_toolpkg_folder(root, source, output)
            self.assertNotEqual(output.read_bytes(), original)
            with zipfile.ZipFile(output) as archive:
                self.assertEqual(archive.read("main.js"), b"updated code")
            before_version_change = output.read_bytes()
            (source / "manifest.json").write_text('{"name":"demo","version":"2.0"}', encoding="utf-8")
            SYNC._pack_toolpkg_folder(root, source, output)
            self.assertNotEqual(output.read_bytes(), before_version_change)
            with zipfile.ZipFile(output) as archive:
                self.assertIn(b'"version":"2.0"', archive.read("manifest.json"))


if __name__ == "__main__":
    unittest.main()
