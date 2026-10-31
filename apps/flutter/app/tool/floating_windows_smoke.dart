// Run: flutter run -d macos -t tool/floating_windows_smoke.dart
// Optionally pass --dart-define=OPERIT_DAFEIYU_SPRITE=/absolute/path/idle.webp.
import 'dart:async';
import 'dart:io';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:floating_windows/floating_windows.dart';

Future<void> main(List<String> arguments) async {
  WidgetsFlutterBinding.ensureInitialized();
  await FloatingWindows.run(
    arguments: arguments,
    windowBuilder: (launch) => MaterialApp(
      color: Colors.transparent,
      home: Material(
        type: MaterialType.transparency,
        child: launch.payload['sprite'] is String
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
