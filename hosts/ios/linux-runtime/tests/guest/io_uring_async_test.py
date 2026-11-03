#!/usr/bin/env python3
"""Run guest io_uring progress checks with a gated native file operation."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile


def tool(variable, candidates):
    for name in [os.environ.get(variable), *candidates]:
        if name and (Path(name).is_file() or shutil.which(name)):
            return shutil.which(name) or name
    raise RuntimeError(f"missing {variable}")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--cpus", nargs="+", type=int, default=[1, 4])
    parser.add_argument("--case", action="append", help="operation:scenario")
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    host_cc = tool("CC", ["clang", "cc"])
    cases = args.case or [f"{op}:ok" for op in ("read", "write", "readv", "writev", "fsync")]
    if not args.case:
        cases += [f"{op}:error" for op in ("read", "write", "readv", "writev", "fsync")]
        cases += ["write:close_fd", "write:close_ring", "write:exec", "readv:concurrent",
                  "read:unmap", "read:remap", "readv:unmap", "readv:remap"]
    failures = 0
    with tempfile.TemporaryDirectory(prefix="operit-io-uring-async-") as directory:
        base = Path(directory)
        guest = base / "guest"
        preload = base / ("gate.dylib" if platform.system() == "Darwin" else "gate.so")
        subprocess.run([clang, "--target=aarch64-linux-gnu", "-O2", "-Wall", "-Wextra", "-Werror",
                        "-ffreestanding", "-nostdlib",
                        "-fno-builtin", "-fno-stack-protector", "-mno-outline-atomics",
                        "-fno-vectorize", "-fno-slp-vectorize", "--ld-path=" + linker,
                        "-Wl,--build-id=none", "-Wl,-z,max-page-size=4096", "-static",
                        "-Wl,-Ttext=0x100004000", str(here / "start.S"),
                        str(here / "io_uring_async.c"), "-o", str(guest)], check=True)
        host_flags = ["-dynamiclib"] if platform.system() == "Darwin" else ["-shared", "-fPIC"]
        if platform.system() != "Darwin":
            host_flags += ["-ldl"]
        subprocess.run([host_cc, "-std=c11", "-Wall", "-Wextra", "-Werror", *host_flags,
                        str(here / "io_uring_slow_host.c"), "-o", str(preload)], check=True)
        for cpus in args.cpus:
            for case in cases:
                operation, scenario = case.split(":", 1)
                root = base / f"root-{cpus}-{operation}-{scenario}"
                root.mkdir()
                shutil.copy(guest, root / "io_uring_async")
                target = root / "io_uring_gate_target"
                target.write_bytes(b"abcdef")
                replacement = root / "io_uring_gate_replacement"
                replacement.write_bytes(b"replace")
                env = os.environ.copy()
                env["DYLD_INSERT_LIBRARIES" if platform.system() == "Darwin" else "LD_PRELOAD"] = str(preload)
                env.update({"OPERIT_IO_URING_GATE_TARGET": str(target),
                            "OPERIT_IO_URING_GATE_OPERATION": "read" if operation.startswith("read") else
                            "write" if operation.startswith("write") else "fsync",
                            "OPERIT_IO_URING_GATE_ENTERED": str(root / "io_uring_gate_entered"),
                            "OPERIT_IO_URING_GATE_RELEASE": str(root / "io_uring_gate_release"),
                            "OPERIT_IO_URING_GATE_FINISHED": str(root / "io_uring_gate_finished"),
                            "OPERIT_IO_URING_GATE_TIMEOUT": str(root / "io_uring_gate_timeout")})
                if scenario == "error":
                    env["OPERIT_IO_URING_GATE_ERROR"] = "1"
                try:
                    result = subprocess.run([str(args.runtime.resolve()), "--cpus", str(cpus), "--root", str(root),
                                             "--max-steps", "100000000", "--", "/io_uring_async", operation,
                                             scenario], env=env, capture_output=True, text=True, timeout=20)
                except subprocess.TimeoutExpired:
                    failures += 1
                    print(f"FAIL cpus={cpus} {case} timed out after 20 seconds", flush=True)
                    continue
                markers = {name: (root / f"io_uring_{name}").exists() for name in
                           ("gate_entered", "submit_returned", "gate_release", "gate_finished", "gate_timeout")}
                expected_target = b"UVWXYZ" if operation.startswith("write") and scenario != "error" else b"abcdef"
                passed = (result.returncode == 0 and result.stdout == "io-uring-background-progress-ok\n" and
                          markers["gate_entered"] and markers["submit_returned"] and markers["gate_release"] and
                          markers["gate_finished"] and not markers["gate_timeout"] and
                          target.read_bytes() == expected_target and replacement.read_bytes() == b"replace")
                print(f"{'PASS' if passed else 'FAIL'} cpus={cpus} {case} exit={result.returncode} "
                      f"markers={markers}", flush=True)
                if not passed:
                    failures += 1
                    if result.stdout:
                        print("stdout:", result.stdout, end="")
                    if result.stderr:
                        print("stderr:", result.stderr, end="")
    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
