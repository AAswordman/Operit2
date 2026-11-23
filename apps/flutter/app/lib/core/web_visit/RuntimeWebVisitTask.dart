// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:webview_all/webview_all.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import 'WebVisitModels.dart';

/// Exposes execution state and controls to workspace observers without giving them ownership.
abstract class RuntimeWebVisitTask extends ChangeNotifier {
  WebVisitRequest get request;
  Future<WebVisitResponse> get response;
  RuntimeBrowserSessionInfo? get browserSession;
  PlatformWebVisitHost? get host => null;
  String get statusText;
  String get leftButtonText;
  String get rightButtonText;
  String get helperText;
  bool get pageLoaded;
  String get currentUrl;
  String get pageTitle;

  /// Releases this task's observer and execution resources.
  Future<void> disposeSession();

  /// Applies the left workspace action to the task.
  void handleLeftAction();

  /// Applies the right workspace action to the task.
  void handleRightAction();

  /// Cancels only this visit.
  void cancel();
}
