import 'dart:convert';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

/// Verifies the established XML tool display normalization.
void main() {
  test('shows the concrete package tool carried by package_proxy', () {
    final display = normalizeToolDisplayForStrictProxy(
      'package_proxy',
      '<param name="tool_name">browser:snapshot</param>'
          '<param name="params">{"include_screenshot":true,"limit":20}</param>',
    );

    expect(display.toolName, 'browser:snapshot');
    expect(
      display.params,
      '<param name="include_screenshot">true</param>\n'
      '<param name="limit">20</param>',
    );
  });

  test('JSON proxy parameters preserve scripts and nested types', () {
    const content =
        'python3 - <<\'PY\'\nprint("&lt;tag&gt; \\"quoted\\"")  \nPY\n\n';
    final expected = <String, Object?>{
      'command': content,
      'enabled': true,
      'nested': {
        'items': [1, null, '0012'],
      },
    };
    for (final name in ['package_proxy', 'proxy']) {
      final display = normalizeToolDisplayForStrictProxy(
        name,
        '<param name="tool_name">files:write</param>'
        '<param name="params">${escapeXmlText(jsonEncode(expected))}</param>',
      );
      expect(display.toolName, 'files:write');
      final parameters = extractToolParamMap(display.params);
      expect(parameters['command'], content);
      expect(parameters['enabled'], 'true');
      expect(jsonDecode(parameters['nested']!), expected['nested']);
    }
  });

  test('keeps YAML and malformed proxy envelopes readable as submitted', () {
    for (final payload in ['command: echo hello', '[1, 2]', '{"command":']) {
      for (final name in ['package_proxy', 'proxy']) {
        final markup =
            '<param name="tool_name">files:write</param>'
            '<param name="params">$payload</param>';
        final display = normalizeToolDisplayForStrictProxy(name, markup);
        expect(display.toolName, name);
        expect(display.params, markup);
      }
    }
  });

  test('keeps direct tool names and XML parameters', () {
    final display = normalizeToolDisplayForStrictProxy(
      'read_file',
      '<param name="path">README.md</param>',
    );

    expect(display.toolName, 'read_file');
    expect(display.params, '<param name="path">README.md</param>');
  });
}
