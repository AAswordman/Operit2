#!/usr/bin/env python3
"""Run Cargo with source paths that are safe to ship in diagnostics and symbols."""
from __future__ import annotations

import os
from pathlib import Path
import subprocess
import sys
from collections.abc import Mapping, Sequence


REPO_ROOT = Path(os.path.abspath(__file__)).parents[2]


def cargo_build_environment(
    command: Sequence[str | Path],
    environment: Mapping[str, str] | None = None,
    *,
    repo_root: Path = REPO_ROOT,
    cwd: Path | None = None,
) -> dict[str, str]:
    env = dict(os.environ if environment is None else environment)
    args = [str(arg) for arg in command]

    def argument(name: str) -> str | None:
        for index, value in enumerate(args):
            if value.startswith(name + "="):
                return value[len(name) + 1:]
            if value == name and index + 1 < len(args):
                return args[index + 1]
        return None

    def absolute(path: Path) -> Path:
        return Path(os.path.abspath(path if path.is_absolute() else (cwd or Path.cwd()) / path))

    # Match Cargo's environment flag precedence. Encoded flags keep paths with
    # spaces intact, including generated sources included through OUT_DIR.
    if "CARGO_ENCODED_RUSTFLAGS" in env:
        flags = [flag for flag in env["CARGO_ENCODED_RUSTFLAGS"].split("\x1f") if flag]
    else:
        rustflags = env.get("RUSTFLAGS")
        if rustflags is None:
            target = argument("--target") or env.get("CARGO_BUILD_TARGET")
            if target:
                rustflags = env.get(f"CARGO_TARGET_{target.upper().replace('-', '_')}_RUSTFLAGS")
        flags = (rustflags if rustflags is not None else "-Awarnings").split()

    home = Path.home()
    mappings = [
        (home, "/build-home"),
        (Path(env.get("CARGO_HOME") or home / ".cargo"), "/cargo"),
        (Path(env.get("RUSTUP_HOME") or home / ".rustup"), "/rustup"),
    ]
    target_dir = argument("--target-dir") or env.get("CARGO_TARGET_DIR") or env.get("CARGO_BUILD_TARGET_DIR")
    if target_dir:
        mappings.append((absolute(Path(target_dir)), "/operit-target"))
    # rustc uses the last matching mapping: keep repository paths more useful
    # than the general home/cache mappings, also for a symlinked checkout.
    mappings.append((repo_root, "/operit"))
    for source, destination in mappings:
        source = absolute(source)
        for path in dict.fromkeys((source, source.resolve())):
            for prefix in dict.fromkeys((str(path), path.as_posix())):
                flag = f"--remap-path-prefix={prefix}={destination}"
                if flag not in flags:
                    flags.append(flag)
    env["CARGO_ENCODED_RUSTFLAGS"] = "\x1f".join(flags)
    env.pop("RUSTFLAGS", None)
    return env


def main() -> int:
    command = sys.argv[1:]
    if not command:
        raise SystemExit("usage: cargo_build.py <cargo executable> <arguments...>")
    return subprocess.run(command, env=cargo_build_environment(command)).returncode


if __name__ == "__main__":
    raise SystemExit(main())
