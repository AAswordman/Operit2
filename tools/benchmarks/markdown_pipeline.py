#!/usr/bin/env python3
"""Run production Rust/Dart Markdown transport + node benchmarks on macOS."""
import argparse
import json
import os
from pathlib import Path
import platform
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[2]
APP = ROOT / 'apps/flutter/app'
MANIFEST = ROOT / 'apps/flutter/native/operit-flutter-bridge/tests/markdown-bench/Cargo.toml'


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, default=APP / 'build/markdown_benchmark')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('This runner currently targets the macOS native Flutter app.')
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    print('Building the release fixture with production FFI/stream/codec modules...', flush=True)
    build_log = output / 'native_build.log'
    with build_log.open('w') as errors:
        build = subprocess.run([
            'cargo', 'build', '--release', '--locked', '--manifest-path', str(MANIFEST),
            '--target-dir', str(ROOT / 'core/target'), '--message-format=json',
        ], cwd=ROOT, text=True, stdout=subprocess.PIPE, stderr=errors)
    if build.returncode:
        raise SystemExit(f'Native build failed; see {build_log}')
    artifacts = [json.loads(line) for line in build.stdout.splitlines() if line.startswith('{')]
    libraries = [Path(name) for item in artifacts
                 if item.get('reason') == 'compiler-artifact'
                 and item.get('target', {}).get('name') == 'operit_markdown_pipeline_bench'
                 for name in item.get('filenames', []) if name.endswith('.dylib')]
    if len(libraries) != 1:
        raise SystemExit('Expected exactly one native benchmark dylib')
    env = dict(os.environ, CI='true', OPERIT_MARKDOWN_BENCH_OUTPUT=str(output))
    # Keep host architecture selection local to this benchmark. No project or
    # SDK configuration is edited, and the fixture matches the executing host.
    xcconfig = output / 'host_arch.xcconfig'
    arch = 'arm64' if platform.machine() == 'arm64' else 'x86_64'
    excluded = 'x86_64' if arch == 'arm64' else 'arm64'
    xcconfig.write_text(f'ARCHS = {arch}\nONLY_ACTIVE_ARCH = YES\nEXCLUDED_ARCHS = {excluded}\n')
    env['XCODE_XCCONFIG_FILE'] = str(xcconfig)
    log = output / 'flutter_profile.log'
    print(f'Running Flutter profile benchmark; build/run log: {log}', flush=True)
    with log.open('w') as stream:
        run = subprocess.run([
            'flutter', 'drive', '--profile', '--ci', '--no-pub', '-d', 'macos',
            '--driver=test_driver/markdown_pipeline_driver.dart',
            '--target=integration_test/markdown_pipeline_performance_test.dart',
            f'--dart-define=OPERIT_MARKDOWN_BENCH_LIBRARY={libraries[0]}',
        ], cwd=APP, env=env, stdout=stream, stderr=subprocess.STDOUT)
    if run.returncode:
        raise SystemExit(f'Flutter benchmark failed; see {log}')
    report = output / 'markdown_pipeline.json'
    data = json.loads(report.read_text())
    if data.get('mode') != 'profile' or data.get('nativeMode') != 'release':
        raise SystemExit(f'Unexpected build modes in {report}')
    data['host'] = {
        'os': platform.platform(), 'arch': platform.machine(),
        'cpu': subprocess.check_output(['sysctl', '-n', 'machdep.cpu.brand_string'], text=True).strip(),
        'memoryBytes': int(subprocess.check_output(['sysctl', '-n', 'hw.memsize'], text=True)),
        'rustc': subprocess.check_output(['rustc', '--version'], text=True).strip(),
    }
    report.write_text(json.dumps(data, ensure_ascii=False, indent=2) + '\n')
    print('bytes\tchunk_chars\tevents\tmedian_wall_ms')
    for sample in data['samples']:
        print(f"{sample['bytes']}\t{sample['chunkChars']}\t{sample['events']}\t{sample['medianWallMs']:.3f}")
    print(f'Complete report: {report}')


if __name__ == '__main__':
    main()
