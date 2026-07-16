#!/usr/bin/env python3
from __future__ import annotations

import argparse
import os
import shutil
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path

from cli_common import CLI_RUST_TARGETS
from common import REPO_ROOT, host_arch, host_platform


FLUTTER_APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
ANDROID_SIGNING = REPO_ROOT / "tools" / "release" / "secrets" / "android-signing.properties"
OHOS_SIGNING = REPO_ROOT / "tools" / "release" / "secrets" / "ohos-signing" / "ohos-signing.properties"
OHOS_LOCAL_PROPERTIES = FLUTTER_APP_DIR / "ohos" / "local.properties"
CLI_ARCH_CHOICES = ("host", "all", "x86_64", "aarch64")
PRODUCT_CHOICES = ("app", "cli", "all")


@dataclass(frozen=True)
class CheckResult:
    name: str
    ok: bool
    detail: str


# Parses command-line options for build environment validation.
def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Check Operit2 build environment requirements.")
    parser.add_argument("--products", choices=PRODUCT_CHOICES, default="all")
    parser.add_argument("--cli-arches", choices=CLI_ARCH_CHOICES, default="host")
    parser.add_argument("--include-ios", action="store_true")
    parser.add_argument("--release-script", action="store_true", help="Check Android, OpenHarmony, and WSL release requirements.")
    parser.add_argument("--wsl", action="store_true", help="Check Fedora WSL release requirements from Windows.")
    parser.add_argument("--wsl-distro", default="FedoraLinux-43")
    parser.add_argument("--github-dispatch", action="store_true", help="Check local GitHub CLI access for cloud build dispatch.")
    parser.add_argument("--github-runner", action="store_true", help="Check requirements expected inside a GitHub-hosted runner job.")
    return parser.parse_args()


# Runs one command and returns its captured text.
def command_output(command: list[str | Path], cwd: Path = REPO_ROOT) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [str(part) for part in command],
        cwd=cwd,
        encoding="utf-8",
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
    )


# Creates one successful check result.
def ok(name: str, detail: str) -> CheckResult:
    return CheckResult(name, True, detail)


# Creates one failed check result.
def missing(name: str, detail: str) -> CheckResult:
    return CheckResult(name, False, detail)


# Checks whether one executable exists and reports its version text.
def check_command(name: str, version_args: list[str] | None = None) -> CheckResult:
    path = shutil.which(name)
    if path is None:
        return missing(name, f"command not found on PATH: {name}")
    if version_args is None:
        return ok(name, path)
    result = command_output([path, *version_args])
    version_text = (result.stdout or result.stderr).strip().splitlines()
    if result.returncode != 0:
        return missing(name, f"{path} failed to report version: {result.stderr.strip()}")
    return ok(name, f"{path} :: {version_text[0] if version_text else 'version reported'}")


# Checks whether one required file exists.
def check_file(name: str, path: Path) -> CheckResult:
    if path.is_file():
        return ok(name, str(path))
    return missing(name, f"required file not found: {path}")


# Checks whether one required directory exists.
def check_directory(name: str, path: Path) -> CheckResult:
    if path.is_dir():
        return ok(name, str(path))
    return missing(name, f"required directory not found: {path}")


# Returns every Rust target installed through rustup.
def installed_rust_targets() -> set[str]:
    result = command_output(["rustup", "target", "list", "--installed"])
    if result.returncode != 0:
        return set()
    return {line.strip() for line in result.stdout.splitlines() if line.strip()}


# Expands a CLI architecture selector into concrete architecture names.
def selected_cli_arches(selector: str) -> tuple[str, ...]:
    if selector == "host":
        return (host_arch(),)
    if selector == "all":
        return ("x86_64", "aarch64")
    return (selector,)


# Checks Rust targets needed by the selected CLI package build.
def check_cli_rust_targets(target_platform: str, cli_arches: str) -> list[CheckResult]:
    installed = installed_rust_targets()
    results = []
    for architecture in selected_cli_arches(cli_arches):
        rust_target = CLI_RUST_TARGETS[(target_platform, architecture)]
        if target_platform == host_platform() and architecture == host_arch():
            results.append(ok(f"rust target {rust_target}", "host target uses the default Rust toolchain target"))
        elif rust_target in installed:
            results.append(ok(f"rust target {rust_target}", "installed"))
        else:
            results.append(missing(f"rust target {rust_target}", f"run: rustup target add {rust_target}"))
    return results


# Checks local common tools used by App and CLI builds.
def check_common_tools(products: str) -> list[CheckResult]:
    results = [
        check_command("git", ["--version"]),
        check_command("cargo", ["--version"]),
        check_command("rustup", ["--version"]),
        check_command("fvm", ["--version"]),
        check_command("dart", ["--version"]),
        check_command("node", ["--version"]),
        check_command("npm", ["--version"]),
        check_file("Flutter FVM pin", FLUTTER_APP_DIR / ".fvmrc"),
        check_file("Flutter pubspec lock", FLUTTER_APP_DIR / "pubspec.lock"),
    ]
    if products in ("app", "all"):
        results.append(check_directory("Flutter app directory", FLUTTER_APP_DIR))
    return results


