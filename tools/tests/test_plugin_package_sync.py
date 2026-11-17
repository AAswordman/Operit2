"""Verify ToolPkg identity depends on file contents rather than filesystem metadata."""

import importlib.util
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
            (source / "manifest.json").write_text('{"main":"dist/main.js"}', encoding="utf-8")
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
