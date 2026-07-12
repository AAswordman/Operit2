#!/usr/bin/env python3
import argparse
import json
import os
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import time
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from enum import Enum
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DIST_DIR = SCRIPT_DIR / "dist"
WORK_DIR = SCRIPT_DIR / "work"
SECRETS_DIR = SCRIPT_DIR / "secrets"
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
WEB_ACCESS_BUNDLE_DIR = REPO_ROOT / "apps" / "web_access" / "build" / "bundle"
WEB_ACCESS_EMBEDDED_ASSETS_DIR = FLUTTER_APP_DIR / "assets" / "web_access"
WEB_ACCESS_ASSET_DECLARATION_PREFIX = "    - path: assets/web_access/"
WEB_ACCESS_REQUIRED_FILES = (
    "index.html",
    "main.dart.js",
    "operit_flutter_bridge.js",
    "operit_flutter_bridge_bg.wasm",
    "sql-wasm.js",
    "sql-wasm.wasm",
)
PUBSPEC_PATH = FLUTTER_APP_DIR / "pubspec.yaml"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
CLI_MANIFEST = REPO_ROOT / "apps" / "cli" / "Cargo.toml"
RUNTIME_MANIFEST = REPO_ROOT / "core" / "crates" / "operit-runtime" / "Cargo.toml"
BUILD_SCRIPTS_DIR = REPO_ROOT / "tools" / "build_scripts"
DEFAULT_GITHUB_ENV = SECRETS_DIR / "github.env"
DEFAULT_RELEASE_REPO = "AAswordman/Operit2"
SEMVER_RE = re.compile(
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
    r"(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?"
)
FLUTTER_PLATFORM_VERSION_RE = re.compile(
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)\."
    r"(0|[1-9][0-9]*)"
    r"\+([1-9][0-9]*)"
)


@dataclass(frozen=True)
class SemanticVersion:
    text: str
    major: int
    minor: int
    patch: int
    prerelease: tuple[str, ...]
    build: tuple[str, ...]

    @property
    def core_text(self):
        return f"{self.major}.{self.minor}.{self.patch}"

    @property
    def is_prerelease(self):
        return bool(self.prerelease)


@dataclass(frozen=True)
class FlutterPlatformVersion:
    build_name: str
    build_number: str


@dataclass(frozen=True)
class GitHubAuth:
    token: str
    api_url: str


@dataclass(frozen=True)
class GitHubRepo:
    owner: str
    name: str


class ValueEnum(str, Enum):
    # Returns the raw enum value for command-line interpolation.
    def __str__(self):
        return self.value


class HostPlatform(ValueEnum):
    WINDOWS = "windows"
    LINUX = "linux"
    MACOS = "macos"


class ReleaseProduct(ValueEnum):
    APP = "app"
    CLI = "cli"
    NONE = "none"


class ReleaseScope(ValueEnum):
    CLI = "cli"
    APP = "app"
    FULL = "full"
    NONE = "none"


class CliArchMode(ValueEnum):
    HOST = "host"
    ALL = "all"


class CliWebAssetMode(ValueEnum):
    EMBEDDED = "embedded"
    EXTERNAL = "external"


@dataclass(frozen=True)
class CliBuildTarget:
    platform: HostPlatform
    arch: str
    rust_target: str


CLI_RUST_TARGETS = {
    (HostPlatform.WINDOWS, "x86_64"): "x86_64-pc-windows-msvc",
    (HostPlatform.WINDOWS, "aarch64"): "aarch64-pc-windows-msvc",
    (HostPlatform.LINUX, "x86_64"): "x86_64-unknown-linux-musl",
    (HostPlatform.LINUX, "aarch64"): "aarch64-unknown-linux-musl",
    (HostPlatform.MACOS, "x86_64"): "x86_64-apple-darwin",
    (HostPlatform.MACOS, "aarch64"): "aarch64-apple-darwin",
}
CLI_RELEASE_ARCHES = ("x86_64", "aarch64")
CLI_WEB_ASSET_MODES = tuple(item.value for item in CliWebAssetMode)
CLI_ALL_TARGETS = tuple(
    CliBuildTarget(platform=plat, arch=arch, rust_target=CLI_RUST_TARGETS[(plat, arch)])
    for plat in HostPlatform
    for arch in CLI_RELEASE_ARCHES
)


def run(command, cwd=REPO_ROOT, env=None):
    print(">> " + " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=cwd, env=env, check=True)


def run_capture(command, cwd=REPO_ROOT):
    return subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        check=True,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    ).stdout.strip()


def require_command(name):
    if shutil.which(name) is None:
        raise RuntimeError(f"Required command not found: {name}")


def path_contains(parent, child):
    parent = parent.resolve()
    child = child.resolve()
    try:
        child.relative_to(parent)
        return True
    except ValueError:
        return False


def ensure_cwd_outside(path):
    cwd = Path.cwd()
    if path_contains(path, cwd):
        os.chdir(SCRIPT_DIR)


