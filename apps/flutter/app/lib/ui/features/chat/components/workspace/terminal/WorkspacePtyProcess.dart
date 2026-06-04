// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'WorkspacePtyProcess_stub.dart'
    if (dart.library.io) 'WorkspacePtyProcess_io.dart';

abstract class WorkspacePtyProcess {
  String get sessionId;
  Stream<Uint8List> get output;
  Future<int> get exitCode;

  void write(Uint8List data);
  void resize(int rows, int columns);
  void kill();
}

Future<WorkspacePtyProcess> startWorkspacePty({
  required String sessionName,
  required String workingDirectory,
  required int rows,
  required int columns,
}) {
  return startWorkspacePtyImpl(
    sessionName: sessionName,
    workingDirectory: workingDirectory,
    rows: rows,
    columns: columns,
  );
}

WorkspacePtyProcess attachWorkspacePty(String sessionId) {
  return attachWorkspacePtyImpl(sessionId);
}
