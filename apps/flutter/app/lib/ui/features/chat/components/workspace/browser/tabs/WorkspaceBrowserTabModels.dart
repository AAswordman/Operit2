// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'dart:typed_data';
import 'package:webview_all/webview_all.dart';

import '../../../../../../../l10n/generated/app_localizations.dart';

class WorkspaceBrowserTabState extends ChangeNotifier {
  /// Creates a browser session state owned by the workspace browser host.
  WorkspaceBrowserTabState({
    required this.id,
    required this.initialUrl,
    required this.title,
    required this.capabilities,
    WebViewController? controller,
    this.localFilePath,
    this.preferredUserAgent,
    this.requestHeaders = const <String, String>{},
  }) : _controller = controller,
       url = initialUrl,
       addressText = initialUrl;

  final String id;
  final WebViewController? _controller;
  final WorkspaceBrowserSessionCapabilities capabilities;
  final String? localFilePath;
  String? preferredUserAgent;
  Map<String, String> requestHeaders;
  WorkspaceBrowserSurfaceDescriptor? surfaceDescriptor;
  WorkspaceBrowserSurfaceFrame? surfaceFrame;
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

  /// Returns the owner-side WebView controller for host-owned tabs.
  WebViewController get controller {
    final value = _controller;
    if (value == null) {
      throw StateError('Browser tab is attached to a compositor surface');
    }
    return value;
  }

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

  /// Replaces the compositor surface descriptor used by attached views.
  void updateSurfaceDescriptor(WorkspaceBrowserSurfaceDescriptor descriptor) {
    if (_disposed) {
      return;
    }
    surfaceDescriptor = descriptor;
    notifyListeners();
  }

  /// Replaces the latest compositor frame received through Core Link.
  void updateSurfaceFrame(WorkspaceBrowserSurfaceFrame frame) {
    if (_disposed) {
      return;
    }
    surfaceFrame = frame;
    notifyListeners();
  }

  /// Replaces request metadata owned by the real browser session.
  void updateRequestMetadata({
    required String? userAgent,
    required Map<String, String> headers,
  }) {
    preferredUserAgent = userAgent;
    requestHeaders = Map<String, String>.unmodifiable(headers);
    notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    addressController.dispose();
    super.dispose();
  }
}

class WorkspaceBrowserSurfaceDescriptor {
  /// Creates the browser compositor surface descriptor.
  const WorkspaceBrowserSurfaceDescriptor({
    required this.transport,
    required this.platform,
    this.textureId,
    this.streamId,
    this.codec,
    this.width,
    this.height,
  });

  /// Decodes one browser compositor descriptor from Core JSON.
  factory WorkspaceBrowserSurfaceDescriptor.fromJson(
    Map<String, Object?> json,
  ) {
    return WorkspaceBrowserSurfaceDescriptor(
      transport: json['transport'] as String,
      platform: json['platform'] as String,
      textureId: json['textureId'] as int?,
      streamId: json['streamId'] as String?,
      codec: json['codec'] as String?,
      width: (json['width'] as num?)?.toDouble(),
      height: (json['height'] as num?)?.toDouble(),
    );
  }

  final String transport;
  final String platform;
  final int? textureId;
  final String? streamId;
  final String? codec;
  final double? width;
  final double? height;
}

class WorkspaceBrowserSurfaceFrame {
  /// Creates one browser compositor frame received through Core Link.
  const WorkspaceBrowserSurfaceFrame({
    required this.data,
    required this.codec,
    required this.width,
    required this.height,
  });

  final Uint8List data;
  final String codec;
  final int width;
  final int height;
}

class TextEditingControllerHandle {
  /// Creates a retained address text editing controller.
  TextEditingControllerHandle();

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

class WorkspaceBrowserSessionCapabilities {
  /// Creates the capability set exposed by one browser host session.
  const WorkspaceBrowserSessionCapabilities({
    required this.pageJavaScript,
    required this.pageHooks,
    required this.permissionRequests,
    required this.javaScriptDialogs,
  });

  final bool pageJavaScript;
  final bool pageHooks;
  final bool permissionRequests;
  final bool javaScriptDialogs;
}
