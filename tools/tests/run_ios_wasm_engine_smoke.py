#!/usr/bin/env python3
"""Build and run the production Host/ToolPkg WASM path with the verified iOS V8 artifacts."""
import argparse
import os
from pathlib import Path
import subprocess
import sys

REPO = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(REPO / 'tools/build_scripts'))
import prepare_ios_v8 as ios_v8


def run(command, **options):
    print('+ ' + ' '.join(map(str, command)), flush=True)
    return subprocess.run(list(map(str, command)), cwd=REPO, check=True, **options)


def transpile_adapter():
    typescript = REPO / 'plugins/packages/buildin/character_cards/node_modules/typescript'
    return subprocess.check_output(['node', '-e', """const fs = require('fs');
const ts = require(process.argv[1]);
const source = fs.readFileSync(process.argv[2], 'utf8');
process.stdout.write(ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2020, module: ts.ModuleKind.CommonJS }}).outputText);
""", str(typescript), str(REPO / 'plugins/packages/buildin/character_cards/src/search/usearch.ts')], cwd=REPO, text=True)


def prepare_script(work):
    script = work / 'smoke.js'
    script.write_text('(function(){ var exports = {};\n' + transpile_adapter() + '\nglobalThis.VectorIndex = exports.VectorIndex; })();\n' +
                      (REPO / 'tools/tests/fixtures/ios_wasm_engine_smoke.js').read_text())
    return script


