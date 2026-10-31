// Run: flutter run -d macos -t tool/dafeiyu_plugin_smoke.dart --no-pub
// Uses a fresh runtime under the system temp directory; never imports into the
// user's configured runtime. Success requires both owner and pet markers.
// Add --dart-define=OPERIT_FLOATING_SMOKE_APPLICATION=true to exercise the
// application renderer on the same desktop host.
import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:floating_windows/floating_windows.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/host/RuntimeFloatingWindowController.dart';
import 'package:operit2/core/host/RuntimeHostInteractionSubscriber.dart';
import 'package:operit2/core/logging/ClientLogger.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/packages/screens/ToolPkgUiLauncherScreen.dart';
import 'package:operit2/ui/window/FloatingWindowApp.dart';

const _package = 'com.operit.deepseek_dafeiyu';
const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
final _root = Directory(
  '${Directory.systemTemp.path}/operit-dafeiyu-smoke-$pid',
);
final _capture = GlobalKey();
final _petCapture = GlobalKey();
const _applicationScope = bool.fromEnvironment(
  'OPERIT_FLOATING_SMOKE_APPLICATION',
);
var _petProbeScheduled = false;

Widget _buildPet(FloatingWindowLaunch launch) {
  if (!_petProbeScheduled) {
    _petProbeScheduled = true;
    WidgetsBinding.instance.addPostFrameCallback((_) => unawaited(_checkPet()));
  }
  return RepaintBoundary(
    key: _petCapture,
    child: FloatingWindowApp(launch: launch),
  );
}

Iterable<Element> _elements([Element? parent]) sync* {
  final element = parent ?? WidgetsBinding.instance.rootElement;
  if (element == null) return;
  yield element;
  final children = <Element>[];
  element.visitChildren(children.add);
  for (final child in children) {
    yield* _elements(child);
  }
}

bool _hasText(String text) => _elements().any(
  (element) =>
      element.widget is Text &&
      (element.widget as Text).data?.contains(text) == true,
);

Future<void> _until(String description, FutureOr<bool> Function() ready) async {
  final deadline = DateTime.now().add(const Duration(seconds: 45));
  while (!await ready()) {
    if (DateTime.now().isAfter(deadline)) {
      final text = _elements()
          .map((element) => element.widget)
          .whereType<Text>()
          .map((widget) => widget.data)
          .toList();
      throw StateError('Timed out: $description; rendered text: $text');
    }
    await Future<void>.delayed(const Duration(milliseconds: 100));
  }
}

Future<void> _press(String label) async {
  ButtonStyleButton? find() {
    for (final element in _elements()) {
      final widget = element.widget;
      if (widget is! ButtonStyleButton || widget.onPressed == null) continue;
      if (_elements(element).any(
        (child) => child.widget is Text && (child.widget as Text).data == label,
      )) {
        return widget;
      }
    }
    return null;
  }

  if (find() == null) {
    for (final element in _elements().toList()) {
      if (element is StatefulElement && element.state is ScrollableState) {
        final position = (element.state as ScrollableState).position;
        if (position.hasContentDimensions) {
          position.jumpTo(position.maxScrollExtent);
        }
      }
    }
  }
  await _until('enabled button $label', () => find() != null);
  find()!.onPressed!();
}

Future<void> _screenshot(String name, {GlobalKey? capture}) async {
  await WidgetsBinding.instance.endOfFrame.timeout(const Duration(seconds: 3));
  final boundary =
      (capture ?? _capture).currentContext!.findRenderObject()!
          as RenderRepaintBoundary;
  final image = await boundary.toImage();
  try {
    final bytes = await image.toByteData(format: ui.ImageByteFormat.png);
    await File(
      '${_root.path}/$name.png',
    ).writeAsBytes(bytes!.buffer.asUint8List());
  } finally {
    image.dispose();
  }
}

Future<List<dynamic>> _windows() async =>
    (await RuntimeFloatingWindowController.command({
          'packageName': _package,
          'operation': 'list',
        }))!
        as List;

Future<void> _checkPet() async {
  try {
    await _until(
      'decoded pet image',
      () =>
          _petCapture.currentContext != null &&
          _elements(_petCapture.currentContext! as Element).any(
            (element) =>
                element.widget is RawImage &&
                (element.widget as RawImage).image != null,
          ),
    );
    final gestures = _elements(_petCapture.currentContext! as Element)
        .map((element) => element.widget)
        .whereType<GestureDetector>()
        .where((widget) => widget.onDoubleTap != null);
    if (gestures.isEmpty) {
      throw StateError('Pet interaction handler was not mounted');
    }
    gestures.first.onDoubleTap!();
    await _until('feeding state from JavaScript', () => _hasText('小鱼干！再来一份！'));
    await _screenshot('pet-feeding', capture: _petCapture);
    await File(
      '${_root.path}/pet-passed',
    ).writeAsString('decoded image; feeding action; rerender');
    debugPrint('DAFEIYU_PET_SMOKE_PASSED');
  } catch (error, stack) {
    await File('${_root.path}/pet-failed').writeAsString('$error\n$stack');
    debugPrint('DAFEIYU_PET_SMOKE_FAILED: $error\n$stack');
  }
}

