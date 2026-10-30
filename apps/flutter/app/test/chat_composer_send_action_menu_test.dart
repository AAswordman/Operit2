import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatComposerSendActionMenu.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/ChatPendingMessageQueuePanel.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/PendingQueueMessageItem.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  testWidgets('busy draft menu exposes all three explicit delivery choices', (
    tester,
  ) async {
    final actions = <ChatComposerSendAction>[];
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('en'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: Center(
            child: ChatComposerSendActionMenu(onAction: actions.add),
          ),
        ),
      ),
    );

    for (final action in ChatComposerSendAction.values) {
      await tester.tap(find.byKey(const ValueKey('chat.send-options')));
      await tester.pumpAndSettle();
      expect(find.text('Queue message'), findsOneWidget);
      expect(find.text('Guide current conversation'), findsOneWidget);
      expect(find.text('Interrupt and send'), findsOneWidget);
      await tester.tap(
        find.byKey(ValueKey('chat.send-option.${_actionKey(action)}')),
      );
      await tester.pumpAndSettle();
      expect(actions.single, action);
      actions.clear();
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('queued message exposes guide, interrupt, and withdraw actions', (
    tester,
  ) async {
    final actions = <String>[];
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: ChatPendingMessageQueuePanel(
            queuedMessages: const <PendingQueueMessageItem>[
              PendingQueueMessageItem(id: 7, text: 'Adjust the current plan'),
            ],
            expanded: true,
            canGuide: true,
            onExpandedChange: (_) {},
            onDeleteMessage: (id) => actions.add('delete:$id'),
            onEditMessage: (id) => actions.add('edit:$id'),
            onGuideMessage: (id) => actions.add('guide:$id'),
            onSendMessage: (id) => actions.add('send:$id'),
          ),
        ),
      ),
    );

    await tester.tap(find.byIcon(Icons.edit));
    await tester.tap(find.byIcon(Icons.alt_route));
    await tester.tap(find.byIcon(Icons.send));
    await tester.tap(find.byIcon(Icons.delete));
    expect(actions, <String>['edit:7', 'guide:7', 'send:7', 'delete:7']);
    expect(tester.takeException(), isNull);
  });
}

String _actionKey(ChatComposerSendAction action) {
  return switch (action) {
    ChatComposerSendAction.queue => 'queue',
    ChatComposerSendAction.guide => 'guide',
    ChatComposerSendAction.sendNow => 'send-now',
  };
}
