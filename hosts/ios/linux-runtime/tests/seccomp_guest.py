#!/usr/bin/env python3
"""Run real AArch64 seccomp guests against an already integrated runtime CLI."""
import argparse
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

HERE = Path(__file__).resolve().parent


def tool(environment, candidates):
    for candidate in [os.environ.get(environment), *candidates]:
        if candidate and (Path(candidate).is_file() or shutil.which(candidate)):
            # Preserve ld.lld's symlink name: the lld binary selects its driver
            # from argv[0], and resolving it to "lld" loses the ELF driver.
            return os.path.abspath(shutil.which(candidate) or candidate)
    raise RuntimeError("missing compiler/linker: " + environment)


def run():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", required=True)
    args = parser.parse_args()
    runtime = str(Path(args.runtime).resolve())
    clang = tool("OPERIT_AARCH64_CLANG", ["/opt/homebrew/opt/llvm/bin/clang", "clang"])
    linker = tool("OPERIT_AARCH64_LLD", ["/opt/homebrew/opt/lld/bin/ld.lld", "ld.lld"])
    expected = {
        "basic": "seccomp-basic-fork-ok\n",
        "threads": "seccomp-thread-inheritance-ok\n",
        "sync": "seccomp-tsync-propagation-ok\n",
        "sync-conflict": "seccomp-tsync-conflict-ok\n",
        "sync-race": "seccomp-tsync-race-ok\n",
        "exec": "seccomp-exec-inheritance-ok\n",
        "trap": "seccomp-trap-siginfo-ok\n",
        "kill-thread": "seccomp-kill-thread-ok\n",
        "kill-process": "seccomp-kill-process-ok\n",
        "exit": "seccomp-exit-blocked-ok\n",
    }
    with tempfile.TemporaryDirectory(prefix="operit-seccomp-guest-") as temporary:
        root = Path(temporary)
        subprocess.run([
            clang, "--target=aarch64-linux-gnu", "-O2", "-ffreestanding", "-nostdlib", "-fno-builtin",
            "-fno-stack-protector", "-mno-outline-atomics", "-fno-vectorize", "-fno-slp-vectorize",
            "--ld-path=" + linker, "-Wl,--build-id=none", "-Wl,-z,max-page-size=4096",
            "-I" + str(HERE / "guest"), "-static", "-Wl,-Ttext=0x100004000",
            str(HERE / "guest/start.S"), str(HERE / "guest/seccomp.c"), str(HERE / "guest/threads.S"),
            "-o", str(root / "seccomp")
        ], check=True, timeout=30)
        for cpus in (1, 4):
            for mode, output in expected.items():
                result = subprocess.run([
                    runtime, "--root", str(root), "--cpus", str(cpus), "--max-steps", "50000000",
                    "--", "/seccomp", mode
                ], capture_output=True, text=True, timeout=30)
                if result.returncode != 0 or result.stdout != output:
                    raise AssertionError(
                        f"cpus={cpus} mode={mode} exit={result.returncode} "
                        f"stdout={result.stdout!r} stderr={result.stderr!r}")
                if (root / "seccomp-escape").exists():
                    raise AssertionError(f"cpus={cpus} mode={mode}: filtered openat created a file")
                print(f"cpus={cpus} {mode}: ok", flush=True)


if __name__ == "__main__":
    run()
