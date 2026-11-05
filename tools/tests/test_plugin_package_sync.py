"""Verify ToolPkg identity depends on file contents rather than filesystem metadata."""

import importlib.util
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile


REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location(
    "sync_plugin_packages", REPO_ROOT / "plugins/tools/sync_plugin_packages.py"
)
SYNC = importlib.util.module_from_spec(SPEC)
sys.modules[SPEC.name] = SYNC
SPEC.loader.exec_module(SYNC)


class PluginPackageSyncTests(unittest.TestCase):
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
