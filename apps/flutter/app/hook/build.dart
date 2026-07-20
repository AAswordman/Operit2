import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:hooks/hooks.dart';

const String _webAccessVersionFile = 'web_access_version.json';
const int _webAccessVersionSchema = 1;
const String _v86PackageVersion = '0.5.424';

const List<_V86GuestAsset> _v86GuestAssets = <_V86GuestAsset>[
  _V86GuestAsset(
    relativePath: 'v86/seabios.bin',
    url: 'https://raw.githubusercontent.com/copy/v86/master/bios/seabios.bin',
    sha256: '73e3f359102e3a9982c35fce98eb7cd08f18303ac7f1ba6ebfbe6cdc1c244d98',
  ),
  _V86GuestAsset(
    relativePath: 'v86/vgabios.bin',
    url: 'https://raw.githubusercontent.com/copy/v86/master/bios/vgabios.bin',
    sha256: 'a4bc0d80cc3ca028c73dafa8fee396b8d054ce87ebd8abfbd31b06b437607880',
  ),
];

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
    final webSourceDir = Directory.fromUri(
      input.packageRoot.resolve('../../../apps/web_access/web/'),
    );
    final webRuntimeSourceDir = Directory.fromUri(
      input.packageRoot.resolve('../../../apps/web_access/src/'),
    );
    final webRuntimeTypescriptConfig = File.fromUri(
      input.packageRoot.resolve('../../../apps/web_access/tsconfig.json'),
    );
    final webBuildDir = Directory.fromUri(
      input.packageRoot.resolve('../../../apps/web_access/build/bundle/'),
    );
    final webAccessAssetsDir = Directory.fromUri(
      input.packageRoot.resolve('assets/web_access/'),
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
    final shouldBundleWebAccessAssets = !isWebTarget;

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
    ], workingDirectory: repoRoot.path);

    if (shouldBuildWebAssets) {
      await _invalidateWebRuntimeArtifacts([webBuildDir, webSourceDir]);
      await _run(
        'cargo',
        const ['build', '--release', '--target', 'wasm32-unknown-unknown'],
        workingDirectory: bridgeCrate.path,
        environment: await _wasmCargoEnvironment(repoRoot),
      );

      await _run('wasm-bindgen', [
        '--target',
        'web',
        '--out-dir',
        webBuildDir.path,
        '--out-name',
        'operit_flutter_bridge',
        wasmSource.path,
      ], workingDirectory: packageRoot.path);
      await _validateWasmBindgenImports(
        File.fromUri(webBuildDir.uri.resolve('operit_flutter_bridge.js')),
      );
      await _writeWorkerWasmBridgeModule(
        File.fromUri(webBuildDir.uri.resolve('operit_flutter_bridge.js')),
        File.fromUri(
          webBuildDir.uri.resolve('operit_flutter_bridge_worker.js'),
        ),
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
      ], workingDirectory: packageRoot.path);

      await _compileWebRuntimeBridge(
        depsDir,
        webRuntimeTypescriptConfig,
        packageRoot,
      );

      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.js'),
      ).copy(File.fromUri(webBuildDir.uri.resolve('sql-wasm.js')).path);
      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.wasm'),
      ).copy(File.fromUri(webBuildDir.uri.resolve('sql-wasm.wasm')).path);
      await _stageV86RuntimeAssets(depsDir, webBuildDir);
      await _syncWebRuntimeArtifacts(webBuildDir, webSourceDir);
      final versionManifest = await _writeWebAccessVersionManifest(
        webBuildDir,
        webAccessAssetsDir,
      );
      stdout.writeln(
        'Web Access version ${versionManifest['version']} '
        'hash=${versionManifest['contentHash']} '
        'files=${versionManifest['fileCount']} '
        'bytes=${versionManifest['byteSize']}',
      );
    }
    if (shouldBundleWebAccessAssets) {
      await _requireWebAccessVersionManifest(webBuildDir);
      await _addDirectoryFileDependencies(output, webBuildDir, {
        '.bin',
        '.gz',
        '.html',
        '.js',
        '.json',
        '.otf',
        '.png',
        '.ttf',
        '.txt',
        '.wasm',
      });
      await _syncDirectory(webBuildDir, webAccessAssetsDir);
    }
  });
}

/// Reads and validates one generated Web Access version manifest.
Future<Map<String, Object?>?> _readWebAccessVersionManifest(File file) async {
  if (!file.existsSync()) {
    return null;
  }
  final decoded = jsonDecode(await file.readAsString());
  if (decoded is! Map) {
    throw StateError(
      'Web Access version manifest must be an object: ${file.path}',
    );
  }
  final manifest = decoded.cast<String, Object?>();
  final schemaVersion = manifest['schemaVersion'];
  final version = manifest['version'];
  final contentHash = manifest['contentHash'];
  final fileCount = manifest['fileCount'];
  final byteSize = manifest['byteSize'];
  if (schemaVersion != _webAccessVersionSchema) {
    throw StateError(
      'Unexpected Web Access manifest schema in ${file.path}: $schemaVersion',
    );
  }
  if (version is! int || version < 1) {
    throw StateError('Invalid Web Access version in ${file.path}: $version');
  }
  if (contentHash is! String || contentHash.isEmpty) {
    throw StateError('Invalid Web Access content hash in ${file.path}');
  }
  if (fileCount is! int || fileCount < 1) {
    throw StateError(
      'Invalid Web Access file count in ${file.path}: $fileCount',
    );
  }
  if (byteSize is! int || byteSize < 1) {
    throw StateError('Invalid Web Access byte size in ${file.path}: $byteSize');
  }
  return manifest;
}

