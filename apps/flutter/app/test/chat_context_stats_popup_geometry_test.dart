import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/features/chat/components/workspace/ChatContextStatsTopBarButton.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

/// Keeps the real context menu inside the primary pane of a split workspace.
void main() {
  for (final direction in TextDirection.values) {
    for (final screenWidth in <double>[400, 900, 1440]) {
      testWidgets('context menu respects pane bounds at $screenWidth / $direction', (
        tester,
      ) async {
        tester.view.physicalSize = Size(screenWidth, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final open = ValueNotifier<bool>(false);
        final width = ValueNotifier<double>(screenWidth >= 1200
            ? screenWidth - 280
            : screenWidth);
        addTearDown(open.dispose);
        addTearDown(width.dispose);
        const chatKey = ValueKey('chat-pane');
        const panelKey = ValueKey('workspace-pane');
        final content = Column(
          key: chatKey,
          children: <Widget>[
            SizedBox(
              height: 56,
              child: Row(
                mainAxisAlignment: MainAxisAlignment.end,
                children: <Widget>[
                  ChatContextStatsTopBarButton(
                    viewModel: ChatViewModel(bridge: _PendingStatsBridge()),
                    currentChatId: null,
                  ),
                ],
              ),
            ),
            const Expanded(child: SizedBox.expand()),
          ],
        );
        await tester.pumpWidget(
          MaterialApp(
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            home: Directionality(
              textDirection: direction,
              child: Scaffold(
                body: Align(
                  alignment: AlignmentDirectional.centerEnd,
                  child: ListenableBuilder(
                    listenable: Listenable.merge(<Listenable>[open, width]),
                    child: content,
                    builder: (context, child) => SizedBox(
                      width: width.value,
                      child: AdaptiveSidePanel(
                        open: open.value,
                        onOpenChanged: (value) => open.value = value,
                        animate: false,
                        defaultWidthFraction: 0.45,
                        panel: const SizedBox.expand(key: panelKey),
                        child: child!,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();

        final hostWidths = screenWidth >= 900
            ? <double>[width.value, 640]
            : <double>[width.value];
        for (final hostWidth in hostWidths) {
          width.value = hostWidth;
          for (final expanded in <bool>[false, if (hostWidth >= 600) true]) {
            open.value = expanded;
            await tester.pumpAndSettle();
            await tester.tap(find.byIcon(Icons.data_usage_rounded));
            await tester.pumpAndSettle();
            final menu = find.ancestor(
              of: find.text('上下文占用'),
              matching: find.byType(SingleChildScrollView),
            ).first;
            final menuRect = tester.getRect(menu);
            final chatRect = tester.getRect(find.byKey(chatKey));
            expect(menuRect.left, greaterThanOrEqualTo(chatRect.left));
            expect(menuRect.right, lessThanOrEqualTo(chatRect.right));
            expect(menuRect.width, closeTo(300, 0.1));
            expect(find.text('添加设备'), findsNothing);
            final addDevice = find.byTooltip('添加设备');
            expect(addDevice, findsOneWidget);
            expect(
              find.descendant(
                of: addDevice,
                matching: find.byIcon(Icons.add_rounded),
              ),
              findsOneWidget,
            );
            expect(
              tester.getCenter(addDevice).dy,
              closeTo(tester.getCenter(find.text('设备')).dy, 0.1),
            );
            expect(
              tester.getCenter(addDevice).dx,
              direction == TextDirection.ltr
                  ? greaterThan(tester.getCenter(find.text('设备')).dx)
                  : lessThan(tester.getCenter(find.text('设备')).dx),
            );
            if (expanded) {
              expect(menuRect.overlaps(tester.getRect(find.byKey(panelKey))), isFalse);
            }
            expect(tester.takeException(), isNull);
            await tester.tap(find.byIcon(Icons.data_usage_rounded));
            await tester.pumpAndSettle();
            expect(find.text('上下文占用'), findsNothing);
          }
        }
        await tester.pumpWidget(const SizedBox.shrink());
      });
    }
  }
}

class _PendingStatsBridge extends OperitRuntimeBridge {
  final _pending = Completer<Uint8List>();

  /// Leaves statistics pending so geometry does not depend on runtime data.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) => _pending.future;

  /// Rejects pushes unrelated to the context menu.
  @override
  Future<CorePushSink> push(CorePushRequest request) => throw UnimplementedError();

  /// Rejects snapshots unrelated to the context menu.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  /// Supplies no runtime events during geometry tests.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream<CoreEvent>.empty();
}
