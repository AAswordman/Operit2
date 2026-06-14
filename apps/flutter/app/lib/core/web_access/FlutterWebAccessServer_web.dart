// ignore_for_file: file_names

class FlutterWebAccessServer {
  FlutterWebAccessServer._();

  static final FlutterWebAccessServer instance = FlutterWebAccessServer._();

  bool get isRunning => false;

  String? get baseUrl => null;

  Future<void> initializeFromConfig() async {}

  Future<void> start(dynamic config) async {
    throw UnsupportedError(
      'Flutter Web cannot host Web Access. Start Web Access from a native client or CLI.',
    );
  }

  Future<void> stop({bool updateConfig = true}) async {}
}
