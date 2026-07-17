#!/usr/bin/env python3
import argparse
import os
import shutil
import sys
from pathlib import Path

from common import (
    FLUTTER_APP_DIR,
    REPO_ROOT,
    WEB_ACCESS_APP_DIR,
    WEB_ACCESS_BUNDLE_DIR,
    copy_required_file,
    dart_pub_get,
    ensure_node_and_npm,
    ensure_terser,
    ensure_typescript,
    flutter_command,
    generate_dart_proxy_artifacts,
    host_platform,
    prepare_python_command,
    prepare_web_access_embedded_assets,
    require_command,
    reset_dir,
    run,
    stage_web_access_source,
    write_web_access_version_manifest,
)


WASI_SDK_MAJOR_VERSION = "20"
WEB_ACCESS_ASSET_PREFIX = "    - path: assets/web_access/"
WEB_ACCESS_ASSET_PLATFORM_LINE = "      platforms: [android, ios, linux, macos, windows]"
WASM_SOURCE = (
    REPO_ROOT
    / "apps"
    / "flutter"
    / "native"
    / "operit-flutter-bridge"
    / "target"
    / "wasm32-unknown-unknown"
    / "release"
    / "operit_flutter_bridge.wasm"
)
SQL_DIST_DIR = (
    FLUTTER_APP_DIR
    / ".dart_tool"
    / "web-build-deps"
    / "node_modules"
    / "sql.js"
    / "dist"
)


# Parses the required deployment base path for the Flutter Web bundle.
def parse_arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--base-href",
        required=True,
        help="Absolute deployment path ending with a slash, such as /Operit2/.",
    )
    arguments = parser.parse_args()
    if not arguments.base_href.startswith("/") or not arguments.base_href.endswith("/"):
        parser.error("--base-href must start and end with '/'")
    return arguments


# Compiles and minifies the browser runtime bridge into Flutter's web shell.
def compile_web_runtime_bridge(typescript_bin: Path, terser_bin: Path) -> None:
    suffix = ".cmd" if sys.platform == "win32" else ""
    run([typescript_bin / f"tsc{suffix}", "-p", WEB_ACCESS_APP_DIR / "tsconfig.json"])
    bridge = WEB_ACCESS_APP_DIR / "web" / "operit_runtime_bridge.js"
    minified = bridge.with_suffix(".min.js")
    run(
        [
            terser_bin / f"terser{suffix}",
            str(bridge),
            "--compress",
            "--mangle",
            "--format",
            "comments=false",
            "--output",
            str(minified),
        ]
    )
    minified.replace(bridge)


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


# Writes a pubspec view that excludes native embedded Web Access assets.
def remove_web_access_native_assets_from_pubspec(pubspec: Path) -> str:
    original = pubspec.read_text(encoding="utf-8")
    lines = original.splitlines(keepends=True)
    staged: list[str] = []
    index = 0
    while index < len(lines):
        line = lines[index]
        if line.startswith(WEB_ACCESS_ASSET_PREFIX):
            next_index = index + 1
            if (
                next_index >= len(lines)
                or lines[next_index].rstrip("\r\n") != WEB_ACCESS_ASSET_PLATFORM_LINE
            ):
                raise RuntimeError(
                    f"Unexpected Web Access asset declaration near line {index + 1}"
                )
            index += 2
            continue
        staged.append(line)
        index += 1
    if len(staged) == len(lines):
        raise RuntimeError("No Web Access native asset declarations were removed.")
    with pubspec.open("w", encoding="utf-8", newline="") as output:
        output.write("".join(staged))
    return original


# Restores the pubspec after the temporary Web Access build view is finished.
def restore_pubspec(pubspec: Path, content: str) -> None:
    with pubspec.open("w", encoding="utf-8", newline="") as output:
        output.write(content)


# Writes bridge and SQL.js runtime files after Flutter finalizes its output.
def stage_web_runtime_files(wasm_bindgen_bin: Path) -> None:
    suffix = ".exe" if sys.platform == "win32" else ""
    run(
        [
            wasm_bindgen_bin / f"wasm-bindgen{suffix}",
            "--target",
            "web",
            "--out-dir",
            WEB_ACCESS_BUNDLE_DIR,
            "--out-name",
            "operit_flutter_bridge",
            WASM_SOURCE,
        ]
    )
    copy_required_file(
        SQL_DIST_DIR / "sql-wasm.js",
        WEB_ACCESS_BUNDLE_DIR / "sql-wasm.js",
    )
    copy_required_file(
        SQL_DIST_DIR / "sql-wasm.wasm",
        WEB_ACCESS_BUNDLE_DIR / "sql-wasm.wasm",
    )