def baseline_node(work):
    generator = work / 'module-generator'
    (generator / 'src').mkdir(parents=True, exist_ok=True)
    (generator / 'Cargo.toml').write_text('[package]\nname="operit-wasm-smoke-modules"\nversion="0.0.0"\nedition="2021"\n[workspace]\n[dependencies]\nwasm-encoder="=0.245.1"\n')
    (generator / 'src/wasm_engine_modules.rs').write_bytes((REPO / 'tools/tests/fixtures/wasm_engine_modules.rs').read_bytes())
    (generator / 'src/main.rs').write_text('mod wasm_engine_modules; fn main(){ let root=std::env::args().nth(1).unwrap(); for (name,bytes) in [("scalar",wasm_engine_modules::scalar_module()),("references",wasm_engine_modules::reference_module())] { std::fs::write(format!("{root}/{name}.wasm"),bytes).unwrap(); }}')
    run(['cargo', 'run', '--manifest-path', generator / 'Cargo.toml', '--', work])
    script = prepare_script(work)
    run(['node', '--expose-gc', '-e', """const fs=require('fs'), vm=require('vm'), path=require('path');
const [root,work,script]=process.argv.slice(1);
global.__operitWasmFixture=name=>({bytes:new Uint8Array(fs.readFileSync(path.join(work,name+'.wasm')))});
const packageRoot=path.join(root,'plugins/packages/buildin/character_cards');
global.__operitNativeLoadToolPkgWasm=(target,name)=>{ if(target!=='character_cards'||name!=='usearch')throw Error('Wrong fixture'); return {bytes:new Uint8Array(fs.readFileSync(path.join(packageRoot,'resources/usearch.wasm'))),metadata:{exports:JSON.parse(fs.readFileSync(path.join(packageRoot,'manifest.json'))).wasm_modules[0].exports}}; };
vm.runInThisContext(fs.readFileSync(path.join(root,'core/crates/plugin/sdk/src/toolpkg/ToolPkgWasmApi.js'),'utf8'));
vm.runInThisContext(fs.readFileSync(script,'utf8'));
setImmediate(()=>{ console.log(JSON.stringify(global.__wasmSmokeResult)); if(global.__wasmSmokeResult?.status!=='passed')process.exitCode=1; });
""", REPO, work, script])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--target', choices=('aarch64-apple-ios-sim', 'aarch64-apple-ios'), default='aarch64-apple-ios-sim')
    parser.add_argument('--skip-build', action='store_true')
    parser.add_argument('--baseline-node', action='store_true', help='validate the same modules and production adapter on Node before testing iOS')
    options = parser.parse_args()
    work = REPO / 'tools/release/work/ios-wasm-engine-smoke'
    work.mkdir(parents=True, exist_ok=True)
    if options.baseline_node:
        baseline_node(work)
        return
    version = ios_v8.version_from_lock()
    directory = ios_v8.CACHE_ROOT / version / options.target
    config = ios_v8.configuration(version, options.target)
    if not ios_v8.cache_valid(directory, config):
        raise SystemExit('Build the matching reviewed iOS Wasm interpreter before running this check')
    work = REPO / 'tools/release/work/ios-wasm-engine-smoke'
    work.mkdir(parents=True, exist_ok=True)
    crate = work / 'probe'
    (crate / 'src').mkdir(parents=True, exist_ok=True)
    (crate / 'Cargo.toml').write_text(f'''[package]
name="operit-ios-wasm-engine-smoke"
version="0.0.0"
edition="2021"
[lib]
crate-type=["staticlib"]
[workspace]
[dependencies]
v8="={version}"
serde_json="1"
wasm-encoder="=0.245.1"
operit-host-api={{path="{REPO}/core/crates/foundation/host-api"}}
operit-host-native-scheduler={{path="{REPO}/hosts/common/operit-host-native-scheduler"}}
''')
    (crate / 'src/lib.rs').write_bytes((REPO / 'tools/tests/fixtures/ios_wasm_engine_smoke.rs').read_bytes())
    (crate / 'src/wasm_engine_modules.rs').write_bytes((REPO / 'tools/tests/fixtures/wasm_engine_modules.rs').read_bytes())
    environment = {**os.environ, 'OPERIT_REPO_ROOT': str(REPO),
                   'RUSTY_V8_ARCHIVE': str(directory / 'librusty_v8.a'),
                   'RUSTY_V8_ARCHIVE_SHA256': ios_v8.digest(directory / 'librusty_v8.a'),
                   'RUSTY_V8_SRC_BINDING_PATH': str(directory / 'src_binding.rs')}
    for key in ('V8_FROM_SOURCE', 'DOCS_RS', 'V8_FORCE_DEBUG', 'GN_ARGS', 'EXTRA_GN_ARGS'):
        environment.pop(key, None)
    if not options.skip_build:
        run(['cargo', 'build', '--manifest-path', crate / 'Cargo.toml', '--release', '--target', options.target, '--jobs', '6'], env=environment)
    device = options.target == 'aarch64-apple-ios'
    sdk_name = 'iphoneos' if device else 'iphonesimulator'
    sdk = subprocess.check_output(['xcrun', '--sdk', sdk_name, '--show-sdk-path'], text=True).strip()
    executable = work / ('smoke-device' if device else 'smoke')
    library = crate / 'target' / options.target / 'release/liboperit_ios_wasm_engine_smoke.a'
    minimum_os = '-miphoneos-version-min=14.0' if device else '-mios-simulator-version-min=14.0'
    run(['xcrun', '--sdk', sdk_name, 'clang++', '-arch', 'arm64', minimum_os,
         '-isysroot', sdk, REPO / 'tools/tests/fixtures/ios_wasm_engine_smoke.mm', library,
         '-framework', 'Foundation', '-framework', 'Security', '-liconv', '-lresolv', '-o', executable])
    # Linking catches native ABI/symbol failures that a static archive build cannot.
    # Device execution still requires a signed runner on an attached device.
    if device:
        print('Device Host engine probe compiled and linked; signed device execution remains a separate check.')
        return
    script = prepare_script(work)
    run(['codesign', '--force', '--sign', '-', executable])
    simulator = os.environ.get('OPERIT_TEST_SIMULATOR', 'booted')
    for _ in range(2):
        run(['xcrun', 'simctl', 'spawn', simulator, executable, script], timeout=120)


if __name__ == '__main__':
    main()
