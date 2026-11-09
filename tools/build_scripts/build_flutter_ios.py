#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from common import (
    DIST_DIR,
    release_asset_name,
    FLUTTER_APP_DIR,
    flutter_command,
    flutter_pub_get,
    generate_dart_proxy_artifacts,
    prepare_python_command,
    require_command,
    run,
)
from prepare_apple_sherpa import prepare_apple_sherpa


IOS_RELEASE_APP_DIR = FLUTTER_APP_DIR / "build" / "ios" / "iphoneos" / "Runner.app"


def validate_ios_runtime_frameworks(app_dir: Path) -> None:
    """Reject bundles that would fail when the FFI video backend initializes."""
    if not app_dir.is_dir():
        raise RuntimeError(f"iOS app bundle was not produced: {app_dir}")
    missing = []
    for name in ("fvp", "mdk"):
        binary = app_dir / "Frameworks" / f"{name}.framework" / name
        if not binary.is_file() or binary.stat().st_size == 0:
            missing.append(f"{name}.framework/{name}")
    if missing:
        raise RuntimeError(
            "iOS app is missing required video playback frameworks: "
            + ", ".join(missing)
            + ". Reinstall iOS pods with dynamic framework linkage and rebuild."
        )


# Packages the unsigned iOS app bundle produced by Flutter.
def package_ios_app(archive_path: Path) -> Path:
    validate_ios_runtime_frameworks(IOS_RELEASE_APP_DIR)
    if archive_path.exists():
        archive_path.unlink()
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    run(["ditto", "-c", "-k", "--keepParent", str(IOS_RELEASE_APP_DIR), str(archive_path)])
    if not archive_path.is_file() or archive_path.stat().st_size == 0:
        raise RuntimeError(f"iOS archive was not produced: {archive_path}")
    return archive_path


# Parses command-line options for the unsigned iOS Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 unsigned iOS Flutter app.")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--archive-path", type=Path, default=DIST_DIR / release_asset_name("app", "ios", "arm64", "zip"))
    parser.add_argument("--skip-proxy-generation", action="store_true")
    parser.add_argument("--skip-package", action="store_true")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the unsigned iOS Flutter app and writes a zip archive for CI artifacts.
def main() -> int:
    args = parse_args()
    prepare_apple_sherpa()
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    require_command("cargo")
    flutter = flutter_command()
    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()

    prepare_python_command()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)

    command = [flutter, "build", "ios", "--release", "--no-pub", "--no-codesign"]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    command.append("-v")
    run(command, cwd=FLUTTER_APP_DIR)
    validate_ios_runtime_frameworks(IOS_RELEASE_APP_DIR)

    if not args.skip_package:
        archive_path = package_ios_app(args.archive_path)
        print(f"iOS archive: {archive_path}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
