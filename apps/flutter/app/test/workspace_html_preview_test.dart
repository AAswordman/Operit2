import 'support/native_sidebar_bridge.dart';

import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/features/chat/components/workspace/html_preview/WorkspaceHtmlPreviewServer.dart';
import 'package:operit2/ui/features/chat/components/workspace/html_preview/WorkspaceHtmlPreviewWidget.dart';
import 'package:operit2/ui/main/components/DrawerConversationState.dart';
import 'package:operit2/ui/main/layout/PhoneLayout.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_flutter_platform_interface/webview_flutter_platform_interface.dart';

Future<Uint8List> _readFile(String path) async => Uint8List(0);

void main() {
  setUp(ClientLogger.initialize);

  for (final target in [TargetPlatform.android, TargetPlatform.iOS]) {
    for (final animate in [false, true]) {
      testWidgets(
        'phone preview owns touch gestures and retains its view (effects: $animate)',
        (tester) async {
          tester.view.devicePixelRatio = 1;
          tester.view.physicalSize = const Size(390, 844);
          addTearDown(tester.view.resetDevicePixelRatio);
          addTearDown(tester.view.resetPhysicalSize);
          final originalPlatform = WebViewPlatform.instance;
          final platform = _PreviewPlatform();
          WebViewPlatform.instance = platform;
          if (originalPlatform != null) {
            addTearDown(() => WebViewPlatform.instance = originalPlatform);
          }
          final drawerOpen = ValueNotifier(false);
          final conversations = ValueNotifier(
            const DrawerConversationState(loading: false),
          );
          addTearDown(drawerOpen.dispose);
          addTearDown(conversations.dispose);
          var ancestorDrags = 0;
          final server = _PreviewServer();

          Widget screen(String path) => OperitTheme(
            initialThemePreferenceSnapshot:
                UserPreferencesManager.defaultThemePreferenceSnapshot,
            initialThemeIsReady: false,
            unconfiguredChildEnabled: true,
            hostInteractionHostsEnabled: false,
            child: PhoneLayout(
              bridge: NativeSidebarBridge(),
              drawerWidth: 280,
              drawerOpenState: drawerOpen,
              drawerConversationState: conversations,
              enableNavigationAnimation: animate,
              navigationEntries: const [],
              pluginSidebarEntries: const [],
              selectedRouteId: 'chat',
              onOpenDrawer: () => drawerOpen.value = true,
              onCloseDrawer: () => drawerOpen.value = false,
              onNavigationEntrySelected: (_) {},
              onConversationActivated: () {},
              content: AdaptiveSidePanel(
                open: true,
                animate: false,
                onOpenChanged: (_) {},
                panel: GestureDetector(
                  onVerticalDragUpdate: (_) => ancestorDrags++,
                  child: WorkspaceHtmlPreviewWidget(
                    relativePath: path,
                    previewServer: server,
                    onReadWorkspaceFileBytes: _readFile,
                  ),
                ),
                child: const SizedBox.expand(),
              ),
            ),
          );

          await tester.pumpWidget(screen('index.html'));
          await tester.pumpAndSettle();
          expect(platform.controller.loads.single.path, '/index.html');
          expect(platform.widgetCreations, 1);
          final view = tester.widget<WebViewWidget>(find.byType(WebViewWidget));
          expect(view.gestureRecognizers.single.type, EagerGestureRecognizer);
          final element = tester.element(find.byType(WebViewWidget));
          final initialBuilds = platform.widgetBuilds;

          // The fake platform surface uses the production recognizer factories.
          // These drags must not reach the parent or open the phone drawer.
          await tester.drag(find.byType(WebViewWidget), const Offset(0, -180));
          await tester.drag(find.byType(WebViewWidget), const Offset(130, 20));
          await tester.pumpAndSettle();
          expect(ancestorDrags, 0);
          expect(drawerOpen.value, isFalse);

          platform.delegate.started!('http://127.0.0.1:8093/index.html');
          await tester.pump();
          expect(find.byType(CircularProgressIndicator), findsOneWidget);
          platform.controller.backAvailable = true;
          platform.delegate.finished!('http://127.0.0.1:8093/index.html');
          await tester.pumpAndSettle();
          expect(find.byType(CircularProgressIndicator), findsNothing);
          expect(platform.widgetCreations, 1);
          expect(platform.widgetBuilds, initialBuilds);
          expect(tester.element(find.byType(WebViewWidget)), same(element));

          // Keep the native view mounted even while navigation is pending.
          final pendingLoad = Completer<void>();
          platform.controller.pendingLoad = pendingLoad.future;
          await tester.pumpWidget(screen('other.html'));
          await tester.pump();
          expect(platform.controller.loads.last.path, '/other.html');
          expect(find.byType(CircularProgressIndicator), findsOneWidget);
          expect(tester.element(find.byType(WebViewWidget)), same(element));
          expect(platform.widgetCreations, 1);
          pendingLoad.complete();
          await tester.pumpAndSettle();
          expect(platform.widgetBuilds, initialBuilds);

          // A slow resource lookup must not overwrite a more recent file.
          platform.controller.pendingLoad = null;
          final slowStart = Completer<void>();
          server.pendingStarts['slow.html'] = slowStart.future;
          await tester.pumpWidget(screen('slow.html'));
          await tester.pumpWidget(screen('latest.html'));
          await tester.pumpAndSettle();
          slowStart.complete();
          await tester.pumpAndSettle();
          expect(platform.controller.loads.last.path, '/latest.html');
          expect(
            platform.controller.loads.map((uri) => uri.path),
            isNot(contains('/slow.html')),
          );
          expect(tester.element(find.byType(WebViewWidget)), same(element));

          // Disposing during lookup must not navigate a detached native view.
          final disposedStart = Completer<void>();
          server.pendingStarts['disposed.html'] = disposedStart.future;
          await tester.pumpWidget(screen('disposed.html'));
          await tester.pumpWidget(const SizedBox.shrink());
          disposedStart.complete();
          await tester.pumpAndSettle();
          expect(platform.controller.loads.last.path, '/latest.html');
          expect(tester.takeException(), isNull);
        },
        variant: TargetPlatformVariant.only(target),
      );
    }
  }
}

