#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import sys
from pathlib import Path

from common import DIST_DIR, host_arch, host_platform, run


BUILD_SCRIPTS_DIR = os.path.dirname(os.path.abspath(__file__))
PRODUCT_CHOICES = ("app", "cli", "all")
CLI_ARCH_CHOICES = ("host", "all", "x86_64", "aarch64")


# Parses options for a native local App and CLI build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Operit2 App and CLI for the current platform.")
    parser.add_argument("--products", choices=PRODUCT_CHOICES, default="all")
    parser.add_argument("--cli-arches", choices=CLI_ARCH_CHOICES, default="host")
    parser.add_argument("--include-ios", action="store_true")
    parser.add_argument("--enforce-lockfile", action="store_true")
    return parser.parse_args()


# Returns the platform-specific Python build script path.
def build_script(name: str, platform_name: str) -> str:
    return os.path.join(BUILD_SCRIPTS_DIR, f"{name}_{platform_name}.py")


# Returns the native App archive path under the release dist directory.
def local_app_archive_path(platform_name: str, architecture: str) -> Path:
    if platform_name == "windows":
        return DIST_DIR / f"operit2-app-windows-{architecture}.zip"
    if platform_name == "linux":
        return DIST_DIR / f"operit2-app-linux-{architecture}.tar.gz"
    if platform_name == "macos":
        return DIST_DIR / f"operit2-app-macos-{architecture}.zip"
    raise RuntimeError(f"Unsupported local App platform: {platform_name}")


# Builds the native Flutter desktop App for the current platform.
def build_local_app(platform_name: str, enforce_lockfile: bool) -> None:
    command = [
        sys.executable,
        build_script("build_flutter", platform_name),
        "--archive-path",
        local_app_archive_path(platform_name, host_arch()),
    ]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command)


# Builds the native CLI archive for the current platform.
def build_local_cli(platform_name: str, cli_arches: str) -> None:
    run(
        [
            sys.executable,
            build_script("build_cli", platform_name),
            "--arches",
            cli_arches,
        ]
    )


# Builds the unsigned iOS archive from a macOS host.
def build_ios_app(enforce_lockfile: bool) -> None:
    command = [sys.executable, os.path.join(BUILD_SCRIPTS_DIR, "build_flutter_ios.py")]
    if enforce_lockfile:
        command.append("--enforce-lockfile")
    run(command)


# Builds the selected native products after producing the shared Web Access bundle.
def main() -> int:
    args = parse_args()
    platform_name = host_platform()
    if args.include_ios and platform_name != "macos":
        raise RuntimeError("--include-ios requires a macOS host")
    if args.include_ios and args.products == "cli":
        raise RuntimeError("--include-ios requires --products app or all")

    os.environ["RUSTFLAGS"] = "-Awarnings"
    run([sys.executable, os.path.join(BUILD_SCRIPTS_DIR, "build_flutter_web_access.py")])
    if args.products in ("app", "all"):
        build_local_app(platform_name, args.enforce_lockfile)
        if args.include_ios:
            build_ios_app(args.enforce_lockfile)
    if args.products in ("cli", "all"):
        build_local_cli(platform_name, args.cli_arches)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
