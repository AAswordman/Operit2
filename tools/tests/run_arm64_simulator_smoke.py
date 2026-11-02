#!/usr/bin/env python3
"""Exercise the production ARM64 bridge and root installer on an iOS simulator."""
import argparse
import io
import os
from pathlib import Path
import subprocess
import tarfile
import tempfile

REPO = Path(__file__).resolve().parents[2]


def run(command, **options):
    return subprocess.run([str(part) for part in command], check=True, cwd=REPO, **options)


def malformed_archive(path):
    with tarfile.open(path, 'w:gz') as archive:
        entry = tarfile.TarInfo('../escaped')
        entry.size = 7
        archive.addfile(entry, io.BytesIO(b'escaped'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--archive', type=Path, default=REPO / 'apps/flutter/app/ios/Runner/arm64-rootfs.tar.gz')
    parser.add_argument('--skip-build', action='store_true')
    options = parser.parse_args()
    archive = options.archive.resolve(strict=True)
    build = REPO / 'tools/ios-runtime/arm64/build/simulator-smoke'
    if not options.skip_build:
        run(['bash', REPO / 'tools/ios-runtime/arm64/build.sh', 'ios-simulator'],
            env={**os.environ, 'OPERIT_ARM64_BUILD_DIR': str(build), 'OPERIT_ARM64_JOBS': '6'})
    sdk = subprocess.check_output(['xcrun', '--sdk', 'iphonesimulator', '--show-sdk-path'], text=True).strip()
    simulator = os.environ.get('OPERIT_TEST_SIMULATOR', 'booted')
    with tempfile.TemporaryDirectory(prefix='operit-arm64-simulator-') as temporary:
        work = Path(temporary)
        executable = work / 'smoke'
        runner = REPO / 'apps/flutter/app/ios/Runner'
        run(['xcrun', '--sdk', 'iphonesimulator', 'clang++', '-std=c++17', '-fobjc-arc', '-O2',
             '-arch', 'arm64', '-mios-simulator-version-min=16.4', '-isysroot', sdk,
             '-I', REPO / 'hosts/ios/linux-runtime/include', '-I', runner,
             REPO / 'tools/tests/fixtures/arm64_simulator_smoke.mm', runner / 'Arm64RootFilesystem.mm',
             build / 'liboperit_arm64.a', '-lresolv', '-lz', '-framework', 'Foundation', '-o', executable])
        run(['codesign', '--force', '--sign', '-', executable])
        workspace = work / 'workspace'
        workspace.mkdir()
        (workspace / 'probe.txt').write_text('HOST_READ_OK')
        for _ in range(2):
            result = subprocess.run(['xcrun', 'simctl', 'spawn', simulator, str(executable), 'success',
                                     str(work / 'runtime'), str(archive), str(workspace)],
                                    capture_output=True, text=True, timeout=180)
            print(result.stderr, end='', flush=True)
            if result.returncode:
                raise SystemExit(result.returncode)
        bad = work / 'unsafe.tar.gz'
        malformed_archive(bad)
        rejected = subprocess.run(['xcrun', 'simctl', 'spawn', simulator, str(executable), 'prepare',
                                   str(work / 'unsafe-runtime'), str(bad), str(workspace)],
                                  capture_output=True, text=True, timeout=15)
        if rejected.returncode != 3 or (work / 'escaped').exists():
            raise SystemExit('Unsafe root archive was not rejected: ' + rejected.stderr)
        print('Root installer rejected path traversal; shell persistence verified across processes.')


if __name__ == '__main__':
    main()
