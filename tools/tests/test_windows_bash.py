"""Verify pinned Windows Bash preparation without a network or system Git dependency."""

import hashlib
import os
import struct
import sys
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import windows_bash as bash


def pe_fixture() -> bytes:
    """Create a valid PE64 header with empty import directories for offline staging."""
    data = bytearray(512)
    data[:2] = b"MZ"
    struct.pack_into("<I", data, 0x3C, 128)
    data[128:132] = b"PE\0\0"
    struct.pack_into("<H", data, 148, 240)
    struct.pack_into("<H", data, 152, 0x20B)
    return bytes(data)


@unittest.skipUnless(os.name == "nt", "Windows runtime preparation requires a Windows host")
class WindowsBashTests(unittest.TestCase):
    """Exercise integrity and focused shell staging independently of installed Git."""

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

    def test_bash_tools_dependencies_and_licenses_are_staged_for_each_architecture(self):
        """Keep required shell tools and licenses while excluding the full Git distribution."""
        for arch in bash.PACKAGES:
            with self.subTest(arch=arch), tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                payload = b"verified portable package"
                digest = hashlib.sha256(payload).hexdigest()
                suffix, _ = bash.PACKAGES[arch]
                cache = root / "cache" / f"{bash.VERSION}-{arch}"
                cache.mkdir(parents=True)
                (cache / f"PortableGit-{bash.VERSION}-{suffix}.7z.exe").write_bytes(payload)
                for name in (*bash.REQUIRED_FILES, "usr/share/licenses/bash/COPYING", "usr/bin/msys-test.dll",
                             "usr/bin/msys-transitive.dll", "usr/bin/msys-unused.dll", "usr/bin/vim.exe", "ucrt64/bin/git.exe"):
                    path = cache / "runtime" / name
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(pe_fixture() if path.suffix in (".exe", ".dll") else name.encode())
                with patch.object(bash, "CACHE", root / "cache"), patch.dict(bash.PACKAGES, {arch: (suffix, digest)}):
                    with patch.object(bash, "binary_imports", side_effect=lambda path: {
                        "bash.exe": {"msys-test.dll", "kernel32.dll"},
                        "msys-test.dll": {"msys-transitive.dll"},
                    }.get(path.name, set())):
                        target = bash.prepare_windows_bash(root / "package", arch)
                    self.assertTrue((target / "usr/bin/msys-transitive.dll").is_file())
                    for name in bash.generated_files(arch):
                        self.assertTrue((target / name).is_file())
                    for excluded in ("usr/bin/msys-unused.dll", "usr/bin/vim.exe", "ucrt64/bin/git.exe"):
                        self.assertFalse((target / excluded).exists())
                    before = {name: (target / name).stat().st_mtime_ns for name in bash.REQUIRED_FILES}
                    with patch.object(bash, "download_verified", side_effect=AssertionError("unchanged build downloaded Bash")), patch.object(bash.shutil, "rmtree", side_effect=AssertionError("unchanged build replaced Bash")):
                        self.assertEqual(bash.prepare_windows_bash(root / "package", arch), target)
                    self.assertEqual(before, {name: (target / name).stat().st_mtime_ns for name in bash.REQUIRED_FILES})
                    (target / "usr/bin/msys-2.0.dll").unlink()
                    bash.prepare_windows_bash(root / "package", arch)
                bash.verify_bash_runtime(target)
                self.assertEqual((target / "usr/share/licenses/bash/COPYING").read_bytes(), b"usr/share/licenses/bash/COPYING")
                self.assertIn(digest, (target / "operit-bundle.json").read_text())

    def test_missing_transitive_dll_is_rejected(self):
        """Fail packaging rather than silently ship a shell with unresolved MSYS imports."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "usr/bin").mkdir(parents=True)
            with patch.object(bash, "binary_imports", return_value={"msys-missing.dll"}):
                with self.assertRaisesRegex(RuntimeError, "msys-missing.dll"):
                    bash.bash_payload(root)

    def test_pe_imports_resolve_regular_and_delayed_dlls(self):
        """Read both import tables through PE sections without loading a Windows binary."""
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / "binary.exe"
            data = bytearray(pe_fixture()) + bytearray(512)
            struct.pack_into("<H", data, 134, 1)
            struct.pack_into("<IIII", data, 400, 512, 4096, 512, 512)
            struct.pack_into("<I", data, 272, 4096)
            struct.pack_into("<I", data, 368, 4160)
            struct.pack_into("<I", data, 524, 4224)
            struct.pack_into("<II", data, 576, 1, 4256)
            data[640:653] = b"msys-one.dll\0"
            data[672:685] = b"msys-two.dll\0"
            path.write_bytes(data)
            self.assertEqual(bash.binary_imports(path), {"msys-one.dll", "msys-two.dll"})

    def test_incomplete_runtime_is_rejected(self):
        """Do not publish an executable without its DLLs and completion-protocol utilities."""
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / "bin").mkdir()
            (root / "bin/bash.exe").write_bytes(b"executable only")
            with self.assertRaisesRegex(RuntimeError, "usr.bin.msys-2.0.dll"):
                bash.verify_bash_runtime(root)


if __name__ == "__main__":
    unittest.main()
