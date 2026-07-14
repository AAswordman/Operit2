#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import shutil
import sys
from pathlib import Path

from common import DIST_DIR, REPO_ROOT, host_arch, run


BUILD_SCRIPTS_DIR = REPO_ROOT / "tools" / "build_scripts"


# Parses command-line options for Apple release asset builds.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Operit2 Apple release assets.")
    parser.add_argument("--build-name", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    parser.add_argument("--cli-arches", default="all", choices=["host", "all", "x86_64", "aarch64"])
    parser.add_argument("--products", nargs="+", choices=["app", "cli"], default=["app", "cli"])
    return parser.parse_args()


# Stops execution when the current machine is not macOS.
def require_macos_host() -> None:
    system = platform.system().lower()
    if system != "darwin":
        raise RuntimeError(f"Apple release assets require macOS; current host is {system}")


# Builds macOS and iOS app packages into the release dist directory.
def build_apple_apps(build_name: str, build_number: str, dist_dir: Path) -> None:
    current_arch = host_arch()
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_flutter_macos.py",
            "--build-name",
            build_name,
            "--build-number",
            build_number,
            "--enforce-lockfile",
            "--archive-path",
            dist_dir / f"operit2-app-macos-{current_arch}.zip",
        ],
    )
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_flutter_ios.py",
            "--build-name",
            build_name,
            "--build-number",
            build_number,
            "--enforce-lockfile",
            "--archive-path",
            dist_dir / "operit2-app-ios-arm64.zip",
        ],
    )


# Builds macOS CLI packages into the release dist directory.
def build_apple_cli(cli_arches: str, dist_dir: Path) -> None:
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_cli_macos.py",
            "--arches",
            cli_arches,
        ],
    )
    dist_dir.mkdir(parents=True, exist_ok=True)
    for asset in sorted(DIST_DIR.glob("operit2-cli-macos-*")):
        if asset.is_file():
            shutil.copy2(asset, dist_dir / asset.name)


# Builds every selected Apple release asset.
def main() -> int:
    args = parse_args()
    require_macos_host()
    args.dist_dir.mkdir(parents=True, exist_ok=True)

    run([sys.executable, BUILD_SCRIPTS_DIR / "build_flutter_web_access.py"])
    products = set(args.products)
    if "app" in products:
        build_apple_apps(args.build_name, args.build_number, args.dist_dir)
    if "cli" in products:
        build_apple_cli(args.cli_arches, args.dist_dir)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
