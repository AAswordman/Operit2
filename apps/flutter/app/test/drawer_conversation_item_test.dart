import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/main/components/CollapsedDrawerContent.dart';
import 'package:operit2/ui/main/components/NavigationDrawerAppearance.dart';

/// Verifies drawer conversation rows expose active chat execution state.
void main() {
  testWidgets('shows a running indicator for an active conversation', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ConversationDrawerItem(
            history: const ChatHistoryListItem(
              id: 'chat-1',
              title: 'Running chat',
              updatedAt: '2026-09-02T00:00:00Z',
              group: null,
              displayOrder: 0,
              characterCardName: null,
              characterGroupId: null,
              locked: false,
              pinned: false,
            ),
            title: 'Running chat',
            selected: false,
            isRunning: true,
            appearance: const NavigationDrawerAppearance(
              containerColor: Colors.white,
              titleColor: Colors.black,
              statusAvailableColor: Colors.blue,
              itemColor: Colors.black87,
              buttonContainerColor: Colors.white70,
              selectedContainerColor: Colors.blue,
              selectedContentColor: Colors.white,
              dividerColor: Colors.black12,
              transparentSurfaceEnabled: false,
            ),
            onClick: () {},
            onRename: () {},
            onDelete: () {},
            onLongPress: () {},
            onMoveTo: (_) {},
            canAcceptDrop: (_) => true,
            canDetach: false,
            onDetach: () {},
          ),
        ),
      ),
    );

    expect(find.byType(CircularProgressIndicator), findsOneWidget);
    final indicatorBox = tester.widget<SizedBox>(
      find.byKey(const ValueKey<String>('conversation-running-indicator')),
    );
    expect(indicatorBox.width, 20);
    expect(indicatorBox.height, 20);
  });

  testWidgets('does not show a running indicator for an idle conversation', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ConversationDrawerItem(
            history: const ChatHistoryListItem(
              id: 'chat-2',
              title: 'Idle chat',
              updatedAt: '2026-09-02T00:00:00Z',
              group: null,
              displayOrder: 1,
              characterCardName: null,
              characterGroupId: null,
              locked: false,
              pinned: false,
            ),
            title: 'Idle chat',
            selected: false,
            isRunning: false,
            appearance: const NavigationDrawerAppearance(
              containerColor: Colors.white,
              titleColor: Colors.black,
              statusAvailableColor: Colors.blue,
              itemColor: Colors.black87,
              buttonContainerColor: Colors.white70,
              selectedContainerColor: Colors.blue,
              selectedContentColor: Colors.white,
              dividerColor: Colors.black12,
              transparentSurfaceEnabled: false,
            ),
            onClick: () {},
            onRename: () {},
            onDelete: () {},
            onLongPress: () {},
            onMoveTo: (_) {},
            canAcceptDrop: (_) => true,
            canDetach: false,
            onDetach: () {},
          ),
        ),
      ),
    );

    expect(find.byType(CircularProgressIndicator), findsNothing);
  });
}
