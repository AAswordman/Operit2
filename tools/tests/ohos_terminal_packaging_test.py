import unittest
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch
import json
import sys
import zipfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'build_scripts'))
import build_flutter_ohos as build


class OhosTerminalPackagingTests(unittest.TestCase):
    """Checks the terminal runtime is present in the actual package before signing."""

    def test_declared_terminal_packages_are_added_with_arm64_executable(self):
        """Packages both declared HNPs rather than leaving module-only references."""
        with TemporaryDirectory() as temporary:
            project = Path(temporary)
            directory = project / 'entry/hnp/arm64-v8a'
            directory.mkdir(parents=True)
            executable = bytearray(64)
            executable[:4] = b'\x7fELF'
            executable[18:20] = (183).to_bytes(2, 'little')
            names = ['harmonix-public.hnp', 'harmonix-private.hnp']
            for name in names:
                with zipfile.ZipFile(directory / name, 'w') as hnp:
                    hnp.writestr('sysroot/bin/qemu-harmonix-aarch64', executable)
            hap = project / 'unsigned.hap'
            with zipfile.ZipFile(hap, 'w') as archive:
                archive.writestr('module.json', json.dumps({
                    'module': {'hnpPackages': [{'package': name} for name in names]}
                }))
                archive.writestr('libs/arm64-v8a/runtime.so', b'other native payload')
            with patch.object(build, 'OHOS_PROJECT_DIR', project), patch.object(build, 'OHOS_UNSIGNED_HAP_PATH', hap):
                build.package_ohos_terminal_hnp()
            with zipfile.ZipFile(hap) as archive:
                self.assertIsNone(archive.testzip())
                self.assertEqual(archive.read('libs/arm64-v8a/runtime.so'), b'other native payload')
                for name in names:
                    self.assertEqual(archive.read(f'hnp/arm64-v8a/{name}'), (directory / name).read_bytes())

    def test_missing_runtime_stops_packaging_before_signing(self):
        """Prevents producing another apparently successful HAP with no terminal executable."""
        with TemporaryDirectory() as temporary:
            with patch.object(build, 'OHOS_PROJECT_DIR', Path(temporary)):
                with self.assertRaisesRegex(RuntimeError, 'Required OHOS terminal HNP is missing'):
                    build.package_ohos_terminal_hnp()


if __name__ == '__main__':
    unittest.main()
