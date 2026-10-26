#!/usr/bin/env python3
"""Sign and package a locally built macOS app for App Store Connect."""
from __future__ import annotations

import argparse
import plistlib
import shutil
import subprocess
from pathlib import Path

from common import REPO_ROOT, release_asset_name, run

DEFAULT_APP = REPO_ROOT / "apps" / "flutter" / "app" / "build" / "macos" / "Build" / "Products" / "Release" / "Operit2.app"
DEFAULT_WORK = REPO_ROOT / "tools" / "release" / "work" / "macos-local"
DEFAULT_PROFILE = REPO_ROOT / "tools" / "release" / "secrets" / "appstoreconnect" / "Operit2-Mac-App-Store-Distribution.provisionprofile"
DEFAULT_ENTITLEMENTS = REPO_ROOT / "apps" / "flutter" / "app" / "macos" / "Runner" / "Release.entitlements"


def first_identity(kind: str) -> str:
    if kind == "codesigning":
        output = subprocess.check_output(["security", "find-identity", "-v", "-p", "codesigning"], text=True)
        marker = None
    elif kind == "installers":
        # macOS does not expose an "installers" security policy.
        output = subprocess.check_output(["security", "find-identity", "-v"], text=True)
        marker = ("3rd Party Mac Developer Installer", "Mac Installer Distribution")
    else:
        raise RuntimeError(f"Unsupported identity kind: {kind}")
    for line in output.splitlines():
        if '"' not in line:
            continue
        identity = line.split('"', 2)[1]
        if marker is None or any(name in identity for name in marker):
            return identity
    raise RuntimeError(f"No valid {kind} signing identity found in the login keychain.")


# Signs the App bundle and creates a versioned macOS installer package.
def sign_macos_app(app_source: Path, work_dir: Path, profile: Path, entitlements_source: Path, app_identity: str, installer_identity: str) -> Path:
    if not app_source.is_dir():
        raise RuntimeError(f"macOS app bundle not found: {app_source}")
    if not profile.is_file():
        raise RuntimeError(f"macOS provisioning profile not found: {profile}")
    if not entitlements_source.is_file():
        raise RuntimeError(f"macOS entitlements not found: {entitlements_source}")
    if work_dir.exists():
        shutil.rmtree(work_dir)
    work_dir.mkdir(parents=True)
    app = work_dir / app_source.name
    shutil.copytree(app_source, app, symlinks=True)

    info_path = app / "Contents" / "Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    info["CFBundleIdentifier"] = "app.operit"
    info["LSApplicationCategoryType"] = "public.app-category.productivity"
    info_path.write_bytes(plistlib.dumps(info, fmt=plistlib.FMT_XML))
    shutil.copy2(profile, app / "Contents" / "embedded.provisionprofile")

    entitlements = plistlib.loads(entitlements_source.read_bytes())
    team_id = "8KLRN8Q2NA"
    entitlements.update({
        "com.apple.application-identifier": f"{team_id}.app.operit",
        "com.apple.developer.team-identifier": team_id,
    })
    entitlements_path = work_dir / "entitlements.plist"
    entitlements_path.write_bytes(plistlib.dumps(entitlements, fmt=plistlib.FMT_XML))

    frameworks_dir = app / "Contents" / "Frameworks"
    for framework in sorted(frameworks_dir.glob("*.framework")):
        run(["codesign", "--force", "--sign", app_identity, "--timestamp", str(framework)])
    run(["codesign", "--force", "--sign", app_identity, "--timestamp", "--entitlements", str(entitlements_path), str(app)])
    run(["codesign", "--verify", "--deep", "--strict", str(app)])

    pkg = work_dir / release_asset_name("app", "macos", "universal", "pkg")
    run(["productbuild", "--component", str(app), "/Applications", "--sign", installer_identity, str(pkg)])
    if not pkg.is_file() or pkg.stat().st_size == 0:
        raise RuntimeError(f"macOS installer was not produced: {pkg}")
    return pkg


def main() -> int:
    parser = argparse.ArgumentParser(description="Sign a local macOS app and produce an App Store Connect .pkg.")
    parser.add_argument("--app", type=Path, default=DEFAULT_APP)
    parser.add_argument("--work-dir", type=Path, default=DEFAULT_WORK)
    parser.add_argument("--profile", type=Path, default=DEFAULT_PROFILE)
    parser.add_argument("--entitlements", type=Path, default=DEFAULT_ENTITLEMENTS)
    parser.add_argument("--app-identity", default=None)
    parser.add_argument("--installer-identity", default=None)
    args = parser.parse_args()
    app_identity = args.app_identity or first_identity("codesigning")
    installer_identity = args.installer_identity or first_identity("installers")
    pkg = sign_macos_app(args.app.resolve(), args.work_dir.resolve(), args.profile.resolve(), args.entitlements.resolve(), app_identity, installer_identity)
    print(f"macOS package: {pkg}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}")
        raise SystemExit(1)
