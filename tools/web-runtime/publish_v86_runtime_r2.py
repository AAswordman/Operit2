#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path
from urllib.parse import quote


RUNTIME_DIR = Path(__file__).resolve().parents[2] / "apps" / "web_access" / "v86" / "runtime"
WEB_V86_DIR = Path(__file__).resolve().parents[2] / "apps" / "web_access" / "web" / "v86"
R2_ACCOUNT_ID = "c667bf70582c5ceceda8d3d183ad8e3b"
R2_BUCKET = "operit-model-assets"
R2_PREFIX = "v86-runtime/i686-buildroot-node20-python312-20260720"
R2_PUBLIC_BASE_URL = f"https://models.operit.app/{R2_PREFIX}/"
R2_API_BASE_URL = f"https://api.cloudflare.com/client/v4/accounts/{R2_ACCOUNT_ID}/r2/buckets/{R2_BUCKET}/objects"
PUBLICATION_FILES = (
    (RUNTIME_DIR, "operit-runtime-manifest.json", "application/json; charset=utf-8"),
    (RUNTIME_DIR, "operit-runtime-bzimage.bin", "application/octet-stream"),
    (RUNTIME_DIR, "operit-runtime-initrd.cpio.gz", "application/gzip"),
    (WEB_V86_DIR, "seabios.bin", "application/octet-stream"),
    (WEB_V86_DIR, "vgabios.bin", "application/octet-stream"),
)
V86_BIOS_HASHES = (
    ("seabios.bin", "73e3f359102e3a9982c35fce98eb7cd08f18303ac7f1ba6ebfbe6cdc1c244d98"),
    ("vgabios.bin", "a4bc0d80cc3ca028c73dafa8fee396b8d054ce87ebd8abfbd31b06b437607880"),
)


# Parses the requested R2 publication mode.
def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="Validate the local runtime manifest without uploading files.",
    )
    return parser.parse_args()


# Reads and validates the local V86 runtime manifest.
def read_runtime_manifest() -> dict[str, object]:
    manifest_path = RUNTIME_DIR / "operit-runtime-manifest.json"
    if not manifest_path.is_file():
        raise RuntimeError(f"V86 runtime manifest does not exist: {manifest_path}")
    manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
    if not isinstance(manifest, dict):
        raise RuntimeError("V86 runtime manifest must be an object")
    return manifest


# Validates one manifest-backed binary runtime file.
def verify_manifest_file(manifest: dict[str, object], section: str) -> None:
    entry = manifest.get(section)
    if not isinstance(entry, dict):
        raise RuntimeError(f"V86 runtime manifest section is invalid: {section}")
    file_name = entry.get("file")
    expected_hash = entry.get("sha256")
    expected_size = entry.get("bytes")
    if (
        not isinstance(file_name, str)
        or not isinstance(expected_hash, str)
        or not isinstance(expected_size, int)
    ):
        raise RuntimeError(f"V86 runtime manifest entry is invalid: {section}")
    path = RUNTIME_DIR / file_name
    if not path.is_file():
        raise RuntimeError(f"V86 runtime file does not exist: {path}")
    actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual_hash != expected_hash or path.stat().st_size != expected_size:
        raise RuntimeError(
            f"V86 runtime file does not match its manifest: {path.name}"
        )


# Validates one V86 BIOS resource against its pinned SHA-256 checksum.
def verify_bios_file(file_name: str, expected_hash: str) -> None:
    path = WEB_V86_DIR / file_name
    if not path.is_file():
        raise RuntimeError(f"V86 BIOS file does not exist: {path}")
    actual_hash = hashlib.sha256(path.read_bytes()).hexdigest()
    if actual_hash != expected_hash:
        raise RuntimeError(f"V86 BIOS file does not match its checksum: {file_name}")


# Verifies every file selected for the immutable R2 runtime release.
def verify_runtime_release() -> None:
    manifest = read_runtime_manifest()
    verify_manifest_file(manifest, "kernel")
    verify_manifest_file(manifest, "initrd")
    for file_name, expected_hash in V86_BIOS_HASHES:
        verify_bios_file(file_name, expected_hash)
    for directory, file_name, _ in PUBLICATION_FILES:
        if not (directory / file_name).is_file():
            raise RuntimeError(f"V86 runtime release is missing a required publication file: {file_name}")


# Uploads one verified runtime file through Cloudflare's R2 object API.
def publish_runtime_file(directory: Path, file_name: str, content_type: str) -> None:
    path = directory / file_name
    curl = shutil.which("curl.exe" if os.name == "nt" else "curl")
    if curl is None:
        raise RuntimeError("curl is required to publish the V86 runtime")
    token = os.environ["CLOUDFLARE_API_TOKEN"]
    object_key = quote(f"{R2_PREFIX}/{file_name}", safe="/")
    object_url = f"{R2_API_BASE_URL}/{object_key}"
    command = [
        curl,
        "--fail",
        "--show-error",
        "--silent",
        "--http1.1",
        "--retry",
        "5",
        "--retry-all-errors",
        "--connect-timeout",
        "30",
        "--max-time",
        "900",
        "--request",
        "PUT",
        "--header",
        f"Authorization: Bearer {token}",
        "--header",
        "cf-r2-data-catalog-check: true",
        "--header",
        f"Content-Type: {content_type}",
        "--upload-file",
        str(path),
        object_url,
    ]
    print(f"+ {curl} --request PUT {object_url} --upload-file {path}", flush=True)
    subprocess.run(command, check=True)


# Publishes the verified V86 runtime release to its immutable R2 location.
def main() -> int:
    arguments = parse_arguments()
    verify_runtime_release()
    if arguments.check:
        print(f"V86 runtime release verified: {R2_PUBLIC_BASE_URL}", flush=True)
        return 0
    if not os.environ.get("CLOUDFLARE_API_TOKEN"):
        raise RuntimeError("CLOUDFLARE_API_TOKEN is required to publish the V86 runtime")
    for directory, file_name, content_type in PUBLICATION_FILES:
        publish_runtime_file(directory, file_name, content_type)
    print(f"V86 runtime release published: {R2_PUBLIC_BASE_URL}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
