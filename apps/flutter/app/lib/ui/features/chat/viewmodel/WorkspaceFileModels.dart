// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Uses the generated Core workspace entry directly in Flutter.
typedef WorkspaceFileEntry = core_proxy.WorkspaceFileEntry;

enum WorkspaceFileOperation { delete, copy, move }

typedef WorkspaceFileMutation =
    Future<void> Function(
      WorkspaceFileOperation operation,
      String path,
      String? destinationPath,
    );

class WorkspaceFileClipboardEntry {
  const WorkspaceFileClipboardEntry(this.entry, {required this.cut});
  final WorkspaceFileEntry entry;
  final bool cut;
}

/// Shared by the file tabs in one chat; cleared when the workspace changes.
class WorkspaceFileClipboard
    extends ValueNotifier<WorkspaceFileClipboardEntry?> {
  WorkspaceFileClipboard() : super(null);
}
