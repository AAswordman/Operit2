import 'dart:io';

import 'package:hooks/hooks.dart';

void main(List<String> args) async {
  await build(args, (input, output) async {
    final packageRoot = Directory.fromUri(input.packageRoot);
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

    await _addRustDependencies(output, bridgeCrate);
    await _addRustDependencies(output, coreRoot);
    await _addRustDependencies(output, webHostRoot);
    output.dependencies.add(
      packageRoot.uri.resolve('web/operit_runtime_bridge.js'),
    );
    output.dependencies.add(packageRoot.uri.resolve('web/index.html'));

    await _run(
      'cargo',
      const ['build', '--release', '--target', 'wasm32-unknown-unknown'],
      workingDirectory: bridgeCrate.path,
      environment: {'RUSTFLAGS': '-Awarnings'},
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
  });
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
