import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/StorageDirectorySelectionError.dart';
import 'package:operit2/ui/features/onboarding/OnboardingStartupRoute.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final originalFolderAccess = OperitFolderAccessPlatform.instance;
  const runtimeChannel = MethodChannel('operit/runtime');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;

  setUp(() {
    messenger.setMockMethodCallHandler(runtimeChannel, (call) async {
      if (call.method == 'localRuntimeStorageDefaults') {
        return <String, String>{
          'runtimeRoot': '/runtime',
          'workspaceRoot': '/workspaces',
        };
      }
      throw StateError('Unexpected host method: ${call.method}');
    });
  });

  tearDown(() {
    OperitFolderAccessPlatform.instance = originalFolderAccess;
    messenger.setMockMethodCallHandler(runtimeChannel, null);
  });

  for (final locale in const <Locale>[Locale('en'), Locale('zh')]) {
    test('directory errors are explained in ${locale.languageCode}', () async {
      final l10n = await AppLocalizations.delegate.load(locale);
      expect(
        storageDirectorySelectionErrorMessage(l10n, _termuxError()),
        l10n.storageDirectoryTermuxUnsupported,
      );
      expect(
        storageDirectorySelectionErrorMessage(
          l10n,
          PlatformException(
            code: 'UnsupportedOperationException',
            message: 'Unsupported storage volume ABCD-1234',
          ),
        ),
        l10n.storageDirectoryProviderUnsupported,
      );
      expect(
        storageDirectorySelectionErrorMessage(
          l10n,
          PlatformException(
            code: 'permission_denied',
            message: 'Access denied',
          ),
        ),
        l10n.storageDirectorySelectionFailed('Access denied'),
      );
      expect(
        storageDirectorySelectionErrorMessage(
          l10n,
          PlatformException(code: 'picker_failed'),
        ),
        l10n.storageDirectorySelectionFailed('picker_failed'),
      );
      expect(
        storageDirectorySelectionErrorMessage(
          l10n,
          StateError('Picker failed'),
        ),
        l10n.storageDirectorySelectionFailed('Bad state: Picker failed'),
      );
    });
  }

  for (final label in <String>['运行时目录', '工作区目录']) {
    testWidgets('$label handles Termux errors and permits a successful retry', (
      tester,
    ) async {
      final picker = _FolderAccess(() async => throw _termuxError());
      OperitFolderAccessPlatform.instance = picker;
      final l10n = await _pumpStoragePage(tester);
      final field = _pathField(label);
      final originalPath = tester.widget<TextField>(field).controller!.text;

      await tester.tap(find.byTooltip('选择$label'));
      await tester.pumpAndSettle();

      expect(find.text(l10n.storageDirectoryTermuxUnsupported), findsOneWidget);
      expect(find.text('模型配置失败'), findsNothing);
      expect(tester.widget<TextField>(field).controller!.text, originalPath);
      expect(tester.takeException(), isNull);

      picker.select = () async => '  /storage/emulated/0/Documents/Operit  ';
      await tester.tap(find.byTooltip('选择$label'));
      await tester.pumpAndSettle();

      expect(
        tester.widget<TextField>(field).controller!.text,
        '/storage/emulated/0/Documents/Operit',
      );
      expect(find.text(l10n.storageDirectoryTermuxUnsupported), findsNothing);
      final otherLabel = label == '运行时目录' ? '工作区目录' : '运行时目录';
      expect(
        tester.widget<TextField>(_pathField(otherLabel)).controller!.text,
        label == '运行时目录' ? '/workspaces' : '/runtime',
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('cancelled or empty selections leave the existing path intact', (
    tester,
  ) async {
    final picker = _FolderAccess(() async => null);
    OperitFolderAccessPlatform.instance = picker;
    await _pumpStoragePage(tester);
    for (final selection in <String?>[null, '', '   ']) {
      picker.select = () async => selection;
      await tester.tap(find.byTooltip('选择工作区目录'));
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(_pathField('工作区目录')).controller!.text,
        '/workspaces',
      );
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets('a new identity skips path selection when roots are mounted', (
    tester,
  ) async {
    messenger.setMockMethodCallHandler(runtimeChannel, (call) async {
      switch (call.method) {
        case 'runtimeBootstrapRead':
          return jsonEncode(<String, Object?>{'ok': true, 'value': null});
        case 'runtimeBootstrapWrite':
          return jsonEncode(<String, Object?>{'ok': true});
        case 'localRuntimeStoragePaths':
          return call.arguments;
        case 'setLocalRuntimeStorage':
          return null;
        case 'connectCoreFfi':
          throw PlatformException(code: 'TEST_CONNECTION_BOUNDARY');
        default:
          throw StateError('Unexpected host method: ${call.method}');
      }
    });
    addTearDown(() async {
      await RuntimeBootstrapManager.instance.initialize();
    });

    await ClientLogger.initialize();
    final manager = RuntimeBootstrapManager.instance;
    await manager.initialize();
    final decision = await const OnboardingStartupRouteStrategy().resolve();
    await manager.confirmLocalRuntimeStorage('/runtime', '/workspaces');
    await tester.pumpWidget(
      MaterialApp(
        locale: const Locale('zh'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(builder: (context) => decision!.builder(context, () {})),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('开始'));
    await tester.pumpAndSettle();

    expect(find.text('2/3'), findsOneWidget);
    expect(find.text('确认本地存储位置'), findsNothing);
    expect(find.text('用户协议与隐私政策'), findsOneWidget);
  });

  testWidgets('other picker failures are shown rather than escaping the zone', (
    tester,
  ) async {
    OperitFolderAccessPlatform.instance = _FolderAccess(
      () async => throw PlatformException(
        code: 'permission_denied',
        message: 'Access denied',
      ),
    );
    final l10n = await _pumpStoragePage(tester);
    await tester.tap(find.byTooltip('选择工作区目录'));
    await tester.pumpAndSettle();
    expect(
      find.text(l10n.storageDirectorySelectionFailed('Access denied')),
      findsOneWidget,
    );
    expect(tester.takeException(), isNull);
  });

  for (final fail in <bool>[false, true]) {
    testWidgets('picker completion after disposal is ignored (fail=$fail)', (
      tester,
    ) async {
      final pending = Completer<String?>();
      OperitFolderAccessPlatform.instance = _FolderAccess(() => pending.future);
      await _pumpStoragePage(tester);
      await tester.tap(find.byTooltip('选择工作区目录'));
      await tester.pumpWidget(const SizedBox.shrink());
      if (fail) {
        pending.completeError(_termuxError());
      } else {
        pending.complete('/selected');
      }
      await tester.pump();
      expect(tester.takeException(), isNull);
    });
  }
}

PlatformException _termuxError() => PlatformException(
  code: 'UnsupportedOperationException',
  message:
      'java.lang.UnsupportedOperationException: Retrieving the path from URIs '
      'with authority com.termux.documents is unsupported by this plugin.',
);

Finder _pathField(String label) => find.byWidgetPredicate(
  (widget) => widget is TextField && widget.decoration?.labelText == label,
);

Future<AppLocalizations> _pumpStoragePage(WidgetTester tester) async {
  final decision = await const OnboardingStartupRouteStrategy().resolve();
  await tester.pumpWidget(
    MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Builder(builder: (context) => decision!.builder(context, () {})),
    ),
  );
  await tester.pumpAndSettle();
  await tester.tap(find.text('开始'));
  await tester.pumpAndSettle();
  // The agreement countdown starts once its page is visible.
  await tester.pump(const Duration(seconds: 6));
  await tester.pumpAndSettle();
  await tester.tap(find.text('同意'));
  await tester.pumpAndSettle();
  expect(find.byTooltip('选择工作区目录'), findsOneWidget);
  return AppLocalizations.of(tester.element(find.byTooltip('选择工作区目录')))!;
}

class _FolderAccess extends OperitFolderAccessPlatform {
  _FolderAccess(this.select);

  Future<String?> Function() select;

  @override
  Future<String?> pickDirectory({String? initialDirectory}) => select();

  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) =>
      throw UnimplementedError();
}
