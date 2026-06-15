// ignore_for_file: file_names

import 'dart:io';

import 'package:path_provider/path_provider.dart';

class OperitClientPaths {
  const OperitClientPaths._();

  static Future<Directory> filesRootDir() async {
    final directory = await getApplicationSupportDirectory();
    await directory.create(recursive: true);
    return directory;
  }

  static Future<Directory> clientRootDir() {
    return _directory(<String>['client']);
  }

  static Future<Directory> logsDir() {
    return _directory(<String>['client', 'logs']);
  }

  static Future<File> clientLogFile() async {
    final directory = await logsDir();
    return File(_join(<String>[directory.path, 'client.log']));
  }

  static Future<Directory> linkDir() {
    return _directory(<String>['client', 'link']);
  }

  static Future<File> linkSessionsFile() async {
    final directory = await linkDir();
    return File(_join(<String>[directory.path, 'link_sessions.json']));
  }

  static Future<Directory> runtimeConnectionDir() {
    return _directory(<String>['client', 'runtime']);
  }

  static Future<File> runtimeConnectionConfigFile() async {
    final directory = await runtimeConnectionDir();
    return File(_join(<String>[directory.path, 'runtime_connection.json']));
  }

  static Future<Directory> webAccessDir() {
    return _directory(<String>['client', 'web_access']);
  }

  static Future<Directory> webAccessBundleDir() {
    return _directory(<String>['client', 'web_access', 'flutter_web']);
  }

  static Future<File> webAccessConfigFile() async {
    final directory = await webAccessDir();
    return File(_join(<String>[directory.path, 'web_access.json']));
  }

  static Future<File> webAccessStateFile() async {
    final directory = await webAccessDir();
    return File(_join(<String>[directory.path, 'web_access_state.json']));
  }

  static Future<File> webAccessAcceptedSessionsFile() async {
    final directory = await webAccessDir();
    return File(_join(<String>[directory.path, 'link_server_sessions.json']));
  }

  static Future<File> webAccessPairingCodeFile() async {
    final directory = await webAccessDir();
    return File(
      _join(<String>[directory.path, 'web_access_pairing_code.json']),
    );
  }

  static Future<Directory> tempDir() {
    return _directory(<String>['client', 'temp']);
  }

  static Future<Directory> composeDslWebviewFilesDir() {
    return _directory(<String>['client', 'temp', 'compose_dsl_webview_files']);
  }

  static Future<Directory> workspaceVideoDir() {
    return _directory(<String>['client', 'temp', 'workspace_video']);
  }

  static Future<Directory> shareImageTempDir() {
    return _directory(<String>['client', 'temp', 'share_image']);
  }

  static Future<Directory> exportsDir() {
    return _directory(<String>['client', 'exports']);
  }

  static Future<Directory> shareImageExportsDir() {
    return _directory(<String>['client', 'exports', 'share_image']);
  }

  static Future<Directory> _directory(List<String> segments) async {
    final root = await filesRootDir();
    final directory = Directory(_join(<String>[root.path, ...segments]));
    await directory.create(recursive: true);
    return directory;
  }

  static String _join(List<String> segments) {
    return segments.join(Platform.pathSeparator);
  }
}
