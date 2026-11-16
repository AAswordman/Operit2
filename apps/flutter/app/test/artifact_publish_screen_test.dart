import 'dart:convert';
import 'dart:typed_data';

import 'package:crypto/crypto.dart' as crypto;
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:http/http.dart' as http;
import 'package:http/testing.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/screens/ArtifactPublishScreen.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Exercises the real publish page with the Core ABI and local HTTP responses.
void main() {
  for (final isToolPkg in [false, true]) {
    final kind = isToolPkg ? 'toolpkg' : 'script';

    testWidgets('Release $kind reads the original VFS path before registering', (
      tester,
    ) async {
      final fixture = _PublishFixture(isToolPkg: isToolPkg);
      await _mount(tester, fixture);
      await _selectRelease(tester, fixture);
      await _publish(tester, fixture);

      expect(find.text('发布完成'), findsOneWidget);
      expect(fixture.releaseReloads, hasLength(1));
      expect(fixture.downloads, hasLength(1));
      _expectOriginalRead(fixture);
      expect(fixture.protectionCalls, isEmpty);
      _expectAssetRegistration(
        fixture.registrations.single,
        fixture,
        bytes: fixture.remoteBytes,
        downloadUrl: fixture.downloadUrl,
      );
      expect(tester.takeException(), isNull);
    });

    final failures = <_FailureCase>[
      _FailureCase(
        'different local bytes',
        (fixture) {
          fixture.localBytes = Uint8List.fromList([
            ...fixture.localBytes,
            42,
          ]);
        },
        '与本地插件文件不一致',
      ),
      _FailureCase(
        'Core read rejection',
        (fixture) {
          fixture.readSuccess = false;
          fixture.readError = 'Fixture VFS access denied';
        },
        'Fixture VFS access denied',
      ),
      _FailureCase(
        'nonbinary Core response',
        (fixture) => fixture.nonbinaryResult = true,
        'binary file content',
      ),
      _FailureCase(
        'malformed Base64',
        (fixture) => fixture.contentBase64 = '!!!invalid-base64!!!',
        'Invalid Base64 file content',
      ),
      _FailureCase(
        'asset download failure',
        (fixture) => fixture.downloadStatus = 503,
        'HTTP 503',
        readCount: 0,
      ),
      _FailureCase(
        'asset removed after selection',
        (fixture) => fixture.assetRemovedOnReload = true,
        '所选 GitHub Release 资产不存在',
        readCount: 0,
        downloadCount: 0,
      ),
    ];

    for (final failure in failures) {
      testWidgets('Release $kind blocks registration after ${failure.name}', (
        tester,
      ) async {
        final fixture = _PublishFixture(isToolPkg: isToolPkg);
        failure.configure(fixture);
        await _mount(tester, fixture);
        await _selectRelease(tester, fixture);
        await _publish(tester, fixture);

        expect(fixture.releaseReloads, hasLength(1));
        expect(fixture.downloads, hasLength(failure.downloadCount));
        expect(fixture.binaryReads, hasLength(failure.readCount));
        if (failure.readCount > 0) _expectOriginalRead(fixture);
        expect(fixture.registrations, isEmpty);
        expect(fixture.protectionCalls, isEmpty);
        expect(find.textContaining(failure.errorText), findsOneWidget);
        expect(find.text('发布完成'), findsNothing);
        expect(find.text('重试市场登记'), findsNothing);
        expect(tester.takeException(), isNull);
      });
    }

    testWidgets('Release $kind continuation uses the existing entry', (
      tester,
    ) async {
      final fixture = _PublishFixture(isToolPkg: isToolPkg);
      await _mount(tester, fixture, continuation: true);
      await _selectRelease(tester, fixture);
      await _publish(tester, fixture, continuation: true);

      expect(find.text('发布完成'), findsOneWidget);
      _expectOriginalRead(fixture);
      expect(fixture.protectionCalls, isEmpty);
      expect(fixture.callsNamed('get_type_page'), isEmpty);
      final request = fixture.registrations.single;
      _expectAssetRegistration(
        request,
        fixture,
        bytes: fixture.remoteBytes,
        downloadUrl: fixture.downloadUrl,
        continuation: true,
      );
      final args = request.args as Map;
      expect(args['entry_id'], 'existing-entry');
      for (final key in [
        'entry_title',
        'entry_description',
        'entry_detail',
        'entry_category_id',
        'entry_allow_public_updates',
      ]) {
        expect(args[key], isNull, reason: 'The entry is locked: $key');
      }
      final title = tester.widget<TextField>(_field('显示名称'));
      expect(title.enabled, isFalse);
      expect(title.controller!.text, 'Locked fixture title');
      expect(tester.takeException(), isNull);
    });

    for (final continuation in [false, true]) {
      testWidgets(
        'Release $kind retries only registration (continuation: $continuation)',
        (tester) async {
          final fixture = _PublishFixture(isToolPkg: isToolPkg)
            ..registrationFailures = 1;
          await _mount(tester, fixture, continuation: continuation);
          await _selectRelease(tester, fixture);
          await _publish(tester, fixture, continuation: continuation);

          expect(fixture.registrations, hasLength(1));
          _expectOriginalRead(fixture);
          expect(find.textContaining('Fixture registration unavailable'),
              findsOneWidget);
          final first = fixture.registrations.single;
          final httpCount = fixture.httpRequests.length;
          await _activateHttpButton(
            tester,
            fixture,
            _buttonWithText('重试市场登记'),
          );

          expect(find.text('发布完成'), findsOneWidget);
          expect(fixture.registrations, hasLength(2));
          final retried = fixture.registrations.last;
          expect(retried.methodName, first.methodName);
          expect(retried.args, equals(first.args));
          _expectAssetRegistration(
            retried,
            fixture,
            bytes: fixture.remoteBytes,
            downloadUrl: fixture.downloadUrl,
            continuation: continuation,
          );
          expect(fixture.httpRequests, hasLength(httpCount));
          _expectOriginalRead(fixture);
          expect(fixture.protectionCalls, isEmpty);
          expect(find.text('重试市场登记'), findsNothing);
          expect(tester.takeException(), isNull);
        },
      );
    }

    for (final minify in [false, true]) {
      testWidgets('Direct $kind uploads the Core result (minify: $minify)', (
        tester,
      ) async {
        final fixture = _PublishFixture(isToolPkg: isToolPkg);
        await _mount(tester, fixture);
        if (minify) {
          await tester.tap(
            find.widgetWithText(SwitchListTile, '压缩并混淆插件脚本'),
          );
          await _settle(tester);
        }
        await _publish(tester, fixture);

        expect(find.text('发布完成'), findsOneWidget);
        expect(fixture.binaryReads, isEmpty);
        expect(fixture.protectionCalls, hasLength(1));
        final args = fixture.protectionCalls.single.args as Map;
        expect(args['sourcePath'], fixture.source.sourcePath);
        expect(args['isToolPkg'], isToolPkg);
        expect(args['packageId'], fixture.source.packageName);
        expect(args['version'], _version);
        expect(args['author'], ['publisher']);
        expect(args['minifyArtifact'], minify);
        expect(fixture.uploads, hasLength(1));
        expect(fixture.uploads.single.bodyBytes,
            orderedEquals(fixture.processedBytes));
        expect(fixture.processedBytes, isNot(orderedEquals(fixture.localBytes)));
        _expectAssetRegistration(
          fixture.registrations.single,
          fixture,
          bytes: fixture.processedBytes,
          downloadUrl: fixture.directDownloadUrl,
          direct: true,
        );
        expect(tester.takeException(), isNull);
      });
    }
  }

  testWidgets('Direct registration retry does not protect or upload again', (
    tester,
  ) async {
    final fixture = _PublishFixture(isToolPkg: true)
      ..registrationFailures = 1;
    await _mount(tester, fixture);
    await _publish(tester, fixture);

    expect(fixture.protectionCalls, hasLength(1));
    expect(fixture.uploads, hasLength(1));
    expect(fixture.registrations, hasLength(1));
    expect(find.textContaining('Fixture registration unavailable'),
        findsOneWidget);
    final first = fixture.registrations.single;
    final httpCount = fixture.httpRequests.length;
    await _activateHttpButton(tester, fixture, _buttonWithText('重试市场登记'));

    expect(find.text('发布完成'), findsOneWidget);
    expect(fixture.registrations, hasLength(2));
    expect(fixture.registrations.last.args, equals(first.args));
    expect(fixture.httpRequests, hasLength(httpCount));
    expect(fixture.protectionCalls, hasLength(1));
    expect(fixture.uploads, hasLength(1));
    expect(fixture.binaryReads, isEmpty);
    expect(tester.takeException(), isNull);
  });
}

