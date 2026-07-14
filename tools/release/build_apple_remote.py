#!/usr/bin/env python3
from __future__ import annotations

import argparse
import shlex
import shutil
import subprocess
import sys
import tarfile
import time
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
WORK_DIR = SCRIPT_DIR / "work"
DIST_DIR = SCRIPT_DIR / "dist"
SOURCE_ARCHIVE = WORK_DIR / "apple-source.tar.gz"
APPLE_DIST_ARCHIVE = WORK_DIR / "apple-dist.tar.gz"
APPLE_DIST_STAGE = WORK_DIR / "apple-dist"


# Parses command-line options for the remote Apple build worker.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build Apple release assets on a macOS SSH worker.")
    parser.add_argument("--ssh", required=True, help="SSH target such as user@mac-host")
    parser.add_argument("--remote-root", default="operit2-apple-release")
    parser.add_argument("--build-name", required=True)
    parser.add_argument("--build-number", required=True)
    parser.add_argument("--cli-arches", default="all", choices=["host", "all", "x86_64", "aarch64"])
    parser.add_argument("--products", nargs="+", choices=["app", "cli"], required=True)
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    return parser.parse_args()


# Runs one local command and streams its output.
def run(command: list[str | Path], cwd: Path = REPO_ROOT) -> None:
    print("+ " + " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=cwd, check=True)


# Captures one local command result as text.
def run_capture(command: list[str | Path], cwd: Path = REPO_ROOT) -> str:
    return subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
    ).stdout


# Verifies one local executable is available.
def require_command(name: str) -> None:
    if shutil.which(name) is None:
        raise RuntimeError(f"Required command not found: {name}")


# Returns repository paths included in the source transfer archive.
def source_paths() -> list[Path]:
    raw = run_capture(["git", "ls-files", "-z", "--cached", "--modified", "--others", "--exclude-standard"])
    paths: list[Path] = []
    seen: set[str] = set()
    for item in raw.split("\0"):
        if not item:
            continue
        normalized = item.replace("\\", "/")
        if normalized in seen:
            continue
        seen.add(normalized)
        path = REPO_ROOT / normalized
        if path.is_file():
            paths.append(path)
    return paths


# Creates a tarball from the current working tree contents.
def create_source_archive() -> None:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    if SOURCE_ARCHIVE.exists():
        SOURCE_ARCHIVE.unlink()
    with tarfile.open(SOURCE_ARCHIVE, "w:gz") as archive:
        for path in source_paths():
            archive.add(path, arcname=path.relative_to(REPO_ROOT).as_posix())


# Runs one shell script on the remote macOS worker through SSH.
def run_remote(ssh_target: str, script: str) -> None:
    command = f"bash -lc {shlex.quote(script)}"
    run(["ssh", ssh_target, command])


# Copies one local file to the remote macOS worker.
def upload_file(ssh_target: str, source: Path, remote_path: str) -> None:
    run(["scp", str(source), f"{ssh_target}:{remote_path}"])


# Copies one remote file from the macOS worker.
def download_file(ssh_target: str, remote_path: str, destination: Path) -> None:
    destination.parent.mkdir(parents=True, exist_ok=True)
    run(["scp", f"{ssh_target}:{remote_path}", str(destination)])


# Extracts Apple build assets into the local release dist directory.
def extract_apple_dist(dist_dir: Path) -> None:
    if APPLE_DIST_STAGE.exists():
        shutil.rmtree(APPLE_DIST_STAGE)
    APPLE_DIST_STAGE.mkdir(parents=True, exist_ok=True)
    with tarfile.open(APPLE_DIST_ARCHIVE, "r:gz") as archive:
        archive.extractall(APPLE_DIST_STAGE)
    dist_dir.mkdir(parents=True, exist_ok=True)
    for asset in sorted(APPLE_DIST_STAGE.iterdir(), key=lambda path: path.name):
        if asset.is_file():
            shutil.copy2(asset, dist_dir / asset.name)


# Builds the shell script executed by the macOS worker.
def remote_build_script(args: argparse.Namespace, remote_archive: str, remote_source: str, remote_dist: str, remote_result: str) -> str:
    products = " ".join(shlex.quote(product) for product in args.products)
    return f"""
set -euo pipefail
rm -rf {shlex.quote(remote_source)} {shlex.quote(remote_dist)}
mkdir -p {shlex.quote(remote_source)} {shlex.quote(remote_dist)}
tar -xzf {shlex.quote(remote_archive)} -C {shlex.quote(remote_source)}
cd {shlex.quote(remote_source)}
python3 tools/build_scripts/build_apple_release.py \
  --build-name {shlex.quote(args.build_name)} \
  --build-number {shlex.quote(args.build_number)} \
  --dist-dir {shlex.quote(remote_dist)} \
  --cli-arches {shlex.quote(args.cli_arches)} \
  --products {products}
tar -czf {shlex.quote(remote_result)} -C {shlex.quote(remote_dist)} .
"""


# Transfers the current source tree, runs the Apple build, and imports produced assets.
def main() -> int:
    args = parse_args()
    require_command("git")
    require_command("ssh")
    require_command("scp")

    run_remote(args.ssh, "uname -s | grep -qx Darwin")
    create_source_archive()

    stamp = str(int(time.time()))
    remote_root = args.remote_root.rstrip("/")
    remote_archive = f"{remote_root}/source-{stamp}.tar.gz"
    remote_source = f"{remote_root}/source"
    remote_dist = f"{remote_root}/dist"
    remote_result = f"{remote_root}/apple-dist-{stamp}.tar.gz"

    run_remote(args.ssh, f"mkdir -p {shlex.quote(remote_root)}")
    upload_file(args.ssh, SOURCE_ARCHIVE, remote_archive)
    run_remote(args.ssh, remote_build_script(args, remote_archive, remote_source, remote_dist, remote_result))
    download_file(args.ssh, remote_result, APPLE_DIST_ARCHIVE)
    extract_apple_dist(args.dist_dir)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
