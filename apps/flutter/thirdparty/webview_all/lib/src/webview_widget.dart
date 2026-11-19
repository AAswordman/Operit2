// Copyright 2013 The Flutter Authors
// Use of this source code is governed by a BSD-style license that can be
// found in the LICENSE file.

import 'dart:async';

import 'package:flutter/cupertino.dart';
import 'package:flutter/foundation.dart' show Factory;
import 'package:flutter/gestures.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

import 'webview_controller.dart';

/// Describes the uniform paint scale applied by the embedding application.
///
/// For native overlays, [WebViewWidget] cancels this scale around the viewport and
/// lays the viewport out at its displayed size instead. This keeps AppKit mouse
/// coordinates aligned with the view without changing the host device pixel
/// ratio. The controller combines this application scale with its page zoom
/// using native content zoom, so web content still follows application zoom.
///
/// Place this scope below the application's scaling widget. [scale] must be the
/// total uniform ancestor scale, not the screen's device pixel ratio.
class WebViewScaleScope extends InheritedWidget {
  const WebViewScaleScope({
    super.key,
    required this.scale,
    required super.child,
  }) : assert(scale > 0 && scale < double.infinity);

  final double scale;

  static double scaleOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<WebViewScaleScope>()?.scale ??
      1;

  @override
  bool updateShouldNotify(WebViewScaleScope oldWidget) =>
      scale != oldWidget.scale;
}

/// Displays a native WebView as a Widget.
///
/// ## Platform-Specific Features
/// This class contains an underlying implementation provided by the current
/// platform. Once a platform implementation is imported, the examples below
/// can be followed to use features provided by a platform's implementation.
///
/// {@macro webview_flutter.WebViewWidget.fromPlatformCreationParams}
///
/// Below is an example of accessing the platform-specific implementation for
/// iOS and Android:
///
/// ```dart
/// final WebViewController controller = WebViewController();
///
/// final WebViewWidget webViewWidget = WebViewWidget(controller: controller);
///
/// if (WebViewPlatform.instance is WebKitWebViewPlatform) {
///   final WebKitWebViewWidget webKitWidget =
///       webViewWidget.platform as WebKitWebViewWidget;
/// } else if (WebViewPlatform.instance is AndroidWebViewPlatform) {
///   final AndroidWebViewWidget androidWidget =
///       webViewWidget.platform as AndroidWebViewWidget;
/// }
/// ```
class WebViewWidget extends StatefulWidget {
  /// Constructs a [WebViewWidget].
  ///
  /// See [WebViewWidget.fromPlatformCreationParams] for setting parameters for
  /// a specific platform.
  WebViewWidget({
    Key? key,
    required WebViewController controller,
    Future<void> Function()? onBeforeDispose,
    TextDirection layoutDirection = TextDirection.ltr,
    Set<Factory<OneSequenceGestureRecognizer>> gestureRecognizers =
        const <Factory<OneSequenceGestureRecognizer>>{},
  }) : this.fromPlatformCreationParams(
         key: key,
         onBeforeDispose: onBeforeDispose,
         params: PlatformWebViewWidgetCreationParams(
           controller: controller.platform,
           layoutDirection: layoutDirection,
           gestureRecognizers: gestureRecognizers,
         ),
       );

  /// Constructs a [WebViewWidget] from creation params for a specific platform.
  ///
  /// {@template webview_flutter.WebViewWidget.fromPlatformCreationParams}
  /// Below is an example of setting platform-specific creation parameters for
  /// iOS and Android:
  ///
  /// ```dart
  /// final WebViewController controller = WebViewController();
  ///
  /// PlatformWebViewWidgetCreationParams params =
  ///     PlatformWebViewWidgetCreationParams(
  ///   controller: controller.platform,
  ///   layoutDirection: TextDirection.ltr,
  ///   gestureRecognizers: const <Factory<OneSequenceGestureRecognizer>>{},
  /// );
  ///
  /// if (WebViewPlatform.instance is WebKitWebViewPlatform) {
  ///   params = WebKitWebViewWidgetCreationParams
  ///       .fromPlatformWebViewWidgetCreationParams(
  ///     params,
  ///   );
  /// } else if (WebViewPlatform.instance is AndroidWebViewPlatform) {
  ///   params = AndroidWebViewWidgetCreationParams
  ///       .fromPlatformWebViewWidgetCreationParams(
  ///     params,
  ///   );
  /// }
  ///
  /// final WebViewWidget webViewWidget =
  ///     WebViewWidget.fromPlatformCreationParams(
  ///   params: params,
  /// );
  /// ```
  /// {@endtemplate}
  WebViewWidget.fromPlatformCreationParams({
    Key? key,
    required PlatformWebViewWidgetCreationParams params,
    Future<void> Function()? onBeforeDispose,
  }) : this.fromPlatform(
         key: key,
         platform: PlatformWebViewWidget(params),
         onBeforeDispose: onBeforeDispose,
       );

  /// Constructs a [WebViewWidget] from a specific platform implementation.
  WebViewWidget.fromPlatform({
    super.key,
    required this.platform,
    this.onBeforeDispose,
  });

