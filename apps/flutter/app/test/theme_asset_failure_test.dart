import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/theme/OperitThemeAssets.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/core/host/AppToastController.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/AppToastHost.dart';

/// Verifies storage failures retain context without editing appearance values.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUp(() async {
    await ClientLogger.initialize();
    AppToastController.instance.reset();
  });
  tearDown(AppToastController.instance.reset);

  ThemeAssetStore store(_AssetStorageBridge bridge) => ThemeAssetStore(
    runtimeStorage: GeneratedCoreProxyClients(
      bridge,
    ).repositoryRuntimeStorageRepository,
  );

  test('a missing resource keeps its logical reference and read stage', () async {
    final bridge = _AssetStorageBridge();
    const reference = 'runtime/data/user_assets/theme/missing.png';
    final failure = await _failure(store(bridge).readBytes(reference));

    expect(failure.reference, reference);
    expect(failure.stage, ThemeAssetStage.read);
    expect(failure.kind, ThemeAssetFailureKind.missing);
    expect(failure.cause, isA<StateError>());
    expect(failure.details, contains(reference));
    expect(bridge.calls.map((call) => call.methodName), ['readBase64']);
    expect(bridge.files, isEmpty);
  });

  test('an existing empty resource is classified before media decoding', () async {
    final bridge = _AssetStorageBridge();
    const reference = 'runtime/data/user_assets/theme/empty.mp4';
    bridge.files[reference] = Uint8List(0);
    final failure = await _failure(store(bridge).readBytes(reference));

    expect(failure.reference, reference);
    expect(failure.stage, ThemeAssetStage.read);
    expect(failure.kind, ThemeAssetFailureKind.empty);
    expect(failure.byteLength, 0);
    expect(bridge.files[reference], isEmpty);
  });

  test('a Core read error keeps its cause and does not affect another asset', () async {
    final bridge = _AssetStorageBridge();
    const broken = 'runtime/data/user_assets/theme/unreadable.png';
    const intact = 'runtime/data/user_assets/theme/intact.svg';
    bridge.files[intact] = _imageBytes();
    bridge.readErrors[broken] = const CoreLinkError(
      code: 'STORAGE_READ_FAILED',
      message: 'The resource existed but could not be read',
      backtrace: 'original Rust diagnostic',
    );
    final assets = store(bridge);
    final failure = await _failure(assets.readBytes(broken));

    expect(failure.reference, broken);
    expect(failure.stage, ThemeAssetStage.read);
    expect(failure.kind, ThemeAssetFailureKind.read);
    final cause = failure.cause as CoreLinkError;
    expect(cause.code, 'STORAGE_READ_FAILED');
    expect(cause.backtrace, 'original Rust diagnostic');
    expect(await assets.readBytes(intact), _imageBytes());
    expect(bridge.calls.every((call) => call.methodName == 'readBase64'), isTrue);
    expect(bridge.files.keys, [intact]);
  });

  for (final reference in [
    'content://documents/old-background.png',
    'file:///old/background.png',
    '/old/background.png',
    'assets/images/not-imported.png',
  ]) {
    test('an invalid host reference is passed through unchanged: $reference', () async {
      final bridge = _AssetStorageBridge();
      bridge.readErrors[reference] = const CoreLinkError(
        code: 'INVALID_STORAGE_PATH',
        message: 'Runtime storage path must use a supported logical root',
      );
      final failure = await _failure(store(bridge).readBytes(reference));

      expect(failure.reference, reference);
      expect(failure.kind, ThemeAssetFailureKind.read);
      expect((bridge.calls.single.args as Map)['path'], reference);
      expect((failure.cause as CoreLinkError).code, 'INVALID_STORAGE_PATH');
      expect(bridge.files, isEmpty);
    });
  }

  test('empty import is rejected before allocating a storage path or writing', () async {
    final bridge = _AssetStorageBridge();
    final failure = await _failure(
      store(bridge).importBytes(bytes: Uint8List(0), fileName: 'empty.png'),
    );

    expect(failure.reference, 'empty.png');
    expect(failure.stage, ThemeAssetStage.importAsset);
    expect(failure.kind, ThemeAssetFailureKind.empty);
    expect(failure.byteLength, 0);
    expect(bridge.calls, isEmpty);
    expect(bridge.files, isEmpty);
  });

  test('valid prepared bytes retain content-addressed import and readback', () async {
    final bridge = _AssetStorageBridge();
    final bytes = _imageBytes();
    final assets = store(bridge);
    final imported = await assets.importBytes(
      bytes: bytes,
      fileName: 'background.svg',
    );
    final digest = sha256.convert(bytes).toString().substring(0, 16);

    expect(imported.storagePath, 'runtime/data/user_assets/theme/$digest.svg');
    expect(imported.fileName, 'background.svg');
    expect(imported.bytes, bytes);
    expect(await assets.readBytes(imported.storagePath), bytes);
    expect(bridge.calls.map((call) => call.methodName), [
      'themeAssetsDirPath',
      'writeBase64',
      'readBase64',
    ]);
  });

  test('a failed resource write keeps the Core cause and existing assets', () async {
    final bridge = _AssetStorageBridge();
    const oldReference = 'runtime/data/user_assets/theme/previous.svg';
    bridge.files[oldReference] = _imageBytes();
    bridge.writeError = const CoreLinkError(
      code: 'STORAGE_WRITE_FAILED',
      message: 'Cannot persist the imported resource',
    );
    final failure = await _failure(
      store(bridge).importBytes(bytes: _imageBytes(), fileName: 'new.svg'),
    );

    expect(failure.stage, ThemeAssetStage.importAsset);
    expect(failure.reference, endsWith('.svg'));
    expect((failure.cause as CoreLinkError).code, 'STORAGE_WRITE_FAILED');
    expect(bridge.files.keys, [oldReference]);
    expect(bridge.files[oldReference], _imageBytes());
  });

  testWidgets('resource notices deduplicate and retry without replacing the draft', (tester) async {
    final bridge = _AssetStorageBridge();
    final assets = store(bridge);
    final diagnostics = ThemeAssetDiagnostics();
    const reference = 'runtime/data/user_assets/theme/later.svg';
    await tester.pumpWidget(_noticeScene(assets, diagnostics, reference));
    await _finishRendering(tester);
    await tester.enterText(find.byKey(const ValueKey('asset-draft')), 'Keep this draft');
    final input = tester.state(find.byType(EditableText));
    expect(diagnostics.failures, hasLength(1));
    expect(AppToastController.instance.notices.value, hasLength(1));
    final notice = AppToastController.instance.notices.value.single;
    expect(notice.actionLabel, '重试');
    expect(notice.text, contains('默认背景'));
    expect(bridge.calls.where((c) => c.methodName == 'readBase64'), hasLength(1));

    bridge.files[reference] = _imageBytes();
    notice.onAction!();
    await _finishRendering(tester);
    expect(diagnostics.failures, isEmpty);
    expect(AppToastController.instance.notices.value, isEmpty);
    expect(diagnostics.retryEpoch(reference), 1);
    expect(bridge.calls.where((c) => c.methodName == 'readBase64'), hasLength(2));
    expect(tester.state(find.byType(EditableText)), same(input));
    expect(find.text('Keep this draft'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
    diagnostics.dispose();
  });

  testWidgets('appearance preview keeps its child and offers local same-path retry', (tester) async {
    final bridge = _AssetStorageBridge();
    final assets = store(bridge);
    const reference = 'runtime/data/user_assets/theme/preview.svg';
    final snapshot = ThemePreferenceSnapshot.fromJson({
      ...ThemePreferenceDefaults.snapshot.toJson(),
      'useBackgroundImage': true,
      'backgroundImageUri': reference,
    });
    await tester.pumpWidget(MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: SizedBox(
        width: 400,
        height: 240,
        child: OperitThemePreview(
          snapshot: snapshot,
          assets: assets,
          child: const Column(children: [
            Text('Preview content stays mounted'),
            SizedBox(height: 80, child: ThemeAssetImage(storagePath: reference, fit: BoxFit.cover)),
          ]),
        ),
      ),
    ));
    await _finishRendering(tester);
    expect(find.text('Preview content stays mounted'), findsOneWidget);
    expect(find.textContaining('默认背景'), findsOneWidget);
    expect(AppToastController.instance.notices.value, isEmpty);
    bridge.files[reference] = _imageBytes();
    await tester.tap(find.text('重试'));
    await _finishRendering(tester);
    expect(find.textContaining('默认背景'), findsNothing);
    expect(find.text('Preview content stays mounted'), findsOneWidget);
    expect(bridge.calls.where((c) => c.methodName == 'readBase64'), hasLength(2));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  test('a stackless media failure records the request site without inventing a stack', () {
    final cause = PlatformException(
      code: 'media open error',
      message: 'invalid or unsupported media',
      details: 'data:video/mp4;base64,PRIVATE_PAYLOAD',
    );
    final request = StackTrace.fromString('video request site');
    final failure = ThemeAssetFailure.capture(
      reference: 'runtime/data/user_assets/theme/failed.mp4',
      purpose: ThemeAssetPurpose.background,
      stage: ThemeAssetStage.videoOpen,
      kind: ThemeAssetFailureKind.media,
      cause: cause,
      originalStackTrace: StackTrace.empty,
      requestStackTrace: request,
      byteLength: 24,
    ).withContext(themeRevision: 3, themeId: 'saved-theme');

    expect(failure.cause, same(cause));
    expect(failure.originalStackTrace.toString(), isEmpty);
    expect(failure.hasOriginalStack, isFalse);
    expect(failure.requestStackTrace, same(request));
    expect(failure.details, contains('<empty>'));
    expect(failure.details, contains('video request site'));
    expect(failure.details, contains('revision=3'));
    expect(failure.details, contains('saved-theme'));
    expect(failure.details, isNot(contains('PRIVATE_PAYLOAD')));
  });
}

Widget _noticeScene(ThemeAssetResolver assets, ThemeAssetDiagnostics diagnostics, String reference) =>
    ThemeAssetScope(
      resolver: assets,
      diagnostics: diagnostics,
      child: MaterialApp(
        locale: const Locale('zh'),
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        builder: (context, child) => AppToastHost(
          child: ThemeAssetNoticeHost(diagnostics: diagnostics, child: child!),
        ),
        home: Scaffold(body: Column(children: [
          for (var i = 0; i < 2; i++)
            SizedBox(height: 80, child: ResolvedThemeImage(
              resolver: assets, reference: reference, fit: BoxFit.cover,
            )),
          const TextField(key: ValueKey('asset-draft')),
        ])),
      ),
    );

Future<void> _finishRendering(WidgetTester tester) async {
  await tester.runAsync(() => Future<void>.delayed(const Duration(milliseconds: 50)));
  await tester.pumpAndSettle();
}

Uint8List _imageBytes() => Uint8List.fromList(
  utf8.encode(
    '<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8">'
    '<rect width="8" height="8" fill="#339966"/></svg>',
  ),
);

Future<ThemeAssetFailure> _failure(Future<Object?> operation) async {
  try {
    await operation;
  } on ThemeAssetFailure catch (failure) {
    return failure;
  }
  throw StateError('Expected a contextual theme resource failure');
}

/// Uses the real Link codec, rejecting any preference, selection or watch call.
class _AssetStorageBridge extends OperitRuntimeBridge {
  final files = <String, Uint8List>{};
  final readErrors = <String, CoreLinkError>{};
  final calls = <CoreCallRequest>[];
  CoreLinkError? writeError;

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final args = request.args as Map;
    final Object? result;
    switch (request.methodName) {
      case 'readBase64':
        final reference = args['path'] as String;
        final error = readErrors[reference];
        if (error != null) return _errorFrame(error);
        final bytes = files[reference];
        result = bytes == null ? null : base64Encode(bytes);
      case 'themeAssetsDirPath':
        result = 'runtime/data/user_assets/theme';
      case 'writeBase64':
        final error = writeError;
        if (error != null) return _errorFrame(error);
        files[args['path'] as String] = base64Decode(args['base64Content'] as String);
        result = null;
      default:
        throw StateError('Unexpected theme asset call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  Uint8List _errorFrame(CoreLinkError error) => encodeCoreLink(<Object?>[
    1,
    error.code,
    error.message,
    error.details,
    null,
    error.backtrace,
  ]);

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();

  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      throw UnimplementedError();
}
