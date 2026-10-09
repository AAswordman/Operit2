import 'dart:io';

import 'package:crypto/crypto.dart';
import 'package:hooks/hooks.dart';

import 'verified_asset_download.dart';

const String _v86PackageVersion = '0.5.424';
const String _v86RuntimeAssetBaseUrl =
    'https://models.operit.app/v86-runtime/i686-buildroot-node20-python312-20260720/';

const List<_V86GuestAsset> _v86GuestAssets = <_V86GuestAsset>[
  _V86GuestAsset(
    relativePath: 'v86/seabios.bin',
    url: '${_v86RuntimeAssetBaseUrl}seabios.bin',
    sha256: '73e3f359102e3a9982c35fce98eb7cd08f18303ac7f1ba6ebfbe6cdc1c244d98',
  ),
  _V86GuestAsset(
    relativePath: 'v86/vgabios.bin',
    url: '${_v86RuntimeAssetBaseUrl}vgabios.bin',
    sha256: 'a4bc0d80cc3ca028c73dafa8fee396b8d054ce87ebd8abfbd31b06b437607880',
  ),
];

/// Builds platform assets and stages the pinned browser runtime dependencies.
void main(List<String> args) async {
  await build(args, (input, output) async {
    final packageRoot = Directory.fromUri(input.packageRoot);
    final repoRoot = Directory.fromUri(input.packageRoot.resolve('../../../'));
    final pluginsRoot = Directory.fromUri(
      input.packageRoot.resolve('../../../plugins/'),
    );
    final syncScript = File.fromUri(
      input.packageRoot.resolve(
        '../../../plugins/tools/sync_plugin_packages.py',
      ),
    );
    final bridgeCrate = Directory.fromUri(
      input.packageRoot.resolve('../native/operit-flutter-bridge/'),
    );
    final coreRoot = Directory.fromUri(
      input.packageRoot.resolve('../../../core/'),
    );
    final webHostRoot = Directory.fromUri(
      input.packageRoot.resolve('../../../hosts/web/'),
    );
    final ohosHostRoot = Directory.fromUri(
      input.packageRoot.resolve('../../../hosts/ohos/'),
    );
    final webSourceDir = Directory.fromUri(input.packageRoot.resolve('web/'));
    final webRuntimeSourceDir = Directory.fromUri(
      input.packageRoot.resolve('web/runtime/src/'),
    );
    final webRuntimeTypescriptConfig = File.fromUri(
      input.packageRoot.resolve('web/runtime/src/tsconfig.json'),
    );
    final webBuildDir = Directory.fromUri(
      input.packageRoot.resolve('.dart_tool/web-runtime-build/'),
    );
    final depsDir = Directory.fromUri(
      input.packageRoot.resolve('.dart_tool/web-build-deps/'),
    );
    final wasmSource = File.fromUri(
      bridgeCrate.uri.resolve(
        'target/wasm32-unknown-unknown/release/operit_flutter_bridge.wasm',
      ),
    );
    final sqlDist = Directory.fromUri(
      depsDir.uri.resolve('node_modules/sql.js/dist/'),
    );
    final targetOs = _targetOs(input);
    final isWebTarget = targetOs == 'web';
    final shouldBuildWebAssets = isWebTarget;

    await _addDirectoryFileDependencies(output, pluginsRoot, {
      '.js',
      '.json',
      '.hjson',
      '.ts',
      '.d.ts',
      '.py',
    }, excludeGeneratedOutputs: true);
    await _addRustDependencies(output, bridgeCrate);
    await _addRustDependencies(output, coreRoot);
    await _addRustDependencies(output, webHostRoot);
    await _addRustDependencies(output, ohosHostRoot);
    await _addDirectoryFileDependencies(output, webSourceDir, {
      '.html',
      '.ico',
      '.js',
      '.json',
      '.png',
      '.wasm',
    }, excludeGeneratedOutputs: true);
    await _addDirectoryFileDependencies(output, webRuntimeSourceDir, {'.ts'});
    output.dependencies.add(webRuntimeTypescriptConfig.uri);

    await _run(_pythonExecutable(repoRoot), [
      syncScript.path,
      '--source',
      'buildin',
      '--no-hot-reload',
    ], workingDirectory: repoRoot.path);

    if (shouldBuildWebAssets) {
      await _invalidateWebRuntimeArtifacts([webBuildDir]);
      await _run(
        'cargo',
        const ['build', '--release', '--target', 'wasm32-unknown-unknown'],
        workingDirectory: bridgeCrate.path,
        environment: await _wasmCargoEnvironment(repoRoot),
      );

      await _run(
        Platform.isWindows ? 'wasm-bindgen.exe' : 'wasm-bindgen',
        [
          '--target',
          'web',
          '--out-dir',
          webBuildDir.path,
          '--out-name',
          'operit_flutter_bridge',
          wasmSource.path,
        ],
        workingDirectory: packageRoot.path,
      );
      await _validateWasmBindgenImports(
        File.fromUri(webBuildDir.uri.resolve('operit_flutter_bridge.js')),
      );
      await _writeWorkerWasmBridgeModule(
        File.fromUri(webBuildDir.uri.resolve('operit_flutter_bridge.js')),
        File.fromUri(
          webBuildDir.uri.resolve('operit_flutter_bridge_worker.js'),
        ),
      );
      await _writeWorkerMessagePackModule(
        File.fromUri(webSourceDir.uri.resolve('runtime/vendor/msgpack.min.js')),
        <File>[
          File.fromUri(webBuildDir.uri.resolve('operit_messagepack.js')),
          File.fromUri(
            webSourceDir.uri.resolve('runtime/generated/operit_messagepack.js'),
          ),
        ],
      );

      await _run(_command('npm'), [
        'install',
        '--silent',
        '--no-audit',
        '--no-fund',
        '--prefix',
        depsDir.path,
        'sql.js@1.14.1',
        'typescript@5.9.3',
        'v86@$_v86PackageVersion',
        'tesseract.js@6.0.1',
        'tesseract.js-core@6.0.0',
        '@tesseract.js-data/eng@1.0.0',
        '@tesseract.js-data/chi_sim@1.0.0',
        '@tesseract.js-data/jpn@1.0.0',
        '@tesseract.js-data/kor@1.0.0',
      ], workingDirectory: packageRoot.path);

      await _compileWebRuntimeBridge(
        depsDir,
        webRuntimeTypescriptConfig,
        webBuildDir,
        packageRoot,
      );

      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.js'),
      ).copy(File.fromUri(webBuildDir.uri.resolve('sql-wasm.js')).path);
      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.wasm'),
      ).copy(File.fromUri(webBuildDir.uri.resolve('sql-wasm.wasm')).path);
      await _stageV86RuntimeAssets(depsDir, webBuildDir);
      await _stageBrowserOcrAssets(depsDir, webBuildDir);
      await _syncWebRuntimeArtifacts(
        webBuildDir,
        Directory.fromUri(webSourceDir.uri.resolve('runtime/generated/')),
      );
    }
  });
}

