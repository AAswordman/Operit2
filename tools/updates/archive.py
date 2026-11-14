"""Retain exact installer bytes locally without publishing a GitHub Release."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import shutil
import sys
import tempfile

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "build_scripts"))
from release_artifacts import RELEASE_VERSION_PATTERN, file_sha256, release_asset_names, release_version

DEFAULT_ROOT = Path(__file__).resolve().parents[1] / "release/bases"


# Maps the current package naming rules to a validated historical product version.
def package_names(version: str) -> set[str]:
    if RELEASE_VERSION_PATTERN.fullmatch(version) is None:
        raise ValueError(f"Invalid archive version: {version}")
    current = release_version()
    return {name.replace(f"-{current}.", f"-{version}.") for name in release_asset_names()}


# Validates every retained original against its immutable local digest inventory.
def inventory(directory: Path) -> dict:
    data = json.loads((directory / "packages.json").read_text(encoding="utf-8"))
    if (set(data) != {"schema", "version", "packages"} or data["schema"] != 1
            or data["version"] != directory.name):
        raise ValueError(f"Invalid base archive: {directory}")
    allowed = package_names(data["version"])
    if not data["packages"] or any(name not in allowed for name in data["packages"]):
        raise ValueError(f"Invalid archived package names: {directory}")
    actual = {path.name for path in directory.iterdir() if path.name != "packages.json"}
    if actual != set(data["packages"]):
        raise ValueError(f"Archive contains missing or unexpected files: {directory}")
    for name, identity in data["packages"].items():
        package = directory / name
        if identity != {"size": package.stat().st_size, "sha256": file_sha256(package)}:
            raise ValueError(f"Archived package digest mismatch: {package}")
    return data


# Publishes one complete immutable local snapshot outside the disposable release work directory.
def retain(dist: Path, root: Path = DEFAULT_ROOT, version: str | None = None) -> Path:
    version = version or release_version()
    expected = package_names(version)
    packages = sorted(path for path in dist.iterdir() if path.name in expected and path.is_file())
    if not packages:
        raise ValueError("No matching full packages to retain")
    data = {"schema": 1, "version": version, "packages": {
        path.name: {"size": path.stat().st_size, "sha256": file_sha256(path)} for path in packages}}
    destination = root / version
    if destination.exists():
        if inventory(destination) != data:
            raise ValueError(f"Archived version is immutable; use a new version: {destination}")
        return destination
    root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix=".staging-", dir=root) as temporary:
        staged = Path(temporary) / version
        staged.mkdir()
        for package in packages:
            shutil.copyfile(package, staged / package.name)
        (staged / "packages.json").write_text(json.dumps(data, indent=2) + "\n", encoding="utf-8")
        inventory(staged)
        staged.rename(destination)
    return destination


# Selects verified older snapshots using the same SemVer ordering as online releases.
def history(root: Path, current: str, count: int = 3) -> list[Path]:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "release"))
    from incremental import version_key
    if not 1 <= count <= 32:
        raise ValueError("Retained local version count per channel must be 1..32")
    if not root.exists():
        return []
    results = []
    candidates = [directory for directory in root.iterdir()
                  if directory.is_dir() and not directory.name.startswith(".staging-")
                  and version_key(directory.name) < version_key(current)]
    candidates.sort(key=lambda directory: version_key(directory.name), reverse=True)
    retained = {False: 0, True: 0}
    for directory in candidates:
        preview = "-" in directory.name
        if retained[preview] >= count:
            continue
        inventory(directory)
        results.append(directory)
        retained[preview] += 1
    return results


# Archives a manually distributed installer set without any GitHub operation.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--dist", type=Path, default=Path("tools/release/dist"))
    parser.add_argument("--root", type=Path, default=DEFAULT_ROOT)
    parser.add_argument("--version", help="Version of an older manually distributed package set")
    args = parser.parse_args()
    print(retain(args.dist, args.root, args.version))


if __name__ == "__main__":
    main()
