import io
import json
import struct
import sys
import unittest
import zipfile
from pathlib import Path
from tempfile import TemporaryDirectory
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / 'build_scripts'))
import build_flutter_ohos as build


# Builds a target-specific ELF64 fixture with a dynamic dependency table.
def elf64(needed: tuple[str, ...], machine: int = 183) -> bytes:
    strings = bytearray(b'\0')
    dynamic = bytearray()
    for name in needed:
        dynamic.extend(struct.pack('<qQ', 1, len(strings)))
        strings.extend(name.encode() + b'\0')
    dynamic.extend(struct.pack('<qQ', 0, 0))
    dynamic_offset = 64 + len(strings)
    section_offset = dynamic_offset + len(dynamic)
    identity = b'\x7fELF\x02\x01\x01' + bytes(9)
    header = struct.pack(
        '<16sHHIQQQIHHHHHH', identity, 3, machine, 1, 0, 0,
        section_offset, 0, 64, 56, 0, 64, 3, 0,
    )
    sections = bytes(64)
    sections += struct.pack('<IIQQQQIIQQ', 0, 3, 0, 0, 64, len(strings), 0, 0, 1, 0)
    sections += struct.pack('<IIQQQQIIQQ', 0, 6, 0, 0, dynamic_offset, len(dynamic), 1, 0, 8, 16)
    return header + strings + dynamic + sections


