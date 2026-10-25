import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as model;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/runtime/RuntimeSettingsPanel.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/startup/RuntimeActivityOverlay.dart';

/// Both native apps use production pairing, files, progress and settings UI.
/// The host coordinator on 39180 only orchestrates phases and screenshots.
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets(
    'native space sync: join, offline backlog, silent small edits',
    (tester) async {
      final role = Platform.isMacOS ? 'macos' : 'ios';
      final other = role == 'macos' ? 'ios' : 'macos';
      final http = HttpClient();
      addTearDown(http.close);
      Future<Object?> exchange(
        String method,
        String key, [
        Object? value,
      ]) async {
        final request = await http.openUrl(
          method,
          Uri.parse('http://127.0.0.1:39180/$key'),
        );
        if (method == 'PUT') {
          final body = utf8.encode(jsonEncode(value));
          request.contentLength = body.length;
          request.add(body);
        }
        final response = await request.close();
        final text = await utf8.decoder.bind(response).join();
        return text.isEmpty ? null : jsonDecode(text);
      }

      Future<void> put(String key, Object? value) async {
        await exchange('PUT', key, value);
      }

      Future<Object?> waitKey(String key) async {
        final end = DateTime.now().add(const Duration(minutes: 8));
        while (DateTime.now().isBefore(end)) {
          final value = await exchange('GET', key);
          if (value != null) return value;
          await Future<void>.delayed(const Duration(milliseconds: 150));
        }
        throw StateError('Timed out waiting for $key');
      }

      Future<void> until(FutureOr<bool> Function() condition) async {
        final end = DateTime.now().add(const Duration(minutes: 5));
        while (DateTime.now().isBefore(end)) {
          if (await condition()) return;
          await Future<void>.delayed(const Duration(milliseconds: 100));
        }
        throw StateError('Native sync condition timed out ($role)');
      }

      final support = await getApplicationSupportDirectory();
      final root = await Directory(
        '${support.path}/device-space-sync-native',
      ).create(recursive: true);
      final run = await root.createTemp('run-');
      final workspace = await Directory('${run.path}/workspaces').create();
      await platformCoreProxy.setRuntimeStorageRoots(
        '${run.path}/runtime',
        workspace.path,
      );
      await ClientLogger.initialize();
      RuntimeHostInteractionSubscriber.install();
      addTearDown(RuntimeHostInteractionSubscriber.uninstall);
      const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
      final service = clients.server.runtimeRemoteLinkService;
      await service.initializeDeviceInfo(
        supplied: model.LinkDeviceInfo(
          platform: role == 'macos' ? 'macOS' : 'iOS',
          model: role == 'macos' ? 'Mac' : 'iPhone',
          pairingIncarnation: null,
        ),
      );
      final node = (await service.deviceSpaceTopology()).currentDeviceId;
      await put('$role-node', node);
      await put('$role-root', run.path);
      tester.binding.platformDispatcher.localesTestValue = const [Locale('zh')];
      addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
      final previewBridge = _PackagePreviewBridge();
      addTearDown(previewBridge.packages.close);
      final page = ValueNotifier<bool>(false);
      addTearDown(page.dispose);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Stack(
            children: [
              ValueListenableBuilder<bool>(
                valueListenable: page,
                builder: (context, alternate, _) => Scaffold(
                  appBar: AppBar(title: Text(alternate ? '工作空间' : '设备空间')),
                  body: alternate
                      ? const Center(child: Text('后台同步期间可继续使用工作空间'))
                      : RuntimeSettingsPanel(
                          onOpenProfile: () {},
                          clients: clients,
                        ),
                ),
              ),
              RuntimeActivityOverlay(
                enabled: true,
                clients: GeneratedCoreProxyClients(previewBridge),
              ),
            ],
          ),
        ),
      );
      final events = <model.SpaceSyncProgress>[];
      final subscription = service.spaceSyncProgressFlow().listen((items) {
        events.addAll(items);
        for (final item in items) {
          if (item.status != model.SpaceSyncStatus.syncing) {
            unawaited(put('$role-last-settled-progress', item.toJson()));
          }
        }
      });
      addTearDown(subscription.cancel);
      var phaseStart = 0;
      Future<void> capture(String name) async {
        await tester.pump(const Duration(milliseconds: 100));
        final title = find.textContaining('正在与 ');
        if (title.evaluate().isNotEmpty) {
          await tester.ensureVisible(title.first);
          await tester.pump(const Duration(milliseconds: 100));
        }
        await put(
          'capture-$role-$name',
          events.isEmpty ? null : events.last.toJson(),
        );
        await waitKey('captured-$role-$name');
      }

      Future<void> largeVisible(String phase) async {
        await until(
          () => events
              .skip(phaseStart)
              .any(
                (e) =>
                    e.status == model.SpaceSyncStatus.syncing &&
                    e.fileBytes > 0,
              ),
        );
        await put('focus-$role-$phase', true);
        await tester.pump(const Duration(milliseconds: 100));
        expect(find.text('正在同步'), findsOneWidget);
        expect(find.byType(LinearProgressIndicator), findsWidgets);
        await capture(phase);
        if (phase == 'initial') {
          page.value = true;
          await tester.pump(const Duration(milliseconds: 100));
          expect(find.text('正在同步'), findsOneWidget);
          await capture('global-navigation');
          page.value = false;
          await tester.pump(const Duration(milliseconds: 100));
        }
        await put('$role-$phase-visible', true);
      }

      Future<String> seed(String fileName, int value) async {
        final dir = await Directory('${workspace.path}/native-proof').create();
        final file = File('${dir.path}/$fileName');
        final chunk = Uint8List(1024 * 1024);
        for (var i = 0; i < chunk.length; i++) {
          chunk[i] = (i + value) % 251;
        }
        final out = file.openWrite();
        for (var i = 0; i < 96; i++) {
          out.add(chunk);
          await out.flush();
        }
        await out.close();
        return (await sha256.bind(file.openRead()).first).toString();
      }

      Future<void> received(String fileName, String expectedHash) async {
        final file = File('${workspace.path}/native-proof/$fileName');
        await until(
          () async =>
              await file.exists() && await file.length() == 96 * 1024 * 1024,
        );
        expect(
          (await sha256.bind(file.openRead()).first).toString(),
          expectedHash,
        );
      }

      if (role == 'macos') {
        await service.renameDeviceSpace(spaceName: '多端同步验证');
        await service.saveLocalHostConfig(
          config: model.PeerHostConfig(
            bindAddress: '127.0.0.1:39181',
            transports: [model.PeerTransport.http],
            discoveryEnabled: false,
            portMode: model.PeerHostPortMode.fixed,
            updatedAt: DateTime.now().millisecondsSinceEpoch,
          ),
        );
        await service.startListening(transports: [model.PeerTransport.http]);
        await service.startSpaceSync();
        await put('initial-hash', await seed('initial.bin', 1));
        await put('mac-ready', true);
        await until(() async {
          final prompts = await service.pairingPrompts();
          if (prompts.isEmpty) return false;
          await put('pair-code', prompts.first.confirmationCode);
          return true;
        });
        await until(() async {
          final requests = await service.incomingDeviceSpaceJoins();
          if (requests.isEmpty) return false;
          final request = requests.first;
          await service.decideDeviceSpaceJoin(
            requestId: request.requestId,
            assignmentVersion: request.assignmentVersion,
            approve: true,
          );
          return true;
        });
      } else {
        await waitKey('mac-ready');
        final target = await waitKey('macos-node') as String;
        final pairing = await service.startPairing(
          nodeId: target,
          address: 'http://127.0.0.1:39181/link',
          transport: model.PeerTransport.http,
        );
        await service.finishPairing(
          pairingId: pairing.pairingId,
          confirmationCode: await waitKey('pair-code') as String,
        );
        final request = await service.requestDeviceSpaceJoin(deviceId: target);
        await until(
          () async =>
              (await service.refreshDeviceSpaceJoin(
                requestId: request.requestId,
              )).status ==
              model.SpaceJoinStatus.joined,
        );
        await service.startSpaceSync();
      }
      await largeVisible('initial');
      if (role == 'ios') {
        await received('initial.bin', await waitKey('initial-hash') as String);
        await put('initial-received', true);
      }
      await waitKey('initial-received');
      await until(
        () => events.any((e) => e.status == model.SpaceSyncStatus.completed),
      );
      await tester.pump(const Duration(seconds: 4));
      expect(find.textContaining('正在与 '), findsNothing);
      await capture('initial-complete');
      await put('$role-initial-settled', true);
      await waitKey('$other-initial-settled');
      // Stop the original worker, accumulate an incremental backlog, then resume.
      await service.stopSpaceSync();
      if (role == 'macos') await service.stopListening();
      await put('$role-paused', true);
      await waitKey('$other-paused');
      if (role == 'macos') {
        final hash = await seed('offline-backlog.bin', 7);
        await service.startListening(transports: [model.PeerTransport.http]);
        await put('incremental-hash', hash);
      }
      await waitKey('incremental-hash');
      phaseStart = events.length;
      if (role == 'ios') {
        final target = await waitKey('macos-node') as String;
        await until(() => service.pairedDeviceOnline(deviceId: target));
      }
      await service.startSpaceSync();
      await largeVisible('incremental');
      if (role == 'ios') {
        await received(
          'offline-backlog.bin',
          await waitKey('incremental-hash') as String,
        );
        await put('incremental-received', true);
      }
      await waitKey('incremental-received');
      await until(
        () => events
            .skip(phaseStart)
            .any((e) => e.status == model.SpaceSyncStatus.completed),
      );
      await tester.pump(const Duration(seconds: 5));
      await put('$role-incremental-settled', true);
      await waitKey('$other-incremental-settled');
      phaseStart = events.length;
      if (role == 'macos') {
        await File(
          '${workspace.path}/native-proof/small.txt',
        ).writeAsString('silent');
        await put('small-written', true);
      }
      await waitKey('small-written');
      if (role == 'ios') {
        final file = File('${workspace.path}/native-proof/small.txt');
        await until(
          () async =>
              await file.exists() && await file.readAsString() == 'silent',
        );
        await put('small-received', true);
      }
      await waitKey('small-received');
      await tester.pump(const Duration(seconds: 5));
      expect(
        events
            .skip(phaseStart)
            .where((e) => e.status != model.SpaceSyncStatus.completed),
        isEmpty,
      );
      expect(find.textContaining('正在与 '), findsNothing);
      await capture('small-silent');
      // Package UI previews use fixture data; sync phases above use real Core.
      previewBridge.showPackages();
      await tester.pump(const Duration(milliseconds: 150));
      expect(find.text('加载扩展'), findsOneWidget);
      await capture('package-collapsed-preview');
      await tester.tap(find.text('加载扩展'));
      await tester.pump(const Duration(milliseconds: 150));
      expect(find.text('后台任务'), findsOneWidget);
      await capture('package-expanded-preview');
      await put('$role-result', {
        'initial': 'passed',
        'incremental': 'passed',
        'small': 'passed',
        'progressEvents': events.length,
        'maxBytes': events
            .map((e) => e.transferredBytes)
            .fold<int>(0, (a, b) => a > b ? a : b),
        'root': run.path,
      });
      // ignore: avoid_print
      print('NATIVE_SYNC_OK $role events=${events.length} root=${run.path}');
      await waitKey('$other-result');
      await service.stopSpaceSync();
      await service.stopListening();
      await tester.pumpWidget(const SizedBox.shrink());
    },
    timeout: const Timeout(Duration(minutes: 20)),
  );
}

