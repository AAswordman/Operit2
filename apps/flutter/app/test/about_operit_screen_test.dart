import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/about/AboutOperitScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:package_info_plus/package_info_plus.dart';

/// Exercises installed package metadata, About page details, and licenses.
void main() {
  testWidgets('about page displays project details and licenses', (
    tester,
  ) async {
    _setPackageMetadata(version: '2.0.0', buildNumber: '15');
    await _pumpAbout(tester);
    await tester.pumpAndSettle();

    expect(find.text('Operit2'), findsWidgets);
    expect(find.text('版本 2.0.0+15'), findsOneWidget);
    expect(find.text('项目源码'), findsOneWidget);

    await tester.ensureVisible(find.text('开源许可证'));
    await tester.tap(find.text('开源许可证'));
    await tester.pumpAndSettle();

    expect(find.text('flutter_math_fork'), findsOneWidget);
    expect(find.text('AGPL-3.0'), findsWidgets);
  });

  testWidgets('about page preserves the exact installed build number', (
    tester,
  ) async {
    _setPackageMetadata(version: '2.0.0', buildNumber: '2015');
    await _pumpAbout(tester);
    await tester.pumpAndSettle();

    expect(find.text('版本 2.0.0+2015'), findsOneWidget);
    expect(find.text('版本 2.0.0+15'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  for (final metadata in <({String version, String buildNumber})>[
    (version: '', buildNumber: '15'),
    (version: '2.0.0', buildNumber: ''),
    (version: '2.0.0', buildNumber: '   '),
  ]) {
    testWidgets('about page rejects incomplete package metadata: $metadata', (
      tester,
    ) async {
      _setPackageMetadata(
        version: metadata.version,
        buildNumber: metadata.buildNumber,
      );
      await _pumpAbout(tester);
      await tester.pumpAndSettle();

      expect(
        find.text(
          '版本信息读取失败：Bad state: '
          'Application package version or build number is empty.',
        ),
        findsOneWidget,
      );
      expect(find.text('版本 2.0.0+6'), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
}

/// Supplies installed package metadata without accessing a native test host.
void _setPackageMetadata({
  required String version,
  required String buildNumber,
}) {
  PackageInfo.setMockInitialValues(
    appName: 'Operit2',
    packageName: 'app.operit',
    version: version,
    buildNumber: buildNumber,
    buildSignature: '',
  );
}

/// Renders the About page inside the application theme scope.
Future<void> _pumpAbout(WidgetTester tester) {
  return tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: const Scaffold(body: AboutOperitScreen()),
    ),
  );
}
