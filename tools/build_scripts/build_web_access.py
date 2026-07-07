#!/usr/bin/env python3
import os
import shutil
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"
WASI_SDK_MAJOR_VERSION = "20"


def run(command: list[str], cwd: Path = REPO_ROOT, env: dict[str, str] | None = None) -> None:
    display = " ".join(command)
    print(f"+ {display}", flush=True)
    merged_env = os.environ.copy()
    if env:
        merged_env.update(env)
    subprocess.run(command, cwd=cwd, env=merged_env, check=True)


def require_command(name: str) -> str:
    path = shutil.which(name)
    if path is None:
        raise RuntimeError(f"Required command is not available: {name}")
    return path


def ensure_typescript(version: str) -> Path:
    root = REPO_ROOT / ".ci-tools" / "typescript"
    tsc = root / "node_modules" / ".bin" / "tsc"
    if not tsc.exists():
        require_command("npm")
        run(
            [
                "npm",
                "install",
                "--prefix",
                str(root),
                "--no-audit",
                "--no-fund",
                f"typescript@{version}",
            ]
        )
    run([str(tsc), "--version"])
    return tsc.parent


def ensure_wasm_bindgen(version: str) -> Path:
    root = REPO_ROOT / ".ci-tools" / "wasm-bindgen"
    binary = root / "bin" / "wasm-bindgen"
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


def ensure_wasi_sdk(version: str) -> Path:
    root = REPO_ROOT / "target" / "operit-build-tools" / f"wasi-sdk-{version}-macos"
    clang = root / "bin" / "clang"
    if not clang.exists():
        archive = REPO_ROOT / "target" / "operit-build-tools" / f"wasi-sdk-{version}-macos.tar.gz"
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
                f"https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-{WASI_SDK_MAJOR_VERSION}/wasi-sdk-{version}-macos.tar.gz",
                "--output",
                str(archive),
            ]
        )
        run(["tar", "-xf", str(archive), "-C", str(root), "--strip-components", "1"])
    run([str(clang), "--version"])
    return root


def main() -> int:
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    typescript_version = os.environ.get("TYPESCRIPT_VERSION", "5.9.3")
    wasm_bindgen_version = os.environ.get("WASM_BINDGEN_VERSION", "0.2.122")
    wasi_sdk_version = os.environ.get("WASI_SDK_VERSION", "20.0")

    require_command("cargo")
    require_command("flutter")
    run(["node", "--version"])
    run(["npm", "--version"])

    env = os.environ.copy()
    typescript_bin = ensure_typescript(typescript_version)
    wasm_bindgen_bin = ensure_wasm_bindgen(wasm_bindgen_version)
    wasi_sdk = ensure_wasi_sdk(wasi_sdk_version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{wasm_bindgen_bin}{os.pathsep}{env['PATH']}"
    env["QUICKJS_WASM_SYS_WASI_SDK_PATH"] = str(wasi_sdk)

    run(["rustup", "target", "add", "wasm32-unknown-unknown"])
    run(["flutter", "build", "web", "--release"], cwd=APP_DIR, env=env)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
