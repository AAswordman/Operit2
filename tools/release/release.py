#!/usr/bin/env python3
import argparse
import platform
import shlex
import shutil
import subprocess
import sys
from pathlib import Path


SCRIPT_DIR = Path(__file__).resolve().parent
REPO_ROOT = SCRIPT_DIR.parent.parent
DIST_DIR = SCRIPT_DIR / "dist"
WORK_DIR = SCRIPT_DIR / "work"
SECRETS_DIR = SCRIPT_DIR / "secrets"
FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
ANDROID_DIR = FLUTTER_APP_DIR / "android"
ANDROID_LOCAL_PROPERTIES = ANDROID_DIR / "local.properties"
CLI_MANIFEST = REPO_ROOT / "apps" / "cli" / "Cargo.toml"


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


def flutter_command():
    found = shutil.which("flutter")
    if found is not None:
        return found

    local = read_properties(ANDROID_LOCAL_PROPERTIES)
    flutter_sdk = local.get("flutter.sdk")
    if not flutter_sdk:
        raise RuntimeError(f"Required command not found: flutter")

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
    pubspec = FLUTTER_APP_DIR / "pubspec.yaml"
    for line in pubspec.read_text(encoding="utf-8").splitlines():
        stripped = line.strip()
        if stripped.startswith("version:"):
            return stripped.split(":", 1)[1].strip()
    raise RuntimeError("pubspec.yaml does not define version")


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


def build_wsl_linux_app(distro):
    if not wsl_check_command(distro, "flutter"):
        raise RuntimeError(
            "WSL Linux app build requires Flutter inside WSL. Install Linux Flutter SDK in WSL and put flutter on PATH."
        )
    repo = shlex.quote(windows_path_to_wsl(REPO_ROOT))
    dist = shlex.quote(windows_path_to_wsl(DIST_DIR))
    script = f"""
set -e
cd {repo}
cd apps/flutter/app
flutter pub get --enforce-lockfile
rm -rf build/linux/x64/release
flutter build linux --release
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


def build_wsl_linux(products, distro):
    if not is_windows_host():
        return
    if shutil.which("wsl.exe") is None:
        raise RuntimeError("wsl.exe not found")
    if "app" in products:
        build_wsl_linux_app(distro)
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


def build_host_app():
    flutter = flutter_command()
    current_platform = host_platform()
    current_arch = host_arch()
    run([flutter, "pub", "get", "--enforce-lockfile"], FLUTTER_APP_DIR)

    if current_platform == "windows":
        run([flutter, "build", "windows", "--release"], FLUTTER_APP_DIR)
        release_dir = FLUTTER_APP_DIR / "build" / "windows" / "x64" / "runner" / "Release"
        compress_zip(release_dir, DIST_DIR / f"operit2-app-windows-{current_arch}.zip")
        return

    if current_platform == "linux":
        run([flutter, "build", "linux", "--release"], FLUTTER_APP_DIR)
        bundle = FLUTTER_APP_DIR / "build" / "linux" / "x64" / "release" / "bundle"
        package = DIST_DIR / f"operit2-app-linux-{current_arch}.tar.gz"
        if package.exists():
            package.unlink()
        run(["tar", "-czf", package, "-C", bundle, "."])
        return

    if current_platform == "macos":
        run([flutter, "build", "macos", "--release"], FLUTTER_APP_DIR)
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


def publish_release(tag, repo, draft, prerelease):
    require_command("gh")
    assets = sorted(path for path in DIST_DIR.iterdir() if path.is_file())
    if not assets:
        raise RuntimeError("No release assets were produced")

    view = subprocess.run(
        ["gh", "release", "view", tag, "--repo", repo],
        cwd=REPO_ROOT,
        stdout=subprocess.DEVNULL,
        stderr=subprocess.DEVNULL,
    )
    release_exists = view.returncode == 0

    if release_exists:
        for asset in assets:
            run(["gh", "release", "upload", tag, asset, "--repo", repo, "--clobber"])
        return

    command = [
        "gh",
        "release",
        "create",
        tag,
        "--repo",
        repo,
        "--title",
        tag,
        "--notes",
        f"Operit2 {tag}",
    ]
    if draft:
        command.append("--draft")
    if prerelease:
        command.append("--prerelease")
    command.extend(assets)
    run(command)


def main():
    parser = argparse.ArgumentParser(description="Build and publish Operit2 release assets.")
    parser.add_argument("--tag", default="")
    parser.add_argument("--repo", default="AAswordman/Operit2")
    parser.add_argument("--build-only", action="store_true")
    parser.add_argument("--draft", action="store_true")
    parser.add_argument("--prerelease", action="store_true")
    parser.add_argument("--products", nargs="+", default=["app", "cli"], choices=["app", "cli", "none"])
    parser.add_argument("--wsl-distro", default="FedoraLinux-43")
    parser.add_argument("--no-wsl", action="store_true")
    args = parser.parse_args()

    version = pubspec_version()
    version_parts = version.split("+", 1)
    build_name = version_parts[0]
    build_number = version_parts[1] if len(version_parts) == 2 else "1"
    tag = args.tag or f"v{version}"

    reset_dir(DIST_DIR)
    reset_dir(WORK_DIR)

    products = set(args.products)
    if "none" in products and len(products) > 1:
        raise RuntimeError("--products none cannot be combined with app or cli")

    if "app" in products:
        build_android_app(build_name, build_number)
        build_host_app()

    if "cli" in products:
        build_host_cli()

    if not args.no_wsl and "none" not in products:
        build_wsl_linux(products, args.wsl_distro)

    print("\nRelease assets:")
    for asset in sorted(path for path in DIST_DIR.iterdir() if path.is_file()):
        print(f" - {asset.name}")

    if not args.build_only:
        publish_release(tag, args.repo, args.draft, args.prerelease)


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"release failed: {error}", file=sys.stderr)
        sys.exit(1)
