"""Stage Bash, common Unix commands, and their dependencies from pinned PortableGit."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import shutil
import struct
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
PAYLOAD_REVISION = 1
COMMANDS = """
    bash sh base32 base64 basename bzip2 bunzip2 bzcat cat chmod chown chgrp
    cksum clear cmp column comm cp csplit cut cygpath date dd df diff diff3
    dirname dos2unix du echo env expand expr false find fmt fold gawk awk
    getopt grep gzip head hostname iconv id install join kill less ln ls
    md5sum mkdir mkfifo mktemp mv nano nice nl nohup nproc od paste patch
    printenv printf ps pwd readlink realpath rm rmdir sed seq sha1sum
    sha224sum sha256sum sha384sum sha512sum shuf sleep sort split stat stty
    sum sync tac tail tar tee test timeout touch tput tr true truncate
    tty uname unexpand uniq unix2dos unlink unzip wc which whoami xargs xxd yes
""".split()
SCRIPTS = ("egrep", "fgrep", "gunzip", "zcat", "zgrep", "zless")
REQUIRED_FILES = (
    "bin/bash.exe", "usr/bin/msys-2.0.dll", "usr/bin/cygwin-console-helper.exe",
    "LICENSE.txt", "etc/inputrc", "etc/nsswitch.conf",
    *(f"usr/bin/{name}.exe" for name in COMMANDS),
    *(f"usr/bin/{name}" for name in SCRIPTS),
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


def binary_imports(path: Path) -> set[str]:
    """Read PE import tables for both AMD64 and ARM64 without executing target code."""
    data = path.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[:2] != b"MZ" or data[pe:pe + 4] != b"PE\0\0":
        raise RuntimeError(f"Invalid bundled Bash PE image: {path}")
    sections = struct.unpack_from("<H", data, pe + 6)[0]
    optional_size = struct.unpack_from("<H", data, pe + 20)[0]
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x20B:
        raise RuntimeError(f"Expected a 64-bit bundled Bash PE image: {path}")
    table = optional + optional_size

    def offset(rva: int) -> int:
        """Resolve an image address through its on-disk section mapping."""
        for index in range(sections):
            size, address, raw_size, raw = struct.unpack_from("<IIII", data, table + index * 40 + 8)
            if address <= rva < address + max(size, raw_size):
                return raw + rva - address
        raise RuntimeError(f"Invalid bundled Bash import address in {path}: {rva}")

    imports = set()
    for directory, descriptor_size, name_offset in ((1, 20, 12), (13, 32, 4)):
        rva = struct.unpack_from("<I", data, optional + 112 + directory * 8)[0]
        if not rva:
            continue
        descriptor = offset(rva)
        while any(data[descriptor:descriptor + descriptor_size]):
            name_rva = struct.unpack_from("<I", data, descriptor + name_offset)[0]
            start = offset(name_rva)
            imports.add(data[start:data.index(b"\0", start)].decode("ascii").lower())
            descriptor += descriptor_size
    return imports


def bash_payload(root: Path) -> list[str]:
    """Select shell tools and their DLL closure, terminal data, and upstream licenses."""
    selected = set(REQUIRED_FILES)
    binaries = {path.name.lower(): path for path in (root / "usr/bin").glob("*.dll")}
    pending = [root / name for name in selected if Path(name).suffix == ".exe"]
    while pending:
        for name in binary_imports(pending.pop()):
            dependency = binaries.get(name)
            if dependency is None:
                if name.startswith("msys-"):
                    raise RuntimeError(f"Missing bundled Bash DLL: {name}")
                continue  # Other imports are supplied by Windows itself.
            relative = dependency.relative_to(root).as_posix()
            if relative not in selected:
                selected.add(relative)
                pending.append(dependency)
    for directory in ("usr/share/licenses", "usr/share/terminfo"):
        selected.update(path.relative_to(root).as_posix() for path in (root / directory).rglob("*") if path.is_file())
    return sorted(selected)


def generated_files(arch: str) -> dict[str, str]:
    """Keep launcher root detection and MSYS temporary files functional without Git."""
    prefix = {"x86_64": "ucrt64", "aarch64": "clangarm64"}[arch]
    return {
        f"{prefix}/bin/README-Operit.txt": (
            "This directory identifies the portable root for the upstream Bash launcher.\n"
            "Operit bundles Bash and Unix tools; Git and its GUI are not included.\n"
        ),
        "tmp/.keep": "Reserved for MSYS temporary files.\n",
    }


def prepare_windows_bash(destination: Path, arch: str = "x86_64") -> Path:
    """Prepare the licensed Bash payload without Git, GUI applications, or development files."""
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
        "payload_revision": PAYLOAD_REVISION,
    }
    manifest = target / "operit-bundle.json"
    generated = generated_files(arch)
    if manifest.is_file():
        installed = json.loads(manifest.read_text(encoding="utf-8"))
        files = installed.get("files", [])
        if (installed == {**bundle, "files": files} and set((*REQUIRED_FILES, *generated)) <= set(files)
                and all((target / name).is_file() for name in files)):
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
    files = bash_payload(extracted)
    destination.mkdir(parents=True, exist_ok=True)
    if target.exists():
        shutil.rmtree(target)
    for relative in files:
        output = target / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(extracted / relative, output)
    for relative, contents in generated.items():
        output = target / relative
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(contents, encoding="utf-8")
    bundle["files"] = sorted([*files, *generated])
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
