import 'package:flutter_test/flutter_test.dart';
import 'package:xterm/xterm.dart';

/// Verifies reflow when a wide glyph fills a reused destination line.
void main() {
  test('growing a wrapped line by one column preserves its text', () {
    final terminal = Terminal();
    terminal.resize(4, 4);
    terminal.write('abc中def');

    terminal.resize(5, 4);

    expect(
      [
        for (var i = 0; i < terminal.buffer.lines.length; i++)
          terminal.buffer.lines[i].getText(),
      ].join(),
      'abc中def',
    );
    expect(terminal.buffer.lines[0].getText(), 'abc中');
    expect(terminal.buffer.lines[1].isWrapped, isTrue);
  });
}
