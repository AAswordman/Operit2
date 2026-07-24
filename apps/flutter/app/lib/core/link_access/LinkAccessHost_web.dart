// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import 'LinkAccessHostConfig.dart';

class LinkAccessHost extends ChangeNotifier {
  LinkAccessHost._();

  static final LinkAccessHost instance = LinkAccessHost._();

  bool get isRunning => false;

  LinkAccessHostConfig? get currentConfig => null;

  String? get deviceId => null;

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
