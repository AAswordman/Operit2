// ignore_for_file: file_names

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class WorkspaceFileEntry {
  const WorkspaceFileEntry({
    required this.name,
    required this.path,
    required this.relativePath,
    required this.isDirectory,
    required this.size,
    required this.lastModified,
  });

  factory WorkspaceFileEntry.fromProxy(core_proxy.WorkspaceFileEntry entry) {
    return WorkspaceFileEntry(
      name: entry.name,
      path: entry.path,
      relativePath: entry.relativePath,
      isDirectory: entry.isDirectory,
      size: entry.size,
      lastModified: entry.lastModified,
    );
  }

  final String name;
  final String path;
  final String relativePath;
  final bool isDirectory;
  final int size;
  final String lastModified;
}
