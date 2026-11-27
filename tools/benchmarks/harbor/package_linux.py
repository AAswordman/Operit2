"""Bundle the native Linux CLI with its exact loader and shared libraries."""

import argparse
import json
import re
import shutil
import subprocess
import tarfile
from pathlib import Path


# Package a stripped copy while leaving the build cache intact.
def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("binary", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    package = args.output / "bundle"
    package.mkdir(parents=True, exist_ok=False)
    library_dir = package / "lib"
    library_dir.mkdir()
    shutil.copy2(binary, package / "operit2.real")
    subprocess.run(["strip", "--strip-unneeded", str(package / "operit2.real")], check=True)
    linked = subprocess.check_output(["ldd", str(binary)], text=True)
    libraries = re.findall(r"(?:=>\s+|^\s*)(/[^\s]+)\s+\(", linked, re.MULTILINE)
    if not libraries or "not found" in linked:
        raise RuntimeError(f"Unresolved executable dependencies: {linked}")
    for library in libraries:
        source = Path(library)
        shutil.copy2(source.resolve(strict=True), library_dir / source.name)
    loader = library_dir / "ld-linux-x86-64.so.2"
    if not loader.is_file():
        raise RuntimeError("The x86_64 Linux dynamic loader was not included")
    wrapper = package / "operit2"
    wrapper.write_text('#!/bin/sh\nexec /opt/operit/lib/ld-linux-x86-64.so.2 --library-path /opt/operit/lib /opt/operit/operit2.real "$@"\n')
    wrapper.chmod(0o755)
    manifest = {"binary_source": str(binary), "libraries": [Path(p).name for p in libraries],
                "purpose": "Local Harbor smoke evaluation; install at /opt/operit"}
    (package / "manifest.json").write_text(json.dumps(manifest, indent=2))
    archive = args.output / "operit2-harbor-linux-x86_64.tar.gz"
    with tarfile.open(archive, "w:gz") as output:
        for entry in package.iterdir():
            output.add(entry, arcname=entry.name)
    print(json.dumps({"archive": str(archive), "compressed_bytes": archive.stat().st_size,
                      "binary_bytes": (package / "operit2.real").stat().st_size}))


if __name__ == "__main__":
    main()
