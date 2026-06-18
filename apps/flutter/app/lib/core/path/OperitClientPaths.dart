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

  static Future<File> outboundLinkSessionsFile() async {
    final directory = await linkDir();
    return File(_join(<String>[directory.path, 'outbound_sessions.json']));
  }

  static Future<File> runtimeConnectionConfigFile() async {
    final directory = await linkDir();
    return File(_join(<String>[directory.path, 'runtime_connection.json']));
  }

  static Future<Directory> linkHostDir() {
    return linkDir();
  }

  static Future<Directory> linkHostWebAccessBundleDir() {
    return _directory(<String>['client', 'link', 'web_access_bundle']);
  }

  static Future<File> linkHostConfigFile() async {
    final directory = await linkHostDir();
    return File(_join(<String>[directory.path, 'host_config.json']));
  }

  static Future<File> linkHostStateFile() async {
    final directory = await linkHostDir();
    return File(_join(<String>[directory.path, 'host_state.json']));
  }

  static Future<File> linkHostDeviceIdFile() async {
    final directory = await linkHostDir();
    return File(_join(<String>[directory.path, 'host_device_id']));
  }

  static Future<File> inboundLinkSessionsFile() async {
    final directory = await linkHostDir();
    return File(_join(<String>[directory.path, 'inbound_sessions.json']));
  }

  static Future<File> pendingLinkPairingCodeFile() async {
    final directory = await linkHostDir();
    return File(_join(<String>[directory.path, 'pending_pairing_code.json']));
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
