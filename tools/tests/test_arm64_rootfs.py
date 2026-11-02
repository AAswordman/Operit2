"""Regression checks for the independent ARM64 rootfs import boundary."""
import importlib.util
import io
from pathlib import Path
import struct
import tarfile
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[1] / 'ios-runtime/arm64/prepare_rootfs.py'
SPEC = importlib.util.spec_from_file_location('arm64_rootfs', SCRIPT)
ROOTFS = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(ROOTFS)


class RootfsTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='operit root import ')
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.archive = self.base / 'root.tar.gz'
        self.output = self.base / 'root'
        self.elf = bytearray(64)
        self.elf[:7] = b'\x7fELF\x02\x01\x01'
        struct.pack_into('<H', self.elf, 18, 183)

    def write_archive(self, entries=(), *, architecture=True):
        with tarfile.open(self.archive, 'w:gz') as tar:
            all_entries = [('bin/busybox', bytes(self.elf), None)] if architecture else []
            for name, data, kind in [*all_entries, *entries]:
                info = tarfile.TarInfo(name)
                info.mode = 0o755
                if kind in ('symlink', 'hardlink'):
                    info.type = tarfile.SYMTYPE if kind == 'symlink' else tarfile.LNKTYPE
                    info.linkname = data
                    tar.addfile(info)
                elif kind == 'device':
                    info.type = tarfile.CHRTYPE
                    tar.addfile(info)
                else:
                    info.size = len(data)
                    tar.addfile(info, io.BytesIO(data))
        return ROOTFS.archive_sha256(self.archive)

    def assert_rejected(self, entries, *, architecture=True):
        digest = self.write_archive(entries, architecture=architecture)
        with self.assertRaises((ValueError, OSError)):
            ROOTFS.prepare(self.archive, self.output, digest)
        self.assertFalse(self.output.exists())
        self.assertFalse(self.output.with_name('root.importing').exists())

    def test_verified_import_is_repeatable_and_keeps_guest_absolute_symlinks(self):
        digest = self.write_archive([('usr/message', b'hello', None), ('bin/sh', '/bin/busybox', 'symlink'),
                                     ('bin/copy', 'bin/busybox', 'hardlink')])
        ROOTFS.prepare(self.archive, self.output, digest)
        self.assertEqual((self.output / 'usr/message').read_bytes(), b'hello')
        self.assertEqual((self.output / 'bin/sh').readlink(), Path('/bin/busybox'))
        self.assertEqual((self.output / 'bin/copy').stat().st_ino, (self.output / 'bin/busybox').stat().st_ino)
        (self.output / 'user-data').write_text('keep')
        ROOTFS.prepare(self.archive, self.output, digest)
        self.assertEqual((self.output / 'user-data').read_text(), 'keep')

    def test_digest_mismatch_creates_no_output(self):
        self.write_archive()
        with self.assertRaisesRegex(ValueError, 'SHA-256'):
            ROOTFS.prepare(self.archive, self.output, '0' * 64)
        self.assertFalse(self.output.exists())

    def test_traversal_and_absolute_member_paths_are_rejected(self):
        for name in ('../escaped', '/absolute', 'a/../../escaped'):
            with self.subTest(name=name):
                self.assert_rejected([(name, b'bad', None)])
        self.assertFalse((self.base / 'escaped').exists())

    def test_symlink_parents_cannot_redirect_regular_files(self):
        self.assert_rejected([('link', '../outside', 'symlink'), ('link/file', b'bad', None)])
        self.assertFalse((self.base / 'outside').exists())

    def test_hardlinks_cannot_escape_or_target_symlinks(self):
        for entries in ([('bad', '../outside', 'hardlink')],
                        [('link', '/bin/busybox', 'symlink'), ('bad', 'link', 'hardlink')]):
            self.assert_rejected(entries)

    def test_duplicate_paths_and_special_nodes_are_rejected(self):
        self.assert_rejected([('bin/busybox', b'bad', None)])
        self.assert_rejected([('dev/mem', b'', 'device')])

    def test_wrong_architecture_is_rejected(self):
        struct.pack_into('<H', self.elf, 18, 3)
        self.assert_rejected([])

    def test_existing_root_and_staging_are_never_overwritten(self):
        digest = self.write_archive()
        self.output.mkdir()
        (self.output / 'keep').write_text('original')
        with self.assertRaisesRegex(ValueError, 'destination exists'):
            ROOTFS.prepare(self.archive, self.output, digest)
        self.assertEqual((self.output / 'keep').read_text(), 'original')
        self.output.joinpath('keep').unlink()
        self.output.rmdir()
        staging = self.output.with_name('root.importing')
        staging.mkdir()
        (staging / 'keep').write_text('interrupted')
        with self.assertRaisesRegex(ValueError, 'staging directory'):
            ROOTFS.prepare(self.archive, self.output, digest)
        self.assertEqual((staging / 'keep').read_text(), 'interrupted')

    def test_archive_cannot_supply_or_redirect_provenance_marker(self):
        outside = self.base / 'outside'
        outside.write_text('must remain intact')
        for kind, data in [(None, b'forged'), ('symlink', str(outside)), ('hardlink', 'bin/busybox')]:
            self.assert_rejected([('.operit-arm64-root.json', data, kind)])
            self.assertEqual(outside.read_text(), 'must remain intact')

    def test_architecture_probe_cannot_traverse_symlink_parent(self):
        outside = self.base / 'outside'
        outside.mkdir()
        (outside / 'busybox').write_bytes(bytes(self.elf))
        self.assert_rejected([('bin', str(outside), 'symlink')], architecture=False)

    def test_destination_symlink_is_rejected(self):
        digest = self.write_archive()
        self.output.symlink_to(self.base / 'missing')
        with self.assertRaisesRegex(ValueError, 'symlink'):
            ROOTFS.prepare(self.archive, self.output, digest)
        self.assertTrue(self.output.is_symlink())


if __name__ == '__main__':
    unittest.main()
