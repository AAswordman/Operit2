// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:video_player/video_player.dart';

import 'WorkspaceVideoControllerFactory_io.dart'
    if (dart.library.html) 'WorkspaceVideoControllerFactory_web.dart';

class WorkspaceVideoControllerHandle {
  const WorkspaceVideoControllerHandle({
    required this.controller,
    required this.disposeSource,
  });

  final VideoPlayerController controller;
  final Future<void> Function() disposeSource;
}

Future<WorkspaceVideoControllerHandle> createWorkspaceVideoController(
  Uint8List bytes,
  String fileName,
) {
  return createWorkspaceVideoControllerImpl(bytes, fileName);
}
