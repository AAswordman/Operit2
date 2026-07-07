#!/usr/bin/env python3
import argparse
import sys

from cli_common import build_cli_platform, parse_cli_arch_mode


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Operit2 CLI for Windows.")
    parser.add_argument("--arches", default="host", choices=["host", "all", "x86_64", "aarch64"])
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    build_cli_platform("windows", parse_cli_arch_mode(args.arches))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
