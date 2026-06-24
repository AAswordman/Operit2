// ignore_for_file: file_names

import 'package:desktop_multi_window/desktop_multi_window.dart';
import 'package:flutter/foundation.dart';

import 'OperitWindowArguments.dart';

bool get operitSupportsDesktopMultiWindow {
  if (kIsWeb) {
    return false;
  }
  return switch (defaultTargetPlatform) {
    TargetPlatform.linux ||
    TargetPlatform.macOS ||
    TargetPlatform.windows => true,
    TargetPlatform.android ||
    TargetPlatform.fuchsia ||
    TargetPlatform.iOS => false,
  };
}

Future<OperitWindowArguments> readOperitWindowArguments() async {
  if (!operitSupportsDesktopMultiWindow) {
    return const MainWindowArguments();
  }
  final windowController = await WindowController.fromCurrentEngine();
  return OperitWindowArguments.parse(windowController.arguments);
}
