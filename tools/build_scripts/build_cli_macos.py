#!/usr/bin/env python3
import argparse
import sys

from cli_common import build_cli_platform, parse_cli_arch_mode, parse_cli_web_assets


# Parses command-line options for the macOS CLI build.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Operit2 CLI for macOS.")
    parser.add_argument("--arches", default="host", choices=["host", "all", "x86_64", "aarch64"])
    parser.add_argument(
        "--web-assets",
        default="embedded",
        choices=["embedded", "external"],
        help="embedded includes Web Access assets in the binary; external requires --web-root at runtime",
    )
    return parser.parse_args()


# Builds the macOS CLI package with the selected asset mode.
def main() -> int:
    args = parse_args()
    build_cli_platform(
        "macos",
        parse_cli_arch_mode(args.arches),
        parse_cli_web_assets(args.web_assets),
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
