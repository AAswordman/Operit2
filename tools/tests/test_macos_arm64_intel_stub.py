import io
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest

REPO = Path(__file__).resolve().parents[2]
RUNNER = REPO / 'apps/flutter/app/macos/Runner'


@unittest.skipUnless(sys.platform == 'darwin' and shutil.which('xcrun'), 'requires macOS and Xcode')
class IntelRuntimeStubTests(unittest.TestCase):
    def test_intel_build_and_bridge(self):
        with tempfile.TemporaryDirectory(prefix='operit-intel-stub-') as temporary:
            root = Path(temporary)
            archive = root / 'rootfs.tar.gz'
            elf = bytearray(64)
            elf[:7] = b'\x7fELF\x02\x01\x01'
            elf[18:20] = (183).to_bytes(2, 'little')
            with tarfile.open(archive, 'w:gz') as tar:
                for name, data in [('bin/busybox', elf), ('sbin/apk', elf),
                                   ('etc/apk/repositories', b'repository'),
                                   ('lib/apk/db/installed', b'packages')]:
                    entry = tarfile.TarInfo(name)
                    entry.mode = 0o755
                    entry.size = len(data)
                    tar.addfile(entry, io.BytesIO(data))
            derived = root / 'derived'
            subprocess.run(['bash', REPO / 'apps/flutter/app/macos/build_arm64_runtime.sh'],
                           cwd=REPO, check=True, env={**os.environ, 'ARCHS': 'x86_64',
                           'DERIVED_FILE_DIR': str(derived), 'OPERIT_ARM64_ROOTFS_ARCHIVE': str(archive)},
                           capture_output=True, text=True)
            runtime = derived / 'Arm64Runtime'
            self.assertFalse((runtime / 'build-x86_64').exists())
            library = runtime / 'liboperit_arm64.a'
            subprocess.run(['xcrun', 'lipo', '-verify_arch', 'x86_64', library], check=True)
            binary = root / 'intel-stub'
            subprocess.run(['xcrun', '--sdk', 'macosx', 'clang++', '-std=c++17', '-arch', 'x86_64',
                            '-I', RUNNER, REPO / 'tools/tests/fixtures/arm64_macos_intel_stub.cpp',
                            RUNNER / 'Arm64TerminalBridge.mm', library, '-o', binary], check=True)
            subprocess.run([binary], check=True)


if __name__ == '__main__':
    unittest.main()
