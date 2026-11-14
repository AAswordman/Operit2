"""Collect release and preview bases, then publish verified incremental assets using gh."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import subprocess
import sys

from gh_publish import ROOT, SOURCE_REPO, gh
from build_release import parse_semver
from release_artifacts import RELEASE_VERSION_PATTERN, file_sha256, release_asset_names, release_version


# Produces SemVer precedence keys with numeric prerelease identifiers ordered numerically.
def version_key(version: str) -> tuple:
    parsed = parse_semver(version, "release version")
    identifiers = tuple((0, int(part)) if part.isdecimal() else (1, part) for part in parsed.prerelease)
    return parsed.major, parsed.minor, parsed.patch, not parsed.is_prerelease, identifiers


# Downloads older full packages while retaining separate release and preview histories.
def download_bases(destination: Path, count: int) -> list[Path]:
    if not 1 <= count <= 32:
        raise ValueError("Retained release count per channel must be 1..32")
    releases = json.loads(gh("release", "list", "--repo", SOURCE_REPO, "--limit", "1000",
                            "--json", "tagName,isDraft,isPrerelease"))
    current = release_version()
    candidates = []
    for release in releases:
        tag = release["tagName"]
        if release["isDraft"] or not tag.startswith("v") or not RELEASE_VERSION_PATTERN.fullmatch(tag[1:]):
            continue
        version = tag[1:]
        if bool(release["isPrerelease"]) != ("-" in version):
            raise ValueError(f"GitHub prerelease flag disagrees with version: {tag}")
        if version_key(version) < version_key(current):
            candidates.append(release)
    candidates.sort(key=lambda release: version_key(release["tagName"][1:]), reverse=True)
    retained = {False: 0, True: 0}
    results = []
    for release in candidates:
        preview = release["isPrerelease"]
        if retained[preview] == count:
            continue
        tag = release["tagName"]
        version = tag[1:]
        expected = {name.replace(f"-{current}.", f"-{version}.") for name in release_asset_names()}
        info = json.loads(gh("api", f"repos/{SOURCE_REPO}/releases/tags/{tag}"))
        assets = [asset for asset in info["assets"] if asset["name"] in expected]
        if not assets:
            continue
        directory = destination / tag
        directory.mkdir(parents=True, exist_ok=True)
        for asset in assets:
            target = directory / asset["name"]
            digest = asset.get("digest")
            if not isinstance(digest, str) or not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
                raise ValueError(f"GitHub digest is missing for {asset['name']}")
            if not target.exists():
                gh("release", "download", tag, "--repo", SOURCE_REPO, "--pattern", asset["name"], "--dir", str(directory))
            if file_sha256(target) != digest[7:]:
                raise ValueError(f"Retained base digest mismatch: {target}")
        results.append(directory)
        retained[preview] += 1
    return results


# Publishes packages from successful builds of the exact checked-out source commit.
def publish_runs(run_ids: list[str], dist: Path, retained: int, draft: bool,
                 local_only: bool = False, full_only: bool = False,
                 history_root: Path | None = None, patch_only: bool = False) -> str:
    if patch_only and full_only:
        raise ValueError("Choose either patch-only or full-only publication")
    from download_action_artifacts import collect_run
    from gh_publish import publish
    sys.path.insert(0, str(ROOT / "tools/updates"))
    from prepare import prepare
    from archive import DEFAULT_ROOT, history, retain
    history_root = history_root or DEFAULT_ROOT
    revision = subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if dist.exists() and any(dist.iterdir()):
        raise ValueError("Use an empty dist directory for a new collected release")
    if not run_ids or any(not run_id.isdecimal() for run_id in run_ids):
        raise ValueError("Build run IDs must be numeric and nonempty")
    for run_id in run_ids:
        run = json.loads(gh("run", "view", run_id, "--repo", SOURCE_REPO,
                            "--json", "headSha,status,conclusion"))
        if run["headSha"] != revision or run["status"] != "completed" or run["conclusion"] != "success":
            raise ValueError(f"Build {run_id} is not a successful build of {revision}")
    for run_id in run_ids:
        collect_run(run_id, ROOT / "tools/release/work/incremental-runs", dist)
    if not full_only:
        bases = history(history_root, release_version())
        if not local_only:
            bases += download_bases(ROOT / "tools/release/work/incremental-bases", retained)
        prepare(dist, bases)
    retain(dist, history_root)
    notes = "Build runs:\n" + "\n".join(
        f"- https://github.com/{SOURCE_REPO}/actions/runs/{run_id}" for run_id in run_ids)
    return publish(dist, SOURCE_REPO, draft, notes, revision, patch_only=patch_only)


# Exposes retained-base collection and publication without changing product versions.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    bases = commands.add_parser("bases")
    bases.add_argument("--destination", type=Path, default=ROOT / "tools/release/work/incremental-bases")
    bases.add_argument("--count", type=int, default=3, help="Retained releases per channel (1..32)")
    publish = commands.add_parser("publish")
    publish.add_argument("--run-id", action="append", required=True)
    publish.add_argument("--dist", type=Path, default=ROOT / "tools/release/dist")
    publish.add_argument("--retain", type=int, default=3, help="Retained releases per channel (1..32)")
    publish.add_argument("--draft", action="store_true")
    publish.add_argument("--local-bases-only", action="store_true", help="Use local installer history without reading old Releases")
    publish.add_argument("--patch-only", action="store_true", help="Publish patches while keeping full installers only in local history")
    publish.add_argument("--full-only", action="store_true", help="Publish full installers without incremental metadata")
    publish.add_argument("--history", type=Path, help="Persistent local installer archive outside disposable work")
    args = parser.parse_args()
    if args.command == "bases":
        for directory in download_bases(args.destination, args.count):
            print(directory)
    else:
        print(publish_runs(args.run_id, args.dist, args.retain, args.draft,
                           args.local_bases_only, args.full_only, args.history, args.patch_only))


if __name__ == "__main__":
    main()
