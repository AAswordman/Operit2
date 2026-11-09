#!/usr/bin/env python3
"""Package a complete Flutter Windows release directory as a branded MSI."""

import argparse
import hashlib
import re
import shutil
import sys
import uuid
import xml.etree.ElementTree as ET
from pathlib import Path

from common import DIST_DIR, FLUTTER_APP_DIR, REPO_ROOT, require_command, run
from release_artifacts import release_asset_name, release_version


INSTALLER_DIR = REPO_ROOT / "tools/windows-installer"
DEFAULT_RELEASE_DIR = FLUTTER_APP_DIR / "build/windows/x64/runner/Release"
WORK_DIR = INSTALLER_DIR / "build"
WIX_NAMESPACE = "http://wixtoolset.org/schemas/v4/wxs"
COMPONENT_NAMESPACE = uuid.UUID("849bac31-cbcb-4ef2-a5a4-b33a9bc7ed58")
REQUIRED_FILES = (
    "operit2.exe", "flutter_windows.dll", "operit_flutter_bridge.dll",
    "operit_local_llm.dll", "msvcp140.dll", "vcruntime140.dll",
    "data/icudtl.dat", "data/app.so", "data/flutter_assets/AssetManifest.bin",
    "Operit2DataCleanup.exe", "README-Windows.txt",
)


# Maps supported SemVer channels to strictly ordered Windows Installer versions.
def msi_version(version: str) -> str:
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(dev|preview|rc)\.([1-9]\d*))?", version)
    if match is None:
        raise ValueError(f"MSI requires major.minor.patch[-dev.N|-preview.N|-rc.N]: {version}")
    major, minor, patch = map(int, match.group(1, 2, 3))
    channel, counter = match.group(4, 5)
    revision = 999
    if channel is not None:
        if int(counter) > 199:
            raise ValueError("MSI prerelease counters must be between 1 and 199")
        revision = {"dev": 0, "preview": 200, "rc": 400}[channel] + int(counter)
    build = patch * 1000 + revision
    if major > 255 or minor > 255 or build > 65535:
        raise ValueError(f"Release version exceeds MSI version limits: {version}")
    return f"{major}.{minor}.{build}"


# Creates an XML-safe, stable identifier from a case-insensitive Windows path.
def path_id(prefix: str, relative: Path) -> str:
    digest = hashlib.sha256(relative.as_posix().casefold().encode("utf-8")).hexdigest()[:24]
    return prefix + digest


# Harvests the complete app tree with stable component identities and HKCU key paths.
def write_payload(release_dir: Path, destination: Path) -> None:
    for relative in REQUIRED_FILES:
        if not (release_dir / relative).is_file():
            raise RuntimeError(f"Required Windows release file not found: {release_dir / relative}")
    ET.register_namespace("", WIX_NAMESPACE)
    root = ET.Element("Wix", xmlns=WIX_NAMESPACE)
    fragment = ET.SubElement(root, "Fragment")
    directory_ref = ET.SubElement(fragment, "DirectoryRef", Id="INSTALLFOLDER")
    group = ET.SubElement(fragment, "ComponentGroup", Id="AppFiles")
    directories = {Path("."): directory_ref}
    paths = sorted(release_dir.rglob("*"), key=lambda item: item.as_posix().casefold())
    seen = set()
    for path in paths:
        relative = path.relative_to(release_dir)
        if path.is_symlink():
            raise RuntimeError(f"Symbolic links are not supported in MSI payloads: {relative}")
        key = relative.as_posix().casefold()
        if key in seen:
            raise RuntimeError(f"Duplicate case-insensitive Windows payload path: {relative}")
        seen.add(key)
        if path.is_dir():
            directories[relative] = ET.SubElement(
                directories[relative.parent], "Directory", Id=path_id("D", relative), Name=path.name,
            )
            continue
        component_id = path_id("C", relative)
        component = ET.SubElement(group, "Component", Id=component_id,
            Directory="INSTALLFOLDER" if relative.parent == Path(".") else path_id("D", relative.parent),
            Guid=str(uuid.uuid5(COMPONENT_NAMESPACE, key)))
        ET.SubElement(component, "File",
            Id="AppExecutable" if relative == Path("operit2.exe") else path_id("F", relative),
            Source=str(path.resolve()), Name=path.name, KeyPath="no")
        ET.SubElement(component, "RegistryValue", Root="HKCU",
            Key="Software\\Operit\\Operit2\\Installer\\Files", Name=component_id,
            Type="integer", Value="1", KeyPath="yes")
    cleanup = ET.SubElement(group, "Component", Id="AppDirectoryCleanup", Directory="INSTALLFOLDER",
        Guid="3B8AB989-61AA-41D0-82F1-E296FF5E5A7C")
    ET.SubElement(cleanup, "RegistryValue", Root="HKCU", Key="Software\\Operit\\Operit2\\Installer",
        Name="Directories", Type="integer", Value="1", KeyPath="yes")
    for relative in directories:
        directory_id = "INSTALLFOLDER" if relative == Path(".") else path_id("D", relative)
        ET.SubElement(cleanup, "RemoveFolder", Id="Remove" + directory_id, Directory=directory_id, On="uninstall")
    ET.indent(root, space="  ")
    destination.write_bytes(ET.tostring(root, encoding="utf-8", xml_declaration=True))