/// Describes one immutable Linux guest artifact used by the browser VM.
class _V86GuestAsset {
  const _V86GuestAsset({
    required this.relativePath,
    required this.url,
    required this.sha256,
  });

  final String relativePath;
  final String url;
  final String sha256;
}

Future<void> _addDirectoryFileDependencies(
  BuildOutputBuilder output,
  Directory root,
  Set<String> extensions, {
  bool excludeGeneratedOutputs = false,
}) async {
  if (!root.existsSync()) {
    throw StateError('Dependency root does not exist: ${root.path}');
  }
  await for (final entity in root.list(recursive: true, followLinks: false)) {
    if (entity is! File) {
      continue;
    }
    final path = entity.path;
    if (path.contains(
      '${Platform.pathSeparator}node_modules${Platform.pathSeparator}',
    )) {
      continue;
    }
    if (excludeGeneratedOutputs && _isGeneratedInputDependency(entity)) {
      continue;
    }
    if (extensions.any(path.endsWith)) {
      output.dependencies.add(entity.uri);
    }
  }
}

/// Detects build-owned outputs that must not be registered as hook inputs.
bool _isGeneratedInputDependency(File file) {
  final path = file.path;
  final separator = Platform.pathSeparator;
  if (path.contains('$separator.out$separator')) {
    return true;
  }
  final fileName = file.uri.pathSegments.isEmpty
      ? path
      : file.uri.pathSegments.last;
  if (fileName == '.sync_state.json' ||
      fileName == '.sync_hot_reload_state.json') {
    return true;
  }
  return _generatedWebRuntimeFileNames.contains(fileName);
}