def close_release_processes(paths):
    if platform.system().lower() != "windows":
        return

    roots = [str(path.resolve()) for path in paths]
    roots_json = json.dumps(roots)
    excluded_pids_json = json.dumps([os.getpid()])
    script = f"""
$ErrorActionPreference = "Stop"
$Roots = ConvertFrom-Json @'
{roots_json}
'@
$ExcludedPids = @(ConvertFrom-Json @'
{excluded_pids_json}
'@) + $PID
function Normalize-OperitReleasePath([string]$Path) {{
    return [System.IO.Path]::GetFullPath($Path).TrimEnd('\\', '/')
}}
function Test-OperitReleasePathUnderRoot([string]$Path) {{
    if ([string]::IsNullOrWhiteSpace($Path)) {{ return $false }}
    $Candidate = Normalize-OperitReleasePath $Path
    foreach ($Root in $Roots) {{
        $NormalizedRoot = Normalize-OperitReleasePath ([string]$Root)
        if ([string]::Equals($Candidate, $NormalizedRoot, [System.StringComparison]::OrdinalIgnoreCase)) {{
            return $true
        }}
        if ($Candidate.StartsWith($NormalizedRoot + '\\', [System.StringComparison]::OrdinalIgnoreCase)) {{
            return $true
        }}
    }}
    return $false
}}
function Test-OperitReleaseCommandLineReferencesRoot([string]$CommandLine) {{
    if ([string]::IsNullOrWhiteSpace($CommandLine)) {{ return $false }}
    foreach ($Root in $Roots) {{
        $NormalizedRoot = Normalize-OperitReleasePath ([string]$Root)
        if ($CommandLine.IndexOf($NormalizedRoot, [System.StringComparison]::OrdinalIgnoreCase) -ge 0) {{
            return $true
        }}
    }}
    return $false
}}
$Processes = Get-CimInstance Win32_Process | Where-Object {{
    ($ExcludedPids -notcontains [int]$_.ProcessId) -and (
        (Test-OperitReleasePathUnderRoot $_.ExecutablePath) -or
        (Test-OperitReleaseCommandLineReferencesRoot $_.CommandLine)
    )
}}
foreach ($Process in $Processes) {{
    Write-Output ("closing release process pid={{0}} exe={{1}}" -f $Process.ProcessId, $Process.ExecutablePath)
    Stop-Process -Id $Process.ProcessId -Force -ErrorAction Stop
    Wait-Process -Id $Process.ProcessId -Timeout 5 -ErrorAction SilentlyContinue
}}
"""
    result = subprocess.run(
        [
            "powershell.exe",
            "-NoProfile",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            script,
        ],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )
    if result.stdout.strip():
        print(result.stdout.strip(), flush=True)
    if result.returncode != 0:
        print(
            f"warning: failed to close release processes: {result.stderr.strip()}",
            file=sys.stderr,
            flush=True,
        )


def load_env_file(path):
    if not path.exists():
        raise RuntimeError(f"GitHub env file not found: {path}")
    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), start=1):
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if "=" not in stripped:
            raise RuntimeError(f"Invalid env line {line_number} in {path}")
        key, value = stripped.split("=", 1)
        key = key.strip()
        value = unquote_env_value(value.strip())
        if not key:
            raise RuntimeError(f"Empty env key at line {line_number} in {path}")
        os.environ[key] = value


def unquote_env_value(value):
    if len(value) >= 2 and value[0] == value[-1] and value[0] in ("'", '"'):
        return value[1:-1]
    return value


def github_auth():
    token = os.environ.get("GITHUB_TOKEN", "").strip()
    if not token:
        raise RuntimeError("GITHUB_TOKEN is empty")
    api_url = os.environ.get("GITHUB_API_URL", "").strip()
    if not api_url:
        raise RuntimeError("GITHUB_API_URL is empty")
    return GitHubAuth(token=token, api_url=api_url.rstrip("/"))


def reset_dir(path):
    ensure_cwd_outside(path)
    path.mkdir(parents=True, exist_ok=True)
    for child in list(path.iterdir()):
        remove_release_path(child)


# Verifies the shared Web Access bundle has been generated.
def require_web_access_bundle():
    missing_files = [
        name for name in WEB_ACCESS_REQUIRED_FILES if not (WEB_ACCESS_BUNDLE_DIR / name).is_file()
    ]
    if missing_files:
        raise RuntimeError(
            "Web Access bundle is incomplete at "
            f"{WEB_ACCESS_BUNDLE_DIR}; missing: {', '.join(missing_files)}. "
            "Run tools/build_scripts/build_flutter_web_access.py before building this product."
        )

    if WEB_ACCESS_EMBEDDED_ASSETS_DIR.is_symlink():
        WEB_ACCESS_EMBEDDED_ASSETS_DIR.unlink()
    elif WEB_ACCESS_EMBEDDED_ASSETS_DIR.exists():
        shutil.rmtree(WEB_ACCESS_EMBEDDED_ASSETS_DIR)
    WEB_ACCESS_EMBEDDED_ASSETS_DIR.parent.mkdir(parents=True, exist_ok=True)
    shutil.copytree(WEB_ACCESS_BUNDLE_DIR, WEB_ACCESS_EMBEDDED_ASSETS_DIR)

    declared_paths = []
    for line in PUBSPEC_PATH.read_text(encoding="utf-8").splitlines():
        if line.startswith(WEB_ACCESS_ASSET_DECLARATION_PREFIX):
            relative_path = line.removeprefix("    - path: ").rstrip("/")
            declared_paths.append(FLUTTER_APP_DIR / Path(relative_path))
    if not declared_paths:
        raise RuntimeError(f"No Web Access asset directories are declared in {PUBSPEC_PATH}")

    missing_directories = [path for path in declared_paths if not path.is_dir()]
    if missing_directories:
        raise RuntimeError(
            "Embedded Web Access asset directories declared in pubspec.yaml are missing: "
            + ", ".join(str(path) for path in missing_directories)
        )