class _PreviewPlatform extends WebViewPlatform {
  final controller = _PreviewController();
  late _PreviewDelegate delegate;
  int widgetCreations = 0;
  int widgetBuilds = 0;

  @override
  PlatformWebViewController createPlatformWebViewController(
    PlatformWebViewControllerCreationParams params,
  ) => controller;

  @override
  PlatformNavigationDelegate createPlatformNavigationDelegate(
    PlatformNavigationDelegateCreationParams params,
  ) => delegate = _PreviewDelegate(params);

  @override
  PlatformWebViewWidget createPlatformWebViewWidget(
    PlatformWebViewWidgetCreationParams params,
  ) {
    widgetCreations++;
    return _PreviewView(params, this);
  }
}

class _PreviewView extends PlatformWebViewWidget {
  _PreviewView(super.params, this.owner) : super.implementation();
  final _PreviewPlatform owner;

  @override
  Widget build(BuildContext context) {
    owner.widgetBuilds++;
    return RawGestureDetector(
      behavior: HitTestBehavior.opaque,
      gestures: {
        for (final factory in params.gestureRecognizers)
          factory.type:
              GestureRecognizerFactoryWithHandlers<EagerGestureRecognizer>(
                () => factory.constructor() as EagerGestureRecognizer,
                (_) {},
              ),
      },
      child: const SizedBox.expand(),
    );
  }
}

class _PreviewController extends PlatformWebViewController {
  _PreviewController()
    : super.implementation(const PlatformWebViewControllerCreationParams());
  final loads = <Uri>[];
  Future<void>? pendingLoad;
  bool backAvailable = false;

  @override
  Future<void> loadRequest(LoadRequestParams params) async {
    loads.add(params.uri);
    await pendingLoad;
  }

  @override
  Future<void> setJavaScriptMode(JavaScriptMode mode) async {}
  @override
  Future<void> setBackgroundColor(Color color) async {}
  @override
  Future<void> setPlatformNavigationDelegate(
    PlatformNavigationDelegate handler,
  ) async {}
  @override
  Future<bool> canGoBack() async => backAvailable;
  @override
  Future<bool> canGoForward() async => false;
}

class _PreviewDelegate extends PlatformNavigationDelegate {
  _PreviewDelegate(super.params) : super.implementation();
  PageEventCallback? started;
  PageEventCallback? finished;

  @override
  Future<void> setOnPageStarted(PageEventCallback callback) async {
    started = callback;
  }

  @override
  Future<void> setOnPageFinished(PageEventCallback callback) async {
    finished = callback;
  }
}

class _PreviewServer implements WorkspaceHtmlPreviewServer {
  final pendingStarts = <String, Future<void>>{};

  @override
  Future<Uri> start(String entryPath) async {
    await pendingStarts[entryPath];
    return Uri.parse('http://127.0.0.1:8093/$entryPath');
  }

  @override
  Future<void> stop() async {}
}
