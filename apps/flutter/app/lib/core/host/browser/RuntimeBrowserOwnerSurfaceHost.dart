// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:operit2/ui/features/chat/components/workspace/browser/tabs/WorkspaceBrowserTabModels.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/webview_all_windows.dart';

import 'RuntimeBrowserOwner.dart';

class RuntimeBrowserOwnerSurfaceHost extends StatelessWidget {
  /// Creates the runtime owner surface host around the application tree.
  const RuntimeBrowserOwnerSurfaceHost({super.key, required this.child});

  final Widget child;

  /// Keeps every real owner WebView mounted independently from workspace views.
  @override
  Widget build(BuildContext context) {
    final sessions = RuntimeBrowserOwner.instance;
    return Stack(
      clipBehavior: Clip.none,
      fit: StackFit.expand,
      children: <Widget>[
        child,
        Positioned(
          left: -4096,
          top: -4096,
          width: 1,
          height: 1,
          child: IgnorePointer(
            child: AnimatedBuilder(
              animation: sessions,
              builder: (context, child) {
                return Stack(
                  children: <Widget>[
                    for (final tab in sessions.tabs)
                      if (!sessions.isWorkspaceSurfaceSession(tab.id))
                        RuntimeBrowserOwnerWebView(
                          tab: tab,
                          layoutControlsSurfaceSize: false,
                        ),
                  ],
                );
              },
            ),
          ),
        ),
      ],
    );
  }
}

class RuntimeBrowserOwnerWebView extends StatelessWidget {
  /// Creates a real owner WebView widget for one browser session.
  const RuntimeBrowserOwnerWebView({
    super.key,
    required this.tab,
    required this.layoutControlsSurfaceSize,
  });

  final WorkspaceBrowserTabState tab;
  final bool layoutControlsSurfaceSize;

  /// Builds the platform WebView attached to the owner controller.
  @override
  Widget build(BuildContext context) {
    final key = ValueKey<String>('runtime-browser-owner-${tab.id}');
    final platformController = tab.controller.platform;
    if (platformController is WindowsWebViewController) {
      return WebViewWidget.fromPlatformCreationParams(
        key: key,
        params: WindowsWebViewWidgetCreationParams(
          controller: platformController,
          layoutDirection: Directionality.of(context),
          layoutControlsSurfaceSize: layoutControlsSurfaceSize,
        ),
      );
    }
    return WebViewWidget(key: key, controller: tab.controller);
  }
}
