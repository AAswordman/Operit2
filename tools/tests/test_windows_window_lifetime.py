"""Exercise production Win32 window teardown with an observable Flutter controller."""
from pathlib import Path
import os
import shutil
import subprocess
import tempfile
import unittest

REPO_ROOT = Path(__file__).resolve().parents[2]
FLUTTER = REPO_ROOT / "apps/flutter/app/windows/flutter/ephemeral"


@unittest.skipUnless(os.name == "nt" and shutil.which("cmake"), "Windows and CMake required")
class WindowsWindowLifetimeTests(unittest.TestCase):
    # Build only the native lifetime fixture, without compiling the app or Rust.
    def test_secondary_windows_unregister_before_engine_teardown(self):
        self.assertTrue((FLUTTER / "flutter_windows.dll.lib").is_file(),
                        "Run FVM Flutter Windows setup to prepare engine headers and libraries")
        with tempfile.TemporaryDirectory(prefix="operit-window-lifetime-") as temporary:
            build = Path(temporary)
            fixture = Path(__file__).parent / "fixtures/windows_window_lifetime"
            commands = [
                ["cmake", "-S", str(fixture), "-B", str(build),
                 "-G", "Visual Studio 17 2022", "-A", "x64",
                 f"-DOPERIT_REPO_ROOT={REPO_ROOT.as_posix()}"],
                ["cmake", "--build", str(build), "--config", "Release"],
                [str(build / "Release/window_lifetime_test.exe")],
                [str(build / "Release/keyboard_message_test.exe")],
            ]
            environment = {**os.environ, "PATH": str(FLUTTER) + os.pathsep + os.environ["PATH"]}
            for command in commands:
                result = subprocess.run(command, capture_output=True, text=True,
                                        errors="replace", env=environment, timeout=60)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)


if __name__ == "__main__":
    unittest.main()
