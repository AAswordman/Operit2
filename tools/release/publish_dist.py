#!/usr/bin/env python3
from __future__ import annotations

import argparse
import sys
from pathlib import Path

from release import (
    DEFAULT_GITHUB_ENV,
    DEFAULT_RELEASE_REPO,
    DIST_DIR,
    github_auth,
    load_env_file,
    parse_semver,
    publish_release,
    release_version,
    verify_github_publish_access,
)


# Parses command-line options for publishing already-built release assets.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Publish existing Operit2 assets from tools/release/dist.")
    parser.add_argument("--tag", default="")
    parser.add_argument("--repo", default=DEFAULT_RELEASE_REPO)
    parser.add_argument("--github-env", default=str(DEFAULT_GITHUB_ENV))
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
    load_env_file(Path(args.github_env))
    auth = github_auth()
    verify_github_publish_access(args.repo, auth)

    print(f"Release tag: {tag}")
    print(f"Release repo: {args.repo}")
    print(f"Release draft: {args.draft}")
    print(f"Release prerelease: {is_prerelease}")
    print("Release assets:")
    for asset in assets:
        print(f" - {asset.name}")

    if args.check_only:
        print("Publish check passed")
        return 0

    publish_release(tag, args.repo, args.draft, is_prerelease, auth)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"publish failed: {error}", file=sys.stderr)
        raise SystemExit(1)
