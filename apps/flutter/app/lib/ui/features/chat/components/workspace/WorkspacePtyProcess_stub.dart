// ignore_for_file: file_names

import 'WorkspacePtyProcess.dart';

Future<WorkspacePtyProcess> startWorkspacePtyImpl({
  required String workingDirectory,
  required int rows,
  required int columns,
}) {
  throw UnsupportedError('PTY terminal is not supported on this platform.');
}