# Checks native platform tools for desktop App builds.
def check_native_app_tools(include_ios: bool) -> list[CheckResult]:
    platform_name = host_platform()
    results = []
    if platform_name == "windows":
        vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:\\Program Files (x86)")) / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
        results.append(check_file("Visual Studio locator", vswhere))
    elif platform_name == "linux":
        results.extend(
            [
                check_command("cmake", ["--version"]),
                check_command("ninja", ["--version"]),
                check_command("pkg-config", ["--version"]),
                check_command("clang", ["--version"]),
            ]
        )
    elif platform_name == "macos":
        results.extend([check_command("xcodebuild", ["-version"]), check_command("xcrun", ["--version"]), check_command("ditto")])
        if include_ios:
            sdk = command_output(["xcrun", "--sdk", "iphoneos", "--show-sdk-path"])
            if sdk.returncode == 0 and sdk.stdout.strip():
                results.append(ok("iOS Xcode platform", sdk.stdout.strip()))
            else:
                results.append(missing("iOS Xcode platform", "xcrun cannot find the iphoneos SDK"))
    return results


# Checks Windows-only requirements for aarch64 CLI packages.
def check_windows_aarch64_cli(cli_arches: str) -> list[CheckResult]:
    if host_platform() != "windows" or "aarch64" not in selected_cli_arches(cli_arches):
        return []
    vswhere = Path(os.environ.get("ProgramFiles(x86)", "C:\\Program Files (x86)")) / "Microsoft Visual Studio" / "Installer" / "vswhere.exe"
    llvm_clang = Path("C:/Program Files/LLVM/bin/clang.exe")
    return [check_file("Visual Studio locator", vswhere), check_file("LLVM clang", llvm_clang)]


# Checks signing files needed by the Windows release script's App scope.
def check_release_signing_files() -> list[CheckResult]:
    return [
        check_file("Android signing properties", ANDROID_SIGNING),
        check_file("OpenHarmony signing properties", OHOS_SIGNING),
        check_file("OpenHarmony local properties", OHOS_LOCAL_PROPERTIES),
    ]


# Checks WSL commands needed for Linux release assets from Windows.
def check_wsl_release_tools(distro: str) -> list[CheckResult]:
    if host_platform() != "windows":
        return [missing("WSL host", "WSL release checks must run on Windows")]
    wsl = shutil.which("wsl.exe")
    if wsl is None:
        return [missing("wsl.exe", "wsl.exe command not found on PATH")]
    script = " && ".join(
        [
            "command -v cargo",
            "command -v rustup",
            "command -v fvm",
            "command -v musl-gcc",
            "command -v aarch64-linux-gnu-gcc",
        ]
    )
    result = command_output([wsl, "-d", distro, "--", "bash", "-lc", script])
    if result.returncode == 0:
        return [ok(f"WSL distro {distro}", "cargo, rustup, fvm, musl-gcc, and aarch64-linux-gnu-gcc are available")]
    return [missing(f"WSL distro {distro}", result.stderr.strip() or result.stdout.strip())]


# Checks GitHub CLI access for dispatching and downloading cloud builds.
def check_github_dispatch_tools() -> list[CheckResult]:
    results = [check_command("gh", ["--version"])]
    gh = shutil.which("gh")
    if gh is None:
        return results
    auth = command_output([gh, "auth", "status"])
    if auth.returncode == 0:
        results.append(ok("GitHub CLI auth", "authenticated"))
    else:
        results.append(missing("GitHub CLI auth", auth.stderr.strip() or auth.stdout.strip()))
    workflow = command_output([gh, "workflow", "list"])
    if workflow.returncode == 0:
        results.append(ok("GitHub workflows", "workflow list is readable"))
    else:
        results.append(missing("GitHub workflows", workflow.stderr.strip() or workflow.stdout.strip()))
    return results


# Builds the full list of checks for the requested profile.
def build_checks(args: argparse.Namespace) -> list[CheckResult]:
    checks = check_common_tools(args.products)
    if args.products in ("app", "all"):
        checks.extend(check_native_app_tools(args.include_ios))
    if args.products in ("cli", "all"):
        checks.extend(check_cli_rust_targets(host_platform(), args.cli_arches))
        checks.extend(check_windows_aarch64_cli(args.cli_arches))
    if args.release_script and args.products in ("app", "all"):
        checks.extend(check_release_signing_files())
    if args.wsl:
        checks.extend(check_wsl_release_tools(args.wsl_distro))
    if args.github_dispatch:
        checks.extend(check_github_dispatch_tools())
    return checks


# Prints check results and returns a failing status when any requirement is missing.
def main() -> int:
    args = parse_args()
    checks = build_checks(args)
    failed = [check for check in checks if not check.ok]
    print("Operit2 build environment check")
    print(f"Host: {host_platform()} {host_arch()}")
    for check in checks:
        prefix = "OK" if check.ok else "MISSING"
        print(f"[{prefix}] {check.name}: {check.detail}")
    if failed:
        print(f"Environment check failed: {len(failed)} missing requirement(s)", file=sys.stderr)
        return 1
    print("Environment check passed")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
