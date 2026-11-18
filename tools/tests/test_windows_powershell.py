"""Verify PowerShell distribution integrity and complete staging without network access."""

import hashlib
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch
from zipfile import ZipFile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import windows_powershell as ps


def pe_fixture(arch: str) -> bytes:
    """Create an architecture-specific PE header without any executable instructions."""
    data = bytearray(256)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 128)
    data[128:132] = b"PE\0\0"
    struct.pack_into("<H", data, 132, {"x86_64": 0x8664, "aarch64": 0xAA64}[arch])
    return bytes(data)


class WindowsPowerShellTests(unittest.TestCase):
    """Check that only verified, complete distributions enter packages."""

    def test_stages_full_runtime_incrementally_and_repairs_missing_dependencies(self):
        """Preserve all upstream files and avoid copying the runtime during unchanged builds."""
        for arch, (suffix, _) in ps.PACKAGES.items():
            with self.subTest(arch=arch), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                archive = root / "cache" / f"{ps.VERSION}-{arch}" / f"PowerShell-{ps.VERSION}-win-{suffix}.zip"
                archive.parent.mkdir(parents=True)
                with ZipFile(archive, "w") as package:
                    for name in (*ps.REQUIRED_FILES, "Modules/PSReadLine/PSReadLine.psm1", "additional-runtime.dll"):
                        package.writestr(name, pe_fixture(arch) if name == "pwsh.exe" else name.encode())
                digest = hashlib.sha256(archive.read_bytes()).hexdigest()
                with patch.object(ps, "CACHE", root / "cache"), patch.dict(ps.PACKAGES, {arch: (suffix, digest)}):
                    target = ps.prepare_windows_powershell(root / "app 中文", arch)
                    self.assertEqual((target / "additional-runtime.dll").read_bytes(), b"additional-runtime.dll")
                    before = (target / "pwsh.exe").stat().st_mtime_ns
                    with patch.object(ps, "download_verified", side_effect=AssertionError("unchanged runtime downloaded again")):
                        self.assertEqual(ps.prepare_windows_powershell(root / "app 中文", arch), target)
                    self.assertEqual((target / "pwsh.exe").stat().st_mtime_ns, before)
                    (target / "additional-runtime.dll").unlink()
                    ps.prepare_windows_powershell(root / "app 中文", arch)
                    self.assertTrue((target / "additional-runtime.dll").is_file())
                    self.assertIn(digest, (target / "operit-bundle.json").read_text())

    def test_corrupt_archive_is_rejected_before_deployment(self):
        """Reject altered downloads before replacing an application's runtime."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            suffix, _ = ps.PACKAGES["x86_64"]
            archive = root / "cache" / f"{ps.VERSION}-x86_64" / f"PowerShell-{ps.VERSION}-win-{suffix}.zip"
            archive.parent.mkdir(parents=True)
            archive.write_bytes(b"corrupt")
            with patch.object(ps, "CACHE", root / "cache"):
                with self.assertRaisesRegex(RuntimeError, "checksum mismatch"):
                    ps.prepare_windows_powershell(root / "app")
            self.assertFalse((root / "app").exists())

    def test_wrong_architecture_and_missing_runtime_are_rejected(self):
        """Prevent installation of a wrong CPU binary or incomplete .NET runtime."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            for name in ps.REQUIRED_FILES:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(pe_fixture("aarch64") if name == "pwsh.exe" else b"fixture")
            with self.assertRaisesRegex(RuntimeError, "Invalid x86_64"):
                ps.verify_powershell_runtime(root, "x86_64")
            (root / "coreclr.dll").unlink()
            with self.assertRaisesRegex(RuntimeError, "coreclr.dll"):
                ps.verify_powershell_runtime(root, "aarch64")

    def test_archive_cannot_write_outside_its_runtime(self):
        """Validate archive paths before extracting any entries."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            archive = root / "invalid.zip"
            with ZipFile(archive, "w") as package:
                package.writestr("../outside.txt", "escape")
            with patch.object(ps, "download_verified", return_value=archive):
                with self.assertRaisesRegex(RuntimeError, "path escapes its runtime"):
                    ps.prepare_windows_powershell(root / "app")
            self.assertFalse((root / "app/outside.txt").exists())


if __name__ == "__main__":
    unittest.main()
