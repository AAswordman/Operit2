#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from common import (
    DIST_DIR,
    FLUTTER_APP_DIR,
    build_env_with_typescript,
    ensure_node_and_npm,
    flutter_command,
    flutter_pub_get,
    generate_dart_proxy_artifacts,
    prepare_python_command,
    require_command,
    prepare_web_access_embedded_assets,
    run,
)
from prepare_apple_sherpa import prepare_apple_sherpa


IOS_RELEASE_APP_DIR = FLUTTER_APP_DIR / "build" / "ios" / "iphoneos" / "Runner.app"
IOS_ARCHIVE_PATH = DIST_DIR / "operit2-app-ios-arm64.zip"
IOS_RUNTIME_PREPARE_SCRIPT = (
    Path(__file__).resolve().parents[1] / "ios-runtime" / "prepare.py"
)


# Packages the unsigned iOS app bundle produced by Flutter.
def package_ios_app(archive_path: Path) -> Path:
    if not IOS_RELEASE_APP_DIR.is_dir():
        raise RuntimeError(f"iOS app bundle was not produced: {IOS_RELEASE_APP_DIR}")
    if archive_path.exists():
        archive_path.unlink()
    archive_path.parent.mkdir(parents=True, exist_ok=True)
    run(["ditto", "-c", "-k", "--keepParent", str(IOS_RELEASE_APP_DIR), str(archive_path)])
    if not archive_path.is_file() or archive_path.stat().st_size == 0:
        raise RuntimeError(f"iOS archive was not produced: {archive_path}")
    return archive_path


# Invokes the dedicated iOS embedded-runtime preparation tool.
def prepare_ios_runtime() -> None:
    run([sys.executable, str(IOS_RUNTIME_PREPARE_SCRIPT)])


# Parses command-line options for the unsigned iOS Flutter build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 unsigned iOS Flutter app.")
    parser.add_argument("--build-name")
    parser.add_argument("--build-number")
    parser.add_argument("--archive-path", type=Path, default=IOS_ARCHIVE_PATH)
    parser.add_argument("--skip-proxy-generation", action="store_true")
    parser.add_argument("--skip-package", action="store_true")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Builds the unsigned iOS Flutter app and writes a zip archive for CI artifacts.
def main() -> int:
    args = parse_args()
    prepare_web_access_embedded_assets()
    prepare_apple_sherpa()
    prepare_ios_runtime()
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    typescript_version = os.environ.get("TYPESCRIPT_VERSION", "5.9.3")

    require_command("cargo")
    flutter = flutter_command()
    ensure_node_and_npm()

    env = build_env_with_typescript(typescript_version)
    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()

    prepare_python_command()
    flutter_pub_get(enforce_lockfile=args.enforce_lockfile, env=env)

    command = [flutter, "build", "ios", "--release", "--no-pub", "--no-codesign"]
    if args.build_name:
        command.extend(["--build-name", args.build_name])
    if args.build_number:
        command.extend(["--build-number", args.build_number])
    command.append("-v")
    run(command, cwd=FLUTTER_APP_DIR, env=env)

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
