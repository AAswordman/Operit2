import 'dart:async';
import 'dart:typed_data';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/model/ExtensionCatalogController.dart';

void main() {
  test(
    'checking updates refreshes bundled skills without market markers',
    () async {
      final bridge = _SkillUpdateBridge();
      final controller = ExtensionCatalogController.of(
        GeneratedCoreProxyClients(bridge),
      );
      await controller.refresh();
      expect(controller.updatableEntries(), isEmpty);

      bridge.updateAvailable = true;
      await controller.checkUpdates();
      expect(controller.updatableEntries().single.id, 'PackageBuilder');
      expect(bridge.calls.skip(bridge.calls.length - 2), [
        'getExtensionCatalogSnapshot',
        'checkExtensionCatalogUpdates',
      ]);
      expect(controller.lastUpdateCheck, isNotNull);
    },
  );

  test(
    'bundled updates remain visible when the marketplace is offline',
    () async {
      final bridge = _SkillUpdateBridge()
        ..updateAvailable = true
        ..marketOffline = true;
      final controller = ExtensionCatalogController.of(
        GeneratedCoreProxyClients(bridge),
      );
      await expectLater(controller.checkUpdates(), throwsStateError);
      expect(controller.updatableEntries().single.id, 'PackageBuilder');
      expect(controller.checkingUpdates, isFalse);
    },
  );

  test('updating a builtin skill uses its stable skill import route', () async {
    final bridge = _SkillUpdateBridge()..updateAvailable = true;
    final controller = ExtensionCatalogController.of(
      GeneratedCoreProxyClients(bridge),
    );
    await controller.checkUpdates();
    await controller.updateAll();
    expect(bridge.importedName, 'PackageBuilder');
    expect(bridge.importTarget, 'core/application.skillRepository');
    expect(controller.updatableEntries(), isEmpty);
    expect(
      controller.findEntry('skill', 'PackageBuilder')!.displayName,
      'Operit 插件开发套件',
    );
  });
}

class _SkillUpdateBridge extends OperitRuntimeBridge {
  bool updateAvailable = false;
  bool marketOffline = false;
  bool updated = false;
  String? importedName;
  String? importTarget;
  final List<String> calls = [];

  ExtensionCatalogEntry get skill => ExtensionCatalogEntry(
    id: 'PackageBuilder',
    kind: 'skill',
    source: 'builtin',
    displayName: updated ? 'Operit 插件开发套件' : 'PackageBuilder',
    description: 'Plugin development skill',
    version: '',
    authors: const [],
    category: 'Skill',
    scope: 'device',
    activationState: 'aiVisible',
    runtimeState: 'ready',
    updateState: updateAvailable ? 'available' : 'current',
    isCustomized: false,
    marketEntryId: null,
    installedMarketVersionId: null,
    logoResourceKey: null,
    logoMimeType: null,
    toolCount: 0,
    subpackageCount: 0,
    uiModuleCount: 0,
    hasUi: false,
    chatSidebarEntryIds: const [],
    dependencyIssueCount: 0,
  );

  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    calls.add(request.methodName);
    final Object? result;
    switch (request.methodName) {
      case 'getExtensionCatalogSnapshot':
        result = ExtensionCatalogSnapshot(
          revision: updated ? 2 : 1,
          installed: [skill],
          bundledAvailable: const [],
          diagnostics: const [],
        ).toJson();
      case 'checkExtensionCatalogUpdates':
        if (marketOffline) throw StateError('Marketplace offline');
        result = <Object?>[];
      case 'importBundledExternalSkill':
        importedName = (request.args as Map)['skillName'] as String;
        importTarget = request.target;
        updated = true;
        updateAvailable = false;
        result = const SkillPackage(
          name: 'PackageBuilder',
          description: 'Plugin development skill',
          directory:
              '/operitapp/data/runtime/extensions/device/skills/PackageBuilder',
          skillFile:
              '/operitapp/data/runtime/extensions/device/skills/PackageBuilder/SKILL.md',
        ).toJson();
      default:
        throw UnsupportedError('Unexpected Core call: ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream.empty();
}
