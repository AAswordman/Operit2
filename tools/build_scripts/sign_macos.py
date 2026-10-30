#!/usr/bin/env python3
"""Sign sandbox App Store packages or unsandbox Developer ID desktop archives."""
from __future__ import annotations

import argparse
import json
from pathlib import Path
import plistlib
import shutil
import subprocess

from common import REPO_ROOT, release_asset_name, run
from macos_distribution import MACOS_TYPES, built_app_path, entitlements_path, verify_app_type

DEFAULT_WORK = REPO_ROOT / "tools" / "release" / "work" / "macos-sign"
DEFAULT_PROFILE = REPO_ROOT / "tools" / "release" / "secrets" / "appstoreconnect" / "Operit2-Mac-App-Store-Distribution.provisionprofile"


def first_identity(kind: str) -> str:
    markers = {
        "sandbox": ("3rd Party Mac Developer Application:", "Mac App Distribution:", "Apple Distribution:"),
        "unsandbox": ("Developer ID Application:",),
        "installers": ("3rd Party Mac Developer Installer:", "Mac Installer Distribution:"),
    }
    if kind not in markers:
        raise ValueError(f"Unsupported signing identity kind: {kind}")
    command = ["security", "find-identity", "-v"]
    if kind != "installers":
        command.extend(["-p", "codesigning"])
    output = subprocess.check_output(command, text=True)
    for line in output.splitlines():
        if '"' in line:
            identity = line.split('"', 2)[1]
            if identity.startswith(markers[kind]):
                return identity
    raise RuntimeError(f"No valid {kind} signing identity found in the login keychain.")


def profile_entitlements(profile: Path) -> dict:
    if not profile.is_file():
        raise RuntimeError(f"macOS provisioning profile not found: {profile}")
    data = subprocess.check_output(["security", "cms", "-D", "-i", str(profile)])
    return plistlib.loads(data)["Entitlements"]


def sign_nested_code(app: Path, identity: str, hardened: bool) -> None:
    # Sign inner code before its containing framework. Do not use --deep to sign.
    targets = [path for path in app.rglob("*") if not path.is_symlink() and
               ((path.is_dir() and path.suffix in (".framework", ".xpc", ".appex", ".app")) or
                (path.is_file() and path.suffix in (".dylib", ".so")))]
    for target in sorted(targets, key=lambda path: len(path.parts), reverse=True):
        command = ["codesign", "--force", "--sign", identity, "--timestamp"]
        if hardened:
            command.extend(["--options", "runtime"])
        run([*command, str(target)])


def archive_app(app: Path, archive: Path) -> None:
    archive.unlink(missing_ok=True)
    run(["ditto", "-c", "-k", "--keepParent", str(app), str(archive)])
    if not archive.is_file() or archive.stat().st_size == 0:
        raise RuntimeError(f"macOS archive was not produced: {archive}")