def remove_release_path(path):
    for attempt in range(3):
        try:
            if path.is_dir() and not path.is_symlink():
                shutil.rmtree(path)
            else:
                path.unlink()
            return
        except FileNotFoundError:
            return
        except PermissionError:
            if attempt == 0:
                close_release_processes([path])
            time.sleep(0.5)

    if path.is_dir() and not path.is_symlink():
        reset_dir(path)
        try:
            path.rmdir()
            return
        except PermissionError:
            print(f"warning: keeping locked release directory: {path}", file=sys.stderr)
            return

    if path.exists():
        raise PermissionError(f"Failed to remove locked release path: {path}")


def read_properties(path):
    result = {}
    if not path.exists():
        return result
    for line in path.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        index = line.find("=")
        if index < 1:
            continue
        result[line[:index]] = line[index + 1 :]
    return result


def write_properties(path, values):
    lines = [f"{key}={value}" for key, value in values.items()]
    path.write_text("\n".join(lines) + "\n", encoding="utf-8")


def java_properties_value(value):
    return str(value).replace("\\", "\\\\").replace(":", "\\:")


def pubspec_version():
    for line in PUBSPEC_PATH.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("version:"):
            return stripped.split(":", 1)[1].strip()
    raise RuntimeError("pubspec.yaml does not define version")


def write_pubspec_version(value):
    lines = PUBSPEC_PATH.read_text(encoding="utf-8").splitlines()
    version_line_index = None
    for index, line in enumerate(lines):
        if line.startswith("version:"):
            if version_line_index is not None:
                raise RuntimeError("pubspec.yaml defines version more than once")
            version_line_index = index
    if version_line_index is None:
        raise RuntimeError("pubspec.yaml does not define version")
    lines[version_line_index] = f"version: {value}"
    PUBSPEC_PATH.write_text("\n".join(lines) + "\n", encoding="utf-8")


def cargo_package_version(manifest):
    with manifest.open("rb") as handle:
        data = tomllib.load(handle)
    try:
        version = data["package"]["version"]
    except KeyError as error:
        raise RuntimeError(f"{manifest} does not define package.version") from error
    if not isinstance(version, str):
        raise RuntimeError(f"{manifest} package.version must be a string")
    return version


def parse_semver(value, name, allow_tag=False):
    text = value.strip()
    if allow_tag:
        if text.startswith("v"):
            text = text[1:]
    elif text.startswith("v"):
        raise RuntimeError(f"{name} must not start with v: {value}")

    match = SEMVER_RE.fullmatch(text)
    if match is None:
        raise RuntimeError(f"{name} must use major.minor.patch[-prerelease][+build]: {value}")

    prerelease = tuple(match.group(4).split(".")) if match.group(4) else ()
    for identifier in prerelease:
        if identifier.isdecimal() and len(identifier) > 1 and identifier.startswith("0"):
            raise RuntimeError(
                f"{name} prerelease numeric identifier has a leading zero: {identifier}"
            )

    build = tuple(match.group(5).split(".")) if match.group(5) else ()
    return SemanticVersion(
        text=text,
        major=int(match.group(1)),
        minor=int(match.group(2)),
        patch=int(match.group(3)),
        prerelease=prerelease,
        build=build,
    )


def validate_product_release_version(version, name):
    if version.build:
        raise RuntimeError(
            f"{name} must not include build metadata: {version.text}. "
            "Use prerelease identifiers for preview releases."
        )


def release_version():
    cli = parse_semver(
        cargo_package_version(CLI_MANIFEST),
        f"{CLI_MANIFEST.relative_to(REPO_ROOT)} package.version",
    )
    validate_product_release_version(
        cli,
        f"{CLI_MANIFEST.relative_to(REPO_ROOT)} package.version",
    )
    runtime = parse_semver(
        cargo_package_version(RUNTIME_MANIFEST),
        f"{RUNTIME_MANIFEST.relative_to(REPO_ROOT)} package.version",
    )
    validate_product_release_version(
        runtime,
        f"{RUNTIME_MANIFEST.relative_to(REPO_ROOT)} package.version",
    )
    if cli.text != runtime.text:
        raise RuntimeError(
            "CLI and runtime release versions differ: "
            f"{CLI_MANIFEST.relative_to(REPO_ROOT)}={cli.text}, "
            f"{RUNTIME_MANIFEST.relative_to(REPO_ROOT)}={runtime.text}"
        )
    return cli


def flutter_platform_version():
    raw = pubspec_version()
    match = FLUTTER_PLATFORM_VERSION_RE.fullmatch(raw)
    if match is None:
        raise RuntimeError(
            "apps/flutter/app/pubspec.yaml version must use major.minor.patch+buildNumber"
        )
    return FlutterPlatformVersion(
        build_name=f"{match.group(1)}.{match.group(2)}.{match.group(3)}",
        build_number=match.group(4),
    )


def validate_flutter_platform_version(platform_version, version):
    expected = f"{version.major}.{version.minor}.{version.patch}"
    if platform_version.build_name != expected:
        raise RuntimeError(
            "apps/flutter/app/pubspec.yaml version build name must match release version "
            f"major.minor.patch: expected {expected}, got {platform_version.build_name}"
        )


def increment_flutter_platform_build_number(platform_version):
    build_number = str(int(platform_version.build_number) + 1)
    write_pubspec_version(f"{platform_version.build_name}+{build_number}")
    return FlutterPlatformVersion(
        build_name=platform_version.build_name,
        build_number=build_number,
    )


