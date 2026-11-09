"""Prevent shipping an iOS app whose FFI video frameworks are missing."""

from argparse import Namespace
from contextlib import ExitStack
from pathlib import Path
import shutil
import sys
import tempfile
import unittest
from unittest.mock import patch
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
import build_flutter_ios as ios


class IOSVideoFrameworkTests(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="operit ios video ")
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name)
        self.app = self.root / "Runner.app"
        self.app.mkdir()
        patcher = patch.object(ios, "IOS_RELEASE_APP_DIR", self.app)
        patcher.start()
        self.addCleanup(patcher.stop)

    def add_framework(self, name, content=b"native library"):
        binary = self.app / "Frameworks" / f"{name}.framework" / name
        binary.parent.mkdir(parents=True, exist_ok=True)
        binary.write_bytes(content)
        return binary

    def test_rejects_the_reported_bundle_with_mdk_but_no_fvp(self):
        self.add_framework("mdk")
        with self.assertRaisesRegex(RuntimeError, r"fvp.framework/fvp"):
            ios.validate_ios_runtime_frameworks(self.app)

    def test_rejects_missing_or_empty_native_binaries(self):
        self.add_framework("fvp")
        binary = self.add_framework("mdk", b"")
        for kind in ("empty", "directory", "missing"):
            with self.subTest(kind=kind):
                with self.assertRaisesRegex(RuntimeError, r"mdk.framework/mdk"):
                    ios.validate_ios_runtime_frameworks(self.app)
                if kind == "empty":
                    binary.unlink()
                    binary.mkdir()
                elif kind == "directory":
                    binary.rmdir()

    def test_accepts_both_embedded_libraries(self):
        self.add_framework("fvp")
        self.add_framework("mdk")
        ios.validate_ios_runtime_frameworks(self.app)

    def test_invalid_bundle_does_not_overwrite_a_previous_archive(self):
        archive = self.root / "release.zip"
        archive.write_bytes(b"previous release")
        with patch.object(ios, "run") as run:
            with self.assertRaisesRegex(RuntimeError, r"fvp.framework/fvp"):
                ios.package_ios_app(archive)
        run.assert_not_called()
        self.assertEqual(archive.read_bytes(), b"previous release")

    def test_build_without_packaging_still_checks_runtime_libraries(self):
        args = Namespace(skip_proxy_generation=True, enforce_lockfile=False,
                         build_name=None, build_number=None, skip_package=True)
        with ExitStack() as stack:
            stack.enter_context(patch.object(ios, "parse_args", return_value=args))
            for name in ("prepare_apple_sherpa", "require_command", "flutter_command",
                         "prepare_python_command", "flutter_pub_get", "run"):
                stack.enter_context(patch.object(ios, name))
            with self.assertRaisesRegex(RuntimeError, r"fvp.framework/fvp"):
                ios.main()

    @unittest.skipUnless(shutil.which("ditto"), "Apple archive tool required")
    def test_packaged_app_contains_both_runtime_binaries(self):
        self.add_framework("fvp")
        self.add_framework("mdk")
        archive = ios.package_ios_app(self.root / "release.zip")
        with zipfile.ZipFile(archive) as package:
            for name in ("fvp", "mdk"):
                self.assertEqual(package.read(f"Runner.app/Frameworks/{name}.framework/{name}"),
                                 b"native library")


if __name__ == "__main__":
    unittest.main()