def sign_macos_app(
    app_source: Path, work_dir: Path, profile: Path | None,
    entitlements_source: Path, app_identity: str, installer_identity: str | None = None,
    build_type: str = "sandbox", notary_profile: str | None = None,
) -> Path:
    if not app_source.is_dir():
        raise RuntimeError(f"macOS app bundle not found: {app_source}; build it with --type {build_type} first")
    verify_app_type(app_source, build_type)
    entitlements = plistlib.loads(entitlements_source.read_bytes())
    if entitlements.get("com.apple.security.app-sandbox", False) != (build_type == "sandbox"):
        raise RuntimeError("Signing entitlements do not match the requested distribution type")
    if build_type == "sandbox":
        if profile is None or installer_identity is None:
            raise RuntimeError("sandbox signing requires a provisioning profile and installer identity")
        if notary_profile:
            raise RuntimeError("--notary-profile applies only to unsandbox Developer ID distribution")
        allowed = profile_entitlements(profile)
        identifier = allowed.get("com.apple.application-identifier") or allowed.get("application-identifier")
        team_id = allowed.get("com.apple.developer.team-identifier")
        if not identifier or not team_id or not identifier.startswith(team_id + ".") or "*" in identifier:
            raise RuntimeError("The App Store profile must contain an explicit application identifier and team")
        entitlements.update({"com.apple.application-identifier": identifier,
                             "com.apple.developer.team-identifier": team_id})
    else:
        # Developer ID distributions do not use the App Store profile or its identity.
        if profile is not None or installer_identity is not None:
            raise RuntimeError("unsandbox signing does not accept an App Store profile or installer identity")
        for key in ("com.apple.application-identifier", "com.apple.developer.team-identifier"):
            entitlements.pop(key, None)

    app = work_dir / app_source.name
    source_root, work_root = app_source.resolve(), work_dir.resolve()
    if source_root.is_relative_to(work_root) or work_root.is_relative_to(source_root):
        raise RuntimeError("Signing work directory must be separate from the source App bundle")
    work_dir.mkdir(parents=True, exist_ok=True)
    if app.exists():
        shutil.rmtree(app)
    shutil.copytree(app_source, app, symlinks=True)
    info_path = app / "Contents" / "Info.plist"
    info = plistlib.loads(info_path.read_bytes())
    if build_type == "sandbox":
        info["CFBundleIdentifier"] = identifier[len(team_id) + 1:]
        info["LSApplicationCategoryType"] = "public.app-category.productivity"
        shutil.copy2(profile, app / "Contents" / "embedded.provisionprofile")
    else:
        (app / "Contents" / "embedded.provisionprofile").unlink(missing_ok=True)
    info_path.write_bytes(plistlib.dumps(info, fmt=plistlib.FMT_XML))
    entitlements_file = work_dir / "entitlements.plist"
    entitlements_file.write_bytes(plistlib.dumps(entitlements, fmt=plistlib.FMT_XML))

    hardened = build_type == "unsandbox"
    sign_nested_code(app, app_identity, hardened)
    command = ["codesign", "--force", "--sign", app_identity, "--timestamp"]
    if hardened:
        command.extend(["--options", "runtime"])
    run([*command, "--entitlements", str(entitlements_file), str(app)])
    run(["codesign", "--verify", "--deep", "--strict", str(app)])
    verify_app_type(app, build_type)

    extension = "pkg" if build_type == "sandbox" else "zip"
    package = work_dir / release_asset_name("app", "macos", "universal", extension, build_type)
    if build_type == "sandbox":
        package.unlink(missing_ok=True)
        run(["productbuild", "--component", str(app), "/Applications", "--sign", installer_identity, str(package)])
    else:
        archive_app(app, package)
        if notary_profile:
            result = subprocess.run(
                ["xcrun", "notarytool", "submit", str(package), "--keychain-profile", notary_profile,
                 "--wait", "--output-format", "json"],
                capture_output=True, text=True, check=True,
            )
            receipt = json.loads(result.stdout)
            if receipt.get("status") != "Accepted":
                raise RuntimeError(f"Notarization was not accepted: {receipt}")
            run(["xcrun", "stapler", "staple", str(app)])
            run(["xcrun", "stapler", "validate", str(app)])
            archive_app(app, package)
    if not package.is_file() or package.stat().st_size == 0:
        raise RuntimeError(f"macOS package was not produced: {package}")
    return package


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--type", choices=MACOS_TYPES, default="sandbox")
    parser.add_argument("--app", type=Path)
    parser.add_argument("--work-dir", type=Path)
    parser.add_argument("--profile", type=Path)
    parser.add_argument("--entitlements", type=Path)
    parser.add_argument("--app-identity")
    parser.add_argument("--installer-identity")
    parser.add_argument("--notary-profile", help="Developer ID notarization credentials already stored by notarytool.")
    args = parser.parse_args()
    if args.type == "unsandbox" and (args.profile or args.installer_identity):
        parser.error("unsandbox does not use --profile or --installer-identity")
    if args.type == "sandbox" and args.notary_profile:
        parser.error("--notary-profile is only available for unsandbox")
    package = sign_macos_app(
        (args.app or built_app_path(args.type)).resolve(),
        (args.work_dir or DEFAULT_WORK / args.type).resolve(),
        (args.profile or DEFAULT_PROFILE).resolve() if args.type == "sandbox" else None,
        (args.entitlements or entitlements_path(args.type)).resolve(),
        args.app_identity or first_identity(args.type),
        (args.installer_identity or first_identity("installers")) if args.type == "sandbox" else None,
        args.type, args.notary_profile,
    )
    print(f"macOS {args.type} package: {package}")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}")
        raise SystemExit(1)
