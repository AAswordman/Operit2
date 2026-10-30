#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
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

from macos_distribution import MACOS_TYPES, build_environment, built_app_path, verify_app_type


# Packages the macOS app bundle as a release archive.
def package_macos_app(archive_path: Path | None = None, build_type: str = "sandbox") -> Path:
    app_path = built_app_path(build_type)
    if archive_path is None:
        archive_path = DIST_DIR / release_asset_name("app", "macos", "universal", "zip", build_type)
    if not app_path.is_dir():
        raise RuntimeError(f"macOS app bundle was not produced: {app_path}")
    verify_app_type(app_path, build_type)
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    if archive_path.exists():
        archive_path.unlink()
    run(["ditto", "-c", "-k", "--keepParent", str(app_path), str(archive_path)])
    if not archive_path.is_file() or archive_path.stat().st_size == 0:
        raise RuntimeError(f"macOS archive was not produced: {archive_path}")
    return archive_path


# Parses command-line options for the macOS Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 macOS Flutter app.")
    parser.add_argument("--type", choices=MACOS_TYPES, default="sandbox", help="macOS distribution type (default: sandbox).")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--archive-path", type=Path)
    parser.add_argument("--skip-proxy-generation", action="store_true")
    parser.add_argument("--skip-package", action="store_true")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the macOS Flutter app and writes a release archive.
def main() -> int:
    args = parse_args()
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    require_command("cargo")
    flutter = flutter_command()
    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()

    prepare_python_command()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile)

    command = [flutter, "build", "macos", "--release", "--no-pub"]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    command.append("-v")
    with build_environment(args.type) as environment:
        run(command, cwd=FLUTTER_APP_DIR, env=environment)

    app = FLUTTER_APP_DIR / "build" / "macos" / "Build" / "Products" / "Release" / "Operit2.app"
    verify_app_type(app, args.type)
    destination = built_app_path(args.type)
    destination.parent.mkdir(parents=True, exist_ok=True)
    if destination.exists():
        shutil.rmtree(destination)
    shutil.copytree(app, destination, symlinks=True)
    print(f"macOS {args.type} app: {destination}", flush=True)
    if not args.skip_package:
        archive_path = package_macos_app(args.archive_path, args.type)
        print(f"macOS archive: {archive_path}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
