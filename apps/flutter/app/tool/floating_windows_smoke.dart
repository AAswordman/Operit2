// Run: flutter run -d macos -t tool/floating_windows_smoke.dart
// Optionally pass --dart-define=OPERIT_DAFEIYU_SPRITE=/absolute/path/idle.webp.
// Full display geometry: --dart-define=OPERIT_FLOATING_SMOKE_FILL=true.
// Native target geometry: --dart-define=OPERIT_FLOATING_SMOKE_REGION=true.
import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:floating_windows/floating_windows.dart';
import 'package:operit2/ui/features/packages/screens/compose_dsl/glow_border.dart';

const _fill = bool.fromEnvironment('OPERIT_FLOATING_SMOKE_FILL');
const _region = bool.fromEnvironment('OPERIT_FLOATING_SMOKE_REGION');

Future<void> main(List<String> arguments) async {
  WidgetsFlutterBinding.ensureInitialized();
  await FloatingWindows.run(
    arguments: arguments,
    windowBuilder: (launch) => MaterialApp(
      color: Colors.transparent,
      home: Material(
        type: MaterialType.transparency,
        child: _fill || _region
            ? LayoutBuilder(
                builder: (context, constraints) {
                  final expected = Size(
                    launch.payload['width'] as double,
                    launch.payload['height'] as double,
                  );
                  if (constraints.biggest == expected) {
                    debugPrint(
                      'FLOATING_WINDOWS_VIEWPORT_PASSED: ${constraints.biggest} DPR=${View.of(context).devicePixelRatio}',
                    );
                  }
                  return const ComposeGlowBorder(
                    colors: [
                      Color(0xFF67F5E2),
                      Color(0xFF62ABFF),
                      Color(0xFFA791FF),
                      Color(0xFFF094D8),
                    ],
                  );
                },
              )
            : launch.payload['sprite'] is String
            ? Image.file(
                File(launch.payload['sprite'] as String),
                fit: BoxFit.contain,
              )
            : const Center(child: Text('🐋', style: TextStyle(fontSize: 90))),
      ),
    ),
    application: () async {
      runApp(
        const MaterialApp(
          home: Scaffold(
            body: Center(child: Text('Floating window smoke test')),
          ),
        ),
      );
      await WidgetsBinding.instance.endOfFrame;
      try {
        final caps = await FloatingWindows.capabilities();
        if (caps['scope'] != 'system' || caps['permissionGranted'] != true) {
          throw StateError(
            'A system floating-window host with permission is required: $caps',
          );
        }
        if (_fill || _region) {
          final display =
              WidgetsBinding.instance.platformDispatcher.displays.first;
          final expected = _region
              ? const Size(800, 600)
              : display.size / display.devicePixelRatio;
          final window = await FloatingWindows.open(
            payload: {'width': expected.width, 'height': expected.height},
            fill: !_region,
            options: FloatingWindowOptions(
              clickThrough: true,
              screenBounds: _region
                  ? const FloatingScreenBounds(100, 120, 900, 720)
                  : null,
            ),
          );
          try {
            final bounds = await window.getBounds();
            if ((bounds.width! - expected.width).abs() > 1 ||
                (bounds.height! - expected.height).abs() > 1 ||
                bounds.x != (_region ? 100 : 0) ||
                bounds.y != (_region ? 120 : 0)) {
              throw StateError(
                'Fill bounds ${bounds.width} × ${bounds.height} at ${bounds.x},${bounds.y} differ from $expected',
              );
            }
            if (_region) {
              await window.update(
                const FloatingWindowOptions(
                  screenBounds: FloatingScreenBounds(220, 180, 860, 580),
                ),
                payload: {'width': 640.0, 'height': 400.0},
              );
              await Future<void>.delayed(const Duration(milliseconds: 150));
              final moved = await window.getBounds();
              if (moved.x != 220 ||
                  moved.y != 180 ||
                  moved.width != 640 ||
                  moved.height != 400) {
                throw StateError(
                  'Target frame failed to follow movement and resize',
                );
              }
              debugPrint(
                'FLOATING_WINDOWS_REGION_PASSED: 800 × 600 at 100,120 -> 640 × 400 at 220,180',
              );
            }
            await window.hide();
            await window.show();
            await Future<void>.delayed(const Duration(seconds: 3));
            debugPrint(
              'FLOATING_WINDOWS_GEOMETRY_PASSED: ${bounds.width} × ${bounds.height} at ${bounds.x},${bounds.y} DPR=${display.devicePixelRatio}',
            );
          } finally {
            await window.close();
          }
          final deadline = DateTime.now().add(const Duration(seconds: 10));
          while ((await FloatingWindows.liveIds()).contains(window.id)) {
            if (DateTime.now().isAfter(deadline)) {
              throw StateError('Fill engine was not released');
            }
            await Future<void>.delayed(const Duration(milliseconds: 100));
          }
          debugPrint('FLOATING_WINDOWS_SMOKE_PASSED');
          return;
        }
        const sprite = String.fromEnvironment('OPERIT_DAFEIYU_SPRITE');
        final window = await FloatingWindows.open(
          payload: {if (sprite.isNotEmpty) 'sprite': sprite},
          options: const FloatingWindowOptions(
            x: 160,
            y: 160,
            width: 208,
            height: 270,
          ),
        );
        if (!(await FloatingWindows.liveIds()).contains(window.id)) {
          throw StateError('Child engine was not registered');
        }
        await window.update(
          const FloatingWindowOptions(
            x: 240,
            y: 180,
            width: 220,
            clickThrough: true,
          ),
        );
        await window.hide();
        if (!(await FloatingWindows.liveIds()).contains(window.id)) {
          throw StateError('Hide destroyed the child');
        }
        await window.show();
        await window.update(const FloatingWindowOptions(clickThrough: false));
        await Future<void>.delayed(const Duration(seconds: 3));
        await window.close();
        final deadline = DateTime.now().add(const Duration(seconds: 10));
        while ((await FloatingWindows.liveIds()).contains(window.id)) {
          if (DateTime.now().isAfter(deadline)) {
            throw StateError('Child engine was not released');
          }
          await Future<void>.delayed(const Duration(milliseconds: 100));
        }
        debugPrint('FLOATING_WINDOWS_SMOKE_PASSED');
      } catch (error, stack) {
        debugPrint('FLOATING_WINDOWS_SMOKE_FAILED: $error\n$stack');
      } finally {
        await SystemNavigator.pop();
      }
    },
  );
}