/// Compiles the browser runtime bridge required by the Flutter Web shell.
Future<void> _compileWebRuntimeBridge(
  Directory dependencies,
  File typescriptConfig,
  Directory outputDirectory,
  Directory workingDirectory,
) async {
  final executable = Platform.isWindows ? 'tsc.cmd' : 'tsc';
  final compiler = File.fromUri(
    dependencies.uri.resolve('node_modules/.bin/$executable'),
  );
  if (!compiler.existsSync()) {
    throw StateError('TypeScript compiler does not exist: ${compiler.path}');
  }
  await _run(compiler.path, [
    '-p',
    typescriptConfig.path,
    '--outDir',
    outputDirectory.path,
  ], workingDirectory: workingDirectory.path);
}

Future<void> _addRustDependencies(
  BuildOutputBuilder output,
  Directory root,
) async {
  if (!root.existsSync()) {
    throw StateError('Rust dependency root does not exist: ${root.path}');
  }
  await for (final entity in root.list(recursive: true, followLinks: false)) {
    if (entity is! File) {
      continue;
    }
    final path = entity.path;
    if (path.contains(
      '${Platform.pathSeparator}target${Platform.pathSeparator}',
    )) {
      continue;
    }
    if (path.endsWith('.rs') ||
        path.endsWith('Cargo.toml') ||
        path.endsWith('Cargo.lock')) {
      output.dependencies.add(entity.uri);
    }
  }
}

/// Copies generated browser runtime files into the Flutter Web source tree.
Future<void> _syncWebRuntimeArtifacts(
  Directory source,
  Directory destination,
) async {
  for (final fileName in _webRuntimeArtifactNames) {
    final sourceFile = File.fromUri(source.uri.resolve(fileName));
    if (!sourceFile.existsSync()) {
      throw StateError(
        'Web runtime artifact does not exist: ${sourceFile.path}',
      );
    }
    final destinationFile = File.fromUri(destination.uri.resolve(fileName));
    await _copyWebRuntimeFileIfChanged(sourceFile, destinationFile);
  }
  await _syncGeneratedWebRuntimeDirectory(
    Directory.fromUri(source.uri.resolve('ocr/')),
    Directory.fromUri(destination.uri.resolve('ocr/')),
  );
  await _syncGeneratedWebRuntimeDirectory(
    Directory.fromUri(source.uri.resolve('v86/')),
    Directory.fromUri(destination.uri.resolve('v86/')),
  );
}

/// Writes the worker bridge from the matching wasm-bindgen output with relative WASI imports.
Future<void> _writeWorkerWasmBridgeModule(
  File bridgeModule,
  File workerBridgeModule,
) async {
  final bridgeContents = await bridgeModule.readAsString();
  final workerContents = bridgeContents.replaceAll(
    'from "wasi_snapshot_preview1"',
    'from "./wasi_snapshot_preview1.js"',
  );
  if (workerContents == bridgeContents) {
    throw StateError(
      'wasm-bindgen bridge did not declare the WASI module import: '
      '${bridgeModule.path}',
    );
  }
  await workerBridgeModule.writeAsString(workerContents, flush: true);
}

/// Writes an ESM wrapper that exposes the UMD MessagePack runtime to module workers.
Future<void> _writeWorkerMessagePackModule(
  File sourceModule,
  List<File> workerModules,
) async {
  final sourceContents = await sourceModule.readAsString();
  if (!sourceContents.startsWith('!function')) {
    throw StateError(
      'MessagePack source does not use the expected UMD wrapper: '
      '${sourceModule.path}',
    );
  }
  final workerContents =
      '''const module = { exports: {} };
const exports = module.exports;
$sourceContents
const MessagePack = module.exports;
export { MessagePack };
''';
  for (final workerModule in workerModules) {
    await _writeTextFileIfChanged(workerModule, workerContents);
  }
}

/// Copies one generated browser runtime directory into the Flutter Web source tree.
Future<void> _syncGeneratedWebRuntimeDirectory(
  Directory source,
  Directory destination,
) async {
  if (!source.existsSync()) {
    throw StateError(
      'Generated web runtime directory does not exist: ${source.path}',
    );
  }
  await destination.create(recursive: true);
  final sourceFiles = await _collectFilesByRelativePath(source);
  final destinationFiles = await _collectFilesByRelativePath(destination);
  for (final entry in sourceFiles.entries) {
    final target = File(_joinPath(destination.path, entry.key));
    await _copyWebRuntimeFileIfChanged(entry.value, target);
  }
  for (final entry in destinationFiles.entries) {
    if (!sourceFiles.containsKey(entry.key)) {
      await entry.value.delete();
    }
  }
}

