// ignore_for_file: file_names

import '../bridge/RuntimeHostBridge.dart';

class WebVisitBridge {
  const WebVisitBridge();

  void Function() registerHandler(WebVisitRequestHandler handler) {
    return RuntimeHostBridge.registerWebVisitHandler(handler);
  }
}
