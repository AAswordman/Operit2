#!/usr/bin/env python3
"""Run multithreaded AArch64 ptrace TRACEEXIT tests in a temporary rootfs."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
CASES = ("group", "individual", "leader", "fatal", "fatal-leader", "detach", "tracer-death")


def tool(environment, candidates):
    for candidate in [os.environ.get(environment), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            # lld selects its driver from argv[0], so preserve the symlink name.
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError("missing compiler/linker: " + environment)


def build_guest(root):
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    binary = root / "ptrace_threads_exit"
    subprocess.run([
        clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib", "-fno-builtin",
        "-fno-stack-protector", "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
        "-Wall", "-Wextra", "-Werror", "--ld-path=" + linker, "-Wl,--build-id=none",
        "-Wl,-z,max-page-size=4096", "-I" + str(HERE / "guest"), "-static",
        "-Wl,-Ttext=0x100004000", str(HERE / "guest/start.S"),
        str(HERE / "guest/threads.S"), str(HERE / "guest/ptrace_threads_exit.c"), "-o", str(binary),
    ], check=True, timeout=30)
    header = binary.read_bytes()[:20]
    if (len(header) != 20 or header[:6] != b"\x7fELF\x02\x01" or
            int.from_bytes(header[16:18], "little") != 2 or
            int.from_bytes(header[18:20], "little") != 183):
        raise RuntimeError("compiler did not produce a little-endian AArch64 executable")


def positive_integer(value):
    try:
        parsed = int(value)
    except ValueError as error:
        raise argparse.ArgumentTypeError("must be a positive integer") from error
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be a positive integer")
    return parsed


def run_case(runtime, root, cpus, case, timeout, max_steps):
    command = [runtime, "--root", str(root), "--cpus", str(cpus), "--max-steps", str(max_steps),
               "--memory-mib", "32", "--stats", "--", "/ptrace_threads_exit", case]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        print(f"cpus={cpus} {case}: TIMEOUT after {timeout}s; "
              f"stdout={error.stdout!r} stderr={error.stderr!r}", flush=True)
        return False
    lines = result.stderr.splitlines(keepends=True)
    stats = None
    guest_stderr = result.stderr
    if lines:
        try:
            stats = json.loads(lines[-1])
            guest_stderr = "".join(lines[:-1])
        except ValueError:
            pass
    ok = (result.returncode == 0 and result.stdout.endswith(f"{case}: PASS\n") and
          not guest_stderr and isinstance(stats, dict) and stats.get("status") == 0 and
          stats.get("unsupported_syscalls") == 0)
    print(f"cpus={cpus} {case}: {'PASS' if ok else 'FAIL'} "
          f"(exit={result.returncode}, stats={stats})", flush=True)
    if result.stdout:
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n", flush=True)
    if guest_stderr:
        print(f"stderr={guest_stderr!r}", flush=True)
    return ok


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--runtime", help="path to an already-built runtime CLI")
    mode.add_argument("--compile-only", action="store_true")
    parser.add_argument("--case", choices=CASES, action="append", help="case to run; repeatable")
    parser.add_argument("--cpus", type=positive_integer, action="append", help="CPU count; repeatable")
    parser.add_argument("--timeout", type=positive_integer, default=30)
    parser.add_argument("--max-steps", type=positive_integer, default=50000000)
    args = parser.parse_args()
    if args.compile_only and (args.case or args.cpus):
        parser.error("--case and --cpus require --runtime")
    runtime = None
    if args.runtime:
        runtime = str(Path(args.runtime).resolve())
        if not Path(runtime).is_file() or not os.access(runtime, os.X_OK):
            parser.error("--runtime must name an executable file")
    with tempfile.TemporaryDirectory(prefix="operit-ptrace-threads-exit-") as temporary:
        root = Path(temporary)
        build_guest(root)
        print("AArch64 guest compiled in temporary rootfs", flush=True)
        if args.compile_only:
            return 0
        failures = 0
        for cpus in args.cpus or (1, 4):
            for case in args.case or CASES:
                failures += not run_case(runtime, root, cpus, case, args.timeout, args.max_steps)
        print(f"summary: {failures} failed", flush=True)
        return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
