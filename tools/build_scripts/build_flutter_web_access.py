#!/usr/bin/env python3
import os
import shutil
import sys
from pathlib import Path

from common import (
    FLUTTER_APP_DIR,
    REPO_ROOT,
    WEB_ACCESS_BUNDLE_DIR,
    ensure_node_and_npm,
    ensure_typescript,
    flutter_command,
    host_platform,
    prepare_python_command,
    require_command,
    require_web_access_bundle,
    reset_dir,
    run,
    stage_web_access_source,
)


WASI_SDK_MAJOR_VERSION = "20"


# Ensures wasm-bindgen is installed at the requested version.
def ensure_wasm_bindgen(version: str) -> Path:
    root = REPO_ROOT / ".ci-tools" / "wasm-bindgen"
    suffix = ".exe" if sys.platform == "win32" else ""
    binary = root / "bin" / f"wasm-bindgen{suffix}"
    if not binary.exists():
        run(
            [
                "cargo",
                "install",
                "wasm-bindgen-cli",
                "--version",
                version,
                "--locked",
                "--root",
                str(root),
            ]
        )
    run([str(binary), "--version"])
    return binary.parent


# Ensures the WASI SDK is available for the host platform.
def ensure_wasi_sdk(version: str) -> Path:
    platform_name = host_platform()
    if platform_name == "windows":
        package_name = f"wasi-sdk-{version}.m-mingw"
        clang_name = "clang.exe"
    elif platform_name == "macos":
        package_name = f"wasi-sdk-{version}-macos"
        clang_name = "clang"
    elif platform_name == "linux":
        package_name = f"wasi-sdk-{version}-linux"
        clang_name = "clang"
    else:
        raise RuntimeError(f"Unsupported WASI SDK host platform: {platform_name}")

    root = REPO_ROOT / "target" / "operit-build-tools" / package_name
    clang = root / "bin" / clang_name
    if not clang.exists():
        archive = REPO_ROOT / "target" / "operit-build-tools" / f"{package_name}.tar.gz"
        root.parent.mkdir(parents=True, exist_ok=True)
        archive.parent.mkdir(parents=True, exist_ok=True)
        if root.exists():
            shutil.rmtree(root)
        root.mkdir(parents=True, exist_ok=True)
        run(
            [
                "curl",
                "--fail",
                "--location",
                "--retry",
                "5",
                "--retry-all-errors",
                f"https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-{WASI_SDK_MAJOR_VERSION}/{package_name}.tar.gz",
                "--output",
                str(archive),
            ]
        )
        run(["tar", "-xf", str(archive), "-C", str(root), "--strip-components", "1"])
    run([str(clang), "--version"])
    return root


# Builds the shared Web Access Flutter Web bundle.
def main() -> int:
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    typescript_version = os.environ.get("TYPESCRIPT_VERSION", "5.9.3")
    wasm_bindgen_version = os.environ.get("WASM_BINDGEN_VERSION", "0.2.122")
    wasi_sdk_version = os.environ.get("WASI_SDK_VERSION", "20.0")

    require_command("cargo")
    flutter = flutter_command()
    ensure_node_and_npm()
    prepare_python_command()

    env = os.environ.copy()
    typescript_bin = ensure_typescript(typescript_version)
    wasm_bindgen_bin = ensure_wasm_bindgen(wasm_bindgen_version)
    wasi_sdk = ensure_wasi_sdk(wasi_sdk_version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{wasm_bindgen_bin}{os.pathsep}{env['PATH']}"
    env["QUICKJS_WASM_SYS_WASI_SDK_PATH"] = str(wasi_sdk)

    run(["rustup", "target", "add", "wasm32-unknown-unknown"])
    stage_web_access_source()
    reset_dir(WEB_ACCESS_BUNDLE_DIR)
    run(
        [flutter, "build", "web", "--release", "--output", WEB_ACCESS_BUNDLE_DIR],
        cwd=FLUTTER_APP_DIR,
        env=env,
    )
    require_web_access_bundle()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