def ensure_android_signing():
    signing_properties = SECRETS_DIR / "android-signing.properties"
    if not signing_properties.exists():
        raise RuntimeError(f"Android signing properties not found: {signing_properties}")

    signing = read_properties(signing_properties)
    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    for key in (
        "RELEASE_STORE_FILE",
        "RELEASE_STORE_PASSWORD",
        "RELEASE_KEY_ALIAS",
        "RELEASE_KEY_PASSWORD",
    ):
        if key not in signing:
            raise RuntimeError(f"Android signing property missing from {signing_properties}: {key}")
        local[key] = signing[key]
    write_properties(ANDROID_LOCAL_PROPERTIES, local)


def copy_required_file(source, destination):
    if not source.exists():
        raise RuntimeError(f"Expected build output not found: {source}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, destination)


def compress_zip(source_dir, destination):
    if not source_dir.exists():
        raise RuntimeError(f"Expected build directory not found: {source_dir}")
    if destination.exists():
        destination.unlink()
    shutil.make_archive(str(destination.with_suffix("")), "zip", source_dir)
    produced = destination.with_suffix("")
    produced = produced.with_suffix(".zip")
    if produced != destination:
        produced.replace(destination)


def compress_tar_gz(source_dir, destination):
    if not source_dir.exists():
        raise RuntimeError(f"Expected build directory not found: {source_dir}")
    if destination.exists():
        destination.unlink()
    with tarfile.open(destination, "w:gz") as archive:
        for child in sorted(source_dir.iterdir(), key=lambda path: path.name):
            archive.add(child, arcname=child.name)


def write_text_file(path, content):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(content, encoding="utf-8", newline="\n")


def write_windows_cli_installer_files(package_dir):
    write_text_file(
        package_dir / "install.bat",
        r'''@echo off
setlocal

set "SOURCE=%~dp0operit2.exe"

if not exist "%SOURCE%" (
  echo operit2.exe not found next to install.bat 1>&2
  exit /b 1
)

"%SOURCE%" cli install --source "%SOURCE%"
exit /b %ERRORLEVEL%
''',
    )
    write_text_file(
        package_dir / "uninstall.bat",
        r'''@echo off
setlocal

set "SOURCE=%~dp0operit2.exe"

if not exist "%SOURCE%" (
  echo operit2.exe not found next to uninstall.bat 1>&2
  exit /b 1
)

"%SOURCE%" cli uninstall
exit /b %ERRORLEVEL%
''',
    )
    write_text_file(
        package_dir / "README.txt",
        """Operit2 CLI for Windows

Install:
  install.bat

Uninstall:
  uninstall.bat

Command after install:
  operit
  operit2
""",
    )


def write_unix_cli_installer_files(package_dir):
    write_text_file(
        package_dir / "install.sh",
        """#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli install --source "$source_file"
""",
    )
    write_text_file(
        package_dir / "uninstall.sh",
        """#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli uninstall
""",
    )
    write_text_file(
        package_dir / "README.txt",
        """Operit2 CLI for Linux/macOS

Install:
  chmod +x install.sh
  ./install.sh

Uninstall:
  chmod +x uninstall.sh
  ./uninstall.sh

Command after install:
  operit
  operit2
""",
    )


def write_cli_installer_files(package_dir, target_platform):
    if target_platform == HostPlatform.WINDOWS:
        write_windows_cli_installer_files(package_dir)
    else:
        write_unix_cli_installer_files(package_dir)


def host_platform():
    name = platform.system().lower()
    if name == HostPlatform.WINDOWS.value:
        return HostPlatform.WINDOWS
    if name == HostPlatform.LINUX.value:
        return HostPlatform.LINUX
    if name == "darwin":
        return HostPlatform.MACOS
    raise RuntimeError(f"Unsupported host OS: {name}")


def host_arch():
    machine = platform.machine().lower()
    if machine in ("amd64", "x86_64"):
        return "x86_64"
    if machine in ("arm64", "aarch64"):
        return "aarch64"
    raise RuntimeError(f"Unsupported host architecture: {machine}")


def cli_build_targets_for_platform(target_platform):
    return tuple(
        CliBuildTarget(
            platform=target_platform,
            arch=arch,
            rust_target=CLI_RUST_TARGETS[(target_platform, arch)],
        )
        for arch in CLI_RELEASE_ARCHES
    )


def cli_binary_name(target_platform):
    return "operit2.exe" if target_platform == HostPlatform.WINDOWS else "operit2"


def cli_archive_extension(target_platform):
    return "zip" if target_platform == HostPlatform.WINDOWS else "tar.gz"


# Returns the package name suffix for the selected Web Access asset mode.
def cli_web_asset_package_suffix(web_assets):
    web_assets = CliWebAssetMode(web_assets)
    if web_assets == CliWebAssetMode.EMBEDDED:
        return ""
    if web_assets == CliWebAssetMode.EXTERNAL:
        return "-external-web"
    raise RuntimeError(f"Unsupported CLI Web Access asset mode: {web_assets}")


# Returns the cargo feature arguments for the selected Web Access asset mode.
def cli_web_asset_cargo_args(web_assets):
    web_assets = CliWebAssetMode(web_assets)
    if web_assets == CliWebAssetMode.EMBEDDED:
        return []
    if web_assets == CliWebAssetMode.EXTERNAL:
        return ["--no-default-features"]
    raise RuntimeError(f"Unsupported CLI Web Access asset mode: {web_assets}")


# Returns the working directory for one packaged CLI target.
def cli_package_dir(target, web_assets=CliWebAssetMode.EMBEDDED):
    return WORK_DIR / f"cli-{target.platform}-{target.arch}{cli_web_asset_package_suffix(web_assets)}"


# Returns the release archive path for one packaged CLI target.
def cli_package_path(target, web_assets=CliWebAssetMode.EMBEDDED):
    return DIST_DIR / (
        f"operit2-cli-{target.platform}-{target.arch}"
        f"{cli_web_asset_package_suffix(web_assets)}.{cli_archive_extension(target.platform)}"
    )


def cli_target_binary_path(target):
    return (
        REPO_ROOT
        / "apps"
        / "cli"
        / "target"
        / target.rust_target
        / "release"
        / cli_binary_name(target.platform)
    )


def parse_github_repo(repo):
    value = repo.strip()
    parts = value.split("/")
    if len(parts) != 2 or not parts[0] or not parts[1]:
        raise RuntimeError(f"GitHub repo must use owner/name: {repo}")
    return GitHubRepo(owner=parts[0], name=parts[1])


def github_api_url(auth, repo, path):
    owner = urllib.parse.quote(repo.owner, safe="")
    name = urllib.parse.quote(repo.name, safe="")
    return f"{auth.api_url}/repos/{owner}/{name}{path}"


def github_request(method, url, auth, payload=None, headers=None, expected_statuses=(200,)):
    request_headers = {
        "Accept": "application/vnd.github+json",
        "Authorization": f"Bearer {auth.token}",
        "User-Agent": "Operit2-Release",
        "X-GitHub-Api-Version": "2022-11-28",
    }
    if headers:
        request_headers.update(headers)

    body = None
    if payload is not None:
        if isinstance(payload, bytes):
            body = payload
        else:
            body = json.dumps(payload).encode("utf-8")
            request_headers["Content-Type"] = "application/json"

    request = urllib.request.Request(url, data=body, headers=request_headers, method=method)
    try:
        with urllib.request.urlopen(request) as response:
            status = response.status
            content = response.read()
    except urllib.error.HTTPError as error:
        status = error.code
        content = error.read()

    if status not in expected_statuses:
        message = content.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"GitHub API {method} {url} returned HTTP {status}: {message}")

    if not content:
        return status, {}
    return status, json.loads(content.decode("utf-8"))


