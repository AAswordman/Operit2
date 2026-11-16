"""Create authenticated update manifests and direct deltas from retained releases."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
from release_artifacts import release_asset_names, release_version
from patch import apply, build, sha256
from archive import DEFAULT_ROOT, history, retain


# Prepares all packages in a staged release, verifying each patch by reconstruction.
def prepare(dist: Path, bases: list[Path]) -> list[Path]:
    version = release_version()
    packages = sorted(path for path in dist.iterdir() if path.name in release_asset_names())
    if not packages:
        raise ValueError("No current-version release packages in dist")
    outputs = []
    for package in packages:
        stem = package.name.split(f"-{version}.", 1)
        if len(stem) != 2:
            raise ValueError(f"Invalid package name: {package.name}")
        prefix, extension = stem
        manifest = {"schema": 4 if extension == "apk" else 3, "version": version, "assetName": package.name,
                    "sha256": sha256(package), "size": package.stat().st_size, "patches": []}
        seen = set()
        for directory in bases:
            for base in sorted(directory.glob(f"{prefix}-*.{extension}")):
                base_version = base.name[len(prefix) + 1:-(len(extension) + 1)]
                if base_version == version:
                    continue
                base_hash = sha256(base)
                if base_hash in seen:
                    continue
                if len(seen) == 64:
                    raise ValueError(f"Too many distinct bases for {package.name}; maximum is 64")
                seen.add(base_hash)
                name = f"{package.name}.from-{base_hash}.patch.gz"
                delta = dist / name
                if delta.exists():
                    raise ValueError(f"Patch already exists: {delta}; use a fresh staging directory")
                info = build(base, package, delta)
                with tempfile.TemporaryDirectory(prefix="operit-delta-") as temporary:
                    apply(base, delta, Path(temporary) / "verified-package")
                if info["size"] >= manifest["size"]:
                    delta.unlink()
                    continue
                manifest["patches"].append({"baseVersion": base_version,
                    "baseSha256": base_hash, "baseSize": info["baseSize"],
                    "assetName": name, "sha256": info["sha256"], "size": info["size"]})
                outputs.append(delta)
        output = dist / f"{package.name}.update.json"
        output.write_text(json.dumps(manifest, indent=2) + "\n", encoding="utf-8")
        outputs.append(output)
    return outputs


# Accepts multiple retained release directories to cover clients on older versions directly.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=Path("tools/release/dist"))
    parser.add_argument("--base", type=Path, action="append", default=[])
    parser.add_argument("--history", type=Path, default=DEFAULT_ROOT,
                        help="Persistent local installer history, independent of GitHub Releases")
    args = parser.parse_args()
    bases = history(args.history, release_version()) + args.base
    for output in prepare(args.dist, bases):
        print(output)
    print(retain(args.dist, args.history))


if __name__ == "__main__":
    main()
