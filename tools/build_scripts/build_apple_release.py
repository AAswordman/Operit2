#!/usr/bin/env python3
from __future__ import annotations

import argparse
import platform
import sys
from pathlib import Path

from common import DIST_DIR, REPO_ROOT, copy_required_file, host_arch, release_asset_name, run


MACOS_TYPES = ("sandbox", "unsandbox")
BUILD_SCRIPTS_DIR = REPO_ROOT / "tools" / "build_scripts"


# Parses command-line options for Apple release asset builds.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Operit2 Apple release assets.")
    parser.add_argument("--build-name", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    parser.add_argument("--cli-arches", default="all", choices=["host", "all", "x86_64", "aarch64"])
    parser.add_argument("--products", nargs="+", choices=["app", "cli"], default=["app", "cli"])
    parser.add_argument("--macos-types", nargs="+", choices=MACOS_TYPES, default=list(MACOS_TYPES))
    parser.add_argument("--include-ios", action="store_true", help="Build the unsigned iOS app package.")
    return parser.parse_args()


# Stops execution when the current machine is not macOS.
def require_macos_host() -> None:
    system = platform.system().lower()
    if system != "darwin":
        raise RuntimeError(f"Apple release assets require macOS; current host is {system}")


# Builds the macOS app package and, when selected, the iOS app package.
def build_apple_apps(build_name: str, build_number: str, dist_dir: Path, include_ios: bool,
                     macos_types: list[str] | tuple[str, ...] = MACOS_TYPES) -> None:
    for build_type in dict.fromkeys(macos_types):
        run(
            [
                sys.executable,
                BUILD_SCRIPTS_DIR / "build_flutter_macos.py",
                "--type", build_type,
                "--build-name", build_name,
                "--build-number", build_number,
                "--enforce-lockfile",
                "--archive-path",
                dist_dir / release_asset_name("app", "macos", "universal", "zip", build_type),
            ],
        )
    if include_ios:
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
                dist_dir / release_asset_name("app", "ios", "arm64", "zip"),
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
    if dist_dir.resolve() == DIST_DIR.resolve():
        return
    if cli_arches == "all":
        arches = ("x86_64", "aarch64")
    elif cli_arches == "host":
        arches = (host_arch(),)
    else:
        arches = (cli_arches,)
    for arch in arches:
        name = release_asset_name("cli", "macos", arch, "tar.gz")
        copy_required_file(DIST_DIR / name, dist_dir / name)


# Builds every selected Apple release asset.
def main() -> int:
    args = parse_args()
    require_macos_host()
    args.dist_dir.mkdir(parents=True, exist_ok=True)

    products = set(args.products)
    if "app" in products:
        build_apple_apps(args.build_name, args.build_number, args.dist_dir, args.include_ios, args.macos_types)
    if "cli" in products:
        build_apple_cli(args.cli_arches, args.dist_dir)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
