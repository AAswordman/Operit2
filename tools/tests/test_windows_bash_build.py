"""Exercise the Windows shell CMake targets without compiling Flutter or Rust."""

import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

REPO_ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO_ROOT / "tools/build_scripts"))
from windows_bash import REQUIRED_FILES
from windows_powershell import REQUIRED_FILES as POWERSHELL_REQUIRED_FILES


@unittest.skipUnless(os.name == "nt" and shutil.which("cmake"), "Requires Windows CMake/MSBuild")
class WindowsBashBuildTests(unittest.TestCase):
    """Check normal build deployment, incremental builds, missing outputs and installation."""

    runtime_name = "bash"
    runtime_files = REQUIRED_FILES
    executable = "bin/bash.exe"
    dependency = "usr/bin/msys-2.0.dll"

    def setUp(self):
        """Use the production CMake target with an offline runtime preparation fixture."""
        work = (REPO_ROOT / "tools/release/work").resolve()
        work.mkdir(parents=True, exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=work, prefix=f"operit-{self.runtime_name}-build-")
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.assertEqual(self.root.parent, work)
        self.build = self.root / "build"
        scripts = self.root / "tools/build_scripts"
        scripts.mkdir(parents=True)
        (scripts / "common.py").write_text("# CMake dependency fixture\n")
        self.preparer = scripts / f"windows_{self.runtime_name}.py"
        self.preparer.write_text(
            "import argparse, os\nfrom pathlib import Path\n"
            "parser = argparse.ArgumentParser()\n"
            "parser.add_argument('--destination', type=Path)\nparser.add_argument('--arch')\n"
            "args = parser.parse_args()\n"
            "root = Path(__file__).resolve().parents[2]\n"
            "if (root / 'fail').exists(): raise SystemExit('runtime preparation failed')\n"
            "with (root / 'calls.txt').open('a') as log: log.write(args.arch + '\\n')\n"
            f"for name in {(*self.runtime_files, 'operit-bundle.json')!r}:\n"
            f"    output = args.destination / {self.runtime_name!r} / name\n"
            "    output.parent.mkdir(parents=True, exist_ok=True)\n"
            "    output.write_bytes(b'bundled runtime fixture')\n"
            "    os.utime(output, (946684800, 946684800))\n",
            encoding="utf-8",
        )
        cmake = REPO_ROOT / f"apps/flutter/app/windows/runner/operit_windows_{self.runtime_name}.cmake"
        (self.root / "CMakeLists.txt").write_text(
            'cmake_minimum_required(VERSION 3.14)\nproject(BashBuild NONE)\n'
            'set(CMAKE_CONFIGURATION_TYPES "Debug;Profile;Release" CACHE STRING "" FORCE)\n'
            'set(FLUTTER_TARGET_PLATFORM "windows-x64")\n'
            f'set(OPERIT_REPO_ROOT "{self.root.as_posix()}")\n'
            f'set(OPERIT_PLUGIN_SYNC_PYTHON "{Path(sys.executable).as_posix()}")\n'
            f'include("{cmake.as_posix()}")\n'
            # The parent Flutter CMake project owns installation of this target's output.
            'install(DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/$<CONFIG>/' + self.runtime_name + '"\n'
            '  DESTINATION "." COMPONENT Runtime)\n', encoding="utf-8",
        )
        configured = self.invoke("cmake", "-S", self.root, "-B", self.build,
                                 "-G", "Visual Studio 17 2022", "-A", "x64")
        self.assertEqual(configured.returncode, 0, configured.stdout)

    def invoke(self, *arguments):
        """Keep build subprocesses hidden and retain errors for failed assertions."""
        return subprocess.run([str(part) for part in arguments], text=True, encoding="utf-8", errors="replace",
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                              creationflags=subprocess.CREATE_NO_WINDOW)

    def stage(self, config):
        """Build only the actual interpreter target without invoking Cargo."""
        return self.invoke("cmake", "--build", self.build, "--config", config,
                           "--target", f"operit_windows_{self.runtime_name}")

    def test_all_build_modes_stage_runtime_incrementally_and_install_it(self):
        """Deploy all runtime files, skip unchanged builds, and repair deleted dependencies."""
        for index, config in enumerate(("Debug", "Profile", "Release"), start=1):
            with self.subTest(config=config):
                result = self.stage(config)
                self.assertEqual(result.returncode, 0, result.stdout)
                runtime = self.build / config / self.runtime_name
                for name in (*self.runtime_files, "operit-bundle.json"):
                    self.assertEqual((runtime / name).read_bytes(), b"bundled runtime fixture")
                before = (runtime / self.executable).stat().st_mtime_ns
                result = self.stage(config)
                self.assertEqual(result.returncode, 0, result.stdout)
                self.assertEqual((runtime / self.executable).stat().st_mtime_ns, before)
                self.assertEqual((self.root / "calls.txt").read_text().splitlines(), ["x86_64"] * index)
        (self.build / "Debug" / self.runtime_name / self.dependency).unlink()
        repaired = self.stage("Debug")
        self.assertEqual(repaired.returncode, 0, repaired.stdout)
        self.assertTrue((self.build / "Debug" / self.runtime_name / self.dependency).is_file())
        installed = self.invoke("cmake", "--install", self.build, "--config", "Debug",
                                "--prefix", self.root / "install")
        self.assertEqual(installed.returncode, 0, installed.stdout)
        self.assertTrue((self.root / "install" / self.runtime_name / self.executable).is_file())

    def test_preparation_failure_fails_the_build(self):
        """Do not report a successful runner build when its runtime could not be prepared."""
        (self.root / "fail").write_text("fail preparation")
        result = self.stage("Debug")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("runtime preparation failed", result.stdout)


class WindowsPowerShellBuildTests(WindowsBashBuildTests):
    """Apply the build, incrementality and installation checks to bundled PowerShell 7."""

    runtime_name = "powershell"
    runtime_files = POWERSHELL_REQUIRED_FILES
    executable = "pwsh.exe"
    dependency = "coreclr.dll"


if __name__ == "__main__":
    unittest.main()