Future<void> main(List<String> arguments) async {
  WidgetsFlutterBinding.ensureInitialized();
  if (_applicationScope) {
    // Exercise the common application renderer on a desktop host. This is not
    // an iOS/OHOS device test; it validates the same plugin/Rust/session path.
    debugDefaultTargetPlatformOverride = TargetPlatform.iOS;
  }
  final child = FloatingWindows.parseLaunch(arguments) != null;
  if (!child) {
    await _root.create(recursive: true);
    await platformCoreProxy.setRuntimeStorageRoots(
      '${_root.path}/runtime',
      '${_root.path}/workspace',
    );
  }
  await ClientLogger.initialize();
  await FloatingWindows.run(
    arguments: arguments,
    windowBuilder: _buildPet,
    application: () async {
      final page = ValueNotifier<Widget>(
        const Center(child: CircularProgressIndicator()),
      );
      final session = ToolPkgUiSessionController();
      runApp(
        RepaintBoundary(
          key: _capture,
          child: FloatingWindowsHost(
            windowBuilder: _buildPet,
            child: MaterialApp(
              locale: const Locale('zh'),
              localizationsDelegates: AppLocalizations.localizationsDelegates,
              supportedLocales: AppLocalizations.supportedLocales,
              home: Scaffold(
                body: ValueListenableBuilder<Widget>(
                  valueListenable: page,
                  builder: (context, value, _) => value,
                ),
              ),
            ),
          ),
        ),
      );
      await WidgetsBinding.instance.endOfFrame;
      RuntimeHostInteractionSubscriber.install();
      try {
        final caps = await FloatingWindows.capabilities();
        final expectedScope = _applicationScope ? 'application' : 'system';
        if (caps['scope'] != expectedScope) {
          throw StateError('Expected $expectedScope presentation: $caps');
        }
        final baselineWindows = await FloatingWindows.liveIds();
        final manager = _clients.application.packageManager();
        await manager.loadAvailablePackages();
        debugPrint(
          await manager.importBundledExternalToolPkgContainer(
            containerPackageName: _package,
          ),
        );
        debugPrint(
          await manager.enableToolPkgContainer(containerPackageName: _package),
        );
        final enabled = await manager.getEnabledToolPkgContainerRuntimes();
        final plugin = enabled.singleWhere(
          (plugin) => plugin.packageName == _package,
        );
        page.value = ToolPkgUiLauncherScreen(
          clients: _clients,
          plugin: plugin,
          initialRouteId: 'toolpkg:$_package:ui:settings',
          showLauncherChrome: false,
          sessionController: session,
        );
        await _until(
          'settings initialized through plugin host',
          () => _hasText('已就绪'),
        );
        await _until(
          'decoded settings preview',
          () => _elements().any(
            (element) =>
                element.widget is RawImage &&
                (element.widget as RawImage).image != null,
          ),
        );
        await _screenshot('settings');
        await _press('保存并打开 / 应用设置');
        await _until(
          'window opened through JavaScript API',
          () async => (await _windows()).length == 1,
        );
        await _until('pet decoded and interacted with plugin', () async {
          final failure = File('${_root.path}/pet-failed');
          if (await failure.exists()) {
            throw StateError(await failure.readAsString());
          }
          return File('${_root.path}/pet-passed').exists();
        });
        await _press('隐藏');
        await _until('hidden window retained', () async {
          final windows = await _windows();
          return windows.length == 1 &&
              (windows.single as Map)['visible'] == false;
        });
        await _press('显示');
        await _until(
          'existing window shown',
          () async => ((await _windows()).single as Map)['visible'] == true,
        );
        await _press('关闭');
        await _until(
          'window registry released',
          () async => (await _windows()).isEmpty,
        );
        await _until(
          'window resources released',
          () async => (await FloatingWindows.liveIds())
              .difference(baselineWindows)
              .isEmpty,
        );
        debugPrint(
          'DAFEIYU_PLUGIN_SMOKE_PASSED scope=$expectedScope root=${_root.path}',
        );
      } catch (error, stack) {
        debugPrint(
          'DAFEIYU_PLUGIN_SMOKE_FAILED: $error\n$stack root=${_root.path}',
        );
      } finally {
        await session.close();
        await RuntimeHostInteractionSubscriber.uninstall();
        await SystemNavigator.pop();
      }
    },
  );
}
