#!/usr/bin/env python3
"""Download the models.dev snapshot embedded in every runtime platform."""
from __future__ import annotations

import gzip
import json
from pathlib import Path
import tempfile
import urllib.request


REPO_ROOT = Path(__file__).resolve().parents[2]
BUNDLE_PATH = REPO_ROOT / "core/crates/runtime/application/assets/models_dev/catalog.json.gz"
_prepared = False


def download_catalog(name: str) -> dict:
    """Read and validate one upstream catalog before replacing the bundled pair."""
    request = urllib.request.Request(
        f"https://models.dev/{name}.json", headers={"User-Agent": "Operit-build"})
    with urllib.request.urlopen(request, timeout=60) as response:
        catalog = json.load(response)
    if not isinstance(catalog, dict) or not catalog:
        raise ValueError(f"models.dev {name}.json must be a nonempty object")
    return catalog


def refresh_bundle(destination: Path = BUNDLE_PATH) -> None:
    """Publish both validated catalogs together, leaving the prior snapshot intact on failure."""
    providers = download_catalog("api")
    models = download_catalog("models")
    if not any(isinstance(provider, dict) and isinstance(provider.get("models"), dict)
               and provider["models"]
               for provider in providers.values()):
        raise ValueError("models.dev api.json lists no provider models")
    payload = json.dumps({"providers": providers, "models": models},
                         ensure_ascii=False, separators=(",", ":")).encode("utf-8")
    compressed = gzip.compress(payload, mtime=0)
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.NamedTemporaryFile(dir=destination.parent, delete=False) as output:
        temporary = Path(output.name)
        output.write(compressed)
    try:
        temporary.replace(destination)
    finally:
        temporary.unlink(missing_ok=True)
    print(f"Bundled models.dev catalog: {len(compressed):,} bytes", flush=True)


def prepare_build_catalog(command: list[str | Path]) -> None:
    """Refresh for release packaging, and prepare the first snapshot for development builds."""
    global _prepared
    args = [str(part) for part in command]
    compiling = any(arg in ("build", "rustc", "check", "test") for arg in args)
    release = "--release" in args and any(arg in ("build", "rustc") for arg in args)
    if not _prepared and compiling and (release or not BUNDLE_PATH.is_file()):
        refresh_bundle()
        _prepared = True


if __name__ == "__main__":
    refresh_bundle()
