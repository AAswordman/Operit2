"""Verify DMG staging, mounted-app checks, and propagation of release failures."""
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/build_scripts"))
import build_flutter_macos
import macos_packaging as packaging
import macos_validation as validation


class MacosPackagingTests(unittest.TestCase):
    def setUp(self):
        """Create an app containing a payload whose bytes must survive staging."""
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.app = self.root / "Operit2.app"
        self.app.mkdir()
        (self.app / "payload").write_bytes(b"signed bundle bytes")
        self.image = self.root / "release.dmg"

    def test_disk_image_contains_app_and_applications_link_and_is_verified(self):
        """Stage the actual bundle without changing it, then validate the final image."""
        def create(command):
            """Inspect staged bytes at the hdiutil boundary and produce an image."""
            staging = Path(command[command.index("-srcfolder") + 1])
            self.assertEqual((staging / "Operit2.app/payload").read_bytes(), b"signed bundle bytes")
            self.assertIn("UDZO", command)
            self.image.write_bytes(b"disk image")

        with patch.object(packaging, "run", side_effect=create), patch.object(Path, "symlink_to") as link, patch.object(packaging, "verify_macos_dmg") as verify:
            self.assertEqual(packaging.package_macos_dmg(self.app, self.image, "unsandbox"), self.image)
        link.assert_called_once_with("/Applications", target_is_directory=True)
        verify.assert_called_once_with(self.image, "unsandbox")

    def test_mounted_app_launch_failure_is_preserved_and_volume_is_detached(self):
        """Detach even when the packaged app fails the real startup check."""
        with patch.object(validation, "run") as run, patch.object(validation, "verify_app_type") as signature, patch.object(validation, "verify_app_launch", side_effect=RuntimeError("dyld rejected library")) as launch:
            with self.assertRaisesRegex(RuntimeError, "dyld rejected library"):
                validation.verify_macos_dmg(self.image, "unsandbox")
        commands = [call.args[0] for call in run.call_args_list]
        self.assertEqual(commands[0], ["hdiutil", "verify", str(self.image.resolve())])
        self.assertEqual(commands[1][:4], ["hdiutil", "attach", "-readonly", "-nobrowse"])
        mounted_app = launch.call_args.args[0]
        signature.assert_called_once_with(mounted_app, "unsandbox")
        self.assertEqual(commands[-1], ["hdiutil", "detach", str(mounted_app.parent)])

    def test_build_packager_requires_both_zip_and_dmg_validation(self):
        """Keep the existing CI entry point responsible for producing the DMG."""
        archive = self.root / "release.zip"

        def create(command):
            """Write the ZIP placeholder expected by the build packager."""
            Path(command[-1]).write_bytes(b"zip")

        with patch.object(build_flutter_macos, "built_app_path", return_value=self.app), patch.object(build_flutter_macos, "verify_app_type"), patch.object(build_flutter_macos, "run", side_effect=create), patch.object(build_flutter_macos, "verify_macos_archive") as zip_check, patch.object(build_flutter_macos, "package_macos_dmg", side_effect=RuntimeError("invalid disk image")) as dmg:
            with self.assertRaisesRegex(RuntimeError, "invalid disk image"):
                build_flutter_macos.package_macos_app(archive, "unsandbox")
        zip_check.assert_called_once_with(archive, "unsandbox")
        dmg.assert_called_once_with(self.app, self.image, "unsandbox")


if __name__ == "__main__":
    unittest.main()
