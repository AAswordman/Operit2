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

  test('keeps direct tool names and XML parameters', () {
    final display = normalizeToolDisplayForStrictProxy(
      'read_file',
      '<param name="path">README.md</param>',
    );

    expect(display.toolName, 'read_file');
    expect(display.params, '<param name="path">README.md</param>');
  });
}
