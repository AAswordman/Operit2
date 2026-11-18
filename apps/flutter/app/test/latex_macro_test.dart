import 'package:flutter_math_fork/flutter_math.dart';
import 'package:flutter_math_fork/src/encoder/tex/encoder.dart';
import 'package:flutter_test/flutter_test.dart';

void main() {
  test(r'\gdef survives nested groups and expands in the same formula', () {
    final formula = Math.tex(r'{\newcommand{\unit}{a}{\gdef\unit{b}}}\unit');

    expect(formula.parseError, isNull);
    expect(
      formula.ast!.greenRoot.encodeTeX(),
      Math.tex(r'{{}}b').ast!.greenRoot.encodeTeX(),
    );
    expect(Math.tex(r'\unit').parseError, isNotNull);
  });

  test(r'\gdef substitutes undelimited and delimited arguments', () {
    final cases = {
      r'\gdef\ratio#1#2{\frac{#1}{#2}}\ratio{a+b}c': r'\frac{a+b}{c}',
      r'\gdef\ratio[#1]/#2;{\frac{#1}{#2}}\ratio[{a]/b}]/c;': r'\frac{a]/b}{c}',
      r'\gdef\before#1#{(#1)}\before a+b{c}': r'(a+b){c}',
      r'\gdef\define{\gdef\double##1{##1+##1}}\define\double{x}': r'x+x',
      r'\gdef\digit{1}\expandafter\digit\gdef\digit{2}\digit': r'12',
    };
    for (final entry in cases.entries) {
      final formula = Math.tex(entry.key);
      expect(formula.parseError, isNull, reason: entry.key);
      expect(
        formula.ast!.greenRoot.encodeTeX(),
        Math.tex(entry.value).ast!.greenRoot.encodeTeX(),
        reason: entry.key,
      );
    }
  });

  test(r'malformed \gdef reports a parse error', () {
    final cases = {
      r'\gdef x{a}': 'Expected a control sequence',
      r'\gdef\unit#2{a}': 'Expected consecutive macro parameters',
      r'\gdef\unit#1{#2}': 'Not a valid argument number',
      r'\gdef\unit{a': 'End of input in macro argument',
    };
    for (final entry in cases.entries) {
      expect(Math.tex(entry.key).parseError?.message, contains(entry.value));
    }
  });
}
