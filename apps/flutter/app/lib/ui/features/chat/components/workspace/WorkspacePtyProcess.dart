// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';

import 'WorkspacePtyProcess_stub.dart'
    if (dart.library.io) 'WorkspacePtyProcess_io.dart';

abstract class WorkspacePtyProcess {
  Stream<Uint8List> get output;
  Future<int> get exitCode;

  void write(Uint8List data);
  void resize(int rows, int columns);
  void kill();
}

Future<WorkspacePtyProcess> startWorkspacePty({
  required String workingDirectory,
  required int rows,
  required int columns,
}) {
  return startWorkspacePtyImpl(
    workingDirectory: workingDirectory,
    rows: rows,
    columns: columns,
  );
}
