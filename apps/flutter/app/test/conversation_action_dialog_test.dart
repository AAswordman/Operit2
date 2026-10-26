import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/main/components/DrawerContentDialogs.dart';

/// Verifies the chat action menu stays inside a short desktop window.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  testWidgets('scrolls the full action menu instead of overflowing', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 600));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    final selected = <ConversationAction?>[];

    await _pumpDialog(tester, selected);
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(find.text('聊天记录'), findsOneWidget);
    expect(find.text('在新窗口打开'), findsOneWidget);
    expect(find.text('解除绑定'), findsOneWidget);
    expect(find.text('删除'), findsOneWidget);

    final dialogHeight = tester.getSize(_dialogSurface()).height;
    expect(dialogHeight, lessThanOrEqualTo(552));
    final cancel = tester.getRect(find.widgetWithText(TextButton, '取消'));
    expect(cancel.top, greaterThanOrEqualTo(0));
    expect(cancel.bottom, lessThanOrEqualTo(600));

    await tester.ensureVisible(find.text('删除'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('删除'));
    await tester.pumpAndSettle();
    expect(selected, <ConversationAction?>[ConversationAction.delete]);
  });

  testWidgets('keeps a short action menu wrapped on a tall window', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(800, 1200));
    addTearDown(() => tester.binding.setSurfaceSize(null));

    await _pumpDialog(tester, <ConversationAction?>[]);
    await tester.tap(find.text('open'));
    await tester.pumpAndSettle();

    expect(tester.getSize(_dialogSurface()).height, lessThan(800));
    expect(
      tester.getRect(find.widgetWithText(TextButton, '取消')).bottom,
      lessThan(1200),
    );
  });
}

Finder _dialogSurface() =>
    find.ancestor(of: find.text('聊天记录'), matching: find.byType(Material)).first;

Future<void> _pumpDialog(
  WidgetTester tester,
  List<ConversationAction?> selected,
) {
  return tester.pumpWidget(
    MaterialApp(
      locale: const Locale('zh'),
      supportedLocales: AppLocalizations.supportedLocales,
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      home: Builder(
        builder: (context) {
          return Scaffold(
            body: TextButton(
              onPressed: () async {
                selected.add(
                  await showDialog<ConversationAction>(
                    context: context,
                    builder: (context) => ConversationActionDialog(
                      history: const core.ChatHistoryListItem(
                        parentChatId: null,
                        id: 'chat-1',
                        title: '长对话标题',
                        group: null,
                        updatedAt: '2026-10-19T00:00:00Z',
                        displayOrder: 0,
                        workspaceId: 'workspace-1',
                        workspaceName: 'Workspace',
                        locked: false,
                        pinned: false,
                      ),
                      canOpenInWindow: true,
                      canMoveUp: true,
                      canMoveDown: true,
                    ),
                  ),
                );
              },
              child: const Text('open'),
            ),
          );
        },
      ),
    ),
  );
}
