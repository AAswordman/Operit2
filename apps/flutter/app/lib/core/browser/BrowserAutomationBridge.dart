// ignore_for_file: file_names

import '../bridge/RuntimeHostBridge.dart';

class BrowserAutomationBridge {
  const BrowserAutomationBridge();

  void Function() registerHandler(BrowserAutomationRequestHandler handler) {
    return RuntimeHostBridge.registerBrowserAutomationHandler(handler);
  }
}
