#!/usr/bin/env python3
from __future__ import annotations

import argparse
import hashlib
import json
import os
import zipfile
from pathlib import Path

from common import DIST_DIR, REPO_ROOT, require_command, run


EDITOR_DIR = REPO_ROOT / "tools" / "esp32-editor"
FIRMWARE_DIST_DIR = REPO_ROOT / "apps" / "esp32" / "dist"
FIRMWARE_MANIFEST = EDITOR_DIR / "generated" / "manifest.json"
PARTITIONS_FILE = REPO_ROOT / "apps" / "esp32" / "partitions.csv"
DEFAULT_ARCHIVE_PATH = DIST_DIR / "operit2-esp32-esp32-2432s028.zip"
IMAGE_SPECS = (
    ("operit-esp32-4mb-full.bin", 0x000000, "initial-flash"),
    ("bootloader.bin", 0x001000, "bootloader"),
    ("partition-table.bin", 0x008000, "partition-table"),
    ("operit-esp32.bin", 0x010000, "application"),
)


# Parses the standalone ESP32 build command options.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build and package the Operit2 ESP32 firmware.")
    parser.add_argument("--archive-path", type=Path, default=DEFAULT_ARCHIVE_PATH)
    return parser.parse_args()


# Computes the SHA-256 digest of one firmware package file.
def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


# Runs the shared editor firmware build with the installed ESP32 toolchain.
def build_firmware() -> None:
    npm = require_command("npm")
    emsdk = os.environ.get("EMSDK", "").strip()
    if not emsdk:
        raise RuntimeError("EMSDK must point to the installed Emscripten SDK for the ESP32 build")
    emsdk_path = Path(emsdk).expanduser().resolve()
    if not emsdk_path.is_dir():
        raise RuntimeError(f"EMSDK directory not found: {emsdk_path}")
    build_env = os.environ.copy()
    build_env["RUSTFLAGS"] = "-Awarnings"
    run([npm, "run", "build:firmware"], cwd=EDITOR_DIR, env=build_env)


# Reads and validates the manifest emitted by the shared ESP32 editor build.
def read_firmware_manifest() -> dict[str, object]:
    if not FIRMWARE_MANIFEST.is_file():
        raise RuntimeError(f"ESP32 build manifest not found: {FIRMWARE_MANIFEST}")
    manifest = json.loads(FIRMWARE_MANIFEST.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise RuntimeError("ESP32 build manifest must be a JSON object")
    if manifest.get("firmwareBuilt") is not True:
        raise RuntimeError("ESP32 build manifest does not contain a completed firmware build")
    return manifest


# Validates the generated images and returns their package metadata.
def image_metadata() -> list[dict[str, object]]:
    metadata: list[dict[str, object]] = []
    for name, offset, role in IMAGE_SPECS:
        path = FIRMWARE_DIST_DIR / name
        if not path.is_file() or path.stat().st_size == 0:
            raise RuntimeError(f"ESP32 firmware image not found: {path}")
        metadata.append(
            {
                "file": name,
                "offset": offset,
                "offsetHex": f"0x{offset:06x}",
                "role": role,
                "bytes": path.stat().st_size,
                "sha256": sha256_file(path),
            }
        )
    return metadata


# Builds the self-describing ESP32 firmware manifest.
def package_manifest(editor_manifest: dict[str, object], images: list[dict[str, object]]) -> dict[str, object]:
    package: dict[str, object] = {
        "schemaVersion": 1,
        "format": "operit2-esp32-firmware",
        "board": "ESP32-2432S028",
        "chip": "esp32",
        "flashSizeBytes": 4 * 1024 * 1024,
        "partitionTable": "partitions.csv",
        "images": images,
    }
    for key in ("runtimeHash", "sourceHash", "builtAt", "renderer"):
        value = editor_manifest.get(key)
        if isinstance(value, str) and value:
            package[key] = value
    return package


# Writes the firmware archive with images, offsets, hashes, and flashing instructions.
def write_archive(archive_path: Path, manifest: dict[str, object]) -> Path:
    if not PARTITIONS_FILE.is_file():
        raise RuntimeError(f"ESP32 partition table source not found: {PARTITIONS_FILE}")
    archive_path = archive_path.resolve()
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if archive_path.exists():
        archive_path.unlink()
    readme = (
        "Operit2 ESP32-2432S028 firmware\n"
        "\n"
        "Initial installation: flash operit-esp32-4mb-full.bin at 0x000000.\n"
        "Selective flashing: bootloader.bin at 0x001000, partition-table.bin at 0x008000,\n"
        "and operit-esp32.bin at 0x010000. Use manifest.json for verified offsets and hashes.\n"
        "\n"
        "The fixed UI requires a firmware rebuild for screen changes; dynamic layout deployment is not supported.\n"
    )
    manifest_text = json.dumps(manifest, ensure_ascii=False, indent=2) + "\n"
    with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
        archive.writestr("manifest.json", manifest_text.encode("utf-8"))
        archive.writestr("README.txt", readme.encode("utf-8"))
        archive.write(PARTITIONS_FILE, "partitions.csv")
        for name, _, _ in IMAGE_SPECS:
            archive.write(FIRMWARE_DIST_DIR / name, name)
    if not archive_path.is_file() or archive_path.stat().st_size == 0:
        raise RuntimeError(f"ESP32 firmware archive was not produced: {archive_path}")
    return archive_path


# Builds the ESP32 firmware and packages the complete board-specific release artifact.
def build_esp32(archive_path: Path = DEFAULT_ARCHIVE_PATH) -> Path:
    build_firmware()
    editor_manifest = read_firmware_manifest()
    images = image_metadata()
    manifest = package_manifest(editor_manifest, images)
    result = write_archive(archive_path, manifest)
    print(f"ESP32 archive: {result}", flush=True)
    return result


# Runs the standalone ESP32 package build entrypoint.
def main() -> int:
    args = parse_args()
    build_esp32(args.archive_path)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", flush=True)
        raise SystemExit(1)
