import 'dart:io';

import 'package:hooks/hooks.dart';

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
    final webDir = Directory.fromUri(input.packageRoot.resolve('web/'));
    final webBuildDir = Directory.fromUri(
      input.packageRoot.resolve('build/web/'),
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
    final shouldBuildWebAssets = targetOs == null || targetOs == 'web';
    final shouldBundleWebAccessAssets = targetOs != null && targetOs != 'web';

    await _addDirectoryFileDependencies(output, pluginsRoot, {
      '.js',
      '.json',
      '.hjson',
      '.ts',
      '.d.ts',
      '.py',
    });
    await _addRustDependencies(output, bridgeCrate);
    await _addRustDependencies(output, coreRoot);
    await _addRustDependencies(output, webHostRoot);
    output.dependencies.add(
      packageRoot.uri.resolve('web/operit_runtime_bridge.js'),
    );
    output.dependencies.add(packageRoot.uri.resolve('web/index.html'));

    await _run(_pythonExecutable(repoRoot), [
      syncScript.path,
      '--source',
      'buildin',
    ], workingDirectory: repoRoot.path);

    if (shouldBuildWebAssets) {
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
        webDir.path,
        '--out-name',
        'operit_flutter_bridge',
        wasmSource.path,
      ], workingDirectory: packageRoot.path);

      await _run(_command('npm'), [
        'install',
        '--silent',
        '--no-audit',
        '--no-fund',
        '--prefix',
        depsDir.path,
        'sql.js@1.14.1',
      ], workingDirectory: packageRoot.path);

      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.js'),
      ).copy(File.fromUri(webDir.uri.resolve('sql-wasm.js')).path);
      await File.fromUri(
        sqlDist.uri.resolve('sql-wasm.wasm'),
      ).copy(File.fromUri(webDir.uri.resolve('sql-wasm.wasm')).path);
    }
    if (shouldBundleWebAccessAssets) {
      await _syncDirectory(webBuildDir, webAccessAssetsDir);
    }
  });
}

Future<void> _addDirectoryFileDependencies(
  BuildOutputBuilder output,
  Directory root,
  Set<String> extensions,
) async {
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
    if (extensions.any(path.endsWith)) {
      output.dependencies.add(entity.uri);
    }
  }
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

Future<void> _syncDirectory(Directory source, Directory destination) async {
  if (!source.existsSync()) {
    throw StateError('Web access source bundle does not exist: ${source.path}');
  }
  if (destination.existsSync()) {
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

String _relativePath(Directory root, FileSystemEntity entity) {
  final rootPath = root.uri.toFilePath(windows: Platform.isWindows);
  final entityPath = entity.uri.toFilePath(windows: Platform.isWindows);
  if (!entityPath.startsWith(rootPath)) {
    throw StateError('Path escapes sync root: $entityPath');
  }
  return entityPath.substring(rootPath.length);
}

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

String? _targetOs(BuildInput input) {
  final config = input.json['config'];
  if (config is! Map<String, Object?>) {
    return null;
  }
  final extensions = config['extensions'];
  if (extensions is! Map<String, Object?>) {
    return null;
  }
  final codeAssets = extensions['code_assets'];
  if (codeAssets is! Map<String, Object?>) {
    return null;
  }
  final targetOs = codeAssets['target_os'];
  return targetOs is String ? targetOs : null;
}

Future<Map<String, String>> _wasmCargoEnvironment(Directory repoRoot) async {
  final environment = {'RUSTFLAGS': '-Awarnings'};
  if (!Platform.isWindows) {
    return environment;
  }

  final toolsDir = Directory.fromUri(
    repoRoot.uri.resolve('target/operit-build-tools/'),
  );
  final wasiSdk = Directory.fromUri(
    toolsDir.uri.resolve('wasi-sdk-20.0.m-mingw/'),
  );
  final libclangDir = Directory.fromUri(
    toolsDir.uri.resolve(
      'libclang.runtime.win-x64.21.1.8/runtimes/win-x64/native/',
    ),
  );

  await _ensureExtractedArchive(
    archiveUrl:
        'https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-20/wasi-sdk-20.0.m-mingw.tar.gz',
    archiveFile: File.fromUri(
      toolsDir.uri.resolve('wasi-sdk-20.0.m-mingw.tar.gz'),
    ),
    destination: wasiSdk,
    requiredFile: File.fromUri(wasiSdk.uri.resolve('bin/clang.exe')),
    stripComponents: 1,
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

  environment['QUICKJS_WASM_SYS_WASI_SDK_PATH'] = wasiSdk.path;
  environment['LIBCLANG_PATH'] = libclangDir.path;
  final clangResourceDir = File.fromUri(
    wasiSdk.uri.resolve('lib/clang/16'),
  ).path.replaceAll(r'\', '/');
  final bindgenClangArgs = '-resource-dir=$clangResourceDir';
  environment['BINDGEN_EXTRA_CLANG_ARGS'] = bindgenClangArgs;
  environment['BINDGEN_EXTRA_CLANG_ARGS_wasm32_unknown_unknown'] =
      bindgenClangArgs;
  return environment;
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
