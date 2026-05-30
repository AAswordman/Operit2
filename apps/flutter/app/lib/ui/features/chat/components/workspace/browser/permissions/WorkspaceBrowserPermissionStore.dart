// ignore_for_file: file_names

import 'package:flutter/foundation.dart';
import 'package:webview_all/webview_all.dart';

class WorkspaceBrowserPermissionRecord {
  const WorkspaceBrowserPermissionRecord({
    required this.origin,
    required this.types,
    required this.allowed,
    required this.updatedAt,
  });

  final String origin;
  final List<WebViewPermissionResourceType> types;
  final bool allowed;
  final DateTime updatedAt;
}

class WorkspaceBrowserPermissionStore extends ChangeNotifier {
  final List<WorkspaceBrowserPermissionRecord> _records =
      <WorkspaceBrowserPermissionRecord>[];

  List<WorkspaceBrowserPermissionRecord> get records =>
      List<WorkspaceBrowserPermissionRecord>.unmodifiable(_records);

  void record({
    required String origin,
    required List<WebViewPermissionResourceType> types,
    required bool allowed,
  }) {
    _records.removeWhere((item) => item.origin == origin);
    _records.insert(
      0,
      WorkspaceBrowserPermissionRecord(
        origin: origin,
        types: types,
        allowed: allowed,
        updatedAt: DateTime.now(),
      ),
    );
    notifyListeners();
  }

  void clear() {
    _records.clear();
    notifyListeners();
  }
}
