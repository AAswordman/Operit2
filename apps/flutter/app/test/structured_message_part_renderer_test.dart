import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/FileDiffDisplay.dart';
import 'package:operit2/ui/features/chat/components/part/StructuredMessagePartRenderer.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

/// Verifies semantic message parts keep the established XML render behavior.
void main() {
  testWidgets('renders semantic thinking with the shared thinking panel', (
    tester,
  ) async {
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Scaffold(
          body: StructuredMessagePartRenderer(
            parts: const <MessagePart>[
              MessagePart(
                partId: 'thinking',
                sequence: 0,
                kind: MessagePartKind.thinking,
                content: 'reasoning',
                toolCallId: null,
                toolName: null,
                attributes: <String, String>{},
              ),
            ],
            textColor: Colors.black,
            backgroundColor: Colors.white,
            showThinkingProcess: true,
          ),
        ),
      ),
    );

    expect(find.byType(CustomXmlRenderer), findsOneWidget);
    expect(find.byType(ExpansionTile), findsNothing);
  });

  testWidgets('renders completion status with the established status card', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'status',
          sequence: 0,
          kind: MessagePartKind.status,
          content: 'ignored completion body',
          toolCallId: null,
          toolName: null,
          attributes: <String, String>{'type': 'completion'},
        ),
      ]),
    );

    expect(find.text('✓ Task completed'), findsOneWidget);
  });

  testWidgets('uses compact rendering for completed structured tool calls', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'tool-call',
          sequence: 0,
          kind: MessagePartKind.toolCall,
          content: '',
          toolCallId: 'call-1',
          toolName: 'read_file',
          attributes: <String, String>{'path': 'README.md'},
        ),
      ]),
    );

    expect(find.byType(CompactToolDisplay), findsOneWidget);
    expect(find.byType(DetailedToolDisplay), findsNothing);
  });

  testWidgets('preserves structured tool-result error and file-diff rendering', (
    tester,
  ) async {
    await tester.pumpWidget(
      _messagePartApp(const <MessagePart>[
        MessagePart(
          partId: 'tool-result-error',
          sequence: 0,
          kind: MessagePartKind.toolResult,
          content: '<error>permission denied</error>',
          toolCallId: 'call-1',
          toolName: 'read_file',
          attributes: <String, String>{'status': 'FAILED'},
        ),
        MessagePart(
          partId: 'tool-result-diff',
          sequence: 1,
          kind: MessagePartKind.toolResult,
          content:
              '<file-diff path="lib/main.dart" details="updated">+line</file-diff>',
          toolCallId: 'call-2',
          toolName: 'apply_file',
          attributes: <String, String>{'status': 'SUCCESS'},
        ),
      ]),
    );

    expect(find.text('permission denied'), findsOneWidget);
    expect(find.byType(FileDiffDisplay), findsOneWidget);
  });
}

/// Wraps structured parts in the localized Material test surface.
Widget _messagePartApp(List<MessagePart> parts) {
  return MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: StructuredMessagePartRenderer(
        parts: parts,
        textColor: Colors.black,
        backgroundColor: Colors.white,
        showThinkingProcess: true,
      ),
    ),
  );
}
