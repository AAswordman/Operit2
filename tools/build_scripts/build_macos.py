#!/usr/bin/env python3
import argparse
import os
import shutil
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]
APP_DIR = REPO_ROOT / "apps" / "flutter" / "app"


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


def prepare_python_command() -> None:
    python3 = require_command("python3")
    venv_bin = REPO_ROOT / ".venv" / "bin"
    venv_bin.mkdir(parents=True, exist_ok=True)
    target = venv_bin / "python"
    if target.exists() or target.is_symlink():
        target.unlink()
    target.symlink_to(python3)
    run([str(target), "--version"])


def generate_dart_proxy_artifacts() -> None:
    manifest = REPO_ROOT / "core" / "crates" / "operit-core-proxy" / "Cargo.toml"
    run(["cargo", "clean", "--manifest-path", str(manifest), "-p", "operit-core-proxy"])
    run(["cargo", "check", "--manifest-path", str(manifest), "--quiet"])
    for path in [
        APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyClients.g.dart",
        APP_DIR / "lib" / "core" / "proxy" / "generated" / "CoreProxyModels.g.dart",
    ]:
        if not path.is_file():
            raise RuntimeError(f"Expected generated Dart proxy artifact is missing: {path}")


def build_macos_app(env: dict[str, str]) -> None:
    run(["flutter", "build", "macos", "--release", "-v"], cwd=APP_DIR, env=env)


def package_macos_app() -> Path:
    app_path = APP_DIR / "build" / "macos" / "Build" / "Products" / "Release" / "Operit2.app"
    archive_path = app_path.parent / "Operit2-macos.zip"
    if not app_path.is_dir():
        raise RuntimeError(f"macOS app bundle was not produced: {app_path}")
    if archive_path.exists():
        archive_path.unlink()
    run(["ditto", "-c", "-k", "--keepParent", str(app_path), str(archive_path)])
    if not archive_path.is_file() or archive_path.stat().st_size == 0:
        raise RuntimeError(f"macOS archive was not produced: {archive_path}")
    return archive_path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Build the Operit2 macOS Flutter app.")
    parser.add_argument(
        "--skip-proxy-generation",
        action="store_true",
        help="Skip core proxy generation checks.",
    )
    parser.add_argument(
        "--skip-package",
        action="store_true",
        help="Skip creating Operit2-macos.zip.",
    )
    return parser.parse_args()


def main() -> int:
    args = parse_args()
    os.environ.setdefault("RUSTFLAGS", "-Awarnings")
    typescript_version = os.environ.get("TYPESCRIPT_VERSION", "5.9.3")

    require_command("cargo")
    require_command("flutter")
    run(["node", "--version"])
    run(["npm", "--version"])

    env = os.environ.copy()
    typescript_bin = ensure_typescript(typescript_version)
    env["PATH"] = f"{typescript_bin}{os.pathsep}{env['PATH']}"

    if not args.skip_proxy_generation:
        generate_dart_proxy_artifacts()

    prepare_python_command()
    run(["flutter", "pub", "get"], cwd=APP_DIR, env=env)

    build_macos_app(env)
    if not args.skip_package:
        archive_path = package_macos_app()
        print(f"macOS archive: {archive_path}", flush=True)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:
        print(f"error: {error}", file=sys.stderr)
        raise SystemExit(1)
