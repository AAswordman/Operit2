import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

/// Verifies structured tool-part display normalization.
void main() {
  test('shows the concrete package tool carried by package_proxy', () {
    final display =
        normalizeStructuredToolDisplay('package_proxy', <String, String>{
          'tool_name': 'browser:snapshot',
          'params': '{"include_screenshot":true,"limit":20}',
        });

    expect(display.toolName, 'browser:snapshot');
    expect(jsonDecode(display.params), <String, Object>{
      'include_screenshot': true,
      'limit': 20,
    });
  });

  test('keeps direct structured tool names and parameters', () {
    final display = normalizeStructuredToolDisplay(
      'read_file',
      <String, String>{'path': 'README.md'},
    );

    expect(display.toolName, 'read_file');
    expect(jsonDecode(display.params), <String, Object>{'path': 'README.md'});
  });
}
