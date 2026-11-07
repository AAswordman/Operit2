#!/usr/bin/env python3
"""Build the Android module application folder and its versioned installer ZIP."""
from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import re
import shutil
import sys
import zipfile

from common import DIST_DIR, FLUTTER_APP_DIR, RELEASE_DIR, REPO_ROOT, reset_dir, run
from release_artifacts import release_asset_name, release_version

MODULE_WORK_DIR = RELEASE_DIR / "work/android-module"


def parse_args() -> argparse.Namespace:
    """Accept the same Flutter platform version and dependency lock options as App release builds."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--enforce-lockfile", action="store_true")
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    return parser.parse_args()


def platform_metadata(build_name: str | None, build_number: str | None) -> tuple[str, str]:
    """Resolve platform metadata and reject a module release with mismatched product versions."""
    pubspec = (FLUTTER_APP_DIR / "pubspec.yaml").read_text(encoding="utf-8")
    matches = re.findall(r"^version:\s*(\d+\.\d+\.\d+)\+([1-9][0-9]*)\s*$", pubspec, re.MULTILINE)
    if len(matches) != 1:
        raise ValueError("pubspec.yaml must contain exactly one major.minor.patch+buildNumber version")
    name = matches[0][0] if build_name is None else build_name
    number = matches[0][1] if build_number is None else build_number
    if name != release_version().split("-", 1)[0]:
        raise ValueError("Flutter build name must match the shared module release version")
    if re.fullmatch(r"[1-9][0-9]*", number) is None or int(number) > 2100000000:
        raise ValueError("Android module build number must be a positive Android versionCode")
    return name, number


def package_module(module: Path, destination: Path) -> None:
    """Archive the verified folder contents at ZIP root with portable executable permissions."""
    checksums = (module / "payload/archive.sha256").read_text(encoding="utf-8")
    expected = {"payload/archive.sha256"}
    for line in checksums.splitlines():
        checksum, relative = line.split(maxsplit=1)
        source = module / relative
        if not source.resolve().is_relative_to(module.resolve()) or source.is_symlink():
            raise ValueError(f"Invalid module payload path: {relative}")
        with source.open("rb") as handle:
            actual_checksum = hashlib.file_digest(handle, "sha256").hexdigest()
        if actual_checksum != checksum:
            raise ValueError(f"Module checksum mismatch: {relative}")
        expected.add(relative)
    actual = {path.relative_to(module).as_posix() for path in module.rglob("*") if path.is_file()}
    if actual != expected or "module.prop" not in expected:
        raise ValueError("Module folder contents do not match the installer manifest")
    destination.parent.mkdir(parents=True, exist_ok=True)
    temporary = destination.with_name(destination.name + ".tmp")
    try:
        with zipfile.ZipFile(temporary, "w", zipfile.ZIP_DEFLATED, compresslevel=1) as archive:
            for relative in sorted(expected):
                entry = zipfile.ZipInfo(relative)
                entry.create_system = 3
                executable = relative.endswith(".sh") or relative.startswith("bin/")
                entry.external_attr = (0o100755 if executable else 0o100644) << 16
                entry.compress_type = zipfile.ZIP_DEFLATED
                with (module / relative).open("rb") as source, archive.open(entry, "w") as target:
                    shutil.copyfileobj(source, target)
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)


def main() -> int:
    """Build the module variant separately and stage its ZIP beside ordinary App release assets."""
    args = parse_args()
    build_name, build_number = platform_metadata(args.build_name, args.build_number)
    version = release_version()
    work = MODULE_WORK_DIR.resolve()
    if MODULE_WORK_DIR.is_symlink() or not work.is_relative_to((RELEASE_DIR / "work").resolve()):
        raise ValueError("Module release workspace must stay inside tools/release/work")
    reset_dir(work)
    module = work / "module-arm64-v8a"
    command = [sys.executable, REPO_ROOT / "apps/android-module/build.py", "--output", module,
               "--release-version", version, "--build-name", build_name, "--build-number", build_number]
    if args.enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command)
    destination = args.dist_dir / release_asset_name("module", "android", "arm64-v8a", "zip")
    package_module(module, destination)
    print(f"Android module folder: {module}", flush=True)
    print(f"Android module ZIP: {destination}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
