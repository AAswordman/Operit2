#!/usr/bin/env python3
"""Run the real Kotlin Root policy/process tests without the Android SDK or a device.

Usage: JAVA_HOME=... KOTLINC=/path/to/kotlinc python3 tools/tests/test_android_root_commands.py
"""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
KOTLIN = ROOT / 'apps/flutter/app/android/app/src/main/kotlin/app/operit/core/tools/system'


class AndroidRootCommandsTest(unittest.TestCase):
    def test_kotlin_root_commands(self):
        compiler = os.environ.get('KOTLINC') or shutil.which('kotlinc')
        if not compiler:
            self.skipTest('Set KOTLINC to a Kotlin compiler; JAVA_HOME must point to Java 17+')
        java_home = os.environ.get('JAVA_HOME')
        java = str(Path(java_home) / 'bin/java') if java_home else shutil.which('java')
        self.assertIsNotNone(java, 'Java runtime is required')
        sources = [KOTLIN / name for name in (
            'AndroidPrivilegedCommandResult.kt',
            'AndroidCommandProcessRunner.kt',
            'AndroidRootCommandRouter.kt',
        )]
        sources.extend(Path(__file__).parent / 'kotlin' / name for name in (
            'AndroidRootCommandsTest.kt', 'HostTaskScheduler.kt',
        ))
        with tempfile.TemporaryDirectory(prefix='operit-root-tests-') as directory:
            jar = str(Path(directory) / 'root-tests.jar')
            subprocess.run([compiler, *map(str, sources), '-jvm-target', '17', '-include-runtime', '-d', jar], check=True, timeout=120)
            subprocess.run([java, '-jar', jar], check=True, timeout=30)


if __name__ == '__main__':
    unittest.main()
