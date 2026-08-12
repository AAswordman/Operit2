import 'package:flutter_test/flutter_test.dart';
import 'package:xterm/src/utils/debugger.dart';
import 'package:xterm/xterm.dart';

/// Registers regression tests for VT220 character sets and keypad modes.
void main() {
  test('designates G2 and G3 character sets', () {
    final debugger = TerminalDebugger();

    debugger.write('\x1b*0\x1b+B');

    expect(
      debugger.commands
          .expand((command) => command.explanation)
          .toList(growable: false),
      equals(['designateCharset(2, 48)', 'designateCharset(3, 66)']),
    );
  });

  test('switches numpad output with DECKPAM and DECKPNM', () {
    final output = <String>[];
    final terminal = Terminal(onOutput: output.add);

    terminal.write('\x1b=');

    expect(terminal.appKeypadMode, isTrue);
    expect(terminal.keyInput(TerminalKey.numpad1), isTrue);
    expect(output, equals(['\x1bOq']));

    terminal.write('\x1b>');

    expect(terminal.appKeypadMode, isFalse);
    expect(terminal.keyInput(TerminalKey.numpad1), isFalse);
    expect(output, equals(['\x1bOq']));
  });
}
