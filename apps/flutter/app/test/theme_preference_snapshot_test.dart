import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';

/// Verifies that a theme snapshot remains identical after window transport.
void main() {
  test(
    'theme preference snapshot survives JSON transport',
    _verifyThemePreferenceSnapshotJsonTransport,
  );
}

/// Verifies that JSON encoding retains every theme preference field.
void _verifyThemePreferenceSnapshotJsonTransport() {
  const snapshot = UserPreferencesManager.defaultThemePreferenceSnapshot;

  final decoded = ThemePreferenceSnapshot.fromJson(snapshot.toJson());

  expect(decoded.toJson(), snapshot.toJson());
}
