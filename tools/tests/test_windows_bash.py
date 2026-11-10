"""Verify pinned Windows Bash preparation without a network or system Git dependency."""

import hashlib
import os
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import windows_bash as bash


@unittest.skipUnless(os.name == "nt", "Windows runtime preparation requires a Windows host")
class WindowsBashTests(unittest.TestCase):
    """Exercise integrity and complete runtime staging independently of installed Git."""

    def test_checksum_failure_prevents_extraction_and_packaging(self):
        """Reject cached corruption before invoking an extractor or creating a payload."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            cache = root / "cache" / f"{bash.VERSION}-x86_64"
            cache.mkdir(parents=True)
            (cache / f"PortableGit-{bash.VERSION}-64-bit.7z.exe").write_bytes(b"corrupt")
            with patch.object(bash, "CACHE", root / "cache"), patch.object(bash, "run") as extract:
                with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
                    bash.prepare_windows_bash(root / "package")
                extract.assert_not_called()
                self.assertFalse((root / "package").exists())

    def test_complete_runtime_and_license_are_staged_for_each_architecture(self):
        """Keep DLLs, Unix tools and all upstream files together for both Windows targets."""
        for arch in bash.PACKAGES:
            with self.subTest(arch=arch), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                payload = b"verified portable package"
                digest = hashlib.sha256(payload).hexdigest()
                suffix, _ = bash.PACKAGES[arch]
                cache = root / "cache" / f"{bash.VERSION}-{arch}"
                cache.mkdir(parents=True)
                (cache / f"PortableGit-{bash.VERSION}-{suffix}.7z.exe").write_bytes(payload)
                for name in (*bash.REQUIRED_FILES, "usr/share/licenses/bash/COPYING"):
                    path = cache / "runtime" / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(name.encode())
                with patch.object(bash, "CACHE", root / "cache"), patch.dict(bash.PACKAGES, {arch: (suffix, digest)}):
                    target = bash.prepare_windows_bash(root / "package", arch)
                    before = {name: (target / name).stat().st_mtime_ns for name in bash.REQUIRED_FILES}
                    with patch.object(bash, "download_verified", side_effect=AssertionError("unchanged build downloaded Bash")), patch.object(bash.shutil, "rmtree", side_effect=AssertionError("unchanged build replaced Bash")):
                        self.assertEqual(bash.prepare_windows_bash(root / "package", arch), target)
                    self.assertEqual(before, {name: (target / name).stat().st_mtime_ns for name in bash.REQUIRED_FILES})
                    (target / "usr/bin/msys-2.0.dll").unlink()
                    bash.prepare_windows_bash(root / "package", arch)
                bash.verify_bash_runtime(target)
                self.assertEqual((target / "usr/share/licenses/bash/COPYING").read_bytes(), b"usr/share/licenses/bash/COPYING")
                self.assertIn(digest, (target / "operit-bundle.json").read_text())

    def test_incomplete_runtime_is_rejected(self):
        """Do not publish an executable without its DLLs and completion-protocol utilities."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "bin").mkdir()
            (root / "bin/bash.exe").write_bytes(b"executable only")
            with self.assertRaisesRegex(RuntimeError, "usr.bin.bash.exe"):
                bash.verify_bash_runtime(root)


if __name__ == "__main__":
    unittest.main()
