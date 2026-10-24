import 'dart:io';

/// Lists dependency files without entering excluded directories or following links.
Stream<File> dependencyFiles(
  Directory root, {
  required Set<String> excludedDirectoryNames,
}) async* {
  final pending = <Directory>[root];
  while (pending.isNotEmpty) {
    final directory = pending.removeLast();
    await for (final entity in directory.list(
      recursive: false,
      followLinks: false,
    )) {
      if (entity is File) {
        yield entity;
      } else if (entity is Directory) {
        final name = entity.uri.pathSegments.lastWhere(
          (segment) => segment.isNotEmpty,
        );
        if (!excludedDirectoryNames.contains(name)) {
          pending.add(entity);
        }
      }
    }
  }
}