def is_windows_host():
    return platform.system().lower() == HostPlatform.WINDOWS.value


def windows_path_to_wsl(path):
    resolved = Path(path).resolve()
    drive = resolved.drive.rstrip(":").lower()
    if not drive:
        raise RuntimeError(f"Cannot convert path to WSL path: {resolved}")
    parts = resolved.parts[1:]
    return "/mnt/" + drive + "/" + "/".join(part.replace("\\", "/") for part in parts)


def wsl_run(distro, script):
    script = "export PATH=\"$HOME/.cargo/bin:$HOME/.pub-cache/bin:$HOME/.local/flutter/bin:$PATH\"\n" + script
    script = script.replace("\r\n", "\n").replace("\r", "\n")
    command = ["wsl.exe"]
    if distro:
        command.extend(["-d", distro])
    command.extend(["bash", "-s"])
    print(">> " + " ".join(command), flush=True)
    subprocess.run(command, cwd=REPO_ROOT, input=script.encode("utf-8"), check=True)


def wsl_check_command(distro, name):
    command = ["wsl.exe"]
    if distro:
        command.extend(["-d", distro])
    command.extend(["bash", "-s"])
    script = (
        "export PATH=\"$HOME/.cargo/bin:$HOME/.pub-cache/bin:$HOME/.local/flutter/bin:$PATH\"\n"
        f"command -v {shlex.quote(name)} >/dev/null\n"
    )
    script = script.replace("\r\n", "\n").replace("\r", "\n")
    return subprocess.run(command, cwd=REPO_ROOT, input=script.encode("utf-8")).returncode == 0


def wsl_single_quote(value):
    return "'" + value.replace("'", "'\"'\"'") + "'"


def build_wsl_linux_app(distro, build_name, build_number):
    require_web_access_bundle()
    if not wsl_check_command(distro, "dart"):
        raise RuntimeError(
            "WSL Linux app build requires Dart on PATH to start FVM."
        )
    if not wsl_check_command(distro, "fvm"):
        raise RuntimeError("WSL Linux app build requires FVM on PATH.")
    repo = shlex.quote(windows_path_to_wsl(REPO_ROOT))
    dist = shlex.quote(windows_path_to_wsl(DIST_DIR))
    build_name_arg = shlex.quote(build_name)
    build_number_arg = shlex.quote(build_number)
    script = f"""
set -e
cd {repo}
cd apps/flutter/app
fvm install --skip-pub-get
fvm dart pub get --enforce-lockfile
rm -rf build/linux/x64/release
fvm flutter build linux --release --no-pub --build-name {build_name_arg} --build-number {build_number_arg}
cd {repo}
mkdir -p {dist}
rm -f {dist}/operit2-app-linux-x86_64.tar.gz
tar -czf {dist}/operit2-app-linux-x86_64.tar.gz -C apps/flutter/app/build/linux/x64/release/bundle .
"""
    wsl_run(distro, script)


