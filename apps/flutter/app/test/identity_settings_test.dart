import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/runtime/RuntimeBootstrapManager.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/profile/UserProfileSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises the real profile panel with isolated sample identities.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final manager = RuntimeBootstrapManager.instance;
  late LocalRuntimeStorageConfig persisted;
  var rejectWrite = false;
  setUp(() async {
    rejectWrite = false;
    const channel = MethodChannel('operit/runtime');
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    persisted = const LocalRuntimeStorageConfig(
      confirmed: false,
      runtimeRoot: '',
      workspaceRoot: '',
      identities: <RuntimeIdentity>[
        RuntimeIdentity(id: 'identity-personal', name: '个人空间', createdAt: 1),
        RuntimeIdentity(id: 'identity-work', name: '工作空间', createdAt: 2),
        RuntimeIdentity(id: 'identity-lab', name: '测试空间', createdAt: 3),
      ],
      activeIdentityId: 'identity-personal',
      updatedAt: 1,
    );
    messenger.setMockMethodCallHandler(channel, (call) async {
      switch (call.method) {
        case 'runtimeBootstrapRead':
          return jsonEncode({
            'ok': true,
            'value': jsonEncode(persisted.toJson()),
          });
        case 'runtimeBootstrapWrite':
          if (rejectWrite) {
            throw PlatformException(code: 'TEST_WRITE_REJECTED');
          }
          persisted = LocalRuntimeStorageConfig.fromJson(
            jsonDecode(call.arguments as String) as Map<String, Object?>,
          );
          return jsonEncode({'ok': true});
        case 'restartApplication':
          return null;
        default:
          throw StateError('Unexpected host method: ${call.method}');
      }
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    await ClientLogger.initialize();
    await manager.initialize();
  });

  test(
    'deletion protects running and selected identities and persists before publishing',
    () async {
      await expectLater(
        manager.deleteIdentity('identity-personal'),
        throwsStateError,
      );
      await manager.switchIdentity('identity-work');
      await expectLater(
        manager.deleteIdentity('identity-work'),
        throwsStateError,
      );
      rejectWrite = true;
      await expectLater(
        manager.deleteIdentity('identity-lab'),
        throwsA(isA<PlatformException>()),
      );
      expect(manager.identities, hasLength(3));
      rejectWrite = false;
      await manager.deleteIdentity('identity-lab');
      await manager.saveStartupThemeMode(ThemeMode.dark);
      expect(persisted.identities.map((identity) => identity.id), [
        'identity-personal',
        'identity-work',
      ]);
      expect(persisted.activeIdentityId, 'identity-work');
      expect(manager.activeIdentity.id, 'identity-personal');
    },
  );

  testWidgets(
    'the inactive identity menu confirms deletion and the current menu omits it',
    (tester) async {
      final screenshot = const String.fromEnvironment('IDENTITY_SCREENSHOT');
      final wideScreenshot = const String.fromEnvironment(
        'IDENTITY_WIDE_SCREENSHOT',
      );
      final boundary = GlobalKey();
      if (screenshot.isNotEmpty) {
        await tester.runAsync(() async {
          final font = await File(
            const String.fromEnvironment('IDENTITY_SCREENSHOT_FONT'),
          ).readAsBytes();
          final loader = FontLoader('Aptos')
            ..addFont(Future.value(ByteData.sublistView(font)));
          await loader.load();
          final icons = await rootBundle.load(
            'fonts/MaterialIcons-Regular.otf',
          );
          await (FontLoader(
            'MaterialIcons',
          )..addFont(Future.value(icons))).load();
        });
      }
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(400, 442);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      tester.binding.platformDispatcher.localesTestValue = const [Locale('zh')];
      addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: OperitTheme(
            initialThemePreferenceSnapshot:
                UserPreferencesManager.defaultThemePreferenceSnapshot,
            initialThemeMode: ThemeMode.light,
            initialThemeIsReady: false,
            hostInteractionHostsEnabled: false,
            unconfiguredChildEnabled: true,
            child: Builder(
              builder: (context) => Scaffold(
                appBar: AppBar(
                  leading: BackButton(onPressed: () {}),
                  title: Text(
                    AppLocalizations.of(context)!.settingsUserProfileTitle,
                  ),
                ),
                body: const UserProfileSettingsPanel(
                  clients: GeneratedCoreProxyClients(_LoggedOutBridge()),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
      final currentMenu = find.byKey(
        const ValueKey('identity-actions-identity-personal'),
      );
      final workMenu = find.byKey(
        const ValueKey('identity-actions-identity-work'),
      );
      _expectAlignedRows(tester, currentMenu, workMenu);
      await _capture(tester, boundary, screenshot);

      tester.view.physicalSize = const Size(860, 442);
      await tester.pumpAndSettle();
      _expectAlignedRows(tester, currentMenu, workMenu);
      expect(tester.takeException(), isNull);
      await _capture(tester, boundary, wideScreenshot);

      tester.view.physicalSize = const Size(320, 580);
      tester.platformDispatcher.textScaleFactorTestValue = 1.6;
      addTearDown(tester.platformDispatcher.clearTextScaleFactorTestValue);
      await tester.pumpAndSettle();
      _expectAlignedRows(tester, currentMenu, workMenu);
      expect(tester.takeException(), isNull);
      tester.platformDispatcher.clearTextScaleFactorTestValue();
      tester.view.physicalSize = const Size(400, 442);
      await tester.pumpAndSettle();

      final delete = find.text('删除身份');
      await tester.tap(currentMenu);
      await tester.pumpAndSettle();
      expect(delete, findsNothing);
      await tester.tapAt(const Offset(8, 410));
      await tester.pumpAndSettle();
      await tester.tap(workMenu);
      await tester.pumpAndSettle();
      await tester.tap(delete);
      await tester.pumpAndSettle();
      expect(find.text('删除「工作空间」？'), findsOneWidget);
      await tester.tap(find.widgetWithText(FilledButton, '删除身份'));
      await tester.pumpAndSettle();
      expect(find.text('删除「工作空间」？'), findsNothing);
      expect(find.text('工作空间'), findsNothing);
      expect(persisted.identities.map((identity) => identity.id), [
        'identity-personal',
        'identity-lab',
      ]);
      expect(tester.takeException(), isNull);
    },
  );
}

/// Checks that names and menus share their respective columns across identities.
void _expectAlignedRows(
  WidgetTester tester,
  Finder currentMenu,
  Finder workMenu,
) {
  expect(
    tester.getRect(currentMenu).right,
    closeTo(tester.getRect(workMenu).right, 0.1),
  );
  expect(
    tester.getTopLeft(find.text('个人空间')).dx,
    closeTo(tester.getTopLeft(find.text('工作空间')).dx, 0.1),
  );
}

/// Saves a test render of the identity page when an output path is requested.
Future<void> _capture(
  WidgetTester tester,
  GlobalKey boundary,
  String path,
) async {
  if (path.isEmpty) {
    return;
  }
  await tester.runAsync(() async {
    final render =
        boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
    final image = await render.toImage(pixelRatio: 2);
    final bytes = (await image.toByteData(format: ui.ImageByteFormat.png))!;
    await File(path).writeAsBytes(bytes.buffer.asUint8List());
    image.dispose();
  });
}

class _LoggedOutBridge extends OperitRuntimeBridge {
  /// Creates a synthetic signed-out GitHub session.
  const _LoggedOutBridge();

  /// Returns encoded GitHub account data for the profile panel.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async =>
      encodeCoreLink(<Object?>[
        0,
        switch (request.methodName) {
          'isLoggedIn' => false,
          'getCurrentUserInfo' => null,
          _ => throw StateError('Unexpected Core call: ${request.methodName}'),
        },
      ]);

  /// Rejects operations outside the profile panel fixture.
  @override
  dynamic noSuchMethod(Invocation invocation) => super.noSuchMethod(invocation);
}
