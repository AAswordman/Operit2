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
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DIST_DIR = SCRIPT_DIR / "dist"
WORK_DIR = SCRIPT_DIR / "work"
SECRETS_DIR = SCRIPT_DIR / "secrets"
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
PUBSPEC_PATH = FLUTTER_APP_DIR / "pubspec.yaml"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
CLI_MANIFEST = REPO_ROOT / "apps" / "cli" / "Cargo.toml"
RUNTIME_MANIFEST = REPO_ROOT / "core" / "crates" / "operit-runtime" / "Cargo.toml"
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


def run(command, cwd=REPO_ROOT):
    print(">> " + " ".join(str(part) for part in command), flush=True)
    subprocess.run([str(part) for part in command], cwd=cwd, check=True)


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


def flutter_command():
    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    flutter_sdk = local.get("flutter.sdk")
    if not flutter_sdk:
        raise RuntimeError(f"flutter.sdk is not defined in {ANDROID_LOCAL_PROPERTIES}")

    sdk_path = Path(flutter_sdk)
    command = sdk_path / "bin" / ("flutter.bat" if platform.system().lower() == "windows" else "flutter")
    if not command.exists():
        raise RuntimeError(f"Flutter command not found from flutter.sdk: {command}")
    return command


def reset_dir(path):
    if path.exists():
        shutil.rmtree(path)
    path.mkdir(parents=True, exist_ok=True)


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


def host_platform():
    name = platform.system().lower()
    if name == "windows":
        return "windows"
    if name == "linux":
        return "linux"
    if name == "darwin":
        return "macos"
    raise RuntimeError(f"Unsupported host OS: {name}")


def host_arch():
    machine = platform.machine().lower()
    if machine in ("amd64", "x86_64"):
        return "x86_64"
    if machine in ("arm64", "aarch64"):
        return "aarch64"
    raise RuntimeError(f"Unsupported host architecture: {machine}")


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
    return platform.system().lower() == "windows"


def windows_path_to_wsl(path):
    resolved = Path(path).resolve()
    drive = resolved.drive.rstrip(":").lower()
    if not drive:
        raise RuntimeError(f"Cannot convert path to WSL path: {resolved}")
    parts = resolved.parts[1:]
    return "/mnt/" + drive + "/" + "/".join(part.replace("\\", "/") for part in parts)


def wsl_run(distro, script):
    script = "export PATH=\"$HOME/.cargo/bin:$HOME/.local/flutter/bin:$PATH\"\n" + script
    command = ["wsl.exe"]
    if distro:
        command.extend(["-d", distro])
    command.extend(["bash", "-lc", script])
    print(">> " + " ".join(command), flush=True)
    subprocess.run(command, cwd=REPO_ROOT, check=True)


def wsl_check_command(distro, name):
    command = ["wsl.exe"]
    if distro:
        command.extend(["-d", distro])
    command.extend(
        [
            "bash",
            "-lc",
            f"export PATH=\"$HOME/.cargo/bin:$HOME/.local/flutter/bin:$PATH\"; command -v {shlex.quote(name)} >/dev/null",
        ]
    )
    return subprocess.run(command, cwd=REPO_ROOT).returncode == 0


def build_wsl_linux_app(distro, build_name, build_number):
    if not wsl_check_command(distro, "flutter"):
        raise RuntimeError(
            "WSL Linux app build requires Flutter inside WSL. Install Linux Flutter SDK in WSL and put flutter on PATH."
        )
    repo = shlex.quote(windows_path_to_wsl(REPO_ROOT))
    dist = shlex.quote(windows_path_to_wsl(DIST_DIR))
    build_name_arg = shlex.quote(build_name)
    build_number_arg = shlex.quote(build_number)
    script = f"""
set -e
cd {repo}
cd apps/flutter/app
flutter pub get --enforce-lockfile
rm -rf build/linux/x64/release
flutter build linux --release --build-name {build_name_arg} --build-number {build_number_arg}
cd {repo}
mkdir -p {dist}
rm -f {dist}/operit2-app-linux-x86_64.tar.gz
tar -czf {dist}/operit2-app-linux-x86_64.tar.gz -C apps/flutter/app/build/linux/x64/release/bundle .
"""
    wsl_run(distro, script)


def build_wsl_linux_cli(distro):
    if not wsl_check_command(distro, "cargo"):
        raise RuntimeError("WSL Linux CLI build requires cargo inside WSL.")
    repo = shlex.quote(windows_path_to_wsl(REPO_ROOT))
    dist = shlex.quote(windows_path_to_wsl(DIST_DIR))
    work = shlex.quote(windows_path_to_wsl(WORK_DIR / "cli-linux-x86_64"))
    script = f"""
set -e
cd {repo}
cargo build --release --manifest-path apps/cli/Cargo.toml
rm -rf {work}
mkdir -p {work} {dist}
cp apps/cli/target/release/operit2 {work}/operit2
rm -f {dist}/operit2-cli-linux-x86_64.tar.gz
tar -czf {dist}/operit2-cli-linux-x86_64.tar.gz -C {work} .
"""
    wsl_run(distro, script)