def build_wsl_linux_cli(distro):
    if not wsl_check_command(distro, "cargo"):
        raise RuntimeError("WSL Linux CLI build requires cargo inside WSL.")
    wsl_build_cli_target(distro, CliBuildTarget(HostPlatform.LINUX, "x86_64", CLI_RUST_TARGETS[(HostPlatform.LINUX, "x86_64")]))


def wsl_build_cli_target(distro, target):
    require_web_access_bundle()
    dist = shlex.quote(windows_path_to_wsl(DIST_DIR))
    work = shlex.quote(windows_path_to_wsl(cli_package_dir(target)))
    package = shlex.quote(windows_path_to_wsl(cli_package_path(target)))
    binary_name = cli_binary_name(target.platform)
    repo = shlex.quote(windows_path_to_wsl(REPO_ROOT))
    triple = target.rust_target
    if target.rust_target == "x86_64-unknown-linux-musl":
        dependency_check = """
command -v musl-gcc >/dev/null 2>&1 || {
    echo "Missing WSL dependency: musl-gcc" >&2
    echo "Fedora: sudo dnf install -y musl-gcc musl-devel musl-libc-static" >&2
    exit 1
}
export CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc
export CC_x86_64_unknown_linux_musl=musl-gcc
"""
    elif target.rust_target == "aarch64-unknown-linux-musl":
        dependency_check = """
command -v aarch64-linux-musl-gcc >/dev/null 2>&1 || {
    echo "Missing WSL dependency: aarch64-linux-musl-gcc" >&2
    echo "Install an aarch64 musl cross C compiler and put aarch64-linux-musl-gcc on PATH." >&2
    exit 1
}
export CARGO_TARGET_AARCH64_UNKNOWN_LINUX_MUSL_LINKER=rust-lld
export CC_aarch64_unknown_linux_musl=aarch64-linux-musl-gcc
"""
    else:
        dependency_check = ""
    script = f"""
set -e
cd {repo}
{dependency_check}
rustup target add {triple}
cargo build --release --target {triple} --manifest-path apps/cli/Cargo.toml
rm -rf {work}
mkdir -p {work} {dist}
cp apps/cli/target/{triple}/release/{binary_name} {work}/{binary_name}
cat > {work}/install.sh <<'WEOF'
#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli install --source "$source_file"
WEOF
cat > {work}/uninstall.sh <<'WEOF'
#!/usr/bin/env sh
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
source_file="$script_dir/operit2"

test -f "$source_file"
chmod +x "$source_file"
"$source_file" cli uninstall
WEOF
cat > {work}/README.txt <<'WEOF'
Operit2 CLI for Linux

Install:
  chmod +x install.sh
  ./install.sh

Uninstall:
  chmod +x uninstall.sh
  ./uninstall.sh

Command after install:
  operit
  operit2
WEOF
chmod +x {work}/{binary_name} {work}/install.sh {work}/uninstall.sh
rm -f {package}
tar -czf {package} -C {work} .
"""
    wsl_run(distro, script)


def build_wsl_linux(products, distro, build_name, build_number, cli_arches=CliArchMode.HOST):
    cli_arches = CliArchMode(cli_arches)
    if not is_windows_host():
        return
    if shutil.which("wsl.exe") is None:
        raise RuntimeError("wsl.exe not found")
    if ReleaseProduct.APP in products:
        build_wsl_linux_app(distro, build_name, build_number)
    if ReleaseProduct.CLI in products:
        if cli_arches == CliArchMode.ALL:
            wsl_build_cli_target(distro, CliBuildTarget(HostPlatform.LINUX, "x86_64", CLI_RUST_TARGETS[(HostPlatform.LINUX, "x86_64")]))
            wsl_build_cli_target(distro, CliBuildTarget(HostPlatform.LINUX, "aarch64", CLI_RUST_TARGETS[(HostPlatform.LINUX, "aarch64")]))
        else:
            build_wsl_linux_cli(distro)


def build_android_app(build_name, build_number):
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_flutter_android.py",
            "--build-name",
            build_name,
            "--build-number",
            build_number,
            "--enforce-lockfile",
        ],
    )


# Builds the OpenHarmony Flutter application package.
def build_ohos_app(build_name, build_number):
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_flutter_ohos.py",
            "--build-name",
            build_name,
            "--build-number",
            build_number,
            "--enforce-lockfile",
            "--output",
            DIST_DIR / "operit2-app-ohos-arm64.hap",
        ],
    )


# Builds the shared Web Access bundle consumed by app and CLI packages.
def build_web_access_bundle():
    run([sys.executable, BUILD_SCRIPTS_DIR / "build_flutter_web_access.py"])


def build_host_app(build_name, build_number):
    current_platform = host_platform()
    current_arch = host_arch()

    if current_platform == HostPlatform.WINDOWS:
        run(
            [
                sys.executable,
                BUILD_SCRIPTS_DIR / "build_flutter_windows.py",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
                "--enforce-lockfile",
                "--archive-path",
                DIST_DIR / f"operit2-app-windows-{current_arch}.zip",
            ]
        )
        return

    if current_platform == HostPlatform.LINUX:
        run(
            [
                sys.executable,
                BUILD_SCRIPTS_DIR / "build_flutter_linux.py",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
                "--enforce-lockfile",
                "--archive-path",
                DIST_DIR / f"operit2-app-linux-{current_arch}.tar.gz",
            ]
        )
        return

    if current_platform == HostPlatform.MACOS:
        run(
            [
                sys.executable,
                BUILD_SCRIPTS_DIR / "build_flutter_macos.py",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
                "--enforce-lockfile",
                "--archive-path",
                DIST_DIR / f"operit2-app-macos-{current_arch}.zip",
            ]
        )
        return

    raise RuntimeError(f"Unsupported host app platform: {current_platform}")


