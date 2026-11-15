"""Provision a pinned, checksum-verified HDiffPatch generator for release tooling."""
from __future__ import annotations

import hashlib
from pathlib import Path
import platform
import subprocess
import tempfile
import zipfile

VERSION = "v5.1.3"
BINARIES = {
    ("Windows", "AMD64"): ("windows64", "77f141386e5d8f785c1c846e10fbbc19b6c05aa00e3f59cc44670fb3f0e2ae94"),
    ("Windows", "ARM64"): ("windows_arm64", "92a627a55ac1b9d54ca258a4e99919c09b9ee3c3674c494196a09a28f308eed7"),
    ("Linux", "x86_64"): ("linux64", "628963bf2ee9108a97260fa5eef44acd9ec94369b76090a957c9182b3abbb558"),
    ("Linux", "aarch64"): ("linux_arm64", "03e404e16d06479deaba645a09ed5c06636778b083b82bc7fc932ba34425430b"),
    ("Darwin", "x86_64"): ("macos", "9e9d7318db1ea5607dbdde41e6614cfe5b73c589034b38ee91a79f4ca80a940c"),
    ("Darwin", "arm64"): ("macos", "9e9d7318db1ea5607dbdde41e6614cfe5b73c589034b38ee91a79f4ca80a940c"),
}


# Retains the authenticated archive and rechecks it before every extraction.
def executable() -> Path:
    variant, digest = BINARIES[(platform.system(), platform.machine())]
    root = Path(__file__).resolve().parents[1] / "release/work/hdiffpatch" / VERSION / variant
    root.mkdir(parents=True, exist_ok=True)
    name = f"hdiffpatch_{VERSION}_bin_{variant}.zip"
    archive = root / name
    if not archive.exists():
        with tempfile.TemporaryDirectory(dir=root) as temporary:
            subprocess.run(["gh", "release", "download", VERSION, "--repo", "sisong/HDiffPatch",
                            "--pattern", name, "--dir", temporary], check=True)
            downloaded = Path(temporary) / name
            if hashlib.sha256(downloaded.read_bytes()).hexdigest() != digest:
                raise ValueError("HDiffPatch download SHA-256 mismatch")
            downloaded.replace(archive)
    if hashlib.sha256(archive.read_bytes()).hexdigest() != digest:
        raise ValueError(f"HDiffPatch cache SHA-256 mismatch: {archive}")
    binary = "hdiffz.exe" if platform.system() == "Windows" else "hdiffz"
    with zipfile.ZipFile(archive) as bundle:
        member, = [entry for entry in bundle.infolist() if Path(entry.filename).name == binary]
        output = root / binary
        data = bundle.read(member)
        if output.exists():
            if output.read_bytes() != data:
                raise ValueError(f"HDiffPatch executable differs from pinned archive: {output}")
        else:
            output.write_bytes(data)
    output.chmod(0o755)
    return output
