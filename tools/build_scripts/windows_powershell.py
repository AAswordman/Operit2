"""Stage checksum-pinned, self-contained PowerShell 7 ZIP distributions for Windows."""

from __future__ import annotations

import argparse
import hashlib
import json
import shutil
import struct
import tempfile
import urllib.request
from pathlib import Path
from zipfile import ZipFile

from common import REPO_ROOT

RELEASE = json.loads((REPO_ROOT / "hosts/windows/src/tools/terminal/powershell_release.json").read_text(encoding="utf-8"))
VERSION = RELEASE["version"]
PACKAGES = {
    arch: (RELEASE[arch]["suffix"], RELEASE[arch]["sha256"])
    for arch in ("x86_64", "aarch64")
}
REQUIRED_FILES = (
    "pwsh.exe", "pwsh.dll", "pwsh.deps.json", "pwsh.runtimeconfig.json",
    "System.Management.Automation.dll", "Microsoft.PowerShell.ConsoleHost.dll",
    "hostfxr.dll", "hostpolicy.dll", "coreclr.dll", "System.Private.CoreLib.dll",
    "Modules/Microsoft.PowerShell.Management/Microsoft.PowerShell.Management.psd1",
    "Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1",
    "LICENSE.txt", "ThirdPartyNotices.txt",
)
CACHE = REPO_ROOT / "tools/release/work/windows-powershell"


def download_verified(url: str, destination: Path, digest: str) -> Path:
    """Verify the pinned archive before extracting or shipping any executable code."""
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
        raise RuntimeError(f"PowerShell checksum mismatch: {destination}; expected {digest}, got {actual}")
    return destination


def verify_powershell_runtime(root: Path, arch: str) -> None:
    """Reject missing runtime dependencies and an interpreter for the wrong architecture."""
    for relative in REQUIRED_FILES:
        if not (root / relative).is_file():
            raise RuntimeError(f"Required bundled PowerShell file not found: {root / relative}")
    data = (root / "pwsh.exe").read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    machine = {"x86_64": 0x8664, "aarch64": 0xAA64}[arch]
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0" or struct.unpack_from("<H", data, pe + 4)[0] != machine:
        raise RuntimeError(f"Invalid {arch} PowerShell executable: {root / 'pwsh.exe'}")


def prepare_windows_powershell(destination: Path, arch: str = "x86_64") -> Path:
    """Deploy the complete upstream runtime and licenses beside the application executable."""
    suffix, digest = PACKAGES[arch]
    destination = destination.resolve()
    target = destination / "powershell"
    if target.resolve().parent != destination:
        raise RuntimeError(f"PowerShell destination escapes its package: {target}")
    bundle = {
        "version": VERSION, "architecture": arch, "sha256": digest,
        "source": f"https://github.com/PowerShell/PowerShell/releases/tag/v{VERSION}",
    }
    manifest = target / "operit-bundle.json"
    if manifest.is_file():
        installed = json.loads(manifest.read_text(encoding="utf-8"))
        files = installed.get("files", [])
        if (installed == {**bundle, "files": files} and set(REQUIRED_FILES) <= set(files)
                and all((target / name).is_file() for name in files)):
            verify_powershell_runtime(target, arch)
            return target
    name = f"PowerShell-{VERSION}-win-{suffix}.zip"
    cache = CACHE / f"{VERSION}-{arch}"
    archive = download_verified(
        f"https://github.com/PowerShell/PowerShell/releases/download/v{VERSION}/{name}",
        cache / name, digest,
    )
    destination.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(dir=destination, prefix="powershell-") as staging:
        staging_root = Path(staging).resolve()
        with ZipFile(archive) as package:
            for entry in package.infolist():
                if not (staging_root / entry.filename).resolve().is_relative_to(staging_root):
                    raise RuntimeError(f"PowerShell archive path escapes its runtime: {entry.filename}")
            package.extractall(staging_root)
        verify_powershell_runtime(staging_root, arch)
        files = sorted(path.relative_to(staging_root).as_posix() for path in staging_root.rglob("*") if path.is_file())
        (staging_root / "operit-bundle.json").write_text(
            json.dumps({**bundle, "files": files}, indent=2) + "\n", encoding="utf-8",
        )
        if target.exists():
            shutil.rmtree(target)
        shutil.copytree(staging_root, target)
    return target


def main() -> None:
    """Stage PowerShell for a packaged executable or standalone development binary."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--destination", type=Path, required=True)
    parser.add_argument("--arch", choices=PACKAGES, default="x86_64")
    args = parser.parse_args()
    print(prepare_windows_powershell(args.destination, args.arch))


if __name__ == "__main__":
    main()
