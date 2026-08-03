#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import shutil
import urllib.request
import zipfile
from dataclasses import dataclass
from pathlib import Path, PurePosixPath


RUNTIME_DIR = Path(__file__).resolve().parent
DOWNLOAD_DIR = RUNTIME_DIR / "downloads"
SOURCE_DIR = RUNTIME_DIR / "sources"


@dataclass(frozen=True)
class ArchiveSpec:
    name: str
    url: str
    archive_name: str
    sha256: str
    extracted_directory: str
    destination: str


ISH_ARCHIVES = (
    ArchiveSpec(
        name="iSH",
        url="https://github.com/ish-app/ish/archive/7864dd601e615d0fc09b888a93d327f458b25a1d.zip",
        archive_name="ish-7864dd601e615d0fc09b888a93d327f458b25a1d.zip",
        sha256="d6c76b0ea4e7a45e75bf9f81005db71b9ed4d5c5dc166feeeda9e9e916df6bce",
        extracted_directory="ish-7864dd601e615d0fc09b888a93d327f458b25a1d",
        destination="ish",
    ),
    ArchiveSpec(
        name="iSH libapps",
        url="https://github.com/ish-app/libapps/archive/b8cacae35e5b11d64bb736a053921c16ca7faf9e.zip",
        archive_name="libapps-b8cacae35e5b11d64bb736a053921c16ca7faf9e.zip",
        sha256="138b9dc168d7fbde8cabcf480263b18a63514a31ba9195c5cf3be0ded554e828",
        extracted_directory="libapps-b8cacae35e5b11d64bb736a053921c16ca7faf9e",
        destination="ish/deps/libapps",
    ),
    ArchiveSpec(
        name="libarchive",
        url="https://github.com/libarchive/libarchive/archive/fc6563f5130d8a7ee1fc27c0e55baef35119f26c.zip",
        archive_name="libarchive-fc6563f5130d8a7ee1fc27c0e55baef35119f26c.zip",
        sha256="2506adf04dbdf3c3c8940435059bb437af55f4239b4016fbee15babf50d2d17c",
        extracted_directory="libarchive-fc6563f5130d8a7ee1fc27c0e55baef35119f26c",
        destination="ish/deps/libarchive",
    ),
    ArchiveSpec(
        name="iSH Linux",
        url="https://github.com/ish-app/linux/archive/8ec9bf17f89c6dba818f3ed2427de4223e78644a.zip",
        archive_name="linux-8ec9bf17f89c6dba818f3ed2427de4223e78644a.zip",
        sha256="eebe45e28b3ecc6894a94cf47eb8ea1ebe27c4caa0441f0c8ed1c06bc2eee49c",
        extracted_directory="linux-8ec9bf17f89c6dba818f3ed2427de4223e78644a",
        destination="ish/deps/linux",
    ),
)

# Returns the SHA-256 digest for one local file.
def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


# Downloads one archive and verifies its exact SHA-256 digest.
def download_archive(url: str, archive_path: Path, expected_sha256: str) -> None:
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    temporary_path = archive_path.with_suffix(archive_path.suffix + ".part")
    temporary_path.unlink(missing_ok=True)
    with urllib.request.urlopen(url, timeout=120) as response:
        with temporary_path.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
    actual_sha256 = file_sha256(temporary_path)
    if actual_sha256 != expected_sha256:
        raise RuntimeError(
            f"SHA-256 mismatch for {archive_path.name}: {actual_sha256}"
        )
    temporary_path.replace(archive_path)


# Rejects ZIP paths that leave the requested destination directory.
def validate_zip_members(archive: zipfile.ZipFile, destination: Path) -> None:
    destination_root = destination.resolve()
    for member in archive.infolist():
        target = (destination / member.filename).resolve()
        if target != destination_root and destination_root not in target.parents:
            raise RuntimeError(f"ZIP entry escapes destination: {member.filename}")


# Extracts one verified source ZIP into its declared source directory.
def extract_source_archive(spec: ArchiveSpec) -> None:
    archive_path = DOWNLOAD_DIR / spec.archive_name
    extraction_root = SOURCE_DIR / ".extract" / spec.name.replace(" ", "-")
    destination = SOURCE_DIR / spec.destination
    shutil.rmtree(extraction_root, ignore_errors=True)
    extraction_root.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(archive_path) as archive:
        validate_zip_members(archive, extraction_root)
        archive.extractall(extraction_root)
    extracted = extraction_root / spec.extracted_directory
    if not extracted.is_dir():
        raise RuntimeError(f"Archive has no expected source directory: {extracted}")
    restore_archive_permissions(archive_path, extracted, spec.extracted_directory)
    shutil.rmtree(destination, ignore_errors=True)
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.move(str(extracted), str(destination))
    shutil.rmtree(extraction_root)


# Restores executable permissions recorded by GitHub's ZIP archive metadata.
def restore_archive_permissions(
    archive_path: Path, extracted: Path, extracted_directory: str
) -> None:
    with zipfile.ZipFile(archive_path) as archive:
        for member in archive.infolist():
            permissions = member.external_attr >> 16
            if permissions & 0o111:
                relative_path = PurePosixPath(member.filename).relative_to(
                    extracted_directory
                )
                extracted.joinpath(*relative_path.parts).chmod(permissions & 0o777)


# Downloads and extracts every iSH source dependency at its pinned revision.
def prepare_ish_sources() -> None:
    for spec in ISH_ARCHIVES:
        download_archive(spec.url, DOWNLOAD_DIR / spec.archive_name, spec.sha256)
        extract_source_archive(spec)
    shutil.rmtree(SOURCE_DIR / ".extract", ignore_errors=True)


# Prepares every iSH input consumed by the iOS Runner build.
def main() -> int:
    prepare_ish_sources()
    print(f"iSH source: {SOURCE_DIR / 'ish'}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
