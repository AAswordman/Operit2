import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/icons/MaterialIconNameResolver.dart';

/// Verifies SDK icon identity, naming conventions, and invalid-name handling.
void main() {
  test('plugin and Flutter names retain the SDK icon and RTL metadata', () {
    for (final name in ['ChevronLeft', 'chevronLeft', 'chevron_left']) {
      expect(MaterialIconNameResolver.resolve(name), Icons.chevron_left);
      expect(MaterialIconNameResolver.resolve(name).matchTextDirection, isTrue);
    }
    expect(
      MaterialIconNameResolver.resolve('ChevronRight'),
      Icons.chevron_right,
    );
    expect(MaterialIconNameResolver.resolve('Assignment'), Icons.assignment);
    expect(MaterialIconNameResolver.resolve('Quiz'), Icons.quiz);
    expect(MaterialIconNameResolver.resolve('QrCode2'), Icons.qr_code_2);
    expect(MaterialIconNameResolver.resolve('Brightness4'), Icons.brightness_4);
    expect(MaterialIconNameResolver.resolve('Class'), Icons.class_);
    expect(MaterialIconNameResolver.resolve('more'), Icons.more);
    expect(MaterialIconNameResolver.resolve('forward'), Icons.forward);
  });

  test('distinct SDK names are not merged or restyled', () {
    expect(MaterialIconNameResolver.resolve('AddChart'), Icons.add_chart);
    expect(MaterialIconNameResolver.resolve('Addchart'), Icons.addchart);
    expect(MaterialIconNameResolver.resolve('delete'), Icons.delete);
    expect(
      MaterialIconNameResolver.resolve('DeleteOutline'),
      Icons.delete_outline,
    );
  });

  test('private aliases and unknown icons are rejected', () {
    for (final name in [
      'plus',
      'account',
      'file',
      'play',
      'back',
      'unknown_icon',
    ]) {
      expect(() => MaterialIconNameResolver.resolve(name), throwsArgumentError);
    }
  });
}
