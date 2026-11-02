#!/usr/bin/env python3
"""Prepare a build-owned AArch64 root without touching installed iSH user data."""
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import shutil
import tarfile
import urllib.request

HERE = Path(__file__).resolve().parent


def archive_sha256(path):
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def relative_path(name):
    path = PurePosixPath(name)
    if path.is_absolute() or '..' in path.parts:
        raise ValueError('archive path escapes root: ' + name)
    return Path(*path.parts)


def safe_parent(root, path):
    current = root
    for part in path.parts[:-1]:
        current = current / part
        if current.is_symlink():
            raise ValueError('archive parent is a symlink: ' + str(path))
        current.mkdir(exist_ok=True)


def extract_root(archive, destination):
    """Write regular files before links; never follow an archive-created link."""
    with tarfile.open(archive, 'r:gz') as tar:
        entries = tar.getmembers()
        seen = set()
        links = []
        expanded = 0
        for entry in entries:
            relative = relative_path(entry.name)
            if relative == Path('.'):
                continue
            if relative == Path('.operit-arm64-root.json'):
                raise ValueError('archive must not supply the runtime provenance marker')
            if relative in seen:
                raise ValueError('duplicate archive path: ' + entry.name)
            seen.add(relative)
            target = destination / relative
            safe_parent(destination, relative)
            if entry.isdir():
                target.mkdir(exist_ok=True)
            elif entry.isfile():
                expanded += entry.size
                if expanded > 2 * 1024**3:
                    raise ValueError('root archive exceeds 2 GiB extraction limit')
                with tar.extractfile(entry) as source, target.open('xb') as output:
                    shutil.copyfileobj(source, output)
                target.chmod(entry.mode & 0o777)
            elif entry.issym() or entry.islnk():
                links.append((entry, relative))
            else:
                raise ValueError('unsupported archive node: ' + entry.name)
        # Hard links are resolved only to regular, non-symlink archive files.
        for entry, relative in links:
            if not entry.islnk():
                continue
            source_relative = relative_path(entry.linkname)
            safe_parent(destination, source_relative)
            source = destination / source_relative
            if not source.is_file() or source.is_symlink():
                raise ValueError('hardlink target is not a regular file: ' + entry.linkname)
            safe_parent(destination, relative)
            os.link(source, destination / relative)
        for entry, relative in sorted(links, key=lambda pair: len(pair[1].parts), reverse=True):
            if entry.issym():
                safe_parent(destination, relative)
                (destination / relative).symlink_to(entry.linkname)


def verify_architecture(root):
    path = root / 'bin/busybox'
    if (root / 'bin').is_symlink() or path.is_symlink():
        raise ValueError('root architecture probe must be a regular bin/busybox')
    with path.open('rb') as stream:
        header = stream.read(64)
    if len(header) != 64 or header[:7] != b'\x7fELF\x02\x01\x01' or int.from_bytes(header[18:20], 'little') != 183:
        raise ValueError('rootfs must contain AArch64 Linux ELF64 binaries')


def prepare(archive, destination, expected_sha256):
    digest = archive_sha256(archive)
    if expected_sha256 and digest != expected_sha256:
        raise ValueError('root archive SHA-256 mismatch')
    marker = destination / '.operit-arm64-root.json'
    if destination.is_symlink():
        raise ValueError('destination must not be a symlink')
    if destination.exists():
        if not marker.is_symlink() and marker.is_file() and json.loads(marker.read_text()).get('archive_sha256') == digest:
            verify_architecture(destination)
            return
        raise ValueError('destination exists with different or missing provenance; choose a new build directory')
    staging = destination.with_name(destination.name + '.importing')
    if staging.exists() or staging.is_symlink():
        raise ValueError('staging directory already exists; inspect it before retrying')
    staging.mkdir(parents=True)
    try:
        extract_root(archive, staging)
        verify_architecture(staging)
        with (staging / '.operit-arm64-root.json').open('x') as provenance:
            provenance.write(json.dumps({'architecture': 'aarch64', 'archive_sha256': digest}, indent=2) + '\n')
        staging.rename(destination)
    except BaseException:
        shutil.rmtree(staging)
        raise


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, help='existing AArch64 root archive; requires --sha256')
    parser.add_argument('--sha256')
    parser.add_argument('--output', type=Path, default=HERE / 'build/rootfs')
    options = parser.parse_args()
    metadata = json.loads((HERE / 'rootfs.json').read_text())
    if options.archive:
        if not options.sha256:
            parser.error('--archive requires --sha256')
        archive, digest = options.archive, options.sha256
    else:
        archive = HERE / 'downloads' / metadata['url'].rsplit('/', 1)[-1]
        digest = metadata['sha256']
        if not archive.is_file() or archive_sha256(archive) != digest:
            archive.parent.mkdir(parents=True, exist_ok=True)
            temporary = archive.with_suffix('.part')
            try:
                with urllib.request.urlopen(metadata['url'], timeout=60) as response, temporary.open('wb') as output:
                    shutil.copyfileobj(response, output)
                if archive_sha256(temporary) != digest:
                    raise ValueError('downloaded root archive SHA-256 mismatch')
                temporary.replace(archive)
            finally:
                temporary.unlink(missing_ok=True)
    prepare(archive, options.output, digest)
    print(options.output.resolve())


if __name__ == '__main__':
    main()
