#!/usr/bin/env python3
"""Check blocked guest waiters wake after a gated asynchronous file write."""
import argparse
import os
from pathlib import Path
import platform
import shutil
import subprocess
import tempfile

from io_uring_async_test import tool


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", required=True, type=Path)
    parser.add_argument("--cpus", nargs="+", type=int, default=[1, 4])
    args = parser.parse_args()
    here = Path(__file__).resolve().parent
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    host_cc = tool("CC", ["clang", "cc"])
    failures = 0
    with tempfile.TemporaryDirectory(prefix="operit-io-uring-wait-") as directory:
        base = Path(directory)
        guest = base / "guest"
        preload = base / ("gate.dylib" if platform.system() == "Darwin" else "gate.so")
        subprocess.run([clang, "--target=aarch64-linux-gnu", "-O2", "-Wall", "-Wextra", "-Werror",
                        "-ffreestanding", "-nostdlib", "-fno-builtin", "-fno-stack-protector",
                        "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
                        "--ld-path=" + linker, "-Wl,--build-id=none", "-Wl,-z,max-page-size=4096",
                        "-static", "-Wl,-Ttext=0x100004000", str(here / "start.S"),
                        str(here / "io_uring_wait.c"), "-o", str(guest)], check=True)
        host_flags = ["-dynamiclib"] if platform.system() == "Darwin" else ["-shared", "-fPIC"]
        host_link = [] if platform.system() == "Darwin" else ["-ldl"]
        subprocess.run([host_cc, "-std=c11", "-Wall", "-Wextra", "-Werror", *host_flags,
                        str(here / "io_uring_slow_host.c"), "-o", str(preload), *host_link], check=True)
        for cpus in args.cpus:
            for mode in ("enter", "poll", "epoll"):
                root = base / f"root-{cpus}-{mode}"
                root.mkdir()
                shutil.copy(guest, root / "io_uring_wait")
                target = root / "io_uring_gate_target"
                target.write_bytes(b"abcdef")
                env = os.environ.copy()
                env["DYLD_INSERT_LIBRARIES" if platform.system() == "Darwin" else "LD_PRELOAD"] = str(preload)
                env.update({"OPERIT_IO_URING_GATE_TARGET": str(target),
                            "OPERIT_IO_URING_GATE_OPERATION": "write",
                            "OPERIT_IO_URING_GATE_ENTERED": str(root / "io_uring_gate_entered"),
                            "OPERIT_IO_URING_GATE_RELEASE": str(root / "io_uring_gate_release"),
                            "OPERIT_IO_URING_GATE_FINISHED": str(root / "io_uring_gate_finished"),
                            "OPERIT_IO_URING_GATE_TIMEOUT": str(root / "io_uring_gate_timeout")})
                try:
                    result = subprocess.run([str(args.runtime.resolve()), "--cpus", str(cpus), "--root", str(root),
                                             "--max-steps", "100000000", "--", "/io_uring_wait", mode],
                                            env=env, capture_output=True, text=True, timeout=15)
                except subprocess.TimeoutExpired:
                    failures += 1
                    print(f"FAIL cpus={cpus} {mode} timed out", flush=True)
                    continue
                markers = {name: (root / f"io_uring_{name}").exists() for name in
                           ("gate_entered", "submit_returned", "wait_started", "gate_release",
                            "gate_finished", "gate_timeout")}
                passed = (result.returncode == 0 and result.stdout == "io-uring-blocking-wakeup-ok\n" and
                          all(markers[name] for name in markers if name != "gate_timeout") and
                          not markers["gate_timeout"] and target.read_bytes() == b"UVWXYZ")
                print(f"{'PASS' if passed else 'FAIL'} cpus={cpus} {mode} exit={result.returncode} "
                      f"markers={markers}", flush=True)
                if not passed:
                    failures += 1
                    if result.stdout:
                        print("stdout:", result.stdout, end="")
                    if result.stderr:
                        print("stderr:", result.stderr, end="")
    return int(failures != 0)


if __name__ == "__main__":
    raise SystemExit(main())