  /// Completes application resource cleanup before the browser is released.
  final Future<void> Function()? onBeforeDispose;

  /// Implementation of [PlatformWebViewWidget] for the current platform.
  final PlatformWebViewWidget platform;

  /// The layout direction to use for the embedded WebView.
  late final TextDirection layoutDirection = platform.params.layoutDirection;

  /// Specifies which gestures should be consumed by the web view.
  ///
  /// It is possible for other gesture recognizers to be competing with the web
  /// view on pointer events, e.g. if the web view is inside a [ListView] the
  /// [ListView] will want to handle vertical drags. The web view will claim
  /// gestures that are recognized by any of the recognizers on this list.
  ///
  /// When `gestureRecognizers` is empty (default), the web view will only
  /// handle pointer events for gestures that were not claimed by any other
  /// gesture recognizer.
  late final Set<Factory<OneSequenceGestureRecognizer>> gestureRecognizers =
      platform.params.gestureRecognizers;

  /// Tracks the mounted browser independently of parent rebuilds.
  @override
  State<WebViewWidget> createState() => _WebViewWidgetState();
}

class _WebViewWidgetState extends State<WebViewWidget> {
  static final _lifetimes = Expando<_WebViewLifetime>();
  late _WebViewLifetime _lifetime;

  /// Acquires ownership of the mounted platform browser.
  @override
  void initState() {
    super.initState();
    _attach();
  }

  /// Uses platform identity across every public widget constructor.
  void _attach() {
    final controller = widget.platform.params.controller;
    _lifetime = _lifetimes[controller] ??= _WebViewLifetime(controller);
    _lifetime.attach();
  }

  /// Releases a replaced controller and acquires the new browser.
  @override
  void didUpdateWidget(WebViewWidget oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (!identical(
      oldWidget.platform.params.controller,
      widget.platform.params.controller,
    )) {
      _lifetime.detach(oldWidget.onBeforeDispose);
      _attach();
    }
  }

  /// Releases ownership when the surface leaves the widget tree.
  @override
  void dispose() {
    _lifetime.detach(widget.onBeforeDispose);
    super.dispose();
  }

  /// Builds the platform surface while retaining application zoom behavior.
  @override
  Widget build(BuildContext context) {
    final platform = widget.platform;
    final Widget view = platform.build(context);
    if (!platform.params.controller.requiresNativeApplicationZoom) {
      return view;
    }
    return _NativeZoomWebViewViewport(
      controller: platform.params.controller,
      scale: WebViewScaleScope.scaleOf(context),
      child: view,
    );
  }
}

class _WebViewLifetime {
  /// Owns the browser shared by its mounted surface wrappers.
  _WebViewLifetime(this.controller);

  final PlatformWebViewController controller;
  int _mounts = 0;
  bool _disposing = false;

  /// Retains the browser during rebuilds and same-frame surface transfers.
  void attach() {
    if (_disposing)
      throw StateError('Cannot mount a disposed WebView controller');
    _mounts++;
  }

  /// Closes the browser after its final surface unmounts.
  void detach(Future<void> Function()? onBeforeDispose) {
    _mounts--;
    scheduleMicrotask(() async {
      if (_mounts != 0 || _disposing) return;
      _disposing = true;
      try {
        try {
          await onBeforeDispose?.call();
        } finally {
          await controller.dispose();
        }
      } catch (error, stackTrace) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: error,
            stack: stackTrace,
            library: 'webview_all',
            context: ErrorDescription('while releasing an unmounted WebView'),
          ),
        );
      }
    });
  }
}

class _NativeZoomWebViewViewport extends StatefulWidget {
  const _NativeZoomWebViewViewport({
    required this.controller,
    required this.scale,
    required this.child,
  });

  final PlatformWebViewController controller;
  final double scale;
  final Widget child;

  @override
  State<_NativeZoomWebViewViewport> createState() =>
      _NativeZoomWebViewViewportState();
}

class _NativeZoomWebViewViewportState
    extends State<_NativeZoomWebViewViewport> {
  @override
  void initState() {
    super.initState();
    unawaited(_applyContentScale());
  }

  @override
  void didUpdateWidget(_NativeZoomWebViewViewport oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.scale != widget.scale ||
        oldWidget.controller != widget.controller) {
      unawaited(_applyContentScale());
    }
  }

  Future<void> _applyContentScale() async {
    try {
      await widget.controller.setApplicationZoomFactor(widget.scale);
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'webview_all',
          context: ErrorDescription(
            'while applying native WebView content zoom',
          ),
        ),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    // AppKit cannot map CALayer scaling into NSView input coordinates, while
    // GTK overlays cannot represent paint scaling at all. These backends opt
    // into native content zoom; composited backends retain Flutter paint zoom.
    // Cancel only the application's scale, preserving positioning and clipping.
    // Keep the same widget structure even at 1x so zoom does not remount AppKitView.
    return LayoutBuilder(
      builder: (BuildContext context, BoxConstraints constraints) => FittedBox(
        fit: BoxFit.fill,
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: constraints.maxWidth * widget.scale,
          height: constraints.maxHeight * widget.scale,
          child: widget.child,
        ),
      ),
    );
  }
}