def build_wsl_linux(products, distro, build_name, build_number):
    if not is_windows_host():
        return
    if shutil.which("wsl.exe") is None:
        raise RuntimeError("wsl.exe not found")
    if "app" in products:
        build_wsl_linux_app(distro, build_name, build_number)
    if "cli" in products:
        build_wsl_linux_cli(distro)


def build_android_app(build_name, build_number):
    ensure_android_signing()
    flutter = flutter_command()
    run([flutter, "pub", "get", "--enforce-lockfile"], FLUTTER_APP_DIR)
    run(
        [
            flutter,
            "build",
            "apk",
            "--release",
            "--split-per-abi",
            "--build-name",
            build_name,
            "--build-number",
            build_number,
        ],
        FLUTTER_APP_DIR,
    )

    apk_dir = FLUTTER_APP_DIR / "build" / "app" / "outputs" / "flutter-apk"
    outputs = {
        "arm64-v8a": "app-arm64-v8a-release.apk",
        "armeabi-v7a": "app-armeabi-v7a-release.apk",
        "x86_64": "app-x86_64-release.apk",
    }
    for abi, filename in outputs.items():
        copy_required_file(apk_dir / filename, DIST_DIR / f"operit2-app-android-{abi}.apk")


def build_host_app(build_name, build_number):
    flutter = flutter_command()
    current_platform = host_platform()
    current_arch = host_arch()
    run([flutter, "pub", "get", "--enforce-lockfile"], FLUTTER_APP_DIR)

    if current_platform == "windows":
        run(
            [
                flutter,
                "build",
                "windows",
                "--release",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
            ],
            FLUTTER_APP_DIR,
        )
        release_dir = FLUTTER_APP_DIR / "build" / "windows" / "x64" / "runner" / "Release"
        compress_zip(release_dir, DIST_DIR / f"operit2-app-windows-{current_arch}.zip")
        return

    if current_platform == "linux":
        run(
            [
                flutter,
                "build",
                "linux",
                "--release",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
            ],
            FLUTTER_APP_DIR,
        )
        bundle = FLUTTER_APP_DIR / "build" / "linux" / "x64" / "release" / "bundle"
        package = DIST_DIR / f"operit2-app-linux-{current_arch}.tar.gz"
        if package.exists():
            package.unlink()
        run(["tar", "-czf", package, "-C", bundle, "."])
        return

    if current_platform == "macos":
        run(
            [
                flutter,
                "build",
                "macos",
                "--release",
                "--build-name",
                build_name,
                "--build-number",
                build_number,
            ],
            FLUTTER_APP_DIR,
        )
        app_parent = FLUTTER_APP_DIR / "build" / "macos" / "Build" / "Products" / "Release"
        package = DIST_DIR / f"operit2-app-macos-{current_arch}.tar.gz"
        if package.exists():
            package.unlink()
        run(["tar", "-czf", package, "-C", app_parent, "operit2.app"])


def build_host_cli():
    require_command("cargo")
    current_platform = host_platform()
    current_arch = host_arch()
    binary_name = "operit2.exe" if current_platform == "windows" else "operit2"
    archive_ext = "zip" if current_platform == "windows" else "tar.gz"
    package_dir = WORK_DIR / f"cli-{current_platform}-{current_arch}"
    package_path = DIST_DIR / f"operit2-cli-{current_platform}-{current_arch}.{archive_ext}"

    run(["cargo", "build", "--release", "--manifest-path", CLI_MANIFEST])
    reset_dir(package_dir)
    copy_required_file(REPO_ROOT / "apps" / "cli" / "target" / "release" / binary_name, package_dir / binary_name)

    if current_platform == "windows":
        compress_zip(package_dir, package_path)
    else:
        if package_path.exists():
            package_path.unlink()
        run(["tar", "-czf", package_path, "-C", package_dir, "."])


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
    if explicit_products is not None:
        if scope != "full":
            raise RuntimeError("--products cannot be combined with --scope app, --scope cli, or --scope none")
        products = set(explicit_products)
    else:
        products = {
            "full": {"app", "cli"},
            "app": {"app"},
            "cli": {"cli"},
            "none": {"none"},
        }[scope]

    if "none" in products and len(products) > 1:
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
    parser.add_argument("--scope", default="full", choices=["cli", "app", "full", "none"])
    parser.add_argument("--products", nargs="+", choices=["app", "cli", "none"])
    parser.add_argument("--wsl-distro", default="FedoraLinux-43")
    parser.add_argument("--no-wsl", action="store_true")
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

    if "app" in products:
        build_android_app(platform_version.build_name, platform_version.build_number)
        build_host_app(platform_version.build_name, platform_version.build_number)

    if "cli" in products:
        build_host_cli()

    if not args.no_wsl and "none" not in products:
        build_wsl_linux(
            products,
            args.wsl_distro,
            platform_version.build_name,
            platform_version.build_number,
        )

    next_platform_version = None
    if "app" in products and not args.build_only:
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