const _version = '1.2.3';
const _releaseTag = 'v1.2.3';

class _FailureCase {
  const _FailureCase(
    this.name,
    this.configure,
    this.errorText, {
    this.readCount = 1,
    this.downloadCount = 1,
  });

  final String name;
  final void Function(_PublishFixture fixture) configure;
  final String errorText;
  final int readCount;
  final int downloadCount;
}

Finder _field(String label) => find.byWidgetPredicate(
      (widget) => widget is TextField && widget.decoration?.labelText == label,
    );

Future<void> _mount(
  WidgetTester tester,
  _PublishFixture fixture, {
  bool continuation = false,
}) async {
  tester.view.physicalSize = const Size(1100, 2800);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  addTearDown(fixture.client.close);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: ArtifactPublishScreen(
        clients: GeneratedCoreProxyClients(fixture.bridge),
        publishContext: continuation
            ? ArtifactPublishClusterContext(
                entryId: 'existing-entry',
                projectId: 'existing-project',
                runtimePackageId: fixture.source.packageName,
                lockedDisplayName: 'Locked fixture title',
              )
            : null,
      ),
    ),
  );
  await _settle(tester);
  expect(find.text('加载失败'), findsNothing);
  expect(tester.widget<TextField>(_field('简介')).controller!.text,
      'Fixture description');
  expect(tester.takeException(), isNull);
}

