"""Exercise Apple build entry points without invoking Xcode or compiling Rust."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest


REPO_ROOT = Path(__file__).resolve().parents[2]
CASES = (
    ("ios", "iphoneos", "arm64", ("aarch64-apple-ios",)),
    ("ios", "iphonesimulator", "arm64 x86_64", (
        "aarch64-apple-ios-sim", "x86_64-apple-ios",
    )),
    ("macos", "macosx", "arm64 x86_64", (
        "aarch64-apple-darwin", "x86_64-apple-darwin",
    )),
)


class ApplePluginBuildTests(unittest.TestCase):
    def run_build(self, platform, sdk, archs, targets, *, sync_exit=0):
        with tempfile.TemporaryDirectory(prefix="operit apple plugins ") as temporary:
            root = Path(temporary)
            project = root / "apps/flutter/app" / platform
            project.mkdir(parents=True)
            commands = root / "commands"
            commands.mkdir()
            log = root / "calls.log"

            def stub(path, body):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("#!/bin/sh\nset -eu\n" + body, encoding="utf-8")
                path.chmod(0o755)

            stub(root / ".venv/bin/python", """
test "$(basename "$1")" = sync_plugin_packages.py
shift
printf 'sync %s\\n' "$*" >> "$OPERIT_TEST_CALL_LOG"
exit "$OPERIT_TEST_SYNC_EXIT"
""")
            for command in ("cargo", "rustup"):
                stub(commands / command,
                     f'printf \'{command} %s\\n\' "$*" >> "$OPERIT_TEST_CALL_LOG"\n')
            for target in targets:
                library = root / "apps/flutter/native/operit-flutter-bridge/target" / target / "release/liboperit_flutter_bridge.a"
                library.parent.mkdir(parents=True, exist_ok=True)
                library.write_bytes(b"test library")

            result = subprocess.run(
                ["/bin/sh", str(REPO_ROOT / "apps/flutter/app" / platform / "build_rust_bridge.sh")],
                env={**os.environ, "PROJECT_DIR": str(project), "PLATFORM_NAME": sdk,
                     "ARCHS": archs, "PATH": f"{commands}{os.pathsep}{os.environ['PATH']}",
                     "OPERIT_TEST_CALL_LOG": str(log), "OPERIT_TEST_SYNC_EXIT": str(sync_exit)},
                capture_output=True, text=True, check=False,
            )
            calls = log.read_text(encoding="utf-8").splitlines() if log.exists() else []
            if result.returncode == 0:
                output = project / "Flutter/ephemeral/rust"
                if platform == "ios":
                    output /= sdk
                for arch in archs.split():
                    self.assertEqual((output / arch / "liboperit_flutter_bridge.a").read_bytes(), b"test library")
            return result, calls

    def test_all_runtime_plugins_are_prepared_before_any_architecture_compiles(self):
        for case in CASES:
            with self.subTest(platform=case[:3]):
                result, calls = self.run_build(*case)
                self.assertEqual(result.returncode, 0, result.stderr)
                self.assertEqual(calls[0], "sync --source runtime --no-hot-reload")
                self.assertEqual(sum(line.startswith("sync ") for line in calls), 1)
                self.assertEqual(len(calls), 1 + 2 * len(case[3]))
                for index, target in enumerate(case[3]):
                    self.assertEqual(calls[1 + index * 2], f"rustup target add {target}")
                    self.assertIn(f"--target {target}", calls[2 + index * 2])

    def test_packaging_failure_prevents_building_an_incomplete_app(self):
        for case in CASES:
            with self.subTest(platform=case[:3]):
                result, calls = self.run_build(*case, sync_exit=7)
                self.assertEqual(result.returncode, 7, result.stderr)
                self.assertEqual(calls, ["sync --source runtime --no-hot-reload"])

    def test_flutter_phases_do_not_repeat_plugin_sync_after_rust(self):
        for platform in ("ios", "macos"):
            with self.subTest(platform=platform):
                project = REPO_ROOT / "apps/flutter/app" / platform / "Runner.xcodeproj/project.pbxproj"
                self.assertNotIn("sync_plugin_packages.py", project.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