/// Lists files below one directory using paths relative to that directory.
Future<Map<String, File>> _collectFilesByRelativePath(
  Directory directory,
) async {
  final files = <String, File>{};
  await for (final entity in directory.list(
    recursive: true,
    followLinks: false,
  )) {
    if (entity is File) {
      files[_relativePath(directory, entity)] = entity;
    }
  }
  return files;
}

/// Copies one generated Web runtime file only when its contents have changed.
Future<void> _copyWebRuntimeFileIfChanged(File source, File destination) async {
  if (destination.existsSync() &&
      await _filesHaveSameContents(source, destination)) {
    return;
  }
  await destination.parent.create(recursive: true);
  await source.copy(destination.path);
}

/// Compares two files by size and SHA-256 content digest.
Future<bool> _filesHaveSameContents(File first, File second) async {
  final firstLength = await first.length();
  if (firstLength != await second.length()) {
    return false;
  }
  final firstDigest = await sha256.bind(first.openRead()).first;
  final secondDigest = await sha256.bind(second.openRead()).first;
  return firstDigest == secondDigest;
}

/// Writes generated text only when its current contents differ.
Future<void> _writeTextFileIfChanged(File destination, String contents) async {
  if (destination.existsSync() &&
      await destination.readAsString() == contents) {
    return;
  }
  await destination.parent.create(recursive: true);
  await destination.writeAsString(contents, flush: true);
}

/// Stages a pinned offline OCR engine and its four supported language models.
Future<void> _stageBrowserOcrAssets(
  Directory dependencies,
  Directory output,
) async {
  for (final name in <String>['tesseract.min.js', 'worker.min.js']) {
    await _copyRequiredWebRuntimeAsset(
      File.fromUri(
        dependencies.uri.resolve('node_modules/tesseract.js/dist/$name'),
      ),
      File.fromUri(output.uri.resolve('ocr/$name')),
    );
  }
  for (final variant in <String>['', '-simd', '-lstm', '-simd-lstm']) {
    for (final extension in <String>['wasm', 'wasm.js']) {
      final name = 'tesseract-core$variant.$extension';
      await _copyRequiredWebRuntimeAsset(
        File.fromUri(
          dependencies.uri.resolve('node_modules/tesseract.js-core/$name'),
        ),
        File.fromUri(output.uri.resolve('ocr/core/$name')),
      );
    }
  }
  for (final language in <String>['eng', 'chi_sim', 'jpn', 'kor']) {
    await _copyRequiredWebRuntimeAsset(
      File.fromUri(
        dependencies.uri.resolve(
          'node_modules/@tesseract.js-data/$language/4.0.0_best_int/$language.traineddata.gz',
        ),
      ),
      File.fromUri(
        output.uri.resolve('ocr/languages/$language.traineddata.gz'),
      ),
    );
  }
  for (final artifact in <({String source, String destination})>[
    (source: 'tesseract.js/LICENSE.md', destination: 'tesseract.js.LICENSE.md'),
    (
      source: 'tesseract.js-core/LICENSE',
      destination: 'tesseract.js-core.LICENSE',
    ),
    (
      source: 'tesseract.js/dist/tesseract.min.js.LICENSE.txt',
      destination: 'tesseract.min.js.LICENSE.txt',
    ),
    (
      source: 'tesseract.js/dist/worker.min.js.LICENSE.txt',
      destination: 'worker.min.js.LICENSE.txt',
    ),
  ]) {
    await _copyRequiredWebRuntimeAsset(
      File.fromUri(dependencies.uri.resolve('node_modules/${artifact.source}')),
      File.fromUri(output.uri.resolve('ocr/${artifact.destination}')),
    );
  }
}

/// Stages the v86 emulator runtime and verified BIOS resources.
Future<void> _stageV86RuntimeAssets(
  Directory dependencies,
  Directory webBuildDir,
) async {
  final v86BuildDir = Directory.fromUri(
    dependencies.uri.resolve('node_modules/v86/build/'),
  );
  await _copyRequiredWebRuntimeAsset(
    File.fromUri(v86BuildDir.uri.resolve('libv86.mjs')),
    File.fromUri(webBuildDir.uri.resolve('v86/libv86.mjs')),
  );
  await _copyRequiredWebRuntimeAsset(
    File.fromUri(v86BuildDir.uri.resolve('v86.wasm')),
    File.fromUri(webBuildDir.uri.resolve('v86/v86.wasm')),
  );
  for (final asset in _v86GuestAssets) {
    await downloadVerifiedWebRuntimeAsset(
      Uri.parse(asset.url),
      File.fromUri(webBuildDir.uri.resolve(asset.relativePath)),
      asset.sha256,
    );
  }
}