/// Only package previews are injected. All sync/topology subscriptions and calls
/// delegate to the native production bridge throughout the test.
class _PackagePreviewBridge extends OperitRuntimeBridge {
  final packages = StreamController<CoreEvent>.broadcast();
  static const delegate = ProxyCoreRuntimeBridge();
  void showPackages() => packages.add(
    CoreEvent.raw(
      requestId: 'package-preview',
      target: 'core/application',
      propertyName: 'pluginLoadingProgressFlow',
      kind: 'Snapshot',
      valueBytes: encodeCoreLink(
        const model.PluginLoadingProgress(
          visible: true,
          forceExpanded: false,
          progress: 0.6,
          phase: 'loading',
          currentTask: '正在启动工具服务',
          pluginsStarted: 3,
          pluginsTotal: 5,
          plugins: [
            model.PluginLoadingItem(
              id: 'tools',
              displayName: '工具服务',
              kind: 'plugin',
              status: 'starting',
              message: '正在初始化工具',
              logText: '',
            ),
            model.PluginLoadingItem(
              id: 'workspace',
              displayName: '工作空间',
              kind: 'plugin',
              status: 'started',
              message: '已就绪',
              logText: '',
            ),
          ],
        ).toJson(),
      ),
      decodeValue: decodeCoreLink<Object?>,
    ),
  );
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      request.propertyName == 'pluginLoadingProgressFlow'
      ? packages.stream
      : delegate.watchStream(request);
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) =>
      delegate.callBytes(request);
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      delegate.watchSnapshot(request);
  @override
  Future<CorePushSink> push(CorePushRequest request) => delegate.push(request);
}
