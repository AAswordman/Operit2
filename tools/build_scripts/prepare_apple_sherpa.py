#!/usr/bin/env python3
from __future__ import annotations

import hashlib
import shutil
import tarfile
import urllib.request
from pathlib import Path

from common import FLUTTER_APP_DIR


SHERPA_VERSION = "1.13.2"
SHERPA_ARCHIVE_NAME = f"sherpa-onnx-v{SHERPA_VERSION}-ios.tar.bz2"
SHERPA_ARCHIVE_URL = (
    f"https://github.com/k2-fsa/sherpa-onnx/releases/download/v{SHERPA_VERSION}/"
    f"{SHERPA_ARCHIVE_NAME}"
)
SHERPA_ARCHIVE_SHA256 = "2886a04df4f8d5066c6c8b6e712278d65d7b60fc9e45990223df50262861d38b"
SHERPA_ARCHIVE_BYTES = 77_611_169
APPLE_DIR = FLUTTER_APP_DIR / "apple"
DOWNLOAD_DIR = APPLE_DIR / "downloads"
FRAMEWORKS_DIR = APPLE_DIR / "Frameworks"
ARCHIVE_PATH = DOWNLOAD_DIR / SHERPA_ARCHIVE_NAME
STAGING_DIR = APPLE_DIR / ".sherpa-staging"


# Returns the SHA-256 digest for one local file.
def file_sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for chunk in iter(lambda: stream.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


# Verifies the exact official Sherpa archive size and digest.
def verify_archive(path: Path) -> None:
    if not path.is_file():
        raise RuntimeError(f"Sherpa archive is missing: {path}")
    if path.stat().st_size != SHERPA_ARCHIVE_BYTES:
        raise RuntimeError(f"Sherpa archive size is invalid: {path}")
    digest = file_sha256(path)
    if digest != SHERPA_ARCHIVE_SHA256:
        raise RuntimeError(f"Sherpa archive checksum is invalid: {path}")


# Downloads the official Sherpa archive into the local Apple build cache.
def download_archive() -> None:
    DOWNLOAD_DIR.mkdir(parents=True, exist_ok=True)
    temporary_path = ARCHIVE_PATH.with_suffix(ARCHIVE_PATH.suffix + ".part")
    temporary_path.unlink(missing_ok=True)
    with urllib.request.urlopen(SHERPA_ARCHIVE_URL, timeout=60) as response:
        with temporary_path.open("wb") as output:
            shutil.copyfileobj(response, output, length=1024 * 1024)
    verify_archive(temporary_path)
    temporary_path.replace(ARCHIVE_PATH)


# Rejects unsafe archive members before extraction into the staging directory.
def validate_archive_members(members: list[tarfile.TarInfo]) -> None:
    staging_root = STAGING_DIR.resolve()
    for member in members:
        target = (STAGING_DIR / member.name).resolve()
        if target != staging_root and staging_root not in target.parents:
            raise RuntimeError(f"Sherpa archive entry escapes staging: {member.name}")
        if member.issym():
            link_target = (target.parent / member.linkname).resolve()
            if link_target != staging_root and staging_root not in link_target.parents:
                raise RuntimeError(f"Sherpa archive link escapes staging: {member.name}")
            continue
        if member.islnk():
            link_target = (STAGING_DIR / member.linkname).resolve()
            if link_target != staging_root and staging_root not in link_target.parents:
                raise RuntimeError(f"Sherpa archive link escapes staging: {member.name}")
            continue
        if not member.isfile() and not member.isdir():
            raise RuntimeError(f"Sherpa archive entry has an unsupported type: {member.name}")


# Extracts and installs the two XCFrameworks required by the Apple runner.
def install_frameworks() -> None:
    shutil.rmtree(STAGING_DIR, ignore_errors=True)
    STAGING_DIR.mkdir(parents=True, exist_ok=True)
    with tarfile.open(ARCHIVE_PATH, "r:bz2") as archive:
        members = archive.getmembers()
        validate_archive_members(members)
        archive.extractall(STAGING_DIR, members=members)

    sherpa_source = STAGING_DIR / "build-ios" / "sherpa-onnx.xcframework"
    onnx_source = (
        STAGING_DIR
        / "build-ios"
        / "ios-onnxruntime"
        / "1.17.1"
        / "onnxruntime.xcframework"
    )
    FRAMEWORKS_DIR.mkdir(parents=True, exist_ok=True)
    for source in [sherpa_source, onnx_source]:
        if not source.is_dir():
            raise RuntimeError(f"Apple inference framework is missing from archive: {source}")
        target = FRAMEWORKS_DIR / source.name
        shutil.rmtree(target, ignore_errors=True)
        shutil.copytree(source, target)
    shutil.rmtree(STAGING_DIR)


# Verifies the framework slices used by iOS devices and simulators.
def verify_frameworks() -> None:
    required_paths = [
        FRAMEWORKS_DIR / "sherpa-onnx.xcframework" / "Info.plist",
        FRAMEWORKS_DIR
        / "sherpa-onnx.xcframework"
        / "ios-arm64"
        / "libsherpa-onnx.a",
        FRAMEWORKS_DIR
        / "sherpa-onnx.xcframework"
        / "ios-arm64_x86_64-simulator"
        / "libsherpa-onnx.a",
        FRAMEWORKS_DIR / "onnxruntime.xcframework" / "Info.plist",
        FRAMEWORKS_DIR
        / "onnxruntime.xcframework"
        / "ios-arm64"
        / "libonnxruntime.a",
        FRAMEWORKS_DIR
        / "onnxruntime.xcframework"
        / "ios-arm64_x86_64-simulator"
        / "libonnxruntime.a",
    ]
    missing = [str(path) for path in required_paths if not path.is_file()]
    if missing:
        raise RuntimeError("Apple inference framework files are missing: " + ", ".join(missing))


# Prepares verified Apple local inference build dependencies.
def prepare_apple_sherpa() -> None:
    archive_ready = (
        ARCHIVE_PATH.is_file()
        and ARCHIVE_PATH.stat().st_size == SHERPA_ARCHIVE_BYTES
        and file_sha256(ARCHIVE_PATH) == SHERPA_ARCHIVE_SHA256
    )
    if not archive_ready:
        download_archive()
    verify_archive(ARCHIVE_PATH)
    install_frameworks()
    verify_frameworks()


# Runs Apple local inference dependency preparation as a standalone command.
def main() -> int:
    prepare_apple_sherpa()
    print(f"Apple Sherpa frameworks: {FRAMEWORKS_DIR}", flush=True)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