/// Copies one required browser runtime artifact into the generated bundle.
Future<void> _copyRequiredWebRuntimeAsset(File source, File destination) async {
  if (!source.existsSync()) {
    throw StateError(
      'Required web runtime asset does not exist: ${source.path}',
    );
  }
  await destination.parent.create(recursive: true);
  await source.copy(destination.path);
}

/// Removes generated web runtime artifacts before compiling their replacement.
Future<void> _invalidateWebRuntimeArtifacts(
  Iterable<Directory> directories,
) async {
  for (final directory in directories) {
    for (final fileName in _generatedWebRuntimeFileNames) {
      final file = File.fromUri(directory.uri.resolve(fileName));
      if (file.existsSync()) {
        await file.delete();
      }
    }
  }
}

const Set<String> _webRuntimeArtifactNames = <String>{
  'operit_runtime_bridge.js',
  'browser_system_capabilities.js',
  'browser_file_open.js',
  'operit_runtime_worker.js',
  'operit_model_install_worker.js',
  'v86_runtime_worker.js',
  'operit_flutter_bridge.js',
  'operit_flutter_bridge_worker.js',
  'operit_messagepack.js',
  'operit_flutter_bridge_bg.wasm',
  'operit_flutter_bridge_bg.wasm.d.ts',
  'operit_flutter_bridge.d.ts',
  'sql-wasm.js',
  'sql-wasm.wasm',
};

const Set<String> _generatedWebRuntimeFileNames = <String>{
  ..._webRuntimeArtifactNames,
  'libv86.mjs',
  'v86.wasm',
  'seabios.bin',
  'vgabios.bin',
};

/// Computes a path relative to a generated Web runtime directory.
String _relativePath(Directory root, FileSystemEntity entity) {
  final rootPath = root.uri.toFilePath(windows: Platform.isWindows);
  final entityPath = entity.uri.toFilePath(windows: Platform.isWindows);
  if (!entityPath.startsWith(rootPath)) {
    throw StateError('Path escapes sync root: $entityPath');
  }
  return entityPath.substring(rootPath.length);
}

String _joinPath(String base, String relative) {
  final segments = relative
      .split(RegExp(r'[\\/]'))
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return <String>[base, ...segments].join(Platform.pathSeparator);
}

Future<void> _run(
  String executable,
  List<String> arguments, {
  required String workingDirectory,
  Map<String, String>? environment,
}) async {
  var resolvedEnvironment = environment;
  if (_isCargoExecutable(executable)) {
    resolvedEnvironment = _withRustupProxyEnvironment(
      Map<String, String>.from(resolvedEnvironment ?? Platform.environment),
    );
  }
  final result = await Process.run(
    executable,
    arguments,
    workingDirectory: workingDirectory,
    environment: resolvedEnvironment,
  );
  stdout.write(result.stdout);
  stderr.write(result.stderr);
  if (result.exitCode != 0) {
    throw ProcessException(
      executable,
      arguments,
      'command failed with exit code ${result.exitCode}',
      result.exitCode,
    );
  }
}

String _command(String executable) {
  if (!Platform.isWindows) {
    return executable;
  }
  // Cargo is installed as a native executable on Windows, while npm is a
  // cmd shim. Process.run cannot resolve a nonexistent cargo.cmd.
  return executable == 'cargo' ? 'cargo.exe' : '$executable.cmd';
}

bool _isCargoExecutable(String executable) {
  final name = executable.split(RegExp(r'[\\/]')).last.toLowerCase();
  return name == 'cargo' ||
      name == 'cargo.exe' ||
      name == 'cargo.cmd' ||
      name == 'cargo.bat';
}

/// Fills rustup proxy variables when IDE/MSBuild builds omit the user env.
Map<String, String> _withRustupProxyEnvironment(
  Map<String, String> environment,
) {
  if ((environment['RUSTUP_HOME'] ?? '').isNotEmpty &&
      (environment['CARGO_HOME'] ?? '').isNotEmpty) {
    return environment;
  }
  final derived = <String, String>{};
  final cargo = _which('cargo');
  if (cargo != null) {
    derived.addAll(_environmentFromCargoCmd(File(cargo)));
    derived.addAll(_rustupProxyEnvironmentFromCargo(File(cargo)));
  }
  if ((derived['RUSTUP_HOME'] ?? '').isEmpty ||
      (derived['CARGO_HOME'] ?? '').isEmpty) {
    derived.addAll(
      _rustupProxyEnvironmentFromPath(
        environment['PATH'] ?? Platform.environment['PATH'] ?? '',
      ),
    );
  }
  for (final entry in derived.entries) {
    if ((environment[entry.key] ?? '').isEmpty) {
      environment[entry.key] = entry.value;
    }
  }
  return environment;
}

