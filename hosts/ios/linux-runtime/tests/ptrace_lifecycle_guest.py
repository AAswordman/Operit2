#!/usr/bin/env python3
"""Build and run freestanding AArch64 ptrace lifecycle guests in a temporary rootfs."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
CASES = ("waitid", "handoff", "tracer-exit", "sigkill", "syscall", "exec", "exec-legacy",
         "exit-regular", "exit-group", "exit-sigkill", "follow-fork", "follow-tree",
         "follow-syscall", "follow-clone3", "follow-thread", "follow-exec",
         "follow-vfork-early", "follow-vfork-late")


def tool(environment, candidates):
    for candidate in [os.environ.get(environment), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            # Keep the ld.lld symlink name: lld chooses its driver from argv[0].
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError("missing compiler/linker: " + environment)


def build_guest(root, follow=False):
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    binary = root / ("ptrace_follow" if follow else "ptrace_lifecycle")
    sources = [HERE / "guest/start.S"]
    if follow:
        sources.append(HERE / "guest/threads.S")
    sources.append(HERE / ("guest/ptrace_follow.c" if follow else "guest/ptrace_lifecycle.c"))
    subprocess.run([
        clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib", "-fno-builtin",
        "-fno-stack-protector", "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
        "-Wall", "-Wextra", "-Werror", "--ld-path=" + linker, "-Wl,--build-id=none",
        "-Wl,-z,max-page-size=4096", "-I" + str(HERE / "guest"), "-static",
        "-Wl,-Ttext=0x100004000", *(str(source) for source in sources), "-o", str(binary),
    ], check=True, timeout=30)
    header = binary.read_bytes()[:20]
    if (len(header) != 20 or header[:6] != b"\x7fELF\x02\x01" or
            int.from_bytes(header[16:18], "little") != 2 or
            int.from_bytes(header[18:20], "little") != 183):
        raise RuntimeError("compiler did not produce a little-endian AArch64 executable")
    return binary


def positive_integer(value):
    parsed = int(value)
    if parsed < 1:
        raise argparse.ArgumentTypeError("must be positive")
    return parsed


def run_case(runtime, root, cpus, case, timeout, max_steps):
    command = [
        runtime, "--root", str(root), "--cpus", str(cpus), "--max-steps", str(max_steps),
        "--memory-mib", "32", "--stats", "--",
        "/ptrace_follow" if case.startswith("follow-") else "/ptrace_lifecycle", case,
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        return f"timeout after {timeout}s; stdout={error.stdout!r} stderr={error.stderr!r}", None
    expected = f"ptrace-lifecycle-{case}-ok\n"
    if result.returncode != 0 or result.stdout != expected:
        return f"exit={result.returncode} stdout={result.stdout!r} stderr={result.stderr!r}", None
    try:
        stats = json.loads(result.stderr.splitlines()[-1])
        if stats["status"] != 0 or stats["unsupported_syscalls"] != 0:
            return f"unexpected runtime stats: {stats}", None
    except (IndexError, KeyError, TypeError, ValueError) as error:
        return f"invalid runtime stats ({error}): {result.stderr!r}", None
    return None, stats.get("maximum_parallel_tasks", "unknown")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--runtime", help="path to an already-built runtime CLI")
    mode.add_argument("--compile-only", action="store_true", help="compile and inspect the guest, do not run it")
    parser.add_argument("--case", choices=CASES, action="append", help="case to run; repeatable (default: all)")
    parser.add_argument("--cpus", type=positive_integer, action="append", help="CPU count; repeatable (default: 1, 4)")
    parser.add_argument("--timeout", type=positive_integer, default=30, help="per-case timeout in seconds")
    parser.add_argument("--max-steps", type=positive_integer, default=50000000)
    args = parser.parse_args()

    runtime = None
    if args.runtime:
        runtime = str(Path(args.runtime).resolve())
        if not Path(runtime).is_file() or not os.access(runtime, os.X_OK):
            parser.error("--runtime must name an executable file")
    if args.compile_only and (args.case or args.cpus):
        parser.error("--case and --cpus require --runtime")

    with tempfile.TemporaryDirectory(prefix="operit-ptrace-lifecycle-") as temporary:
        root = Path(temporary)
        build_guest(root)
        build_guest(root, follow=True)
        print("AArch64 guest compiled in temporary rootfs", flush=True)
        if args.compile_only:
            print("compile only: guest was not run", flush=True)
            return 0
        failures = 0
        for cpus in args.cpus or (1, 4):
            for case in args.case or CASES:
                error, maximum_parallel_tasks = run_case(runtime, root, cpus, case, args.timeout, args.max_steps)
                if error is None:
                    print(f"cpus={cpus} {case}: ok (maximum_parallel_tasks={maximum_parallel_tasks})", flush=True)
                else:
                    failures += 1
                    print(f"cpus={cpus} {case}: FAIL {error}", file=sys.stderr, flush=True)
        return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
