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

  test(
    'shows YAML proxy parameters without rewriting code or trailing newlines',
    () {
      const content = 'print("\\d+ <tag>& \\"quoted\\"")  \n\n';
      final payload = 'path: /tmp/demo.py\ncontent: |+\n  $content';
      for (final toolName in ['package_proxy', 'proxy']) {
        final display = normalizeToolDisplayForStrictProxy(
          toolName,
          '<param name="tool_name">files:write</param>'
          '<param name="params">${escapeXmlText(payload)}</param>',
        );
        expect(display.toolName, 'files:write');
        final parameters = extractToolParamMap(display.params);
        expect(parameters['path'], '/tmp/demo.py');
        expect(parameters['content'], content);
      }
    },
  );

  test('decodes nested YAML types and keeps malformed proxies readable', () {
    final parameters = decodeProxyParameters(
      'enabled: true\nlimit: 3\nitems: [one, two]\n'
      'options: {mode: fast}\noptional: null\ntext: "0012"\n',
    );
    expect(parameters, {
      'enabled': true,
      'limit': 3,
      'items': ['one', 'two'],
      'options': {'mode': 'fast'},
      'optional': null,
      'text': '0012',
    });
    for (final payload in ['[1, 2]', 'content: [unfinished', 'just text']) {
      final markup =
          '<param name="tool_name">files:write</param>'
          '<param name="params">$payload</param>';
      final display = normalizeToolDisplayForStrictProxy(
        'package_proxy',
        markup,
      );
      expect(display.toolName, 'package_proxy');
      expect(display.params, markup);
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
