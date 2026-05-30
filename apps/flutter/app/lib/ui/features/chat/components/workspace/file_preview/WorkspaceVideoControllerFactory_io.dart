// ignore_for_file: file_names

import 'dart:io';
import 'dart:typed_data';

import 'package:path_provider/path_provider.dart';
import 'package:video_player/video_player.dart';

import 'WorkspaceVideoControllerFactory.dart';

Future<WorkspaceVideoControllerHandle> createWorkspaceVideoControllerImpl(
  Uint8List bytes,
  String fileName,
) async {
  final directory = await getTemporaryDirectory();
  final file = File(
    '${directory.path}/operit-workspace-${DateTime.now().microsecondsSinceEpoch}-$fileName',
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
