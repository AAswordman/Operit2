// ignore_for_file: file_names

import 'package:flutter/foundation.dart';

import 'WebAccessConfig.dart';

class FlutterWebAccessServer extends ChangeNotifier {
  FlutterWebAccessServer._();

  static final FlutterWebAccessServer instance = FlutterWebAccessServer._();

  bool get isRunning => false;

  WebAccessPairingCodeRecord? get lastPairingCode => null;

  String? get baseUrl => null;

  Future<List<String>> pairingBaseUrls(WebAccessConfig config) async {
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
