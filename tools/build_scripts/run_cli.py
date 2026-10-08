#!/usr/bin/env python3
"""Build the local CLI, restore its configured macOS signing identity, then optionally run it."""
from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys

from cli_macos_signing import sign_cli_macos
from common import REPO_ROOT


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release", action="store_true")
    parser.add_argument("--no-build", action="store_true", help="Sign and run an existing artifact")
    parser.add_argument("--identity", help="Use this real macOS signing identity instead of local settings")
    parser.add_argument("cli_args", nargs=argparse.REMAINDER, help="Arguments to operit2, after --")
    args = parser.parse_args()
    cli = REPO_ROOT / "apps" / "cli"
    if not args.no_build:
        command = ["cargo", "build", "--manifest-path", str(cli / "Cargo.toml")]
        if args.release:
            command.append("--release")
        subprocess.run(command, cwd=REPO_ROOT, check=True, stdout=sys.stderr)
    profile = "release" if args.release else "debug"
    target = Path(os.environ.get("CARGO_TARGET_DIR", cli / "target"))
    if not target.is_absolute():
        target = REPO_ROOT / target
    binary = target / profile / ("operit2.exe" if os.name == "nt" else "operit2")
    sign_cli_macos(binary, args.identity)
    command_args = args.cli_args[1:] if args.cli_args[:1] == ["--"] else args.cli_args
    if command_args:
        os.execv(str(binary), [str(binary), *command_args])
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
