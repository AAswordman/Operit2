"""Check CLI signing commands without accessing certificates, private keys, or real Keychain items."""
import importlib.util
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

REPO_ROOT = Path(__file__).resolve().parents[2]
SPEC = importlib.util.spec_from_file_location("cli_signing_under_test", REPO_ROOT / "tools/build_scripts/cli_macos_signing.py")
signing = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(signing)


class CliMacosSigningTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="operit-cli-signing-tests-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.config = self.root / "settings.local"
        for patcher in (patch.object(signing, "LOCAL_SIGNING_CONFIG", self.config), patch.dict(os.environ, {}, clear=True)):
            patcher.start()
            self.addCleanup(patcher.stop)

    def test_missing_configuration_never_selects_a_certificate_implicitly(self):
        self.assertIsNone(signing.signing_identity())
        with patch.object(signing.platform, "system", return_value="Darwin"), patch.object(signing.subprocess, "run") as run:
            self.assertFalse(signing.sign_cli_macos(self.root / "missing-binary"))
            run.assert_not_called()

    def test_explicit_environment_and_local_identity_precedence(self):
        self.config.write_text(json.dumps({"identity": "local-certificate"}), encoding="utf-8")
        self.assertEqual(signing.signing_identity(), "local-certificate")
        with patch.dict(os.environ, {"OPERIT_MACOS_SIGNING_IDENTITY": "environment-certificate"}):
            self.assertEqual(signing.signing_identity(), "environment-certificate")
            self.assertEqual(signing.signing_identity("explicit-certificate"), "explicit-certificate")
            with self.assertRaises(ValueError):
                signing.signing_identity("")

    def test_blank_adhoc_and_malformed_settings_are_rejected(self):
        for identity in ("", " ", "-", " - "):
            with self.subTest(identity=identity), self.assertRaises(ValueError):
                signing.signing_identity(identity)
        for config in ({}, {"identity": 1}, [], None):
            self.config.write_text(json.dumps(config), encoding="utf-8")
            with self.subTest(config=config), self.assertRaises(ValueError):
                signing.signing_identity()

    def test_every_rebuilt_artifact_uses_the_same_identifier_and_is_verified(self):
        with patch.object(signing.platform, "system", return_value="Darwin"), patch.object(signing.subprocess, "run") as run:
            for version in ("old", "rebuilt"):
                binary = self.root / version / "operit2"
                binary.parent.mkdir()
                binary.write_bytes(version.encode())
                self.assertTrue(signing.sign_cli_macos(binary, "test-certificate"))
                self.assertEqual(run.call_args_list[-2].args[0], [
                    "codesign", "--force", "--sign", "test-certificate", "--identifier", "com.operit.cli",
                    "--timestamp=none", str(binary.resolve()),
                ])
                self.assertEqual(run.call_args_list[-1].args[0], ["codesign", "--verify", "--strict", str(binary.resolve())])
                self.assertTrue(run.call_args_list[-1].kwargs["check"])

    def test_non_macos_never_invokes_codesign(self):
        with patch.object(signing.platform, "system", return_value="Linux"), patch.object(signing.subprocess, "run") as run:
            self.assertFalse(signing.sign_cli_macos(self.root / "operit2", "test-certificate"))
            run.assert_not_called()

    def test_signing_failure_propagates_without_an_adhoc_or_insecure_fallback(self):
        binary = self.root / "operit2"
        binary.write_bytes(b"fixture")
        with patch.object(signing.platform, "system", return_value="Darwin"), patch.object(signing.subprocess, "run", side_effect=subprocess.CalledProcessError(1, "codesign")) as run:
            with self.assertRaises(subprocess.CalledProcessError):
                signing.sign_cli_macos(binary, "test-certificate")
            self.assertEqual(run.call_count, 1)


if __name__ == "__main__":
    unittest.main()
