import 'dart:convert';
import 'dart:io';
import 'dart:ui' show PointerDeviceKind;

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

import 'change_review_workspace_parity_test.dart' show captureWorkspace;

/// Walks serialized plugin nodes, retaining their original gesture action addresses.
Iterable<Map> nodes(Map node) sync* {
  yield node;
  for (final child in node['children'] as List? ?? const []) {
    yield* nodes(child as Map);
  }
  for (final slot in (node['slots'] as Map? ?? const {}).values) {
    for (final child in slot as List) {
      yield* nodes(child as Map);
    }
  }
}

/// Verifies actual plugin layouts and pointer hit testing, not a rebuilt native sidebar.
void main() {
  final fixtures =
      jsonDecode(
            File(
              'test/fixtures/change_review_workspace.json',
            ).readAsStringSync(),
          )
          as Map;
  for (final brightness in Brightness.values) {
    for (final variant in ['directory-tree', 'resized-tree']) {
      testWidgets('change tree layout: ${brightness.name}/$variant', (
        tester,
      ) async {
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final tree = Map<String, Object?>.from(
          fixtures['${brightness.name}-$variant'] as Map,
        );
        final actions = <(String, Object?)>[];
        await captureWorkspace(
          tester,
          buildComposeDslLayoutForTest(
            node: tree,
            hostContext: ComposeDslWebViewHostContext(
              packageName: 'com.operit.change_review',
              routeInstanceId: 'tree-layout',
              executionContextKey: 'tree-layout',
              dispatchAction: (id, [payload]) async {
                actions.add((id, payload));
                return null;
              },
              runtimeOptionsProvider: () => {},
            ),
            splitMarkdownContent: (_) async => [],
            onTextInput: (_, _) async => null,
          ),
          brightness,
          '${brightness.name}-$variant',
        );
        expect(find.text('src'), findsOneWidget);
        expect(find.text('chat'), findsOneWidget);
        expect(find.text('workspace'), findsOneWidget);
        expect(find.text('ChatInput.dart'), findsOneWidget);
        expect(tester.takeException(), isNull);
        final horizontal = find
            .ancestor(
              of: find.text('ChatInput.dart'),
              matching: find.byType(Scrollable),
            )
            .evaluate()
            .map(
              (element) =>
                  tester.state<ScrollableState>(find.byWidget(element.widget)),
            )
            .singleWhere((state) => state.position.axis == Axis.horizontal);
        expect(horizontal.position.maxScrollExtent, greaterThan(0));
        await tester.dragFrom(const Offset(90, 170), const Offset(-70, 0));
        await tester.pumpAndSettle();
        expect(horizontal.position.pixels, greaterThan(0));
        for (final kind in [PointerDeviceKind.touch, PointerDeviceKind.mouse]) {
          for (final key in ['sidebar-splitter', 'diff-splitter']) {
            final handle = nodes(
              tree,
            ).singleWhere((node) => (node['props'] as Map?)?['key'] == key);
            final props = handle['props'] as Map;
            final ops = (props['modifier'] as Map)['__modifierOps'] as List;
            final gesture =
                (ops.singleWhere((op) => op['name'] == 'dragGestures')['args']
                            as List)
                        .single
                    as Map;
            final offset =
                (ops.singleWhere((op) => op['name'] == 'offset')['args']
                            as List)
                        .first
                    as num;
            final sidebarWidth = variant == 'directory-tree' ? 220.0 : 150.0;
            final rootX = key == 'diff-splitter' ? sidebarWidth + 1 : 0.0;
            actions.clear();
            // Four pixels away from the visible 1px rule must still be draggable.
            final pointer = await tester.startGesture(
              Offset(rootX + offset + 10, 300),
              kind: kind,
            );
            await pointer.moveBy(const Offset(30, 0));
            await tester.pump();
            await pointer.moveBy(const Offset(20, 0));
            await tester.pump();
            await pointer.up();
            await tester.pumpAndSettle();
            expect(
              actions.any(
                (entry) => entry.$1 == (gesture['onDrag'] as Map)['__actionId'],
              ),
              isTrue,
              reason: '$key accepts $kind near the rule',
            );
            expect(tester.takeException(), isNull);
          }
        }
      });
    }
  }
}