Map<String, String> _environmentFromCargoCmd(File cargo) {
  var cmd = cargo;
  final name = cargo.uri.pathSegments.isEmpty
      ? cargo.path
      : cargo.uri.pathSegments.last.toLowerCase();
  if (!name.endsWith('.cmd') && !name.endsWith('.bat')) {
    cmd = File.fromUri(cargo.parent.uri.resolve('cargo.cmd'));
    if (!cmd.existsSync()) {
      cmd = File.fromUri(cargo.parent.uri.resolve('cargo.bat'));
    }
  }
  if (!cmd.existsSync()) {
    return const <String, String>{};
  }
  final dp0 = '${cmd.parent.path}${Platform.pathSeparator}';
  final derived = <String, String>{};
  for (final line in cmd.readAsLinesSync()) {
    final stripped = line.trim();
    if (!stripped.toLowerCase().startsWith('set ')) {
      continue;
    }
    final assignment = stripped.substring(4).trim().replaceAll('"', '');
    final separator = assignment.indexOf('=');
    if (separator < 0) {
      continue;
    }
    final key = assignment.substring(0, separator).trim();
    if (key != 'CARGO_HOME' &&
        key != 'RUSTUP_HOME' &&
        key != 'RUSTUP_TOOLCHAIN') {
      continue;
    }
    derived[key] = assignment
        .substring(separator + 1)
        .replaceAll('%~dp0', dp0)
        .replaceAll('%~DP0', dp0);
  }
  return derived;
}

Map<String, String> _rustupProxyEnvironmentFromPath(String pathValue) {
  final pathSeparator = Platform.isWindows ? ';' : ':';
  final names = Platform.isWindows
      ? <String>['cargo.exe', 'cargo']
      : <String>['cargo'];
  for (final directory in pathValue.split(pathSeparator)) {
    if (directory.isEmpty) {
      continue;
    }
    for (final name in names) {
      final derived = _rustupProxyEnvironmentFromCargo(
        File('$directory${Platform.pathSeparator}$name'),
      );
      if ((derived['RUSTUP_HOME'] ?? '').isNotEmpty &&
          (derived['CARGO_HOME'] ?? '').isNotEmpty) {
        return derived;
      }
    }
  }
  return const <String, String>{};
}

String? _which(String executable) {
  final path = Platform.environment['PATH'];
  if (path == null || path.isEmpty) {
    return null;
  }
  final pathSeparator = Platform.isWindows ? ';' : ':';
  final extensions = Platform.isWindows
      ? <String>['.exe', '.cmd', '.bat', '']
      : <String>[''];
  for (final directory in path.split(pathSeparator)) {
    if (directory.isEmpty) {
      continue;
    }
    for (final extension in extensions) {
      final candidate = File(
        '$directory${Platform.pathSeparator}$executable$extension',
      );
      if (candidate.existsSync()) {
        return candidate.path;
      }
    }
  }
  return null;
}

Map<String, String> _rustupProxyEnvironmentFromCargo(File cargo) {
  if (!cargo.existsSync()) {
    return const <String, String>{};
  }
  final binDir = Directory(cargo.parent.path);
  final rustup = File.fromUri(binDir.uri.resolve('rustup.exe')).existsSync()
      ? File.fromUri(binDir.uri.resolve('rustup.exe'))
      : File.fromUri(binDir.uri.resolve('rustup'));
  if (!rustup.existsSync()) {
    return const <String, String>{};
  }
  final cargoHome = binDir.parent;
  final root = cargoHome.parent;
  // Distro rustup shims (for example /usr/bin/cargo next to /usr/bin/rustup)
  // resolve to system directories; deriving CARGO_HOME from them poisons the
  // environment with paths like /usr. Only trust a full rustup-managed layout
  // where the sibling rustup home also exists.
  for (final name in <String>['rustup', '.rustup']) {
    final rustupHome = Directory.fromUri(root.uri.resolve('$name/'));
    if (!rustupHome.existsSync()) {
      continue;
    }
    final derived = <String, String>{
      'CARGO_HOME': cargoHome.path,
      'RUSTUP_HOME': rustupHome.path,
    };
    final toolchain = _rustupDefaultToolchain(rustupHome);
    if (toolchain != null) {
      derived['RUSTUP_TOOLCHAIN'] = toolchain;
    }
    return derived;
  }
  return const <String, String>{};
}