class OhosTerminalPackagingTests(unittest.TestCase):
    """Checks terminal packaging preserves dependencies and excludes unused programs."""

    # Writes the upstream package shape, including unrelated emulators and libraries.
    def write_hnp(self, path: Path, dependencies: dict[str, bytes], machine: int = 183) -> None:
        with zipfile.ZipFile(path, 'w') as package:
            for directory in ('sysroot/', 'sysroot/bin/', 'sysroot/lib/'):
                package.writestr(directory, b'')
            package.writestr('sysroot/hnp.json', json.dumps({
                'type': 'hnp-config', 'name': 'harmonix', 'version': '1.0', 'install': {},
            }))
            executable = zipfile.ZipInfo('sysroot/bin/qemu-harmonix-aarch64')
            executable.external_attr = 0o100755 << 16
            package.writestr(executable, elf64(('libz.so', 'libc.so'), machine=machine))
            package.writestr('sysroot/bin/qemu-system-aarch64', b'unused full-system emulator')
            package.writestr('sysroot/bin/qemu-system-x86_64', b'unused x86 emulator')
            package.writestr('sysroot/lib/libcrypto.so', b'unused crypto library')
            for name, payload in dependencies.items():
                package.writestr(f'sysroot/lib/{name}', payload)

    # Verifies real dependency closure and ZIP metadata survive package compaction.
    def test_declared_packages_keep_transitive_dependencies_and_exclude_unused_files(self):
        with TemporaryDirectory() as temporary:
            project = Path(temporary)
            directory = project / 'entry/hnp/arm64-v8a'
            directory.mkdir(parents=True)
            dependencies = {
                'libz.so': elf64(('libcompression.so', 'libc.so')),
                'libcompression.so': elf64(('libc.so',)),
            }
            names = ['harmonix-public.hnp', 'harmonix-private.hnp']
            for name in names:
                self.write_hnp(directory / name, dependencies)
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
                    with zipfile.ZipFile(io.BytesIO(archive.read(f'hnp/arm64-v8a/{name}'))) as package:
                        self.assertIsNone(package.testzip())
                        self.assertEqual(set(package.namelist()), {
                            'sysroot/', 'sysroot/bin/', 'sysroot/lib/',
                            'sysroot/hnp.json', 'sysroot/bin/qemu-harmonix-aarch64',
                            'sysroot/lib/libz.so', 'sysroot/lib/libcompression.so',
                        })
                        for library, payload in dependencies.items():
                            self.assertEqual(package.read(f'sysroot/lib/{library}'), payload)
                        executable = package.getinfo('sysroot/bin/qemu-harmonix-aarch64')
                        self.assertEqual(executable.external_attr >> 16, 0o100755)

    # Prevents incomplete archives from modifying the unsigned HAP.
    def test_missing_shared_library_stops_before_hap_is_modified(self):
        with TemporaryDirectory() as temporary:
            project = Path(temporary)
            directory = project / 'entry/hnp/arm64-v8a'
            directory.mkdir(parents=True)
            self.write_hnp(directory / 'harmonix-public.hnp', {
                'libz.so': elf64(('libmissing.so', 'libc.so')),
            })
            hap = project / 'unsigned.hap'
            hap.write_bytes(b'unchanged package')
            with patch.object(build, 'OHOS_PROJECT_DIR', project), patch.object(build, 'OHOS_UNSIGNED_HAP_PATH', hap):
                with self.assertRaisesRegex(RuntimeError, 'libmissing.so'):
                    build.package_ohos_terminal_hnp()
            self.assertEqual(hap.read_bytes(), b'unchanged package')

    # Rejects incompatible native libraries instead of shipping a broken terminal.
    def test_non_arm64_dependency_is_rejected(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'terminal.hnp'
            self.write_hnp(path, {'libz.so': elf64(('libc.so',), machine=62)})
            with self.assertRaisesRegex(RuntimeError, 'not a target-compatible arm64-v8a ELF'):
                build.ohos_terminal_hnp_payload(path)

    # Preserves the ARM64 guest emulator while validating its x86_64 host ABI.
    def test_x64_host_packages_arm64_guest_runtime_with_complete_dependencies(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'terminal.hnp'
            dependencies = {'libz.so': elf64(('libc.so',), machine=62)}
            self.write_hnp(path, dependencies, machine=62)
            payload = build.ohos_terminal_hnp_payload(path, build.OhosTargetPlatform.X64)
            with zipfile.ZipFile(io.BytesIO(payload)) as package:
                executable = package.read('sysroot/bin/qemu-harmonix-aarch64')
                self.assertEqual(struct.unpack_from('<H', executable, 18)[0], 62)
                self.assertEqual(package.read('sysroot/lib/libz.so'), dependencies['libz.so'])
                self.assertNotIn('sysroot/bin/qemu-system-x86_64', package.namelist())

    # Rejects an ARM64 shared library inside an x86_64 host package.
    def test_x64_package_rejects_arm64_dependencies(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'terminal.hnp'
            self.write_hnp(path, {'libz.so': elf64(('libc.so',))}, machine=62)
            with self.assertRaisesRegex(RuntimeError, 'not a target-compatible x86_64 ELF'):
                build.ohos_terminal_hnp_payload(path, build.OhosTargetPlatform.X64)

    # Prevents producing another apparently successful HAP with no terminal executable.
    def test_missing_runtime_stops_packaging_before_signing(self):
        with TemporaryDirectory() as temporary:
            with patch.object(build, 'OHOS_PROJECT_DIR', Path(temporary)):
                with self.assertRaisesRegex(RuntimeError, 'Required OHOS terminal HNP is missing'):
                    build.package_ohos_terminal_hnp()


class OhosHapArchitectureTests(unittest.TestCase):
    """Rejects mixed architectures and missing engines before signing."""

    # Writes all libraries required for a complete emulator HAP.
    def write_hap(self, path: Path) -> None:
        with zipfile.ZipFile(path, 'w') as hap:
            for name in ('libapp.so', 'libflutter.so', 'libc++_shared.so',
                         'liboperit_runtime_ohos.so', build.OHOS_BRIDGE_LIBRARY_NAME,
                         build.OHOS_ENGINE_LIBRARY_NAME):
                hap.writestr(f'libs/x86_64/{name}', elf64(('libc.so',), machine=62))

    # Accepts a complete HAP whose native libraries all target the emulator.
    def test_complete_x64_hap_is_accepted(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'app.hap'
            self.write_hap(path)
            with patch.object(build, 'OHOS_UNSIGNED_HAP_PATH', path):
                build.verify_ohos_hap_native_libraries(build.OhosTargetPlatform.X64)

    # Rejects stale libraries from a previous architecture build.
    def test_mixed_hap_is_rejected(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'app.hap'
            self.write_hap(path)
            with zipfile.ZipFile(path, 'a') as hap:
                hap.writestr('libs/arm64-v8a/old.so', elf64(('libc.so',)))
            with patch.object(build, 'OHOS_UNSIGNED_HAP_PATH', path):
                with self.assertRaisesRegex(RuntimeError, 'Unexpected native ABI'):
                    build.verify_ohos_hap_native_libraries(build.OhosTargetPlatform.X64)

    # Rejects a manifest that declares an engine without packaging its shared library.
    def test_missing_engine_is_rejected(self):
        with TemporaryDirectory() as temporary:
            path = Path(temporary) / 'app.hap'
            with zipfile.ZipFile(path, 'w') as hap:
                hap.writestr('libs/x86_64/libapp.so', elf64(('libc.so',), machine=62))
            with patch.object(build, 'OHOS_UNSIGNED_HAP_PATH', path):
                with self.assertRaisesRegex(RuntimeError, 'libflutter.so'):
                    build.verify_ohos_hap_native_libraries(build.OhosTargetPlatform.X64)


if __name__ == '__main__':
    unittest.main()