def vs_installation_path():
    import subprocess
    vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:\\Program Files (x86)")) / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
    if not vswhere.exists():
        return None
    try:
        result = subprocess.run([str(vswhere), "-latest", "-property", "installationPath"], capture_output=True, text=True, check=True)
        path = result.stdout.strip()
        return Path(path) if path else None
    except (subprocess.CalledProcessError, OSError):
        return None


def vs_dev_env(vcvars_path, arch):
    import subprocess
    import tempfile
    import os

    # Write a temp batch file to avoid cmd.exe quoting issues with paths containing spaces
    tmp = tempfile.NamedTemporaryFile(
        mode="w", suffix=".bat", delete=False, dir=SCRIPT_DIR
    )
    try:
        tmp.write(f'@call "{vcvars_path}" {arch}\n')
        tmp.write('@set\n')
        tmp.close()

        result = subprocess.run(
            ["cmd.exe", "/c", tmp.name],
            capture_output=True, text=True,
        )
    finally:
        os.unlink(tmp.name)

    if result.returncode != 0:
        raise RuntimeError(f"vcvarsall.bat failed for {arch}: {result.stderr.strip()}")

    env = {}
    for line in result.stdout.splitlines():
        if "=" in line:
            key, _, value = line.partition("=")
            env[key] = value
    canonical_env = {}
    for key in ("INCLUDE", "LIB", "PATH"):
        actual_key = next(
            (name for name in env.keys() if name.upper() == key),
            None,
        )
        if actual_key is None:
            raise RuntimeError(f"vcvarsall.bat did not produce {key} for {arch}")
        canonical_env[key] = env[actual_key]
    return canonical_env


def build_cli_target(target, use_default_target=False):
    require_command("cargo")
    require_web_access_bundle()
    binary_name = cli_binary_name(target.platform)
    package_dir = cli_package_dir(target)
    package_path = cli_package_path(target)

    if use_default_target:
        run(["cargo", "build", "--release", "--manifest-path", CLI_MANIFEST])
        binary_source = REPO_ROOT / "apps" / "cli" / "target" / "release" / binary_name
    else:
        build_env = {**os.environ}
        if target.platform == HostPlatform.WINDOWS and target.arch == "aarch64":
            vs_path = vs_installation_path()
            if not vs_path:
                raise RuntimeError("Visual Studio installation not found for Windows aarch64 CLI build")
            vcvars = vs_path / "VC" / "Auxiliary" / "Build" / "vcvarsall.bat"
            if not vcvars.exists():
                raise RuntimeError(f"vcvarsall.bat not found: {vcvars}")
            merged = vs_dev_env(vcvars, "x64_arm64")
            build_env.update(merged)
            llvm_bin = Path("C:/Program Files/LLVM/bin")
            clang = llvm_bin / "clang.exe"
            if not clang.exists():
                raise RuntimeError(f"LLVM clang not found for Windows aarch64 CLI build: {clang}")
            path = build_env.get("PATH", "")
            build_env["PATH"] = f"{llvm_bin};{path}"
        run(["cargo", "build", "--release", "--target", target.rust_target, "--manifest-path", CLI_MANIFEST], env=build_env)
        binary_source = cli_target_binary_path(target)

    reset_dir(package_dir)
    copy_required_file(binary_source, package_dir / binary_name)
    write_cli_installer_files(package_dir, target.platform)

    if target.platform == HostPlatform.WINDOWS:
        compress_zip(package_dir, package_path)
    else:
        os.chmod(package_dir / binary_name, 0o755)
        os.chmod(package_dir / "install.sh", 0o755)
        os.chmod(package_dir / "uninstall.sh", 0o755)
        compress_tar_gz(package_dir, package_path)


def build_host_cli(cli_arches="host"):
    run(
        [
            sys.executable,
            BUILD_SCRIPTS_DIR / "build_cli_current.py",
            "--arches",
            cli_arches,
        ]
    )


def publish_release(tag, repo_value, draft, prerelease, auth):
    assets = sorted(path for path in DIST_DIR.iterdir() if path.is_file())
    if not assets:
        raise RuntimeError("No release assets were produced")

    repo = parse_github_repo(repo_value)
    tag_path = "/releases/tags/" + urllib.parse.quote(tag, safe="")
    status, release = github_request(
        "GET",
        github_api_url(auth, repo, tag_path),
        auth,
        expected_statuses=(200, 404),
    )
    release_exists = status == 200

    if release_exists:
        if bool(release["prerelease"]) != prerelease:
            raise RuntimeError(f"Existing GitHub release {tag} prerelease flag does not match")
    else:
        _, release = github_request(
            "POST",
            github_api_url(auth, repo, "/releases"),
            auth,
            payload={
                "tag_name": tag,
                "name": tag,
                "body": f"Operit2 {tag}",
                "draft": draft,
                "prerelease": prerelease,
            },
            expected_statuses=(201,),
        )

    upload_url = release["upload_url"].split("{", 1)[0]
    existing_assets = release.get("assets", [])
    if not isinstance(existing_assets, list):
        raise RuntimeError(f"GitHub release {tag} assets field is invalid")

    for asset in assets:
        delete_existing_release_asset(asset.name, existing_assets, auth, repo)
        upload_release_asset(upload_url, asset, auth)


