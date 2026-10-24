"""Stage the locked Flutter engine font resources for direct same-origin loading."""
from __future__ import annotations

import argparse
from concurrent.futures import ThreadPoolExecutor, as_completed
import hashlib
import json
from pathlib import Path, PurePosixPath
import re
import shutil
import urllib.request

ROOT = Path(__file__).resolve().parents[1]
LOCK = ROOT / "tools/flutter_web_fonts.lock.json"
CACHE = ROOT / ".venv/flutter-web-fonts"
ORIGIN = "https://fonts.gstatic.com/s/"
MAX_FONT_BYTES = 8 * 1024 * 1024


def engine_font_paths(sdk: Path) -> list[str]:
    """Read the complete resource inventory directly from the selected engine sources."""
    engine = sdk / "engine/src/flutter/lib/web_ui/lib/src/engine"
    table = (engine / "font_fallback_data.dart").read_text(encoding="utf-8")
    canvas = (engine / "canvaskit/fonts.dart").read_text(encoding="utf-8")
    paths = set(re.findall(r"'([^']+\.woff2)'", table))
    paths.update(re.findall(r"\$\{configuration\.fontFallbackBaseUrl\}([^']+\.woff2)", canvas))
    if not paths or not any(path.startswith("roboto/") for path in paths):
        raise ValueError("The selected Flutter SDK does not declare its engine font resources")
    for path in paths:
        parsed = PurePosixPath(path)
        if parsed.is_absolute() or ".." in parsed.parts or ":" in path or "\\" in path:
            raise ValueError(f"Invalid engine font resource path: {path}")
    return sorted(paths)


def digest(data: bytes) -> str:
    """Compute the content identity used by the committed font inventory."""
    return hashlib.sha256(data).hexdigest()


def materialize(path: str, expected: dict | None) -> dict:
    """Read one cache entry or fetch its exact source, failing on any integrity mismatch."""
    cached = CACHE / path
    if cached.exists():
        data = cached.read_bytes()
    else:
        request = urllib.request.Request(ORIGIN + path, headers={"User-Agent": "Operit2-Flutter-Fonts"})
        with urllib.request.urlopen(request, timeout=45) as response:
            data = response.read(MAX_FONT_BYTES + 1)
        if not data or len(data) > MAX_FONT_BYTES or data[:4] != b"wOF2":
            raise ValueError(f"Invalid WOFF2 font resource: {path}")
        if expected is not None and (len(data) != expected["bytes"] or digest(data) != expected["sha256"]):
            raise ValueError(f"Downloaded font differs from its lock entry: {path}")
        cached.parent.mkdir(parents=True, exist_ok=True)
        cached.write_bytes(data)
    actual = {"path": path, "bytes": len(data), "sha256": digest(data)}
    if data[:4] != b"wOF2" or (expected is not None and actual != expected):
        raise ValueError(f"Cached font differs from its lock entry: {path}")
    return actual


def main() -> None:
    """Verify every locked font and stage the complete inventory into one Web build input."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--flutter-sdk", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    parser.add_argument("--write-lock", action="store_true")
    args = parser.parse_args()
    paths = engine_font_paths(args.flutter_sdk.resolve(strict=True))
    entries = None
    if not args.write_lock:
        locked = json.loads(LOCK.read_text(encoding="utf-8"))
        if locked["formatVersion"] != 1 or locked["origin"] != ORIGIN:
            raise ValueError("Unsupported Flutter font lock format or source origin")
        if [entry["path"] for entry in locked["fonts"]] != paths:
            raise ValueError("Flutter engine font inventory differs from the committed lock")
        entries = {entry["path"]: entry for entry in locked["fonts"]}
    actual = []
    failures = []
    with ThreadPoolExecutor(max_workers=4) as pool:
        pending = {pool.submit(materialize, path, None if args.write_lock else entries[path]): path for path in paths}
        for future in as_completed(pending):
            try:
                actual.append(future.result())
            except Exception as error:
                failures.append(RuntimeError(f"Engine font {pending[future]}: {error}"))
    if failures:
        raise ExceptionGroup("Engine font resource verification failed", failures)
    actual.sort(key=lambda entry: entry["path"])
    if args.write_lock:
        LOCK.write_text(json.dumps({"formatVersion": 1, "origin": ORIGIN, "fonts": actual}, indent=2) + "\n", encoding="utf-8")
    destination = args.output_dir.resolve()
    destination.mkdir(parents=True, exist_ok=True)
    for entry in actual:
        target = destination / entry["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(CACHE / entry["path"], target)
    shutil.copyfile(LOCK, destination / "font-resources.json")
    shutil.copyfile(ROOT / "tools/licenses/flutter-roboto.txt", destination / "Roboto-LICENSE.txt")
    shutil.copyfile(ROOT / "apps/flutter/packages/operit_ui/assets/licenses/noto-sans-sc.txt", destination / "Noto-LICENSE.txt")
    print(f"Staged {len(actual)} verified engine fonts ({sum(row['bytes'] for row in actual):,} bytes)", flush=True)


if __name__ == "__main__":
    main()
