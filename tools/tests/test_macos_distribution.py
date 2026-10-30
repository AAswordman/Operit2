"""Exercise channel separation, signature checks and both macOS signing workflows."""
from pathlib import Path
import os
import importlib.util
from importlib.machinery import SourceFileLoader
import plistlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/build_scripts"))
import build_apple_release
import build_flutter_macos
import build_local
import macos_distribution as distribution
import release_artifacts
import sign_macos


class MacosDistributionTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)

    def test_names_and_publish_validation_accept_both_channels_and_reject_old_names(self):
        for version in ("2.0.0", "2.0.0-preview.16"):
            with patch.object(release_artifacts, "release_version", return_value=version):
                for kind in distribution.MACOS_TYPES:
                    for ext in ("zip", "pkg"):
                        name = release_artifacts.release_asset_name("app", "macos", "universal", ext, kind)
                        self.assertEqual(name, f"operit2-app-macos-universal-{kind}-{version}.{ext}")
                        release_artifacts.validate_release_assets([Path(name)])
                with self.assertRaises(RuntimeError):
                    release_artifacts.validate_release_assets([Path(f"operit2-app-macos-universal-{version}.zip")])
        with self.assertRaises(ValueError):
            release_artifacts.release_asset_name("app", "macos", "universal", "zip", "typo")

    def test_build_override_is_temporary_and_preserves_existing_xcconfig(self):
        existing = self.root / "User settings.xcconfig"
        existing.write_text("CUSTOM = preserved\n")
        with patch.dict(os.environ, {"XCODE_XCCONFIG_FILE": str(existing)}):
            with distribution.build_environment("sandbox") as env:
                path = Path(env["XCODE_XCCONFIG_FILE"])
                self.assertIn(f'#include "{existing.resolve()}"', path.read_text())
                self.assertTrue(path.read_text().endswith("OPERIT_MACOS_TYPE = sandbox\n"))
                self.assertEqual(os.environ["XCODE_XCCONFIG_FILE"], str(existing))
            self.assertFalse(path.exists())
        self.assertNotEqual(distribution.built_app_path("sandbox"), distribution.built_app_path("unsandbox"))

    def test_mislabelled_signature_is_rejected(self):
        for actual in (True, False):
            signed = subprocess.CompletedProcess([], 0, plistlib.dumps({"com.apple.security.app-sandbox": actual}), b"")
            with patch.object(distribution.subprocess, "run", return_value=signed):
                distribution.verify_app_type(self.root, "sandbox" if actual else "unsandbox")
                with self.assertRaisesRegex(RuntimeError, "distribution mismatch"):
                    distribution.verify_app_type(self.root, "unsandbox" if actual else "sandbox")

    def test_dispatchers_forward_channel_and_match_archive_names(self):
        with patch.object(build_local, "run") as run:
            build_local.build_local_app("macos", False, "unsandbox")
        command = run.call_args.args[0]
        self.assertEqual(command[command.index("--type") + 1], "unsandbox")
        self.assertIn("-unsandbox-", Path(command[command.index("--archive-path") + 1]).name)
        with patch.object(build_apple_release, "run") as run:
            build_apple_release.build_apple_apps("2.0.0", "1", self.root, False)
        self.assertEqual(run.call_count, 2)
        for call, kind in zip(run.call_args_list, distribution.MACOS_TYPES):
            command = call.args[0]
            self.assertEqual(command[command.index("--type") + 1], kind)
            self.assertIn(f"-{kind}-", Path(command[-1]).name)
        with patch.object(sys, "argv", ["build_flutter_macos.py"]):
            self.assertEqual(build_flutter_macos.parse_args().type, "sandbox")

    def test_signing_uses_channel_specific_profile_identity_and_package(self):
        app = self.root / "source" / "Operit2.app"
        (app / "Contents" / "Frameworks" / "Example.framework").mkdir(parents=True)
        (app / "Contents" / "Info.plist").write_bytes(plistlib.dumps({"CFBundleIdentifier": "app.original"}))
        (app / "Contents" / "embedded.provisionprofile").write_bytes(b"stale profile")
        profile = self.root / "store.profile"
        profile.write_bytes(b"profile")
        for kind in distribution.MACOS_TYPES:
            calls = []
            def fake_run(command):
                command = [str(part) for part in command]
                calls.append(command)
                if command[0] in ("ditto", "productbuild"):
                    Path(command[-1]).write_bytes(b"package")
            allowed = {"com.apple.application-identifier": "TEAM123.app.store", "com.apple.developer.team-identifier": "TEAM123"}
            with patch.object(sign_macos, "run", side_effect=fake_run), patch.object(sign_macos, "verify_app_type"), patch.object(sign_macos, "profile_entitlements", return_value=allowed) as read_profile:
                package = sign_macos.sign_macos_app(
                    app, self.root / kind, profile if kind == "sandbox" else None,
                    distribution.entitlements_path(kind), "channel app identity",
                    "store installer" if kind == "sandbox" else None, kind,
                )
            self.assertTrue(package.is_file())
            signed_app = self.root / kind / "Operit2.app"
            info = plistlib.loads((signed_app / "Contents/Info.plist").read_bytes())
            app_sign = next(command for command in calls if command[0] == "codesign" and "--entitlements" in command)
            if kind == "sandbox":
                read_profile.assert_called_once()
                self.assertEqual(info["CFBundleIdentifier"], "app.store")
                self.assertEqual(package.suffix, ".pkg")
                self.assertTrue((signed_app / "Contents/embedded.provisionprofile").exists())
            else:
                read_profile.assert_not_called()
                self.assertEqual(info["CFBundleIdentifier"], "app.original")
                self.assertEqual(package.suffix, ".zip")
                self.assertFalse((signed_app / "Contents/embedded.provisionprofile").exists())
                self.assertIn("runtime", app_sign)
                self.assertFalse(any(command[0] == "productbuild" for command in calls))

    def test_lipo_compatibility_checks_every_arch_and_preserves_failures(self):
        source = REPO / "apps/flutter/app/macos/toolchain/lipo"
        loader = SourceFileLoader("operit_lipo_compat", str(source))
        spec = importlib.util.spec_from_loader(loader.name, loader)
        compat = importlib.util.module_from_spec(spec)
        loader.exec_module(compat)
        binary = self.root / "fat-binary"
        binary.write_bytes(b"test")
        with patch.object(compat.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)) as run:
            self.assertEqual(compat.run_lipo("real-lipo", [str(binary), "-verify_arch", "arm64", "x86_64"]), 0)
            self.assertEqual([call.args[0][-1] for call in run.call_args_list], ["arm64", "x86_64"])
        with patch.object(compat.subprocess, "run", side_effect=[subprocess.CompletedProcess([], 0), subprocess.CompletedProcess([], 1)]):
            self.assertEqual(compat.run_lipo("real-lipo", [str(binary), "-verify_arch", "arm64", "x86_64"]), 1)
        with patch.object(compat.subprocess, "run", return_value=subprocess.CompletedProcess([], 0)) as run:
            args = ["-create", "first", "second", "-output", "universal"]
            self.assertEqual(compat.run_lipo("real-lipo", args), 0)
            run.assert_called_once_with(["real-lipo", *args])

    def test_identity_selection_does_not_take_development_certificate(self):
        identities = '\n'.join([
            '1) AAA "Apple Development: Developer"',
            '2) BBB "Developer ID Application: Direct (TEAM)"',
            '3) CCC "Apple Distribution: Store (TEAM)"',
            '4) DDD "3rd Party Mac Developer Installer: Store (TEAM)"',
        ])
        with patch.object(sign_macos.subprocess, "check_output", return_value=identities):
            self.assertTrue(sign_macos.first_identity("unsandbox").startswith("Developer ID Application:"))
            self.assertTrue(sign_macos.first_identity("sandbox").startswith("Apple Distribution:"))
            self.assertTrue(sign_macos.first_identity("installers").startswith("3rd Party Mac Developer Installer:"))


if __name__ == "__main__":
    unittest.main()
