import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

import '../hook/dependency_files.dart';

class _ProbeDirectory implements Directory {
  /// Models directory contents and failures without accessing the filesystem.
  _ProbeDirectory(String path, this.children, {this.listingError})
    : uri = Directory(path).uri;

  @override
  final Uri uri;
  final List<FileSystemEntity> children;
  final FileSystemException? listingError;
  var listCalls = 0;

  /// Fails the check if traversal requests implicit recursion or follows links.
  @override
  Stream<FileSystemEntity> list({
    bool recursive = false,
    bool followLinks = true,
  }) {
    expect(recursive, isFalse);
    expect(followLinks, isFalse);
    listCalls++;
    if (listingError != null) {
      throw listingError!;
    }
    return Stream<FileSystemEntity>.fromIterable(children);
  }

  /// Rejects filesystem operations outside the dependency walker's contract.
  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw UnsupportedError(invocation.memberName.toString());
}

/// Verifies directory pruning happens before any excluded tree is enumerated.
void main() {
  for (final name in ['target', 'node_modules', '.out']) {
    test('never lists the excluded $name directory', () async {
      final excluded = _ProbeDirectory(
        'root/$name',
        [],
        listingError: FileSystemException('Directory listing failed', name),
      );
      final sourceFile = File('root/src/main.rs');
      final source = _ProbeDirectory('root/src', [sourceFile]);
      final root = _ProbeDirectory('root', [excluded, source]);

      final files = await dependencyFiles(
        root,
        excludedDirectoryNames: {name},
      ).toList();

      expect(files.map((file) => file.uri), [sourceFile.uri]);
      expect(excluded.listCalls, 0);
      expect(source.listCalls, 1);
    });
  }

  test('matches excluded directory names exactly at every depth', () async {
    final excluded = _ProbeDirectory('root/crate/target', []);
    final sourceFile = File('root/crate/target_sources/model.rs');
    final source = _ProbeDirectory('root/crate/target_sources', [sourceFile]);
    final crate = _ProbeDirectory('root/crate', [excluded, source]);
    final root = _ProbeDirectory('root', [crate]);

    final files = await dependencyFiles(
      root,
      excludedDirectoryNames: const {'target'},
    ).toList();

    expect(files.map((file) => file.uri), [sourceFile.uri]);
    expect(excluded.listCalls, 0);
    expect(crate.listCalls, 1);
    expect(source.listCalls, 1);
  });

  test('does not follow filesystem links', () async {
    final sourceFile = File('root/main.rs');
    final root = _ProbeDirectory('root', [Link('root/alias'), sourceFile]);

    final files = await dependencyFiles(
      root,
      excludedDirectoryNames: const {},
    ).toList();

    expect(files.map((file) => file.uri), [sourceFile.uri]);
  });

  test('propagates directory listing errors from source inputs', () async {
    final failure = FileSystemException('Source directory is unreadable');
    final source = _ProbeDirectory('root/src', [], listingError: failure);
    final root = _ProbeDirectory('root', [source]);

    await expectLater(
      dependencyFiles(root, excludedDirectoryNames: const {}).toList(),
      throwsA(same(failure)),
    );
    expect(source.listCalls, 1);
  });
}
