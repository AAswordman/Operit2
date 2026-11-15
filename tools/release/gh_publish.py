"""Publish immutable releases using the authenticated GitHub CLI."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tools/build_scripts"))
from release_artifacts import file_sha256, release_version, validate_release_assets

SOURCE_REPO = "AAswordman/Operit2"
PATCH_REPO = "AAswordman/Operit2-nightly"


# Runs gh with its current authenticated session and retains structured output.
def gh(*args: str, payload: dict | None = None) -> str:
    command = ["gh", *args]
    if payload is not None:
        command.extend(["--input", "-"])
    result = subprocess.run(command, input=json.dumps(payload) if payload is not None else None,
                            text=True, encoding="utf-8", capture_output=True, check=False, cwd=ROOT)
    if result.returncode:
        raise RuntimeError(result.stderr.strip())
    return result.stdout.strip()


# Verifies all assets before creating a draft, then publishes the complete set atomically.
def publish(dist: Path, repo: str, draft: bool, notes: str | None = None,
            source_revision: str | None = None, check_only: bool = False, patch_only: bool = False) -> str:
    version = release_version()
    if repo == PATCH_REPO and not patch_only:
        raise ValueError("The patch repository accepts only manifests and patches")
    if repo != (PATCH_REPO if patch_only else SOURCE_REPO):
        raise ValueError("Publish full installers to Operit2 and patches to Operit2-nightly")
    assets = sorted(path for path in dist.iterdir() if path.is_file())
    if patch_only:
        manifests = [path for path in assets if path.name.endswith(".update.json")
                     and json.loads(path.read_text(encoding="utf-8"))["patches"]]
        selected = {path.name for path in manifests}
        for manifest in manifests:
            selected.update(patch["assetName"] for patch in json.loads(manifest.read_text(encoding="utf-8"))["patches"])
        assets = [path for path in assets if path.name in selected]
    else:
        assets = [path for path in assets if not path.name.endswith((".update.json", ".patch.gz"))]
    if not assets:
        raise ValueError("No staged release assets")
    validate_release_assets(assets)
    if gh("api", f"repos/{repo}", "--jq", ".permissions.push") != "true":
        raise ValueError(f"The current gh account cannot publish to {repo}")
    releases = json.loads(gh("release", "list", "--repo", repo, "--limit", "1000", "--json", "tagName,isDraft,isPrerelease"))
    tag = "v" + version
    existing = next((release for release in releases if release["tagName"] == tag), None)
    prerelease = "-" in version
    if existing is not None and (not existing["isDraft"] or existing["isPrerelease"] != prerelease):
        raise ValueError("A published release is immutable; use a new release version")
    revision = source_revision or subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip()
    if len(revision) != 40 or any(char not in "0123456789abcdef" for char in revision):
        raise ValueError("Source revision must be a full Git commit SHA")
    description = ("Incremental patches only. Keep the exact original installer for your installed version."
                   if patch_only else "Full installers. Incremental patches are published in Operit2-nightly.")
    body = (f"Operit2 {version}\n\nSource: https://github.com/{SOURCE_REPO}/tree/{revision}\n\n"
            + (notes or description))
    if check_only:
        return f"Validated {len(assets)} assets for {repo} {tag} from {revision}"
    with tempfile.TemporaryDirectory(prefix="operit-release-") as temporary:
        notes_path = Path(temporary) / "notes.md"
        notes_path.write_text(body, encoding="utf-8")
        if existing is None:
            gh("release", "create", tag, "--repo", repo, "--title", f"Operit2 {version}",
               "--notes-file", str(notes_path), "--draft", f"--prerelease={str(prerelease).lower()}",
               *(("--target", revision) if repo == SOURCE_REPO else ()))
        else:
            gh("release", "edit", tag, "--repo", repo, "--notes-file", str(notes_path))
        for asset in assets:
            gh("release", "upload", tag, str(asset), "--repo", repo, "--clobber")
        remote = json.loads(gh("release", "view", tag, "--repo", repo, "--json", "assets"))
        published = {asset["name"]: asset for asset in remote["assets"]}
        if set(published) != {asset.name for asset in assets}:
            raise ValueError("Draft contains unexpected or missing assets; inspect the draft before retrying")
        for asset in assets:
            if published[asset.name].get("digest") != "sha256:" + file_sha256(asset):
                raise ValueError(f"GitHub asset digest mismatch: {asset.name}")
        if not draft:
            gh("release", "edit", tag, "--repo", repo, "--draft=false")
    return f"https://github.com/{repo}/releases/tag/{tag}"


# Provides a CLI entry point that also works in GitHub Actions with GH_TOKEN.
def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--repo")
    parser.add_argument("--dist", type=Path, default=ROOT / "tools/release/dist")
    parser.add_argument("--draft", action="store_true")
    mode = parser.add_mutually_exclusive_group()
    mode.add_argument("--patch-only", dest="patch_only", action="store_true", default=True,
                      help="Publish only nonempty manifests and patches (default)")
    mode.add_argument("--full", dest="patch_only", action="store_false",
                      help="Explicitly publish full installers to the source repository")
    parser.add_argument("--check-only", action="store_true")
    parser.add_argument("--notes-file", type=Path)
    parser.add_argument("--source-revision")
    args = parser.parse_args()
    notes = args.notes_file.read_text(encoding="utf-8") if args.notes_file else None
    repo = args.repo or (PATCH_REPO if args.patch_only else SOURCE_REPO)
    print(publish(args.dist, repo, args.draft, notes, args.source_revision, args.check_only, args.patch_only))


if __name__ == "__main__":
    main()