# Builds the shared Web Access Flutter Web bundle for one deployment base path.
def main(base_href: str) -> int:
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    typescript_version = os.environ.get("TYPESCRIPT_VERSION", "5.9.3")
    terser_version = os.environ.get("TERSER_VERSION", "5.44.0")
    wasm_bindgen_version = os.environ.get("WASM_BINDGEN_VERSION", "0.2.122")
    wasi_sdk_version = os.environ.get("WASI_SDK_VERSION", "20.0")

    require_command("cargo")
    flutter = flutter_command()
    ensure_node_and_npm()
    prepare_python_command()

    env = os.environ.copy()
    typescript_bin = ensure_typescript(typescript_version)
    terser_bin = ensure_terser(terser_version)
    wasm_bindgen_bin = ensure_wasm_bindgen(wasm_bindgen_version)
    wasi_sdk = ensure_wasi_sdk(wasi_sdk_version)
    clang_resource_includes = sorted(
        path
        for path in (wasi_sdk / "lib" / "clang").glob("*/include")
        if path.is_dir()
    )
    if len(clang_resource_includes) != 1:
        raise RuntimeError(
            "Expected exactly one Clang resource include directory in the WASI SDK, "
            f"found {len(clang_resource_includes)}"
    )
    wasi_lib_dir = wasi_sdk / "share" / "wasi-sysroot" / "lib" / "wasm32-wasi"
    wasi_builtins_dir = clang_resource_includes[0].parent / "lib" / "wasi"
    wasi_libc = wasi_lib_dir / "libc.a"
    wasi_builtins = wasi_builtins_dir / "libclang_rt.builtins-wasm32.a"
    if not wasi_libc.is_file() or not wasi_builtins.is_file():
        raise RuntimeError(
            "WASI SDK is missing the WebAssembly libc or compiler builtins required "
            f"by QuickJS: libc={wasi_libc} builtins={wasi_builtins}"
        )
    env["PATH"] = f"{typescript_bin}{os.pathsep}{wasm_bindgen_bin}{os.pathsep}{env['PATH']}"
    env["QUICKJS_WASM_SYS_WASI_SDK_PATH"] = str(wasi_sdk)
    env["BINDGEN_EXTRA_CLANG_ARGS_wasm32_unknown_unknown"] = (
        f'-I"{clang_resource_includes[0].as_posix()}"'
    )
    env["RUSTFLAGS"] = " ".join(
        [
            env.get("RUSTFLAGS", "-Awarnings"),
            "-L",
            f"native={wasi_lib_dir.as_posix()}",
            "-L",
            f"native={wasi_builtins_dir.as_posix()}",
            "-l",
            "static=c",
            "-l",
            "static=clang_rt.builtins-wasm32",
        ]
    )

    generate_dart_proxy_artifacts()
    dart_pub_get(enforce_lockfile=True, env=env)
    run(["rustup", "target", "add", "wasm32-unknown-unknown"])
    compile_web_runtime_bridge(typescript_bin, terser_bin)
    stage_web_access_source()
    reset_dir(WEB_ACCESS_BUNDLE_DIR)
    pubspec = FLUTTER_APP_DIR / "pubspec.yaml"
    original_pubspec = remove_web_access_native_assets_from_pubspec(pubspec)
    try:
        run(
            [
                flutter,
                "build",
                "web",
                "--release",
                "--no-pub",
                "--no-wasm-dry-run",
                "--base-href",
                base_href,
                "--output",
                WEB_ACCESS_BUNDLE_DIR,
            ],
            cwd=FLUTTER_APP_DIR,
            env=env,
        )
    finally:
        restore_pubspec(pubspec, original_pubspec)
    stage_web_runtime_files(wasm_bindgen_bin)
    manifest = write_web_access_version_manifest()
    print(
        "Web Access version "
        f"{manifest['version']} hash={manifest['contentHash']} "
        f"files={manifest['fileCount']} bytes={manifest['byteSize']}",
        flush=True,
    )
    prepare_web_access_embedded_assets()
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main(parse_arguments().base_href))
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
