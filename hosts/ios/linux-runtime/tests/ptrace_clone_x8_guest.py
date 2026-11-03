#!/usr/bin/env python3
"""Test ptrace clone/clone3 entry-stop x8 edits in a temporary AArch64 rootfs."""

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile

HERE = Path(__file__).resolve().parent
CASES = ("clone-control", "clone-mutated", "clone3-control", "clone3-mutated")


def tool(environment, candidates):
    for candidate in [os.environ.get(environment), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            # lld selects its driver from argv[0], so keep the symlink name.
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError("missing compiler/linker: " + environment)


def build_guest(root):
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    binary = root / "ptrace_clone_x8"
    subprocess.run([
        clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib", "-fno-builtin",
        "-fno-stack-protector", "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
        "-Wall", "-Wextra", "-Werror", "--ld-path=" + linker, "-Wl,--build-id=none",
        "-Wl,-z,max-page-size=4096", "-I" + str(HERE / "guest"), "-static",
        "-Wl,-Ttext=0x100004000", str(HERE / "guest/start.S"),
        str(HERE / "guest/ptrace_clone_x8.c"), "-o", str(binary),
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
    command = [
        runtime, "--root", str(root), "--cpus", str(cpus), "--max-steps", str(max_steps),
        "--memory-mib", "32", "--stats", "--", "/ptrace_clone_x8", case,
    ]
    try:
        result = subprocess.run(command, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        print(f"cpus={cpus} {case}: TIMEOUT after {timeout}s")
        print(f"stdout={error.stdout!r} stderr={error.stderr!r}")
        return False

    stats = None
    guest_stderr = result.stderr
    lines = result.stderr.splitlines(keepends=True)
    if lines:
        try:
            stats = json.loads(lines[-1])
            guest_stderr = "".join(lines[:-1])
        except (TypeError, ValueError):
            pass
    stats_ok = (isinstance(stats, dict) and stats.get("status") == 0 and
                stats.get("unsupported_syscalls") == 0)
    ok = (result.returncode == 0 and result.stdout.endswith(f"{case}: PASS\n") and
          not guest_stderr and stats_ok)
    status = stats.get("status") if isinstance(stats, dict) else None
    unsupported = stats.get("unsupported_syscalls") if isinstance(stats, dict) else None
    print(f"cpus={cpus} {case}: {'PASS' if ok else 'FAIL'} "
          f"(exit={result.returncode}, status={status}, unsupported_syscalls={unsupported})")
    if result.stdout:
        print(result.stdout, end="" if result.stdout.endswith("\n") else "\n")
    if guest_stderr:
        print(f"stderr={guest_stderr!r}")
    if stats is None:
        print("missing or invalid runtime stats")
    return ok


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    mode = parser.add_mutually_exclusive_group(required=True)
    mode.add_argument("--runtime", help="path to an already-built runtime CLI")
    mode.add_argument("--compile-only", action="store_true", help="compile the guest without running it")
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

    with tempfile.TemporaryDirectory(prefix="operit-ptrace-clone-x8-") as temporary:
        root = Path(temporary)
        build_guest(root)
        print("AArch64 guest compiled in temporary rootfs", flush=True)
        if args.compile_only:
            print("compile only: guest was not run")
            return 0
        failures = 0
        for cpus in args.cpus or (1, 4):
            for case in args.case or CASES:
                if not run_case(runtime, root, cpus, case, args.timeout, args.max_steps):
                    failures += 1
        print(f"summary: {failures} failed")
        return 1 if failures else 0


if __name__ == "__main__":
    sys.exit(main())
