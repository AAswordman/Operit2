"""Import or verify the independent handwritten v1 declaration snapshot."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[2]
DESTINATION = ROOT / "plugins" / "types-v1"


def digest(data: bytes) -> str:
    """Compute the recorded byte-level identity of a declaration."""
    return hashlib.sha256(data).hexdigest()


def check_snapshot() -> None:
    """Check provenance hashes without accessing another repository."""
    provenance = json.loads((DESTINATION / "source.json").read_text(encoding="utf-8"))
    expected = provenance["files"]
    actual = {path.name: digest(path.read_bytes()) for path in DESTINATION.glob("*.d.ts")}
    if actual != expected:
        names = sorted(set(actual) | set(expected))
        changed = [name for name in names if actual.get(name) != expected.get(name)]
        raise ValueError("v1 declaration snapshot differs from provenance: " + ", ".join(changed))
    print(f"Verified {len(actual)} v1 declarations from {provenance['commit']}")


def import_snapshot(source_repository: Path) -> None:
    """Import committed v1 declarations and apply the documented runtime alias extension."""
    repository = source_repository.resolve(strict=True)
    source = repository / "examples" / "types"
    paths = sorted(source.glob("*.d.ts"))
    if not paths or not (source / "index.d.ts").is_file():
        raise ValueError(f"Missing v1 declaration entry point: {source / 'index.d.ts'}")
    commit = subprocess.check_output(
        ["git", "-C", str(repository), "rev-parse", "HEAD"], text=True
    ).strip()
    dirty = subprocess.check_output(
        ["git", "-C", str(repository), "status", "--porcelain", "--", "examples/types"],
        text=True,
    ).strip()
    if dirty:
        raise ValueError("Commit v1 declaration changes before importing a reproducible snapshot")
    data = {
        path.name: subprocess.check_output(
            ["git", "-C", str(repository), "show", f"{commit}:examples/types/{path.name}"]
        )
        for path in paths
    }
    runtime_header = b"declare global {\n"
    runtime = data["quickjs-runtime.d.ts"]
    if not runtime.startswith(runtime_header):
        raise ValueError("Unsupported v1 runtime declaration header")
    data["quickjs-runtime.d.ts"] = runtime_header + (
        b"    /** Operit2 runtime extension: alias of the standard global object. */\n"
        b"    var global: typeof globalThis;\n\n"
    ) + runtime[len(runtime_header):]
    license_data = subprocess.check_output(
        ["git", "-C", str(repository), "show", f"{commit}:LICENSE"]
    )
    removed = {path.name for path in DESTINATION.glob("*.d.ts")} - set(data)
    if removed:
        raise ValueError("Review and explicitly remove obsolete v1 declarations: " + ", ".join(sorted(removed)))
    DESTINATION.mkdir(parents=True, exist_ok=True)
    for name, content in data.items():
        (DESTINATION / name).write_bytes(content)
    (DESTINATION / "LICENSE").write_bytes(license_data)
    provenance = {
        "repository": "assistance",
        "directory": "examples/types",
        "commit": commit,
        "files": {name: digest(content) for name, content in data.items()},
        "localExtensions": {
            "quickjs-runtime.d.ts": [
                "Declare global as an alias of globalThis supplied by the shared engine bootstrap."
            ]
        },
    }
    (DESTINATION / "source.json").write_text(
        json.dumps(provenance, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    check_snapshot()


def main() -> None:
    """Select an explicit import or local consistency check operation."""
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("check", help="verify the committed snapshot without the source repository")
    importer = commands.add_parser("import", help="import a clean, committed v1 type tree")
    importer.add_argument("source_repository", type=Path)
    args = parser.parse_args()
    if args.command == "check":
        check_snapshot()
    elif args.command == "import":
        import_snapshot(args.source_repository)
    else:
        raise ValueError(f"Unsupported operation: {args.command}")


if __name__ == "__main__":
    main()