/// Finds the last generated Web Access version manifest from build-owned outputs.
Future<Map<String, Object?>?> _readPreviousWebAccessVersionManifest(
  Directory webBuildDir,
  Directory webAccessAssetsDir,
) async {
  final embeddedManifest = await _readWebAccessVersionManifest(
    File.fromUri(webAccessAssetsDir.uri.resolve(_webAccessVersionFile)),
  );
  if (embeddedManifest != null) {
    return embeddedManifest;
  }
  return _readWebAccessVersionManifest(
    File.fromUri(webBuildDir.uri.resolve(_webAccessVersionFile)),
  );
}

/// Computes the generated Web Access bundle digest used for versioning.
Future<_WebAccessBundleDigest> _computeWebAccessBundleDigest(
  Directory bundle,
) async {
  if (!bundle.existsSync()) {
    throw StateError('Web access bundle does not exist: ${bundle.path}');
  }
  final files = <File>[];
  await for (final entity in bundle.list(recursive: true, followLinks: false)) {
    if (entity is! File) {
      continue;
    }
    final relativePath = _relativePath(bundle, entity);
    if (relativePath == _webAccessVersionFile) {
      continue;
    }
    files.add(entity);
  }
  files.sort(
    (left, right) =>
        _relativePath(bundle, left).compareTo(_relativePath(bundle, right)),
  );
  if (files.isEmpty) {
    throw StateError('Web access bundle contains no files: ${bundle.path}');
  }
  final digestSink = _DigestSink();
  final byteSink = sha256.startChunkedConversion(digestSink);
  var fileCount = 0;
  var byteSize = 0;
  for (final file in files) {
    final relativePath = _relativePath(bundle, file).replaceAll('\\', '/');
    final data = await file.readAsBytes();
    byteSink.add(utf8.encode(relativePath));
    byteSink.add(const <int>[0]);
    byteSink.add(_uint64Bytes(data.length));
    byteSink.add(data);
    fileCount += 1;
    byteSize += data.length;
  }
  byteSink.close();
  return _WebAccessBundleDigest(
    contentHash: digestSink.digest.toString(),
    fileCount: fileCount,
    byteSize: byteSize,
  );
}

/// Writes the Web Access version manifest after the generated bundle is complete.
Future<Map<String, Object?>> _writeWebAccessVersionManifest(
  Directory webBuildDir,
  Directory webAccessAssetsDir,
) async {
  final digest = await _computeWebAccessBundleDigest(webBuildDir);
  final previousManifest = await _readPreviousWebAccessVersionManifest(
    webBuildDir,
    webAccessAssetsDir,
  );
  var version = 1;
  if (previousManifest != null) {
    final previousVersion = previousManifest['version'] as int;
    final previousHash = previousManifest['contentHash'] as String;
    version = previousHash == digest.contentHash
        ? previousVersion
        : previousVersion + 1;
  }
  final manifest = <String, Object?>{
    'byteSize': digest.byteSize,
    'contentHash': digest.contentHash,
    'fileCount': digest.fileCount,
    'schemaVersion': _webAccessVersionSchema,
    'version': version,
  };
  const encoder = JsonEncoder.withIndent('  ');
  await File.fromUri(
    webBuildDir.uri.resolve(_webAccessVersionFile),
  ).writeAsString('${encoder.convert(manifest)}\n');
  return manifest;
}

/// Requires a generated Web Access bundle version manifest before native asset sync.
Future<void> _requireWebAccessVersionManifest(Directory webBuildDir) async {
  final manifest = await _readWebAccessVersionManifest(
    File.fromUri(webBuildDir.uri.resolve(_webAccessVersionFile)),
  );
  if (manifest == null) {
    throw StateError(
      'Web Access version manifest does not exist: '
      '${File.fromUri(webBuildDir.uri.resolve(_webAccessVersionFile)).path}',
    );
  }
}

/// Encodes one unsigned 64-bit integer in big-endian order.
Uint8List _uint64Bytes(int value) {
  final data = ByteData(8)..setUint64(0, value, Endian.big);
  return data.buffer.asUint8List();
}

/// Holds the content digest and size metadata for one Web Access bundle.
class _WebAccessBundleDigest {
  const _WebAccessBundleDigest({
    required this.contentHash,
    required this.fileCount,
    required this.byteSize,
  });

  final String contentHash;
  final int fileCount;
  final int byteSize;
}