def verify_github_publish_access(repo_value, auth):
    repo = parse_github_repo(repo_value)
    try:
        github_request(
            "GET",
            github_api_url(auth, repo, ""),
            auth,
            expected_statuses=(200,),
        )
    except RuntimeError as error:
        raise RuntimeError(
            "GitHub publish preflight failed. Check GITHUB_TOKEN and repository access for "
            f"{repo.owner}/{repo.name}: {error}"
        ) from error


def delete_existing_release_asset(asset_name, existing_assets, auth, repo):
    for existing_asset in existing_assets:
        if existing_asset.get("name") != asset_name:
            continue
        asset_id = existing_asset.get("id")
        if not isinstance(asset_id, int):
            raise RuntimeError(f"GitHub asset id is invalid for {asset_name}")
        github_request(
            "DELETE",
            github_api_url(auth, repo, f"/releases/assets/{asset_id}"),
            auth,
            expected_statuses=(204,),
        )


def upload_release_asset(upload_url, asset, auth):
    query = urllib.parse.urlencode({"name": asset.name})
    url = f"{upload_url}?{query}"
    print(f">> upload {asset.name}", flush=True)
    github_request(
        "POST",
        url,
        auth,
        payload=asset.read_bytes(),
        headers={"Content-Type": "application/octet-stream"},
        expected_statuses=(201,),
    )


def products_for_scope(scope, explicit_products):
    scope = ReleaseScope(scope)
    if explicit_products is not None:
        if scope != ReleaseScope.FULL:
            raise RuntimeError("--products cannot be combined with --scope app, --scope cli, or --scope none")
        products = {ReleaseProduct(product) for product in explicit_products}
    else:
        products = {
            ReleaseScope.FULL: {ReleaseProduct.APP, ReleaseProduct.CLI},
            ReleaseScope.APP: {ReleaseProduct.APP},
            ReleaseScope.CLI: {ReleaseProduct.CLI},
            ReleaseScope.NONE: {ReleaseProduct.NONE},
        }[scope]

    if ReleaseProduct.NONE in products and len(products) > 1:
        raise RuntimeError("--products none cannot be combined with app or cli")
    return products


def main():
    parser = argparse.ArgumentParser(description="Build and publish Operit2 release assets.")
    parser.add_argument("--tag", default="")
    parser.add_argument("--repo", default=DEFAULT_RELEASE_REPO)
    parser.add_argument("--github-env", default=str(DEFAULT_GITHUB_ENV))
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--draft", action="store_true")
    parser.add_argument(
        "--prerelease",
        action="store_true",
        help="Accepted for prerelease versions. The GitHub flag is derived from the release version.",
    )
    parser.add_argument("--scope", default=ReleaseScope.FULL.value, choices=[item.value for item in ReleaseScope])
    parser.add_argument("--products", nargs="+", choices=[item.value for item in ReleaseProduct])
    parser.add_argument("--wsl-distro", default="FedoraLinux-43")
    parser.add_argument("--no-wsl", action="store_true")
    parser.add_argument("--cli-arches", default=CliArchMode.HOST.value, choices=[item.value for item in CliArchMode],
                        help="CLI target architectures: host (current only) or all (all desktop arches)")
    args = parser.parse_args()

    version = release_version()
    platform_version = flutter_platform_version()
    validate_flutter_platform_version(platform_version, version)
    tag = args.tag or f"v{version.text}"
    tag_version = parse_semver(tag, "--tag", allow_tag=True)
    if tag_version.text != version.text:
        raise RuntimeError(f"--tag {tag} does not match release version {version.text}")
    if args.prerelease and not version.is_prerelease:
        raise RuntimeError("--prerelease was set for a stable release version")
    products = products_for_scope(args.scope, args.products)

    publish_auth = None
    if not args.build_only:
        load_env_file(Path(args.github_env))
        publish_auth = github_auth()
        verify_github_publish_access(args.repo, publish_auth)

    reset_dir(DIST_DIR)
    reset_dir(WORK_DIR)

    if ReleaseProduct.APP in products or ReleaseProduct.CLI in products:
        build_web_access_bundle()

    if ReleaseProduct.APP in products:
        build_android_app(platform_version.build_name, platform_version.build_number)
        build_ohos_app(platform_version.build_name, platform_version.build_number)
        build_host_app(platform_version.build_name, platform_version.build_number)

    if ReleaseProduct.CLI in products:
        build_host_cli(args.cli_arches)

    if not args.no_wsl and ReleaseProduct.NONE not in products:
        build_wsl_linux(
            products,
            args.wsl_distro,
            platform_version.build_name,
            platform_version.build_number,
            args.cli_arches,
        )

    next_platform_version = None
    if ReleaseProduct.APP in products and not args.build_only:
        next_platform_version = increment_flutter_platform_build_number(platform_version)

    print(f"\nRelease version: {version.text}")
    print(f"Flutter platform version: {platform_version.build_name}+{platform_version.build_number}")
    if next_platform_version is not None:
        print(
            "Next Flutter platform version: "
            f"{next_platform_version.build_name}+{next_platform_version.build_number}"
        )
    print("\nRelease assets:")
    for asset in sorted(path for path in DIST_DIR.iterdir() if path.is_file()):
        print(f" - {asset.name}")

    if not args.build_only:
        publish_release(tag, args.repo, args.draft, version.is_prerelease, publish_auth)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"release failed: {error}", file=sys.stderr)
        sys.exit(1)
