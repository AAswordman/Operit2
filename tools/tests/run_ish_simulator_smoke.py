#!/usr/bin/env python3
"""Run real iSH boot, PTY/shell and failed-mount paths on a booted simulator.

Requires current Debug arm64 iSH libraries; does not build or install the app.
All root files and executables live in a disposable temporary directory.
"""
import os
from pathlib import Path
import platform
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[2]


def main():
    if platform.system() != "Darwin" or platform.machine() != "arm64":
        raise SystemExit("This smoke test requires an Apple Silicon Mac and a booted iOS simulator")
    products = REPO / "apps/flutter/app/apple/ish-build/Debug-iphonesimulator"
    # The build script verifies the complete source signature before reusing it.
    subprocess.run(["bash", str(REPO / "tools/ios-runtime/ish/build_ish_ios.sh"),
                    "Debug", "iphonesimulator", "iphonesimulator", "arm64"], check=True, cwd=REPO)
    sdk = subprocess.check_output(["xcrun", "--sdk", "iphonesimulator", "--show-sdk-path"], text=True).strip()
    with tempfile.TemporaryDirectory(prefix="operit-ish-simulator-") as temporary:
        work = Path(temporary)
        executable = work / "smoke"
        runner = REPO / "apps/flutter/app/ios/Runner"
        source = REPO / "tools/ios-runtime/ish/sources/ish"
        subprocess.run([
            "xcrun", "--sdk", "iphonesimulator", "clang++", "-std=c++17", "-fobjc-arc",
            "-arch", "arm64", "-mios-simulator-version-min=16.4", "-isysroot", sdk,
            "-I", str(source), "-I", str(source / "app"), "-I", str(runner),
            str(REPO / "tools/tests/fixtures/ish_simulator_smoke.mm"),
            str(runner / "IshRootFilesystem.mm"),
            "-Wl,-ld_classic", "-Wl,-sectalign,__DATA,__percpu_first,1000",
            "-Wl,-sectalign,__DATA,__tracepoints,20", "-L", str(products),
            "-Wl,-force_load," + str(products / "liblinux.a"),
            "-Wl,-force_load," + str(products / "libiSHLinux.a"),
            "-liSHLinuxUser", "-liSHFakefs", "-liSHFchdir", "-lfakefs", "-lish_emu",
            "-larchive", "-lbz2", "-liconv", "-lresolv", "-lsqlite3", "-lz",
            "-framework", "Foundation", "-framework", "UIKit", "-o", str(executable),
        ], check=True)
        subprocess.run(["codesign", "--force", "--sign", "-", str(executable)], check=True)
        for scenario, runtime in (("success", "runtime"), ("success", "runtime"), ("fail", "failed-runtime")):
            result = subprocess.run([
                "xcrun", "simctl", "spawn", os.environ.get("OPERIT_TEST_SIMULATOR", "booted"),
                str(executable), scenario, str(work / runtime), str(runner / "ish-root.tar.gz"),
            ], capture_output=True, text=True, timeout=60)
            if result.returncode != 0:
                raise SystemExit(result.stdout + result.stderr)
            print(f"{scenario}: passed")
            for line in result.stderr.splitlines():
                if line.startswith(("BOOT_RESULT=", "FAILURE_RETURNED_WITHOUT_PANIC:", "SHELL_RESULT=")):
                    print(line)


if __name__ == "__main__":
    main()
