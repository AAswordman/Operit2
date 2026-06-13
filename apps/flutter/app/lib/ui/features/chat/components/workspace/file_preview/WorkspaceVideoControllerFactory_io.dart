// ignore_for_file: file_names

import 'dart:io';
import 'dart:typed_data';

import 'package:operit2/core/path/OperitClientPaths.dart';
import 'package:video_player/video_player.dart';

import 'WorkspaceVideoControllerFactory.dart';

Future<WorkspaceVideoControllerHandle> createWorkspaceVideoControllerImpl(
  Uint8List bytes,
  String fileName,
) async {
  final directory = await OperitClientPaths.workspaceVideoDir();
  final file = File(
    '${directory.path}${Platform.pathSeparator}${DateTime.now().microsecondsSinceEpoch}-$fileName',
  );
  await file.writeAsBytes(bytes, flush: true);
  final controller = VideoPlayerController.file(file);
  await controller.initialize();
  return WorkspaceVideoControllerHandle(
    controller: controller,
    disposeSource: () async {
      await file.delete();
    },
  );
}
