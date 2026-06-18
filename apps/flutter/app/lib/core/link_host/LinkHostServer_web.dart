// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import 'LinkHostConfig.dart';

class LinkHostServer extends ChangeNotifier {
  LinkHostServer._();

  static final LinkHostServer instance = LinkHostServer._();

  bool get isRunning => false;

  LinkHostConfig? get currentConfig => null;

  PendingLinkPairingCodeRecord? get lastPairingCode => null;

  String? get baseUrl => null;

  Future<List<String>> pairingBaseUrls(LinkHostConfig config) async {
    return <String>[];
  }

  Future<void> initializeFromConfig() async {}

  Future<void> start(dynamic config) async {
    throw UnsupportedError(
      'Flutter Web cannot host Web Access. Start Web Access from a native client or CLI.',
    );
  }

  Future<String> discoverDevices(int timeoutMs) async {
    return '[]';
  }

  Future<void> stop({bool updateConfig = true}) async {}
}
