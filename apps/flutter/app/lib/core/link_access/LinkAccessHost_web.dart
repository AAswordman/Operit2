// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import 'LinkAccessHostConfig.dart';
import 'WebAccessLaunchInfo.dart';

class LinkAccessHost extends ChangeNotifier {
  LinkAccessHost._();

  static final LinkAccessHost instance = LinkAccessHost._();

  bool get isRunning => false;

  LinkAccessHostConfig? get currentConfig => null;

  String? get deviceId => null;

  bool get supportsDeviceSpaceDiscovery => false;

  WebAccessLaunchInfo? get webAccessLaunchInfo {
    final uri = Uri.base;
    final token = uri.queryParameters['token'];
    if (token == null || token.isEmpty || uri.host.isEmpty) {
      return null;
    }
    final baseUrl = uri.origin;
    return WebAccessLaunchInfo(baseUrl: baseUrl, token: token);
  }

  String? get baseUrl => null;

  Future<List<String>> pairingBaseUrls(LinkAccessHostConfig config) async {
    return <String>[];
  }

  Future<void> initializeFromConfig() async {}

  Future<void> start(dynamic config) async {
    throw UnsupportedError(
      'Flutter Web cannot host Web Access. Start Web Access from a native client or CLI.',
    );
  }

  Future<void> stop({bool updateConfig = true}) async {}
}
