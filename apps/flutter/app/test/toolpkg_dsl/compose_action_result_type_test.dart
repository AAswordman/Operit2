import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Verifies callback values survive both typed action events without re-decoding.
void main() {
  final cases = <String, Object?>{
    'JSON success string': '{"success":true}',
    'JSON failure string': '{"success":false,"message":"write failed"}',
    'JSON config string': '{"character_card_name":"Ash","chat_query":"巡检"}',
    'JSON array string': '[{"id":1}]',
    'JSON scalar strings': 'false',
    'JSON number string': '75',
    'JSON null string': 'null',
    'JSON quoted string': '"hello"',
    'empty string': '',
    'whitespace string': '  \n ',
    'formatted JSON string': ' \n {"success":true}\n ',
    'plain text': 'hello 🌸',
    'object': <String, Object?>{'success': true, 'message': '保存成功'},
    'array': <Object?>[
      1,
      'two',
      <String, Object?>{'id': 3},
    ],
    'boolean': false,
    'number': 75,
    'null': null,
  };

  for (final entry in cases.entries) {
    for (final phase in ['intermediate', 'final']) {
      test('$phase preserves ${entry.key} in the typed session event', () {
        final parsed = parseComposeDslActionEventForTest(
          _event(entry.value, phase: phase, includeTree: true),
        );
        expect(parsed.hasRenderResult, isTrue);
        expect(parsed.actionResult, equals(entry.value));
        expect(parsed.renderedActionResult, equals(entry.value));
      });
    }

    test('tree-less action preserves ${entry.key}', () {
      final parsed = parseComposeDslActionEventForTest(
        _event(entry.value, includeTree: false),
      );
      expect(parsed.hasRenderResult, isFalse);
      expect(parsed.actionResult, equals(entry.value));
    });
  }

  test('Gentle Guardian can parse its JSON response exactly once', () {
    for (final success in [true, false]) {
      final response = jsonEncode({
        'success': success,
        if (!success) 'message': 'write failed',
      });
      final parsed = parseComposeDslActionEventForTest(_event(response));
      // Mirrors the host envelope that resolves Guardian.saveConfig in the page.
      final envelope =
          jsonDecode(jsonEncode({'success': true, 'data': parsed.actionResult}))
              as Map<String, Object?>;
      expect(envelope['data'], isA<String>());
      final pageResult = jsonDecode(envelope['data'] as String);
      expect(pageResult['success'], success);
      if (!success) expect(pageResult['message'], 'write failed');
    }
  });

  test('outer action errors are still propagated', () {
    expect(
      () => parseComposeDslActionEventForTest(
        core_proxy.ToolPkgComposeDslEvent(
          requestId: 'test',
          phase: 'error',
          update: null,
          actionResult: null,
          navigationCommands: const [],
          error: 'action failed',
        ),
      ),
      throwsA(predicate((error) => error.toString().contains('action failed'))),
    );
  });
}

/// Supplies a structured action envelope without a UI-tree text codec.
core_proxy.ToolPkgComposeDslEvent _event(
  Object? actionResult, {
  String phase = 'final',
  bool includeTree = true,
}) => core_proxy.ToolPkgComposeDslEvent(
  requestId: 'test',
  phase: phase,
  actionResult: actionResult,
  error: null,
  navigationCommands: const [],
  update: includeTree
      ? core_proxy.ToolPkgComposeDslNodeUpdate(
          reset: true,
          revision: 1,
          rootId: 'root',
          removed: const [],
          upserts: [
            core_proxy.ToolPkgComposeDslNodeRecord(
              id: 'root',
              nodeType: 'Text',
              props: const {'text': 'Guardian'},
              children: const [],
              slots: const {},
            ),
          ],
        )
      : null,
);
