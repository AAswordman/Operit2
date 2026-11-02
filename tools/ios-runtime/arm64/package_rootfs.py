#!/usr/bin/env python3
"""Stage the prepared Linux ARM64 archive as an iOS bundle resource."""
import argparse
import os
from pathlib import Path
import shutil
import tarfile
import tempfile

from prepare_rootfs import archive_sha256, relative_path

MAX_ARCHIVE = 512 * 1024 * 1024
MAX_EXPANDED = 2 * 1024**3
MAX_FILE = 512 * 1024 * 1024
MAX_ENTRIES = 100000


def verify_rootfs(archive):
    required = {'bin/busybox', 'sbin/apk', 'etc/apk/repositories', 'lib/apk/db/installed'}
    found = set()
    seen = set()
    expanded = 0
    with tarfile.open(archive, 'r:gz') as root:
        for entry in root:
            name = relative_path(entry.name).as_posix()
            if name in seen:
                raise ValueError('duplicate rootfs archive path: ' + name)
            seen.add(name)
            if len(seen) > MAX_ENTRIES:
                raise ValueError('rootfs archive exceeds entry limit')
            if name == '.operit-arm64-root.json' or name.startswith('.operit-arm64-root.json/'):
                raise ValueError('archive must not supply the installation marker')
            if not (entry.isfile() or entry.isdir() or entry.issym() or entry.islnk()):
                raise ValueError('unsupported rootfs node: ' + name)
            if entry.isfile():
                expanded += entry.size
                if entry.size > MAX_FILE or expanded > MAX_EXPANDED:
                    raise ValueError('rootfs archive exceeds extraction size limits')
            if entry.islnk():
                relative_path(entry.linkname)
            if name not in required:
                continue
            if not entry.isfile():
                raise ValueError('rootfs requires a regular ' + name)
            found.add(name)
            with root.extractfile(entry) as stream:
                if name in ('bin/busybox', 'sbin/apk'):
                    header = stream.read(64)
                    if (len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01'
                            or int.from_bytes(header[18:20], 'little') != 183 or not entry.mode & 0o111):
                        raise ValueError(name + ' is not an executable AArch64 ELF64 binary')
                elif entry.size == 0:
                    raise ValueError('rootfs has empty ' + name)
    if required != found:
        raise ValueError('rootfs is missing: ' + ', '.join(sorted(required - found)))


def package(archive, output):
    if archive.is_symlink() or not archive.is_file():
        raise ValueError('input root archive must be a regular file')
    if not 0 < archive.stat().st_size <= MAX_ARCHIVE:
        raise ValueError('rootfs archive must be at most 512 MiB')
    expected = archive_sha256(archive)
    checksum = archive.with_name(archive.name + '.sha256')
    if checksum.exists():
        if checksum.is_symlink() or not checksum.is_file() or checksum.stat().st_size > 4096:
            raise ValueError('invalid rootfs checksum file')
        values = checksum.read_text().split()
        if len(values) != 2 or values[0].lower() != expected or values[1] != archive.name:
            raise ValueError('prepared rootfs SHA-256 mismatch')
    output = output.absolute()
    if output.is_symlink():
        raise ValueError('bundle output must not be a symlink')
    if output != archive.absolute() and output.is_file() and archive_sha256(output) == expected:
        return
    verify_rootfs(archive)
    if output == archive.absolute():
        return
    output.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix='.arm64-rootfs-', suffix='.tar.gz', dir=output.parent)
    try:
        with os.fdopen(descriptor, 'wb') as destination, archive.open('rb') as source:
            shutil.copyfileobj(source, destination)
            destination.flush()
            os.fsync(destination.fileno())
        if archive_sha256(Path(temporary)) != expected:
            raise ValueError('rootfs changed while packaging')
        os.chmod(temporary, 0o644)
        os.replace(temporary, output)
    finally:
        Path(temporary).unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, required=True, help='Linux-prepared ARM64 archive')
    parser.add_argument('--output', type=Path, required=True)
    options = parser.parse_args()
    package(options.archive, options.output)
    print(options.output.absolute())


if __name__ == '__main__':
    main()
