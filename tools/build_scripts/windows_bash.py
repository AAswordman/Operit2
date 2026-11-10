"""Stage the pinned, complete Git for Windows portable Bash runtime."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import tempfile
import urllib.request
from pathlib import Path

from common import REPO_ROOT, run

VERSION = "2.56.0.4"
RELEASE = "v2.56.0.windows.4"
PACKAGES = {
    "x86_64": ("64-bit", "b1f3e52216e2ed6882d1bebfd4d3fdf319bec40d2edbbf9fd26ec4fd3fce7ed1"),
    "aarch64": ("arm64", "044110f9d951858452ddb5e4b5415ea0f2b5b2598e794d6fec77fe28b9aa8cc0"),
}
REQUIRED_FILES = (
    "bin/bash.exe", "usr/bin/bash.exe", "usr/bin/msys-2.0.dll",
    "usr/bin/base64.exe", "usr/bin/tr.exe", "usr/bin/uname.exe", "LICENSE.txt",
)
CACHE = REPO_ROOT / "tools/release/work/windows-bash"
EXTRACTOR_URL = "https://github.com/ip7z/7zip/releases/download/26.06/7zr.exe"
EXTRACTOR_SHA256 = "e80db2dad715a6c5adabe292d31ed54e11f4e9b99ca349827b50573b175891bc"


def download_verified(url: str, destination: Path, digest: str) -> Path:
    """Cache an upstream artifact and verify its pinned digest before every use."""
    destination.parent.mkdir(parents=True, exist_ok=True)
    if not destination.is_file():
        with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False, suffix=".download") as output:
            temporary = Path(output.name)
            try:
                with urllib.request.urlopen(url, timeout=120) as response:
                    shutil.copyfileobj(response, output)
            except BaseException:
                output.close()
                temporary.unlink(missing_ok=True)
                raise
        temporary.replace(destination)
    with destination.open("rb") as package:
        actual = hashlib.file_digest(package, "sha256").hexdigest()
    if actual != digest:
        raise RuntimeError(f"Bundled Bash checksum mismatch: {destination}; expected {digest}, got {actual}")
    return destination


def verify_bash_runtime(root: Path) -> None:
    """Reject incomplete payloads before they can enter an archive or installer."""
    for relative in REQUIRED_FILES:
        if not (root / relative).is_file():
            raise RuntimeError(f"Required bundled Bash file not found: {root / relative}")


def prepare_windows_bash(destination: Path, arch: str = "x86_64") -> Path:
    """Download a checksum-pinned portable runtime and copy its complete licensed tree."""
    if os.name != "nt":
        raise RuntimeError("Preparing the Windows Bash runtime requires a Windows build host")
    suffix, digest = PACKAGES[arch]
    destination = destination.resolve()
    target = destination / "bash"
    if target.resolve().parent != destination:
        raise RuntimeError(f"Bundled Bash destination escapes its package: {target}")
    bundle = {
        "version": VERSION, "architecture": arch, "sha256": digest,
        "source": f"https://github.com/git-for-windows/git/releases/tag/{RELEASE}",
    }
    manifest = target / "operit-bundle.json"
    if manifest.is_file():
        installed = json.loads(manifest.read_text(encoding="utf-8"))
        if installed == bundle and all((target / name).is_file() for name in REQUIRED_FILES):
            return target
    name = f"PortableGit-{VERSION}-{suffix}.7z.exe"
    cache = CACHE / f"{VERSION}-{arch}"
    cache.mkdir(parents=True, exist_ok=True)
    archive = download_verified(
        f"https://github.com/git-for-windows/git/releases/download/{RELEASE}/{name}", cache / name, digest,
    )
    extracted = cache / "runtime"
    if not extracted.is_dir():
        extractor = download_verified(EXTRACTOR_URL, CACHE / "7zr-26.06.exe", EXTRACTOR_SHA256)
        with tempfile.TemporaryDirectory(dir=cache, prefix="extract-") as staging:
            run([extractor, "x", "-y", f"-o{staging}", archive])
            verify_bash_runtime(Path(staging))
            shutil.copytree(staging, extracted)
    verify_bash_runtime(extracted)
    destination.mkdir(parents=True, exist_ok=True)
    if target.exists():
        shutil.rmtree(target)
    shutil.copytree(extracted, target)
    manifest.write_text(json.dumps(bundle, indent=2) + "\n", encoding="utf-8")
    verify_bash_runtime(target)
    return target


def main() -> None:
    """Prepare a runtime beside a development binary or an existing release executable."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--arch", choices=PACKAGES, default="x86_64")
    args = parser.parse_args()
    print(prepare_windows_bash(args.destination, args.arch))


if __name__ == "__main__":
    main()