Future<void> _selectRelease(
  WidgetTester tester,
  _PublishFixture fixture,
) async {
  await tester.tap(find.text('引用 GitHub Release 资产'));
  await _settle(tester);
  await tester.enterText(
    _field('GitHub 仓库链接'),
    'https://github.com/publisher/fixture-project',
  );
  await _activateHttpButton(tester, fixture, _buttonWithText('读取 Release'));
  expect(fixture.catalogLoads, hasLength(1));
  await tester.tap(find.byType(DropdownButtonFormField<String>).at(1));
  await _settle(tester);
  await tester.tap(find.text(_releaseTag).last);
  await _settle(tester);
  await tester.tap(find.byType(DropdownButtonFormField<String>).at(2));
  await _settle(tester);
  await tester.tap(find.text(fixture.assetName).last);
  await _settle(tester);
}

Future<void> _publish(
  WidgetTester tester,
  _PublishFixture fixture, {
  bool continuation = false,
}) =>
    _activateHttpButton(
      tester,
      fixture,
      _buttonWithText(continuation ? '发布更新版本' : '发布到市场'),
    );

/// Runs the rendered button's real callback in the HTTP zone. Flutter gesture
/// callbacks need not retain a Zone, so tapping alone cannot guarantee isolation.
Future<void> _activateHttpButton(
  WidgetTester tester,
  _PublishFixture fixture,
  Finder finder,
) async {
  await tester.ensureVisible(finder);
  final button = tester.widget<ButtonStyleButton>(finder);
  expect(button.onPressed, isNotNull);
  http.runWithClient(button.onPressed!, () => fixture.client);
  await _settle(tester);
}

Future<void> _settle(WidgetTester tester) async {
  await tester.pumpAndSettle(const Duration(milliseconds: 50));
}

/// Matches the rendered button regardless of the `.icon` implementation type.
Finder _buttonWithText(String label) => find.ancestor(
      of: find.text(label),
      matching: find.byWidgetPredicate(
        (widget) => widget is ButtonStyleButton,
      ),
    );

void _expectOriginalRead(_PublishFixture fixture) {
  expect(fixture.binaryReads, hasLength(1));
  expect((fixture.binaryReads.single.args as Map)['tool'], {
    'name': 'read_file_binary',
    'parameters': [
      {'name': 'path', 'value': fixture.source.sourcePath},
    ],
  });
}

void _expectAssetRegistration(
  CoreCallRequest request,
  _PublishFixture fixture, {
  required Uint8List bytes,
  required String downloadUrl,
  bool direct = false,
  bool continuation = false,
}) {
  expect(request.methodName,
      continuation ? 'publish_artifact_version' : 'publish_artifact');
  final args = request.args as Map;
  expect(args['runtime_package_id'], fixture.source.packageName);
  expect(args['version'], _version);
  expect(args['format_ver'], fixture.isToolPkg ? 'toolpkg_v2' : 'script_v2');
  expect(args['asset_kind'], 'github_release_asset');
  expect(args['asset_url'], downloadUrl);
  expect(args['gh_owner'], 'publisher');
  expect(args['gh_repo'], direct ? 'OperitForge' : 'fixture-project');
  expect(args['gh_release_tag'], direct ? fixture.directTag : _releaseTag);
  expect(args['asset_name'], direct ? fixture.directAssetName : fixture.assetName);
  expect(args['sha256'], crypto.sha256.convert(bytes).toString());
  expect(args['api_version'], fixture.isToolPkg ? '2.0.0' : null);
  expect(args.containsKey('sha'), isFalse);
}