# Restores the pinned WiX tools and builds a validated, self-contained MSI.
def build_windows_msi(release_dir: Path, destination: Path) -> Path:
    version = release_version()
    windows_version = msi_version(version)
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    dotnet = require_command("dotnet")
    run([dotnet, "build", INSTALLER_DIR / "cleanup/Operit2DataCleanup.csproj", "-c", "Release",
         "--nologo", "-p:RestoreLockedMode=true"], cwd=INSTALLER_DIR)
    shutil.copy2(INSTALLER_DIR / "build/cleanup/bin/Release/net472/Operit2DataCleanup.exe", release_dir)
    shutil.copy2(INSTALLER_DIR / "README-Windows.txt", release_dir)
    payload = WORK_DIR / "Payload.wxs"
    write_payload(release_dir.resolve(), payload)
    run([dotnet, "build", INSTALLER_DIR / "ui/OperitInstaller.csproj", "-c", "Release",
         "--nologo", "-p:RestoreLockedMode=true"], cwd=INSTALLER_DIR)
    run([dotnet, "build", INSTALLER_DIR / "actions/OperitCleanupActions.csproj", "-c", "Release",
         "--nologo", "-p:RestoreLockedMode=true"], cwd=INSTALLER_DIR)
    run([dotnet, "tool", "restore"], cwd=INSTALLER_DIR)
    wix = [dotnet, "tool", "run", "wix", "--"]
    destination = destination.resolve()
    destination.parent.mkdir(parents=True, exist_ok=True)
    run([*wix, "build", INSTALLER_DIR / "Package.wxs", payload,
         "-arch", "x64",
         "-d", f"MsiVersion={windows_version}", "-d", f"ReleaseVersion={version}",
         "-d", f"AppIcon={FLUTTER_APP_DIR / 'windows/runner/resources/app_icon.ico'}",
         "-d", f"EmbeddedUI={WORK_DIR / 'ui/bin/x64/Release/net472/OperitInstaller.CA.dll'}",
         "-d", f"CleanupActions={WORK_DIR / 'actions/bin/x64/Release/net472/OperitCleanupActions.CA.dll'}",
         "-intermediatefolder", WORK_DIR / "obj",
         "-pdbtype", "none", "-out", destination], cwd=INSTALLER_DIR)
    print(f"Windows installer: {destination} (MSI version {windows_version})", flush=True)
    return destination


# Parses options for packaging an existing Windows release without recompilation.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--release-dir", type=Path, default=DEFAULT_RELEASE_DIR)
    parser.add_argument("--installer-path", type=Path)
    return parser.parse_args()


# Packages the selected release directory using the shared release asset naming.
def main() -> int:
    args = parse_args()
    destination = args.installer_path or DIST_DIR / release_asset_name("app", "windows", "x86_64", "msi")
    build_windows_msi(args.release_dir, destination)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
