// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

class WorkspaceBrowserTabState extends ChangeNotifier {
  WorkspaceBrowserTabState({
    required this.id,
    required this.initialUrl,
    required this.controller,
    required this.title,
    this.localFilePath,
  }) : url = initialUrl,
       addressText = initialUrl;

  final String id;
  final WebViewController controller;
  final String? localFilePath;
  final TextEditingControllerHandle addressController =
      TextEditingControllerHandle();

  String initialUrl;
  String url;
  String addressText;
  String title;
  String? errorText;
  bool isLoading = false;
  bool canGoBack = false;
  bool canGoForward = false;
  bool desktopMode = true;
  double zoomFactor = 0.4;
  int progress = 0;
  bool _disposed = false;

  bool get isDisposed => _disposed;
  int get zoomPercent => (zoomFactor * 100).round();

  String get siteHost {
    final uri = Uri.tryParse(url);
    if (uri == null || uri.host.isEmpty) {
      return '';
    }
    return uri.host;
  }

  String siteHostLabel(AppLocalizations l10n) {
    final host = siteHost;
    if (host.isEmpty) {
      return l10n.local;
    }
    return host;
  }

  String get siteInitial {
    final host = siteHost;
    if (host.isEmpty) {
      return 'L';
    }
    return host.characters.first.toUpperCase();
  }

  void update({
    String? url,
    String? addressText,
    String? title,
    String? errorText,
    bool? isLoading,
    bool? canGoBack,
    bool? canGoForward,
    bool? desktopMode,
    double? zoomFactor,
    int? progress,
  }) {
    if (_disposed) {
      return;
    }
    if (url != null) {
      this.url = url;
    }
    if (addressText != null) {
      this.addressText = addressText;
      addressController.text = addressText;
    }
    if (title != null) {
      this.title = title;
    }
    this.errorText = errorText;
    if (isLoading != null) {
      this.isLoading = isLoading;
    }
    if (canGoBack != null) {
      this.canGoBack = canGoBack;
    }
    if (canGoForward != null) {
      this.canGoForward = canGoForward;
    }
    if (desktopMode != null) {
      this.desktopMode = desktopMode;
    }
    if (zoomFactor != null) {
      this.zoomFactor = zoomFactor;
    }
    if (progress != null) {
      this.progress = progress;
    }
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    addressController.dispose();
    super.dispose();
  }
}

class TextEditingControllerHandle {
  final controller = TextEditingController();

  String get text => controller.text;

  set text(String value) {
    if (controller.text == value) {
      return;
    }
    controller.text = value;
  }

  void dispose() {
    controller.dispose();
  }
}
