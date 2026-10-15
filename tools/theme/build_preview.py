"""Build one immutable preview/contract release; never install SDK components.

Invoke explicitly with the repository virtualenv and pinned tool paths. A failed
build is left visible in its output directory; this tool does not delete it.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
import tomllib

ROOT = Path(__file__).resolve().parents[2]
PREVIEW = ROOT / "tools/theme/preview"


def run(*args: str, cwd: Path = ROOT) -> None:
    subprocess.run(args, cwd=cwd, check=True)


def output(*args: str, cwd: Path = ROOT) -> str:
    return subprocess.check_output(args, cwd=cwd, text=True, encoding="utf-8").strip()


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--renderer-release-id", required=True)
    parser.add_argument("--flutter-sdk", required=True, type=Path)
    parser.add_argument("--wasm-bindgen", required=True, type=Path)
    parser.add_argument("--base-prefix", default="")
    args = parser.parse_args()
    release_id = args.renderer_release_id
    base_prefix = args.base_prefix.rstrip("/")
    if base_prefix and (not base_prefix.startswith("/") or "//" in base_prefix or ".." in base_prefix):
        raise ValueError("--base-prefix must start with '/' and contain no '..' or '//'")
    if not re.fullmatch(r"[A-Za-z0-9._-]{1,128}", release_id) or release_id in {".", ".."}:
        raise ValueError("Renderer release ID must be one canonical path component")
    sdk = args.flutter_sdk.resolve(strict=True)
    # Select the SDK's declared executable, not another SDK discovered on PATH.
    flutter = sdk / "bin" / ("flutter.bat" if os.name == "nt" else "flutter")
    bindgen = args.wasm_bindgen.resolve(strict=True)
    lock = tomllib.loads((ROOT / "core/Cargo.lock").read_text(encoding="utf-8"))
    versions = {row["version"] for row in lock["package"] if row["name"] == "wasm-bindgen"}
    if len(versions) != 1 or output(str(bindgen), "--version") != "wasm-bindgen " + next(iter(versions)):
        raise ValueError("wasm-bindgen CLI must exactly match the locked crate version")
    destination = ROOT / "build/theme-preview" / release_id
    if not destination.resolve().is_relative_to(ROOT.resolve()):
        raise ValueError("Renderer output must remain inside the repository build directory")
    if destination.exists():
        raise FileExistsError(f"Refusing to reuse a renderer release directory: {destination}")
    destination.parent.mkdir(parents=True, exist_ok=True)
    fixture_sources = sorted([
        *(ROOT / "apps/flutter/packages/operit_ui/lib/src").rglob("*.dart"),
        ROOT / "tools/theme/preview/lib/main.dart",
        ROOT / "tools/theme/preview/lib/protocol/theme_studio_message_port.dart",
        ROOT / "tools/theme/preview/lib/protocol/theme_contract_client.dart",
    ], key=lambda item: item.relative_to(ROOT).as_posix())
    fixture_hasher = hashlib.sha256()
    for source in fixture_sources:
        fixture_hasher.update(source.relative_to(ROOT).as_posix().encode("utf-8"))
        fixture_hasher.update(b"\0")
        fixture_hasher.update(source.read_bytes())
        fixture_hasher.update(b"\0")
    fixture_source_sha256 = fixture_hasher.hexdigest()
    machine = json.loads(output(str(flutter), "--version", "--machine", cwd=PREVIEW))
    workspace = json.loads(output("cargo", "metadata", "--manifest-path", "core/Cargo.toml", "--no-deps", "--format-version", "1", "--locked"))
    run("cargo", "build", "--manifest-path", "core/Cargo.toml", "-p", "operit-plugin-theme",
        "--features", "wasm", "--target", "wasm32-unknown-unknown", "--release", "--locked")
    wasm = Path(workspace["target_directory"]) / "wasm32-unknown-unknown/release/operit_plugin_theme.wasm"
    run(str(flutter), "pub", "get", "--enforce-lockfile", cwd=PREVIEW)
    run(str(flutter), "build", "web", "--release", "--no-pub", "--output=" + str(destination),
        "--base-href=" + base_prefix + "/theme-preview/" + release_id + "/",
        "--dart-define=OPERIT_THEME_RENDERER_RELEASE_ID=" + release_id,
        "--dart-define=OPERIT_THEME_FIXTURE_SOURCE_SHA256=" + fixture_source_sha256, cwd=PREVIEW)
    run(str(bindgen), str(wasm), "--target", "web", "--out-name", "operit_plugin_theme",
        "--out-dir", str(destination / "validator"))
    files = []
    for path in sorted(destination.rglob("*")):
        if not path.is_file() or path.name == "renderer-release.json":
            continue
        content = path.read_bytes()
        files.append({"path": path.relative_to(destination).as_posix(), "bytes": len(content),
                      "sha256": hashlib.sha256(content).hexdigest()})
    descriptor = {
        "rendererReleaseId": release_id, "validatorVersion": 1,
        "fixtureSourceSha256": fixture_source_sha256,
        "flutterRevision": machine["frameworkRevision"], "engineRevision": machine["engineRevision"],
        "coreCommit": output("git", "rev-parse", "HEAD"),
        "sourceDirty": bool(output("git", "status", "--porcelain")),
        "files": files,
    }
    (destination / "renderer-release.json").write_text(
        json.dumps(descriptor, ensure_ascii=False, sort_keys=True, indent=2) + "\n", encoding="utf-8")
    print(f"Built renderer release (not parity-accepted): {destination}")


if __name__ == "__main__":
    main()
