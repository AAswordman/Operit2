"""Exercise Apple build entry points without invoking Xcode or compiling Rust."""

import os
import shutil
import sys
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
    def run_build(self, platform, sdk, archs, targets, *, sync_exit=0, v8_exit=0):
        with tempfile.TemporaryDirectory(prefix="operit apple plugins ") as temporary:
            root = Path(temporary)
            project = root / "apps/flutter/app" / platform
            project.mkdir(parents=True)
            commands = root / "commands"
            commands.mkdir()
            log = root / "calls.log"
            cargo_wrapper = root / "tools/build_scripts/cargo_build.py"
            cargo_wrapper.parent.mkdir(parents=True)
            shutil.copy2(REPO_ROOT / "tools/build_scripts/cargo_build.py", cargo_wrapper)

            def stub(path, body):
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("#!/bin/sh\nset -eu\n" + body, encoding="utf-8")
                path.chmod(0o755)

            stub(root / ".venv/bin/python", """
case "$(basename "$1")" in
  sync_plugin_packages.py)
    shift
    printf 'sync %s\\n' "$*" >> "$OPERIT_TEST_CALL_LOG"
    exit "$OPERIT_TEST_SYNC_EXIT"
    ;;
  prepare_ios_v8.py)
    shift
    printf 'v8 %s\\n' "$*" >> "$OPERIT_TEST_CALL_LOG"
    test "$OPERIT_TEST_V8_EXIT" = 0 || exit "$OPERIT_TEST_V8_EXIT"
    test "$1" = --target
    target="$2"
    test "$3" = --env-file
    printf 'export RUSTY_V8_ARCHIVE=/verified/%s/library.a\\nexport RUSTY_V8_SRC_BINDING_PATH=/verified/%s/bindings.rs\\n' "$target" "$target" > "$4"
    ;;
  cargo_build.py) exec "$OPERIT_TEST_PYTHON" "$@" ;;
  *) exit 99 ;;
esac
""")
            stub(commands / "rustup", 'printf \'rustup %s\\n\' "$*" >> "$OPERIT_TEST_CALL_LOG"\n')
            stub(commands / "cargo", """
if test "$PLATFORM_NAME" != macosx; then
  case "$RUSTY_V8_ARCHIVE:$RUSTY_V8_SRC_BINDING_PATH" in
    /verified/*/library.a:/verified/*/bindings.rs) ;;
    *) exit 98 ;;
  esac
fi
test -z "${RUSTFLAGS:-}"
case "$CARGO_ENCODED_RUSTFLAGS" in
  *"--remap-path-prefix=$OPERIT_TEST_REPO_ROOT=/operit"*) ;;
  *) exit 97 ;;
esac
printf 'cargo %s\\n' "$*" >> "$OPERIT_TEST_CALL_LOG"
""")
            for target in targets:
                library = root / "apps/flutter/native/operit-flutter-bridge/target" / target / "release/liboperit_flutter_bridge.a"
                library.parent.mkdir(parents=True, exist_ok=True)
                library.write_bytes(b"test library")

            result = subprocess.run(
                ["/bin/sh", str(REPO_ROOT / "apps/flutter/app" / platform / "build_rust_bridge.sh")],
                env={**os.environ, "PROJECT_DIR": str(project), "PLATFORM_NAME": sdk,
                     "ARCHS": archs, "PATH": f"{commands}{os.pathsep}{os.environ['PATH']}",
                     "OPERIT_TEST_PYTHON": sys.executable, "OPERIT_TEST_REPO_ROOT": str(root),
                     "OPERIT_TEST_CALL_LOG": str(log), "OPERIT_TEST_SYNC_EXIT": str(sync_exit),
                     "OPERIT_TEST_V8_EXIT": str(v8_exit)},
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
                stride = 3 if case[0] == "ios" else 2
                self.assertEqual(len(calls), 1 + stride * len(case[3]))
                for index, target in enumerate(case[3]):
                    start = 1 + index * stride
                    self.assertEqual(calls[start], f"rustup target add {target}")
                    if case[0] == "ios":
                        self.assertTrue(calls[start + 1].startswith(f"v8 --target {target} --env-file "))
                    self.assertIn(f"--target {target}", calls[start + stride - 1])

    def test_packaging_failure_prevents_building_an_incomplete_app(self):
        for case in CASES:
            with self.subTest(platform=case[:3]):
                result, calls = self.run_build(*case, sync_exit=7)
                self.assertEqual(result.returncode, 7, result.stderr)
                self.assertEqual(calls, ["sync --source runtime --no-hot-reload"])

    def test_v8_preparation_failure_prevents_linking_the_published_archive(self):
        result, calls = self.run_build(*CASES[0], v8_exit=8)
        self.assertEqual(result.returncode, 8, result.stderr)
        self.assertFalse(any(line.startswith("cargo ") for line in calls))

    def test_flutter_phases_do_not_repeat_plugin_sync_after_rust(self):
        for platform in ("ios", "macos"):
            with self.subTest(platform=platform):
                project = REPO_ROOT / "apps/flutter/app" / platform / "Runner.xcodeproj/project.pbxproj"
                self.assertNotIn("sync_plugin_packages.py", project.read_text(encoding="utf-8"))


if __name__ == "__main__":
    unittest.main()