/// Captures the final digest emitted by a chunked hash conversion.
class _DigestSink implements Sink<Digest> {
  Digest? _digest;

  Digest get digest {
    final digest = _digest;
    if (digest == null) {
      throw StateError('Digest has not been closed');
    }
    return digest;
  }

  /// Stores the digest emitted by the hasher.
  @override
  void add(Digest data) {
    _digest = data;
  }

  /// Completes the digest sink.
  @override
  void close() {}
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

/// Copies the shared Web Access bundle into Flutter native assets.
Future<void> _syncDirectory(Directory source, Directory destination) async {
  if (!source.existsSync()) {
    throw StateError('Web access source bundle does not exist: ${source.path}');
  }
  if (destination.existsSync() || await Link(destination.path).exists()) {
    await destination.delete(recursive: true);
  }
  await destination.create(recursive: true);
  await for (final entity in source.list(recursive: true, followLinks: false)) {
    final relativePath = _relativePath(source, entity);
    if (_isFlutterBundledWebAccessCopy(relativePath)) {
      continue;
    }
    final targetPath = _joinPath(destination.path, relativePath);
    if (entity is Directory) {
      await Directory(targetPath).create(recursive: true);
    } else if (entity is File) {
      final targetFile = File(targetPath);
      await targetFile.parent.create(recursive: true);
      await entity.copy(targetFile.path);
    }
  }
}

/// Copies generated wasm runtime files into the Web static-file directory.
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
    await destinationFile.parent.create(recursive: true);
    await sourceFile.copy(destinationFile.path);
  }
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
  if (destination.existsSync()) {
    await destination.delete(recursive: true);
  }
  await destination.create(recursive: true);
  await for (final entity in source.list(recursive: true, followLinks: false)) {
    if (entity is! File) {
      continue;
    }
    final relativePath = _relativePath(source, entity);
    final target = File(_joinPath(destination.path, relativePath));
    await target.parent.create(recursive: true);
    await entity.copy(target.path);
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
    await _downloadVerifiedWebRuntimeAsset(
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

/// Downloads one browser runtime artifact and verifies its pinned SHA-256 digest.
Future<void> _downloadVerifiedWebRuntimeAsset(
  Uri url,
  File destination,
  String expectedSha256,
) async {
  final client = HttpClient();
  try {
    final request = await client.getUrl(url);
    final response = await request.close();
    if (response.statusCode != HttpStatus.ok) {
      throw HttpException(
        'Failed to download $url: HTTP ${response.statusCode}',
        uri: url,
      );
    }
    final content = await response.fold<BytesBuilder>(
      BytesBuilder(copy: false),
      (builder, chunk) => builder..add(chunk),
    );
    final bytes = content.takeBytes();
    final actualSha256 = sha256.convert(bytes).toString();
    if (actualSha256 != expectedSha256) {
      throw StateError(
        'Invalid SHA-256 for $url: expected $expectedSha256, got $actualSha256',
      );
    }
    await destination.parent.create(recursive: true);
    await destination.writeAsBytes(bytes, flush: true);
  } finally {
    client.close(force: true);
  }
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
  'operit_flutter_bridge.js',
  'operit_flutter_bridge_worker.js',
  'operit_flutter_bridge_bg.wasm',
  'operit_flutter_bridge_bg.wasm.d.ts',
  'operit_flutter_bridge.d.ts',
  'sql-wasm.js',
  'sql-wasm.wasm',
};

const Set<String> _generatedWebRuntimeFileNames = <String>{
  ..._webRuntimeArtifactNames,
  'operit_runtime_bridge.js',
  'libv86.mjs',
  'v86.wasm',
  'seabios.bin',
  'vgabios.bin',
};

/// Computes a path relative to the copied Web Access bundle root.
String _relativePath(Directory root, FileSystemEntity entity) {
  final rootPath = root.uri.toFilePath(windows: Platform.isWindows);
  final entityPath = entity.uri.toFilePath(windows: Platform.isWindows);
  if (!entityPath.startsWith(rootPath)) {
    throw StateError('Path escapes sync root: $entityPath');
  }
  return entityPath.substring(rootPath.length);
}

/// Detects recursive copies of the embedded Web Access asset directory.
bool _isFlutterBundledWebAccessCopy(String relativePath) {
  final segments = relativePath
      .split(RegExp(r'[\\/]'))
      .where((segment) => segment.isNotEmpty)
      .toList(growable: false);
  return segments.length >= 3 &&
      segments[0] == 'assets' &&
      segments[1] == 'assets' &&
      segments[2] == 'web_access';
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
  final result = await Process.run(
    executable,
    arguments,
    workingDirectory: workingDirectory,
    environment: environment,
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
  if (Platform.isWindows) {
    return '$executable.cmd';
  }
  return executable;
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

Future<Map<String, String>> _wasmCargoEnvironment(Directory repoRoot) async {
  final environment = Map<String, String>.from(Platform.environment)
    ..['RUSTFLAGS'] = '-Awarnings';

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
