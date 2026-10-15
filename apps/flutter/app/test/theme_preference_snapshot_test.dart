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
}

/// Verifies that JSON encoding retains every theme preference field.
void _verifyThemePreferenceSnapshotJsonTransport() {
  const snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;

  final decoded = ThemePreferenceSnapshot.fromJson(snapshot.toJson());

  expect(decoded.toJson(), snapshot.toJson());
}
