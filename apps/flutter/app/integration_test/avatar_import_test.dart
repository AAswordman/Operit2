import 'dart:async';
import 'dart:io';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:integration_test/integration_test.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:path_provider/path_provider.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';

/// Uses the real plugin UI, native engine and storage, controlling only the external picker.
void main() {
  final binding = IntegrationTestWidgetsFlutterBinding.ensureInitialized();
  binding.framePolicy = LiveTestWidgetsFlutterBindingFramePolicy.fullyLive;
  testWidgets('avatar remains in plugin storage after its source is deleted', (
    tester,
  ) async {
    final support = await getApplicationSupportDirectory();
    final parent = await Directory(
      '${support.path}/avatar-import-integration',
    ).create(recursive: true);
    final directory = await parent.createTemp('run-');
    final runtime = '${directory.path}/runtime';
    await platformCoreProxy.setRuntimeStorageRoots(
      runtime,
      '${directory.path}/workspace',
    );
    await ClientLogger.initialize();
    await LiquidGlassWidgets.initialize();
    RuntimeHostInteractionSubscriber.install();
    addTearDown(RuntimeHostInteractionSubscriber.uninstall);
    final image = await rootBundle.load('assets/images/operit_avatar.png');
    final bytes = image.buffer.asUint8List(
      image.offsetInBytes,
      image.lengthInBytes,
    );
    final source = await File(
      '${directory.path}/selected-avatar.png',
    ).writeAsBytes(bytes);
    final original = FileSelectorPlatform.instance;
    final picker = _AvatarSelector(
      XFile(source.path, name: 'avatar.png', mimeType: 'image/png'),
    );
    FileSelectorPlatform.instance = picker;
    addTearDown(() => FileSelectorPlatform.instance = original);
    const clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
    final manager = clients.application.packageManager();
    const package = 'com.operit.character_cards';
    await manager.getToolPkgPublicApiOwners(
      apiName: 'chat.configuration.resolve',
    );
    final card =
        await manager.invokeToolPkgPublicApi(
              packageName: package,
              methodName: 'character.create',
              payload: {
                'values': {'name': 'Native avatar import'},
              },
            )
            as Map;
    final plugin = await manager.getToolPkgContainerRuntime(
      containerPackageName: package,
    );
    expect(plugin, isNotNull);
    final routes = await manager.getToolPkgUiRoutes(
      runtime: 'compose_dsl',
      useEnglish: false,
    );
    final route = routes.firstWhere(
      (value) =>
          value.containerPackageName == package &&
          value.routeId == 'toolpkg:$package:ui:main',
    );
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ToolPkgUiLauncherScreen(
            clients: clients,
            plugin: plugin!,
            initialRouteId: route.routeId,
            embeddedScreenPath: route.screen,
            initialModuleSpec: route.moduleSpec,
            initialState: {
              'presentation': {
                'requestId': 'native-avatar-import',
                'input': {'mode': 'edit', 'entity': 'card', 'id': card['id']},
              },
            },
            showLauncherChrome: false,
          ),
        ),
      ),
    );
    Future<void> waitFor(bool Function() ready, String message) async {
      final watch = Stopwatch()..start();
      while (watch.elapsed < const Duration(seconds: 30)) {
        final error = tester.takeException();
        if (error != null) throw error;
        if (ready()) return;
        await Future<void>.delayed(const Duration(milliseconds: 50));
      }
      throw TimeoutException('$message\n${await ClientLogger.readText()}');
    }

    await waitFor(
      () => find.text('选择').evaluate().isNotEmpty,
      'Avatar editor did not load',
    );
    await tester.tap(find.text('选择').first);
    await waitFor(
      () => find.textContaining('/avatars/').evaluate().isNotEmpty,
      'Avatar import did not finish',
    );
    expect(picker.calls, 1);
    expect(picker.requestedImages, true);
    final imported = tester
        .widgetList<Text>(find.byType(Text))
        .map((value) => value.data)
        .whereType<String>()
        .firstWhere(
          (value) =>
              value.startsWith('/app/data/') && value.contains('/avatars/'),
        );
    expect(imported, isNot(source.path));
    final stored = File('$runtime/${imported.substring('/app/data/'.length)}');
    expect(await stored.readAsBytes(), bytes);
    await source.delete();
    expect(await source.exists(), false);
    await tester.tap(find.text('保存').first);
    Map? saved;
    final watch = Stopwatch()..start();
    while (watch.elapsed < const Duration(seconds: 30)) {
      final cards =
          await manager.invokeToolPkgPublicApi(
                packageName: package,
                methodName: 'character.list',
                payload: {},
              )
              as List;
      saved = cards.cast<Map>().firstWhere(
        (value) => value['id'] == card['id'],
      );
      if (saved['avatarUri'] == imported) break;
      await Future<void>.delayed(const Duration(milliseconds: 50));
    }
    expect(saved!['avatarUri'], imported);
    expect(await stored.readAsBytes(), bytes);
    binding.reportData = {
      'avatarImport': {
        'result': 'passed',
        'selectorCalls': picker.calls,
        'bytes': bytes.length,
        'sourceDeleted': true,
        'avatarUri': imported,
      },
    };
    // ignore: avoid_print
    print(
      'AVATAR_IMPORT_PASSED bytes=${bytes.length} sourceDeleted=true avatarUri=$imported',
    );
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

class _AvatarSelector extends FileSelectorPlatform {
  _AvatarSelector(this.file);
  final XFile file;
  int calls = 0;
  bool requestedImages = false;
  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    calls++;
    requestedImages =
        acceptedTypeGroups?.any(
          (group) => group.mimeTypes?.contains('image/*') ?? false,
        ) ??
        false;
    return file;
  }
}
