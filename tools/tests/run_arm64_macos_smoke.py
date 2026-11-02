#!/usr/bin/env python3
"""Exercise the macOS bridge with its bundled root and no host mounts."""
import argparse
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tempfile

REPO = Path(__file__).resolve().parents[2]


def run(command, **options):
    return subprocess.run([str(value) for value in command], cwd=REPO, check=True, **options)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--derived-dir', type=Path)
    options = parser.parse_args()
    with tempfile.TemporaryDirectory(prefix='operit-arm64-macos-') as temporary:
        work = Path(temporary)
        derived = options.derived_dir or work / 'derived'
        artifacts = derived / 'Arm64Runtime'
        if options.derived_dir is None:
            run(['bash', REPO / 'apps/flutter/app/macos/build_arm64_runtime.sh'],
                env={**os.environ, 'DERIVED_FILE_DIR': str(derived), 'ARCHS': 'arm64'})
        bundle = work / 'Smoke.app/Contents'
        executable = bundle / 'MacOS/smoke'
        executable.parent.mkdir(parents=True)
        resources = bundle / 'Resources'
        resources.mkdir()
        shutil.copyfile(artifacts / 'arm64-rootfs.tar.gz', resources / 'arm64-rootfs.tar.gz')
        (bundle / 'Info.plist').write_bytes(plistlib.dumps({
            'CFBundleExecutable': 'smoke', 'CFBundleIdentifier': 'com.operit.arm64-smoke',
            'CFBundlePackageType': 'APPL',
        }))
        runner = REPO / 'apps/flutter/app/macos/Runner'
        run(['xcrun', '--sdk', 'macosx', 'clang++', '-std=c++17', '-fobjc-arc', '-O2', '-arch', 'arm64',
             '-mmacosx-version-min=13.0', '-I', REPO / 'hosts/ios/linux-runtime/include', '-I', runner,
             REPO / 'tools/tests/fixtures/arm64_macos_smoke.mm', runner / 'Arm64TerminalBridge.mm',
             runner / 'Arm64RootFilesystem.mm', artifacts / 'liboperit_arm64.a',
             '-framework', 'Foundation', '-lz', '-lresolv', '-o', executable])
        for attempt in range(2):
            run([executable, work / 'support', str(attempt)], timeout=120)
        print('macOS native bridge, zero default host mounts, PTY, streaming and restart passed.')


if __name__ == '__main__':
    main()
