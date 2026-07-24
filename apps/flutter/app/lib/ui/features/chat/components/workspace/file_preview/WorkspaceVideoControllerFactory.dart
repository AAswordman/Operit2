// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:video_player/video_player.dart';

class WorkspaceVideoControllerHandle {
  const WorkspaceVideoControllerHandle({
    required this.controller,
    required this.disposeSource,
  });

  final VideoPlayerController controller;
  final Future<void> Function() disposeSource;
}

/// Creates a video controller from an in-memory media URI.
Future<WorkspaceVideoControllerHandle> createWorkspaceVideoController(
  Uint8List bytes,
  String fileName,
) async {
  final uri = Uri.dataFromBytes(bytes, mimeType: _videoMimeType(fileName));
  final controller = VideoPlayerController.networkUrl(uri);
  await controller.initialize();
  return WorkspaceVideoControllerHandle(
    controller: controller,
    disposeSource: () async {},
  );
}

/// Returns the MIME type used for one in-memory video source.
String _videoMimeType(String fileName) {
  final extension = fileName.split('.').last.toLowerCase();
  return switch (extension) {
    'webm' => 'video/webm',
    'mov' => 'video/quicktime',
    'm4v' => 'video/x-m4v',
    _ => 'video/mp4',
  };
}