String? _rustupDefaultToolchain(Directory rustupHome) {
  final settings = File.fromUri(rustupHome.uri.resolve('settings.toml'));
  if (!settings.existsSync()) {
    return null;
  }
  for (final line in settings.readAsLinesSync()) {
    final stripped = line.trim();
    if (!stripped.startsWith('default_toolchain')) {
      continue;
    }
    final separator = stripped.indexOf('=');
    if (separator < 0) {
      return null;
    }
    final toolchain = stripped
        .substring(separator + 1)
        .trim()
        .replaceAll('"', '')
        .replaceAll("'", '');
    return toolchain.isEmpty ? null : toolchain;
  }
  return null;
}

String _pythonExecutable(Directory repoRoot) {
  if (Platform.isWindows) {
    return File.fromUri(repoRoot.uri.resolve('.venv/Scripts/python.exe')).path;
  }
  return File.fromUri(repoRoot.uri.resolve('.venv/bin/python')).path;
}

/// Reads the exact Flutter build target from the hook configuration schema.
String _targetOs(BuildInput input) {
  final config = input.json['config'];
  if (config is! Map<String, Object?>) {
    throw StateError('Build hook config is not an object.');
  }
  final extensions = config['extensions'];
  if (extensions is Map<String, Object?>) {
    final codeAssets = extensions['code_assets'];
    if (codeAssets is! Map<String, Object?>) {
      throw StateError('Build hook code_assets config is not an object.');
    }
    final targetOs = codeAssets['target_os'];
    if (targetOs is! String || targetOs.isEmpty) {
      throw StateError('Build hook code_assets target_os is missing.');
    }
    return targetOs;
  }
  final buildAssetTypes = config['build_asset_types'];
  if (buildAssetTypes is List<Object?> && buildAssetTypes.isEmpty) {
    return 'web';
  }
  throw StateError('Unsupported build hook target configuration: $config');
}

