"""Verify Windows runtime packaging and Flutter staging without building Rust."""
from contextlib import ExitStack
from pathlib import Path
import os
import shutil
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch
from zipfile import ZipFile

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/build_scripts"))
import cli_common


class WindowsRuntimePackagingTests(unittest.TestCase):
    # Isolate build outputs and replace only commands that would compile Rust.
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="operit-runtime-tests-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.patches = ExitStack()
        self.addCleanup(self.patches.close)
        for name, value in (
            ("REPO_ROOT", self.root),
            ("CLI_WORK_DIR", self.root / "work"),
            ("DIST_DIR", self.root / "dist"),
        ):
            self.patches.enter_context(patch.object(cli_common, name, value))
        for name in ("require_command", "prepare_plugin_assets", "run", "verify_cli_plugin_assets"):
            self.patches.enter_context(patch.object(cli_common, name))
        self.patches.enter_context(patch.object(cli_common, "windows_aarch64_env", return_value={}))

    # Exercise actual file staging and ZIP creation for both Cargo output layouts.
    def test_windows_archives_include_the_native_runtime(self):
        for arch, default_target in (("x86_64", True), ("x86_64", False), ("aarch64", False)):
            with self.subTest(arch=arch, default_target=default_target):
                target = cli_common.cli_target("windows", arch)
                binary = (self.root / "apps/cli/target/release/operit2.exe"
                          if default_target else cli_common.cli_target_binary_path(target))
                binary.parent.mkdir(parents=True, exist_ok=True)
                binary.write_bytes(b"cli executable")
                binary.with_name("operit_local_llm.dll").write_bytes(b"native inference runtime")
                archive_path = cli_common.build_cli_target(target, use_default_target=default_target)
                with ZipFile(archive_path) as archive:
                    self.assertEqual(archive.read("operit2.exe"), b"cli executable")
                    self.assertEqual(archive.read("operit_local_llm.dll"), b"native inference runtime")
                    self.assertIn("install.bat", archive.namelist())

    # A missing native dependency must prevent publication of an unusable package.
    def test_windows_package_rejects_a_missing_runtime(self):
        target = cli_common.cli_target("windows", "x86_64")
        binary = cli_common.cli_target_binary_path(target)
        binary.parent.mkdir(parents=True)
        binary.write_bytes(b"cli executable")
        with self.assertRaisesRegex(RuntimeError, r"operit_local_llm\.dll"):
            cli_common.build_cli_target(target)
        self.assertFalse(cli_common.cli_package_path(target).exists())


@unittest.skipUnless(shutil.which("cmake"), "CMake is required for the staging check")
class FlutterRuntimeStagingTests(unittest.TestCase):
    # Provide prebuilt fixture artifacts and a Cargo stub for the production script.
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory(prefix="operit-flutter-staging-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        self.cargo = self.root / ("cargo.cmd" if os.name == "nt" else "cargo")
        self.cargo.write_text("@exit /b 0\n" if os.name == "nt" else "#!/bin/sh\nexit 0\n")
        self.cargo.chmod(0o755)
        self.crate = self.root / "crate"
        self.crate.mkdir()
        self.output = self.root / "runner"

    # Invoke the same deployment script that MSBuild runs after Cargo finishes.
    def stage(self, config):
        return subprocess.run([
            "cmake", f"-DOPERIT_BUILD_CONFIG={config}",
            f"-DOPERIT_CARGO_EXECUTABLE={self.cargo.as_posix()}",
            f"-DOPERIT_REPO_ROOT={REPO_ROOT.as_posix()}",
            f"-DOPERIT_PLUGIN_SYNC_PYTHON={Path(sys.executable).as_posix()}",
            f"-DOPERIT_FLUTTER_BRIDGE_CRATE={self.crate.as_posix()}",
            f"-DOPERIT_OUTPUT_DIRECTORY={self.output.as_posix()}",
            "-P", str(REPO_ROOT / "apps/flutter/app/windows/runner/operit_flutter_bridge_build.cmake"),
        ], capture_output=True, text=True, check=False)

    # Debug, Release, and Profile must all deploy the DLL pair from their profile.
    def test_all_flutter_configurations_stage_both_runtime_dlls(self):
        for config, profile in (("Debug", "debug"), ("Release", "release"), ("Profile", "release")):
            with self.subTest(config=config):
                source = self.crate / "target" / profile
                source.mkdir(parents=True, exist_ok=True)
                files = {"operit_flutter_bridge.dll": b"bridge", "operit_local_llm.dll": b"inference"}
                if config == "Debug":
                    files["operit_flutter_bridge.pdb"] = b"debug symbols"
                for name, payload in files.items():
                    (source / name).write_bytes(payload + config.encode())
                result = self.stage(config)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                for name in files:
                    self.assertEqual((self.output / name).read_bytes(), (source / name).read_bytes())

    # A successful Cargo command alone cannot make missing runtime assets valid.
    def test_flutter_staging_fails_when_the_native_runtime_is_missing(self):
        source = self.crate / "target/release"
        source.mkdir(parents=True)
        (source / "operit_flutter_bridge.dll").write_bytes(b"bridge")
        result = self.stage("Release")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Failed to deploy", result.stderr)
        self.assertIn("operit_local_llm.dll", result.stderr)


if __name__ == "__main__":
    unittest.main()
