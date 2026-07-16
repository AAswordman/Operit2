#!/usr/bin/env python3
from __future__ import annotations

import argparse
import re
import shutil
import subprocess
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
DIST_DIR = SCRIPT_DIR / "dist"
WORK_DIR = SCRIPT_DIR / "work" / "github-actions"
RELEASE_ASSET_RE = re.compile(
    r"^operit2-(?:"
    r"cli-(?:"
    r"windows-(?:x86_64|aarch64)\.zip|"
    r"linux-(?:x86_64|aarch64)\.tar\.gz|"
    r"macos-(?:x86_64|aarch64)\.tar\.gz"
    r")|"
    r"app-(?:"
    r"android-(?:arm64-v8a|armeabi-v7a|x86_64)\.apk|"
    r"ohos-arm64\.hap|"
    r"windows-x86_64\.zip|"
    r"linux-x86_64\.tar\.gz|"
    r"macos-(?:x86_64|aarch64)\.zip|"
    r"ios-arm64\.zip"
    r")"
    r")$"
)


# Parses command-line options for collecting GitHub Actions artifacts.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Download GitHub Actions release artifacts into tools/release/dist.")
    parser.add_argument("--run-id", action="append", required=True, help="GitHub Actions run id to download")
    parser.add_argument("--dist-dir", type=Path, default=DIST_DIR)
    parser.add_argument("--work-dir", type=Path, default=WORK_DIR)
    return parser.parse_args()


# Runs one command and streams its output to the terminal.
def run(command: list[str | Path]) -> None:
    print("+ " + " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], check=True)


# Returns every release asset file found under one downloaded run directory.
def release_assets(download_dir: Path) -> list[Path]:
    return sorted(
        path
        for path in download_dir.rglob("*")
        if path.is_file() and RELEASE_ASSET_RE.fullmatch(path.name)
    )


# Downloads one run's artifacts into the work directory.
def download_run(run_id: str, work_dir: Path) -> Path:
    download_dir = work_dir / run_id
    if download_dir.exists():
        shutil.rmtree(download_dir)
    download_dir.mkdir(parents=True)
    run(["gh", "run", "download", run_id, "--dir", download_dir])
    return download_dir


# Copies release assets from one downloaded run into the dist directory.
def collect_run(run_id: str, work_dir: Path, dist_dir: Path) -> list[Path]:
    download_dir = download_run(run_id, work_dir)
    assets = release_assets(download_dir)
    if not assets:
        raise RuntimeError(f"No release asset files were found in GitHub Actions run {run_id}")
    dist_dir.mkdir(parents=True, exist_ok=True)
    copied = []
    for asset in assets:
        destination = dist_dir / asset.name
        shutil.copy2(asset, destination)
        copied.append(destination)
    return copied


# Downloads each selected run and prints the files copied into dist.
def main() -> int:
    args = parse_args()
    copied = []
    for run_id in args.run_id:
        copied.extend(collect_run(run_id, args.work_dir, args.dist_dir))
    print("Collected release assets:")
    for asset in sorted(copied):
        print(f" - {asset}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
