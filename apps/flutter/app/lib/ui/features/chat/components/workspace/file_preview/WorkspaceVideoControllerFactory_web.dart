// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:video_player/video_player.dart';

import 'WorkspaceVideoControllerFactory.dart';

Future<WorkspaceVideoControllerHandle> createWorkspaceVideoControllerImpl(
  Uint8List bytes,
  String fileName,
) async {
  final uri = Uri.dataFromBytes(bytes, mimeType: _mimeType(fileName));
  final controller = VideoPlayerController.networkUrl(uri);
  await controller.initialize();
  return WorkspaceVideoControllerHandle(
    controller: controller,
    disposeSource: () async {},
  );
}

String _mimeType(String fileName) {
  final extension = fileName.split('.').last.toLowerCase();
  switch (extension) {
    case 'webm':
      return 'video/webm';
    case 'mov':
      return 'video/quicktime';
    case 'm4v':
      return 'video/x-m4v';
    case 'mp4':
    default:
      return 'video/mp4';
  }
}
