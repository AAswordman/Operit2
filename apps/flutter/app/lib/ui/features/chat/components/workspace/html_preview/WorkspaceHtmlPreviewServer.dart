// ignore_for_file: file_names

import 'WorkspaceHtmlPreviewServer_io.dart'
    if (dart.library.html) 'WorkspaceHtmlPreviewServer_web.dart';

abstract class WorkspaceHtmlPreviewServer {
  factory WorkspaceHtmlPreviewServer({
    required WorkspaceHtmlFileReader onReadWorkspaceFileBytes,
  }) = WorkspaceHtmlPreviewServerImpl;

  Future<Uri> start(String entryPath);
  Future<void> stop();
}

typedef WorkspaceHtmlFileReader = Future<List<int>> Function(String path);
