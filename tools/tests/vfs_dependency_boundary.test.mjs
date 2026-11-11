import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import test from 'node:test';

const root = new URL('../../', import.meta.url);

/** Reads one repository-owned architecture declaration. */
function source(path) {
  return readFileSync(new URL(path, root), 'utf8');
}

/** Extracts direct Operit dependencies before development-only dependencies. */
function dependencies(path) {
  return [...source(path).split(/\[target\.[^\n]+\.dev-dependencies\]/)[0]
    .matchAll(/^(operit-[\w-]+)\s*=/gm)].map(match => match[1]).sort();
}

test('VFS depends only on platform contracts and host assembly composes both layers', () => {
  assert.deepEqual(dependencies('core/crates/foundation/host-contract/Cargo.toml'), []);
  assert.deepEqual(dependencies('core/crates/foundation/vfs/Cargo.toml'), ['operit-host-contract']);
  assert.deepEqual(dependencies('core/crates/foundation/host-api/Cargo.toml'), ['operit-host-contract', 'operit-vfs']);
  for (const backend of ['filesystem', 'storage']) {
    assert.deepEqual(dependencies(`hosts/common/operit-host-native-${backend}/Cargo.toml`), ['operit-host-contract']);
  }
});

test('tools no longer own or reexport the VFS implementation', () => {
  assert.doesNotMatch(source('core/crates/foundation/vfs/src/lib.rs'), /pub mod PathMapper|pub use PathMapper::PathMapper/);
  for (const file of ['PathMapper', 'MountRegistry', 'VisualFileSystem']) {
    assert.equal(existsSync(new URL(`core/crates/tool/services/src/files/${file}.rs`, root)), false);
    assert.equal(existsSync(new URL(`core/crates/foundation/vfs/src/${file}.rs`, root)), true);
  }
  assert.doesNotMatch(source('core/crates/tool/services/src/files/mod.rs'), /PathMapper|VisualFileSystem|MountRegistry/);
});

test('migrated business modules use the owning VFS without native file access', () => {
  for (const file of [
    'core/crates/tool/services/src/tools/defaultTool/ToolGetter.rs',
    'core/crates/tool/services/src/tools/defaultTool/standard/StandardFileSystemTools.rs',
    'core/crates/tool/services/src/tools/defaultTool/standard/StandardMediaTools.rs',
    'core/crates/tool/services/src/tools/transfer/FileTransferTools.rs',
    'core/crates/runtime/application/src/services/WorkspaceService.rs',
  ]) {
    const production = source(file).split('#[cfg(test)]')[0];
    assert.doesNotMatch(production, /\bPathMapper\b|\bFileSystemHost\b|\.fileSystemHost\b|\.nativePath\(|\.physicalPath\b/, file);
  }
});

test('system and download hosts consume VFS file capabilities', () => {
  const contract = source('core/crates/foundation/host-contract/src/lib.rs');
  assert.match(contract, /fn installApp\(&self, source: &dyn FileSource/);
  assert.match(contract, /fn recognizeText\([\s\S]*?imagePath: &dyn FileSource/);
  assert.match(contract, /fn downloadToFile\([\s\S]*?_target: &dyn FileTarget/);
  assert.match(contract, /fn downloadFilesTo\(/);
  assert.match(contract, /fn startRuntimeProcessWithFiles\(/);
  assert.match(contract, /fn transcribeLocalFiles\(/);
  assert.match(contract, /fn synthesizeLocalFiles\(/);
  assert.match(contract, /fn synthesizeSpeechBytes\(/);
});

test('native resolver and physical storage roots are unavailable to Core consumers', () => {
  assert.match(source('core/crates/foundation/vfs/src/VisualFileSystem.rs'), /pub\(crate\) fn resolvePath\(/);
  assert.doesNotMatch(source('core/crates/foundation/vfs/src/lib.rs'), /pub use .*ResolvedVfsPath/);
  const paths = source('core/crates/persistence/store/src/RuntimeStorePaths.rs');
  assert.match(paths, /pub struct RuntimeStorePaths;/);
  assert.doesNotMatch(paths, /pub fn new\(|runtime_root:|workspace_root:/);
  assert.doesNotMatch(source('core/crates/foundation/util/src/RuntimeStoreRoot.rs'), /RuntimeStoreRootConfig|setRuntimeStoreRootConfig/);
  const guard = source('core/crates/tool/services/build_support/native_filesystem.rs');
  for (const method of ['nativePath', 'resolvePath', 'runtimeRootDir', 'workspaceRootDir']) {
    assert.ok(guard.includes(`"${method}"`));
  }
  const exemptions = guard.slice(guard.indexOf('const NATIVE_MODULES'), guard.indexOf('#[derive(Default)]'));
  assert.doesNotMatch(exemptions, /core\/crates\/(runtime|provider|plugin|tool)\//);
});

test('native local inference and generated audio stay within platform hosts', () => {
  assert.equal(existsSync(new URL('core/crates/provider/local-model/build.rs', root)), false);
  assert.equal(existsSync(new URL('hosts/common/operit-host-local-inference/native/inference.cpp', root)), true);
  const provider = source('core/crates/provider/local-model/src/LocalModelProvider.rs');
  assert.doesNotMatch(provider, /std::process|Command::|runtimeRootDir\(|workspaceRootDir\(/);
  assert.match(provider, /VfsPath::joinVfsPath/);
  const voice = source('core/crates/provider/services/src/tts/SystemVoiceProvider.rs');
  assert.match(voice, /synthesizeSpeechBytes/);
  assert.doesNotMatch(voice, /audioPath|resolvePath|nativePath/);
});
