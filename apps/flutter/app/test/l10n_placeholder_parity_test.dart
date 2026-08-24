import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

List<String> _placeholders(Object? value) {
  return RegExp(r'\{([A-Za-z_][A-Za-z0-9_]*)\}')
      .allMatches(value?.toString() ?? '')
      .map((match) => match.group(1)!)
      .toList()
    ..sort();
}

void main() {
  test('Japanese ARB preserves every English message placeholder', () {
    final english = jsonDecode(
      File('lib/l10n/app_en.arb').readAsStringSync(),
    ) as Map<String, dynamic>;
    final japanese = jsonDecode(
      File('lib/l10n/app_ja.arb').readAsStringSync(),
    ) as Map<String, dynamic>;

    final messageKeys = english.keys.where((key) => !key.startsWith('@'));
    for (final key in messageKeys) {
      expect(japanese, contains(key), reason: 'Japanese ARB is missing $key');
      expect(
        _placeholders(japanese[key]),
        _placeholders(english[key]),
        reason: 'Placeholder mismatch for $key',
      );
    }
  });
}