class _PublishFixture {
  _PublishFixture({required this.isToolPkg}) {
    // Deliberately include non-text bytes. The page must transport these bytes,
    // not parse, install, repackage, or register a plugin from their contents.
    localBytes = Uint8List.fromList([0x50, 0x4b, 0x03, 0x04, 0, 127, 255]);
    remoteBytes = Uint8List.fromList(localBytes);
    bridge = _PublishBridge(this);
    client = MockClient(_respond);
  }

  final bool isToolPkg;
  late Uint8List localBytes;
  late Uint8List remoteBytes;
  final processedBytes = Uint8List.fromList([99, 111, 114, 101, 0, 255, 42]);
  late final _PublishBridge bridge;
  late final MockClient client;
  final httpRequests = <http.Request>[];
  bool readSuccess = true;
  String? readError;
  bool nonbinaryResult = false;
  String? contentBase64;
  int downloadStatus = 200;
  bool assetRemovedOnReload = false;
  int registrationFailures = 0;

  String get extension => isToolPkg ? 'toolpkg' : 'js';
  String get assetName => 'fixture.$extension';
  String get downloadUrl =>
      'https://github.com/publisher/fixture-project/releases/download/'
      '$_releaseTag/$assetName';
  String get directTag =>
      '${isToolPkg ? 'package' : 'script'}-dev-fixture-v$_version';
  String get directAssetName => 'dev-fixture-v$_version.$extension';
  String get directDownloadUrl =>
      'https://github.com/publisher/OperitForge/releases/download/'
      '$directTag/$directAssetName';

  core_proxy.PublishablePackageSource get source =>
      core_proxy.PublishablePackageSource(
        packageName: 'dev.fixture',
        displayName: 'Fixture artifact',
        description: 'Fixture description',
        author: const ['publisher'],
        sourcePath: '/operitapp/data/extensions/device/packages/$assetName',
        sourceFileName: assetName,
        fileExtension: extension,
        isToolPkg: isToolPkg,
        inferredVersion: _version,
        apiVersion: isToolPkg ? '2.0.0' : null,
      );

  List<CoreCallRequest> callsNamed(String method) => bridge.calls
      .where((request) => request.methodName == method)
      .toList();
  List<CoreCallRequest> get binaryReads => callsNamed('executeTool');
  List<CoreCallRequest> get protectionCalls => callsNamed('protectArtifactFile');
  List<CoreCallRequest> get registrations => bridge.calls
      .where((request) => {
            'publish_artifact',
            'publish_artifact_version',
          }.contains(request.methodName))
      .toList();
  List<http.Request> get catalogLoads => httpRequests
      .where((request) => request.url.path ==
          '/repos/publisher/fixture-project/releases')
      .toList();
  List<http.Request> get releaseReloads => httpRequests
      .where((request) => request.url.path ==
          '/repos/publisher/fixture-project/releases/tags/$_releaseTag')
      .toList();
  List<http.Request> get downloads => httpRequests
      .where((request) => request.url.toString() == downloadUrl)
      .toList();
  List<http.Request> get uploads => httpRequests
      .where((request) => request.url.host == 'uploads.github.com')
      .toList();

  Map<String, Object?> _asset({bool direct = false}) => {
        'id': 402,
        'name': direct ? directAssetName : assetName,
        'browser_download_url': direct ? directDownloadUrl : downloadUrl,
      };

  Map<String, Object?> _release({bool direct = false, bool removed = false}) => {
        'id': 401,
        'tag_name': direct ? directTag : _releaseTag,
        'name': null,
        'assets': direct || removed ? <Object?>[] : [_asset()],
      };

  http.Response _json(Object? value, [int status = 200]) => http.Response(
        jsonEncode(value),
        status,
        headers: {'content-type': 'application/json'},
      );

