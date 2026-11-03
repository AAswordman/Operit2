#!/usr/bin/env python3
"""Run the AArch64 ptrace tracer/tracee against an integrated runtime CLI."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent


def tool(environment, candidates):
    for candidate in [os.environ.get(environment), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError("missing compiler/linker: " + environment)


def run():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", required=True)
    args = parser.parse_args()
    runtime = str(Path(args.runtime).resolve())
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    with tempfile.TemporaryDirectory(prefix="operit-ptrace-guest-") as temporary:
        root = Path(temporary)
        subprocess.run([
            clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib", "-fno-builtin",
            "-fno-stack-protector", "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
            "--ld-path=" + linker, "-Wl,--build-id=none", "-Wl,-z,max-page-size=4096",
            "-I" + str(HERE / "guest"), "-static", "-no-pie", "-Wl,-Ttext=0x100004000",
            str(HERE / "guest/start.S"), str(HERE / "guest/ptrace.c"), "-o", str(root / "ptrace")
        ], check=True, timeout=30)
        for cpus in (1, 4):
            result = subprocess.run([
                runtime, "--root", str(root), "--cpus", str(cpus), "--max-steps", "5000000",
                "--memory-mib", "32", "--stats", "--", "/ptrace"
            ], capture_output=True, text=True, timeout=30)
            expected = "guest ptrace traceme/attach/registers/memory/syscall ok\n"
            if result.returncode != 0 or result.stdout != expected:
                raise AssertionError(
                    f"cpus={cpus} exit={result.returncode} stdout={result.stdout!r} stderr={result.stderr!r}")
            stats = json.loads(result.stderr.splitlines()[-1])
            if stats["status"] != 0 or stats["unsupported_syscalls"] != 0:
                raise AssertionError(f"cpus={cpus}: unexpected stats: {stats}")
            print(f"cpus={cpus}: ok (maximum_parallel_tasks={stats['maximum_parallel_tasks']})", flush=True)


if __name__ == "__main__":
    run()
