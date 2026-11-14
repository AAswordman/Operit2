#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from build_release import (
    DEFAULT_RELEASE_REPO,
    DIST_DIR,
    load_env_file,
    parse_semver,
    release_version,
    validate_release_assets,
)
from gh_publish import publish


# Parses command-line options for publishing already-built release assets.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Publish existing Operit2 assets from tools/release/dist.")
    parser.add_argument("--tag", default="")
    parser.add_argument("--repo")
    parser.add_argument("--github-env", help="Explicit credentials file; otherwise use the current gh login")
    parser.add_argument("--notes-file", type=Path)
    parser.add_argument("--source-revision")
    parser.add_argument("--draft", action="store_true")
    parser.add_argument(
        "--prerelease",
        action="store_true",
        help="Accepted for prerelease versions. The GitHub flag is derived from the release version.",
    )
    parser.add_argument("--check-only", action="store_true", help="Validate auth, tag, and dist assets without uploading.")
    return parser.parse_args()


# Returns every file currently staged for release upload.
def release_assets() -> list[Path]:
    if not DIST_DIR.is_dir():
        raise RuntimeError(f"Release dist directory not found: {DIST_DIR}")
    assets = sorted(path for path in DIST_DIR.iterdir() if path.is_file())
    if not assets:
        raise RuntimeError(f"Release dist directory contains no files: {DIST_DIR}")
    validate_release_assets(assets)
    return assets


# Resolves and validates the Git tag used for this publish operation.
def release_tag(tag_arg: str) -> tuple[str, bool]:
    version = release_version()
    tag = tag_arg or f"v{version.text}"
    tag_version = parse_semver(tag, "--tag", allow_tag=True)
    if tag_version.text != version.text:
        raise RuntimeError(f"--tag {tag} does not match release version {version.text}")
    return tag, version.is_prerelease


# Publishes the existing dist assets to GitHub Release.
def main() -> int:
    args = parse_args()
    tag, is_prerelease = release_tag(args.tag)
    if args.prerelease and not is_prerelease:
        raise RuntimeError("--prerelease was set for a stable release version")

    assets = release_assets()
    if args.github_env:
        load_env_file(Path(args.github_env))
    repo = args.repo or DEFAULT_RELEASE_REPO

    print(f"Release tag: {tag}")
    print(f"Release repo: {repo}")
    print(f"Release draft: {args.draft}")
    print(f"Release prerelease: {is_prerelease}")
    print("Release assets:")
    for asset in assets:
        print(f" - {asset.name}")

    notes = args.notes_file.read_text(encoding="utf-8") if args.notes_file else None
    print(publish(DIST_DIR, repo, args.draft, notes, args.source_revision, args.check_only))
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"publish failed: {error}", file=sys.stderr)
        raise SystemExit(1)
