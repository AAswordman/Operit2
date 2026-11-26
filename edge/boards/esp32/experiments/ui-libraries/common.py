"""Pinned source preparation and native builds for the optional UI experiments."""
import base64
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

HERE = Path(__file__).resolve().parent
BOARD = HERE.parents[1]
ROOT = BOARD.parents[2]
BUILD = BOARD / ".embuild" / "ui-libraries"


def run(args, *, cwd=None):
    result = subprocess.run([str(arg) for arg in args], cwd=cwd,
                            capture_output=True, text=True, encoding="utf-8", errors="replace")
    if result.returncode:
        sys.stderr.write(result.stdout + result.stderr)
        result.check_returncode()
    return result.stdout


def prepare(name):
    """Reuse a verified cache, or download missing files through authenticated gh."""
    spec = json.loads((HERE / "sources.json").read_text(encoding="utf-8"))[name]
    directory = BOARD / ".embuild" / f"{name}-probe"
    for local, item in spec["files"].items():
        path = directory / local
        path.parent.mkdir(parents=True, exist_ok=True)
        if not path.exists():
            endpoint = (f"repos/{spec['repository']}/contents/{item['source']}"
                        f"?ref={spec['commit']}")
            response = json.loads(run(["gh", "api", endpoint]))
            data = base64.b64decode(response["content"])
            if hashlib.sha256(data).hexdigest() != item["sha256"]:
                raise RuntimeError(f"Download checksum mismatch: {path}")
            path.write_bytes(data)
        if hashlib.sha256(path.read_bytes()).hexdigest() != item["sha256"]:
            raise RuntimeError(f"Cached source checksum mismatch: {path}")
    if name == "ugui":
        for filename in ("system.h", "ugui_config.h"):
            shutil.copyfile(HERE / "ugui" / filename, directory / filename)
    return directory


def compile_c(output, sources, includes=(), flags=()):
    compiler = os.environ.get("CC", "gcc")
    args = [compiler, "-O2", "-std=c11"]
    # Match the packed glyph bitfields used by Xtensa GCC on MinGW hosts.
    if sys.platform == "win32":
        args.append("-mno-ms-bitfields")
    args.extend(f"-I{path}" for path in includes)
    args.extend(flags)
    args.extend(sources)
    args.extend(["-o", output])
    run(args)


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, ensure_ascii=False) + "\n", encoding="utf-8")