/// Configures the shared WASM C toolchain and runtime linker dependencies.
Future<Map<String, String>> _wasmCargoEnvironment(Directory repoRoot) async {
  final environment = _withRustupProxyEnvironment(
    Map<String, String>.from(Platform.environment),
  )..['RUSTFLAGS'] = '-Awarnings';

  final toolsDir = Directory.fromUri(
    repoRoot.uri.resolve('target/operit-build-tools/'),
  );
  final wasiSdkName = switch (Platform.operatingSystem) {
    'windows' => 'wasi-sdk-20.0.m-mingw',
    'macos' => 'wasi-sdk-20.0-macos',
    'linux' => 'wasi-sdk-20.0-linux',
    _ => throw StateError(
      'Unsupported Web Access WASI SDK host: ${Platform.operatingSystem}',
    ),
  };
  final wasiSdk = Directory.fromUri(toolsDir.uri.resolve('$wasiSdkName/'));
  final clangName = Platform.isWindows ? 'clang.exe' : 'clang';

  await _ensureExtractedArchive(
    archiveUrl:
        'https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-20/$wasiSdkName.tar.gz',
    archiveFile: File.fromUri(toolsDir.uri.resolve('$wasiSdkName.tar.gz')),
    destination: wasiSdk,
    requiredFile: File.fromUri(wasiSdk.uri.resolve('bin/$clangName')),
    stripComponents: 1,
  );

  environment['QUICKJS_WASM_SYS_WASI_SDK_PATH'] = wasiSdk.path;
  environment['CC_wasm32_unknown_unknown'] = File.fromUri(
    wasiSdk.uri.resolve('bin/$clangName'),
  ).path;
  final arName = Platform.isWindows ? 'llvm-ar.exe' : 'llvm-ar';
  environment['AR_wasm32_unknown_unknown'] = File.fromUri(
    wasiSdk.uri.resolve('bin/$arName'),
  ).path;
  final clangResourceDir = File.fromUri(
    wasiSdk.uri.resolve('lib/clang/16'),
  ).path.replaceAll(r'\', '/');
  if (!Directory(clangResourceDir).existsSync()) {
    throw StateError(
      'WASI SDK Clang resource directory does not exist: $clangResourceDir',
    );
  }
  final bindgenClangArgs = '-resource-dir=$clangResourceDir';
  final wasiLibDir = Directory.fromUri(
    wasiSdk.uri.resolve('share/wasi-sysroot/lib/wasm32-wasi/'),
  );
  final wasiBuiltinsDir = Directory.fromUri(
    wasiSdk.uri.resolve('lib/clang/16/lib/wasi/'),
  );
  final wasiLibc = File.fromUri(wasiLibDir.uri.resolve('libc.a'));
  final wasiBuiltins = File.fromUri(
    wasiBuiltinsDir.uri.resolve('libclang_rt.builtins-wasm32.a'),
  );
  if (!wasiLibc.existsSync() || !wasiBuiltins.existsSync()) {
    throw StateError(
      'WASI SDK is missing the WebAssembly libc or compiler builtins required '
      'by QuickJS: libc=${wasiLibc.path} builtins=${wasiBuiltins.path}',
    );
  }
  final wasiLibDirPath = wasiLibDir.path.replaceAll(r'\', '/');
  final wasiBuiltinsDirPath = wasiBuiltinsDir.path.replaceAll(r'\', '/');
  environment['RUSTFLAGS'] = <String>[
    '-Awarnings',
    '-L',
    'native=$wasiLibDirPath',
    '-L',
    'native=$wasiBuiltinsDirPath',
    '-l',
    'static=c',
    '-l',
    'static=clang_rt.builtins-wasm32',
  ].join(' ');
  environment['BINDGEN_EXTRA_CLANG_ARGS_wasm32_unknown_unknown'] =
      bindgenClangArgs;
  if (Platform.isWindows) {
    final libclangDir = Directory.fromUri(
      toolsDir.uri.resolve(
        'libclang.runtime.win-x64.21.1.8/runtimes/win-x64/native/',
      ),
    );
    await _ensureExtractedArchive(
      archiveUrl:
          'https://www.nuget.org/api/v2/package/libclang.runtime.win-x64/21.1.8',
      archiveFile: File.fromUri(
        toolsDir.uri.resolve('libclang.runtime.win-x64.21.1.8.nupkg'),
      ),
      destination: Directory.fromUri(
        toolsDir.uri.resolve('libclang.runtime.win-x64.21.1.8/'),
      ),
      requiredFile: File.fromUri(libclangDir.uri.resolve('libclang.dll')),
      stripComponents: 0,
    );
    environment['LIBCLANG_PATH'] = libclangDir.path;
    environment['BINDGEN_EXTRA_CLANG_ARGS'] = bindgenClangArgs;
  }
  return environment;
}

/// Verifies that wasm-bindgen generated browser-resolvable imports.
Future<void> _validateWasmBindgenImports(File bridgeScript) async {
  if (!bridgeScript.existsSync()) {
    throw StateError(
      'wasm-bindgen bridge script does not exist: ${bridgeScript.path}',
    );
  }
  final source = await bridgeScript.readAsString();
  final invalidImport = RegExp(r'''from\s+["']env["']''').firstMatch(source);
  if (invalidImport != null) {
    throw StateError(
      'wasm-bindgen generated an unresolved env import in ${bridgeScript.path}. '
      'Check the Web Access WASI SDK link configuration.',
    );
  }
}

Future<void> _ensureExtractedArchive({
  required String archiveUrl,
  required File archiveFile,
  required Directory destination,
  required File requiredFile,
  required int stripComponents,
}) async {
  if (requiredFile.existsSync()) {
    return;
  }
  await destination.create(recursive: true);
  await archiveFile.parent.create(recursive: true);
  if (!archiveFile.existsSync()) {
    await _downloadFile(archiveUrl, archiveFile);
  }
  final arguments = <String>['-xf', archiveFile.path, '-C', destination.path];
  if (stripComponents > 0) {
    arguments.addAll(['--strip-components', stripComponents.toString()]);
  }
  await _run('tar', arguments, workingDirectory: destination.path);
  if (!requiredFile.existsSync()) {
    throw StateError(
      'Required build tool was not extracted: ${requiredFile.path}',
    );
  }
}

Future<void> _downloadFile(String url, File destination) async {
  final client = HttpClient();
  try {
    final request = await client.getUrl(Uri.parse(url));
    final response = await request.close();
    if (response.statusCode < 200 || response.statusCode >= 300) {
      throw StateError('Download failed: $url (${response.statusCode})');
    }
    final sink = destination.openWrite();
    try {
      await for (final data in response) {
        sink.add(data);
      }
    } finally {
      await sink.close();
    }
  } finally {
    client.close(force: true);
  }
}
