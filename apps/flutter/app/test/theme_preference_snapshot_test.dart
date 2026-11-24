import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit_ui/operit_ui.dart' as shared_theme;

/// Verifies that a theme snapshot remains identical after window transport.
void main() {
  test(
    'theme preference snapshot survives JSON transport',
    _verifyThemePreferenceSnapshotJsonTransport,
  );
  test('application reuses the shared snapshot type and defaults', () {
    expect(
      identical(
        UserPreferencesManager.defaultThemePreferenceSnapshot,
        shared_theme.ThemePreferenceDefaults.snapshot,
      ),
      isTrue,
    );
    expect(
      UserPreferencesManager.defaultThemePreferenceSnapshot,
      isA<shared_theme.ThemePreferenceSnapshot>(),
    );
  });
  test(
    'Rust saved theme fixture decodes with the current Flutter contract',
    () {
      final json = Map<String, Object?>.from(
        jsonDecode(
              File(
                '../../../core/crates/runtime/application/src/data/preferences/'
                'theme_snapshot_fixture.json',
              ).readAsStringSync(),
            )
            as Map,
      );
      final snapshot = ThemePreferenceSnapshot.fromJson(json);
      expect(snapshot.chatContentHorizontalPadding, 16);
      expect(
        snapshot.chatContentHorizontalPadding,
        shared_theme
            .ThemePreferenceDefaults
            .snapshot
            .chatContentHorizontalPadding,
      );
      for (final entry in json.entries) {
        expect(snapshot.toJson()[entry.key], entry.value, reason: entry.key);
      }
    },
  );
  test(
    'older themes use the original transcript padding without rewriting JSON',
    () {
      final json =
          UserPreferencesManager.defaultThemePreferenceSnapshot.toJson()
            ..remove('chatContentHorizontalPadding');
      expect(
        ThemePreferenceSnapshot.fromJson(json).chatContentHorizontalPadding,
        16,
      );
      expect(json.containsKey('chatContentHorizontalPadding'), isFalse);
      json['chatContentHorizontalPadding'] = null;
      expect(
        ThemePreferenceSnapshot.fromJson(json).chatContentHorizontalPadding,
        16,
      );
      for (final padding in [0, 24.0, 32]) {
        json['chatContentHorizontalPadding'] = padding;
        expect(
          ThemePreferenceSnapshot.fromJson(json).chatContentHorizontalPadding,
          padding,
        );
      }
    },
  );
}

/// Verifies that JSON encoding retains every theme preference field.
void _verifyThemePreferenceSnapshotJsonTransport() {
  const snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;

  final decoded = ThemePreferenceSnapshot.fromJson(snapshot.toJson());

  expect(decoded.toJson(), snapshot.toJson());
}
