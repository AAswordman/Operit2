"""Exercise release launch failures without running macOS applications in unit tests."""
from pathlib import Path
import plistlib
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / "tools/build_scripts"))
import build_flutter_macos
import macos_validation as validation


class MacosValidationTests(unittest.TestCase):
    def setUp(self):
        """Create a minimal executable bundle for isolated process checks."""
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.app = self.root / "Operit2.app"
        self.executable = self.app / "Contents/MacOS/Operit2"
        self.executable.parent.mkdir(parents=True)
        self.executable.write_bytes(b"executable")
        (self.app / "Contents/Info.plist").write_bytes(plistlib.dumps({"CFBundleExecutable": "Operit2"}))

    def test_early_exit_fails_even_when_status_is_zero_and_keeps_dyld_output(self):
        """Require a living GUI process and preserve the loader's diagnostic output."""
        for status in (0, -6, 1):
            process = MagicMock()
            process.wait.return_value = status
            process.poll.return_value = status

            def launch(command, **options):
                """Simulate dyld rejecting an embedded library at process startup."""
                options["stdout"].write(b"dyld: library load rejected: no Team ID")
                return process

            with self.subTest(status=status), patch.object(validation.subprocess, "Popen", side_effect=launch):
                with self.assertRaisesRegex(RuntimeError, f"status {status}.*\n.*no Team ID"):
                    validation.verify_app_launch(self.app, 0.1)
            process.kill.assert_not_called()

    def test_surviving_process_is_stopped_and_reaped(self):
        """Bound the launch check and leave no main application process running."""
        process = MagicMock()
        process.wait.side_effect = [subprocess.TimeoutExpired([], 15), -9]
        process.poll.return_value = None
        with patch.object(validation.subprocess, "Popen", return_value=process) as launch:
            validation.verify_app_launch(self.app)
        self.assertEqual(launch.call_args.args[0], [str(self.executable)])
        self.assertEqual(launch.call_args.kwargs["cwd"], self.root)
        process.kill.assert_called_once_with()
        self.assertEqual(process.wait.call_args_list[-1].kwargs, {"timeout": 10})

    def test_archive_validation_propagates_launch_failure_and_cleans_extraction(self):
        """Check the extracted ZIP and retain failures instead of publishing success."""
        archive = self.root / "release.zip"
        with patch.object(validation, "run") as run, patch.object(validation, "verify_app_type") as signature, patch.object(validation, "verify_app_launch", side_effect=RuntimeError("startup crash")) as launch:
            with self.assertRaisesRegex(RuntimeError, "startup crash"):
                validation.verify_macos_archive(archive, "unsandbox")
        extracted_app = launch.call_args.args[0]
        self.assertNotEqual(extracted_app, self.app)
        signature.assert_called_once_with(extracted_app, "unsandbox")
        self.assertEqual(run.call_args_list[0].args[0][:3], ["ditto", "-x", "-k"])
        self.assertEqual(run.call_args_list[1].args[0], ["codesign", "--verify", "--deep", "--strict", str(extracted_app)])
        self.assertFalse(extracted_app.parent.exists())

    def test_build_packager_does_not_report_success_after_launch_failure(self):
        """Ensure the CI entry point enforces the packaged application's launch check."""
        archive = self.root / "release.zip"

        def package(command):
            """Produce an archive placeholder for the platform-independent unit test."""
            Path(command[-1]).write_bytes(b"zip")

        with patch.object(build_flutter_macos, "built_app_path", return_value=self.app), patch.object(build_flutter_macos, "verify_app_type"), patch.object(build_flutter_macos, "run", side_effect=package), patch.object(build_flutter_macos, "verify_macos_archive", side_effect=RuntimeError("startup crash")) as validate:
            with self.assertRaisesRegex(RuntimeError, "startup crash"):
                build_flutter_macos.package_macos_app(archive, "unsandbox")
        validate.assert_called_once_with(archive, "unsandbox")


if __name__ == "__main__":
    unittest.main()