  Future<http.Response> _respond(http.Request request) async {
    httpRequests.add(request);
    final uri = request.url;
    if (request.method == 'GET' &&
        uri.host == 'api.github.com' &&
        uri.path == '/repos/publisher/fixture-project/releases') {
      return _json([_release()]);
    }
    if (request.method == 'GET' &&
        uri.host == 'api.github.com' &&
        uri.path ==
            '/repos/publisher/fixture-project/releases/tags/$_releaseTag') {
      return _json(_release(removed: assetRemovedOnReload));
    }
    if (request.method == 'GET' && uri.toString() == downloadUrl) {
      if (downloadStatus != 200) {
        return http.Response('Fixture download unavailable', downloadStatus);
      }
      return http.Response.bytes(remoteBytes, 200);
    }
    if (request.method == 'GET' &&
        uri.host == 'api.github.com' &&
        uri.path == '/repos/publisher/OperitForge') {
      return _json({
        'size': 1,
        'name': 'OperitForge',
        'html_url': 'https://github.com/publisher/OperitForge',
      });
    }
    if (request.method == 'GET' &&
        uri.host == 'api.github.com' &&
        uri.path == '/repos/publisher/OperitForge/releases/tags/$directTag') {
      return _json({'message': 'Not Found'}, 404);
    }
    if (request.method == 'POST' &&
        uri.host == 'api.github.com' &&
        uri.path == '/repos/publisher/OperitForge/releases') {
      expect((jsonDecode(request.body) as Map)['tag_name'], directTag);
      return _json(_release(direct: true), 201);
    }
    if (request.method == 'POST' &&
        uri.host == 'uploads.github.com' &&
        uri.path == '/repos/publisher/OperitForge/releases/401/assets') {
      expect(uri.queryParameters['name'], directAssetName);
      return _json(_asset(direct: true), 201);
    }
    throw StateError('Unexpected fixture HTTP: ${request.method} $uri');
  }
}

/// Uses generated client decoding, with no permissive fallbacks for Core calls.
class _PublishBridge extends OperitRuntimeBridge {
  _PublishBridge(this.fixture);

  final _PublishFixture fixture;
  final calls = <CoreCallRequest>[];

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request);
    final Object? result;
    switch (request.methodName) {
      case 'get_manifest':
        result = {
          'ok': true,
          'marketVersion': 2,
          'generatedAt': null,
          'types': <Object?>[],
          'formatVersions': <Object?>[],
          'categories': [
            {'id': 'fixture', 'name': 'Fixture category', 'description': null},
          ],
          'states': <Object?>[],
        };
      case 'getPublishablePackageSources':
        result = [fixture.source.toJson()];
      case 'get_current_github_user':
        result = {
          'id': 1,
          'login': 'publisher',
          'name': null,
          'email': null,
          'avatar_url': '',
          'bio': null,
          'public_repos': null,
          'followers': null,
          'following': null,
        };
      case 'get_type_page':
        result = {
          'generatedAt': null,
          'sort': 'updated',
          'page': 1,
          'pageSize': 100,
          'total': 0,
          'items': <Object?>[],
        };
      case 'getCurrentAccessToken':
        result = 'fixture-token';
      case 'executeTool':
        expect((request.args as Map)['tool'], {
          'name': 'read_file_binary',
          'parameters': [
            {'name': 'path', 'value': fixture.source.sourcePath},
          ],
        });
        result = {
          'toolName': 'read_file_binary',
          'success': fixture.readSuccess,
          'error': fixture.readError,
          'result': fixture.nonbinaryResult
              ? {'__type': 'StringResultData', 'value': 'not binary'}
              : {
                  '__type': 'BinaryFileContentData',
                  'path': fixture.source.sourcePath,
                  'size': fixture.localBytes.length,
                  'contentBase64': fixture.contentBase64 ??
                      base64Encode(fixture.localBytes),
                },
        };
      case 'protectArtifactFile':
        result = fixture.processedBytes;
      case 'publish_artifact':
      case 'publish_artifact_version':
        if (fixture.registrationFailures > 0) {
          fixture.registrationFailures--;
          throw StateError('Fixture registration unavailable');
        }
        result = {
          'ok': true,
          'entryId': request.methodName == 'publish_artifact_version'
              ? 'existing-entry'
              : 'new-entry',
          'versionId': 'new-version',
        };
      default:
        throw StateError('Unexpected fixture Core call: ${request.methodName}');
    }
    return encodeCoreLink([0, result]);
  }

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
