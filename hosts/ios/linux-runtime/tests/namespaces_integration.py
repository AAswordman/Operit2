#!/usr/bin/env python3
"""Build and run the AArch64 namespace fixture against a wired runtime."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile


def tool(override, candidates):
    for value in (override, *candidates):
        if value and (Path(value).is_file() or shutil.which(value)):
            return os.path.abspath(shutil.which(value) or value)
    raise RuntimeError("missing compiler or linker")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, required=True)
    args = parser.parse_args()
    runtime = args.runtime.resolve(strict=True)
    guest = Path(__file__).resolve().parent / "guest"
    clang = tool(os.environ.get("OPERIT_AARCH64_CLANG"),
                 ("/opt/homebrew/opt/llvm/bin/clang", "clang"))
    linker = tool(os.environ.get("OPERIT_AARCH64_LLD"),
                  ("/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"))
    with tempfile.TemporaryDirectory(prefix="operit-guest-namespaces-") as directory:
        root = Path(directory)
        (root / "child_dir").mkdir()
        command = [clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib",
                   "-fno-builtin", "-fno-stack-protector", "-mno-outline-atomics",
                   "-fno-vectorize", "-fno-slp-vectorize", "--ld-path=" + linker,
                   "-Wl,--build-id=none", "-Wl,-z,max-page-size=4096", "-I" + str(guest),
                   "-static", "-Wl,-Ttext=0x100004000", str(guest / "start.S"),
                   str(guest / "namespaces.c"), str(guest / "namespaces.S"), "-o", str(root / "test")]
        subprocess.run(command, check=True, timeout=30)
        for cpus in (1, 2, 4):
            command = [str(runtime), "--cpus", str(cpus), "--root", str(root),
                       "--max-steps", "10000000", "--memory-mib", "32", "--", "/test"]
            result = subprocess.run(command, capture_output=True, text=True, timeout=30)
            if result.returncode or result.stdout != "namespaces-ok\n":
                raise RuntimeError(f"guest namespaces cpus={cpus}, exit={result.returncode}\n"
                                   f"stdout:\n{result.stdout}\nstderr:\n{result.stderr}")
            print(f"guest namespaces cpus={cpus}: passed")


if __name__ == "__main__":
    main()
