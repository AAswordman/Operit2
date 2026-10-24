import 'dart:async';

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/logging/ClientLogger.dart';

/// Captures actual console output while preserving the logger backend writes.
List<String> captureConsole(void Function() action) {
  final lines = <String>[];
  runZoned(
    action,
    zoneSpecification: ZoneSpecification(
      print: (self, parent, zone, line) => lines.add(line),
    ),
  );
  return lines;
}

/// Verifies severity filtering and cumulative Verbose depths against real writes.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  setUpAll(ClientLogger.initialize);

  setUp(() {
    final minimum = ClientLogger.consoleMinimumLevel;
    final maximum = ClientLogger.consoleMaximumVerboseLevel;
    addTearDown(() {
      ClientLogger.setConsoleMinimumLevel(minimum);
      ClientLogger.setConsoleMaximumVerboseLevel(maximum);
    });
  });

  test('Info hides all diagnostic depths and Debug', () {
    ClientLogger.setConsoleMinimumLevel(ClientLogLevel.info);
    ClientLogger.setConsoleMaximumVerboseLevel(VerboseLevel.level6);
    final lines = captureConsole(() {
      for (final level in VerboseLevel.values) {
        ClientLogger.v(
          'v${level.value}',
          verboseLevel: level,
          tag: 'LevelPolicy',
        );
      }
      ClientLogger.d('debug', tag: 'LevelPolicy');
      ClientLogger.i('info', tag: 'LevelPolicy');
      ClientLogger.w('warn', tag: 'LevelPolicy');
      ClientLogger.e('error', tag: 'LevelPolicy');
    });
    expect(lines.map((line) => line.split(': ').last), [
      'info',
      'warn',
      'error',
    ]);
  });

  test('each maximum includes lower depths without filtering severity', () {
    ClientLogger.setConsoleMinimumLevel(ClientLogLevel.verbose);
    for (final maximum in VerboseLevel.values) {
      ClientLogger.setConsoleMaximumVerboseLevel(maximum);
      final lines = captureConsole(() {
        for (final level in VerboseLevel.values) {
          ClientLogger.v(
            'v${level.value}',
            verboseLevel: level,
            tag: 'LevelPolicy',
          );
        }
        ClientLogger.d('debug', tag: 'LevelPolicy');
        ClientLogger.i('info', tag: 'LevelPolicy');
        ClientLogger.w('warn', tag: 'LevelPolicy');
        ClientLogger.e('error', tag: 'LevelPolicy');
      });
      expect(lines.map((line) => line.split(': ').last), [
        for (var level = 1; level <= maximum.value; level++) 'v$level',
        'debug',
        'info',
        'warn',
        'error',
      ]);
      for (var index = 0; index < maximum.value; index++) {
        expect(
          lines[index],
          matches(' V${index + 1}/LevelPolicy: v${index + 1}\$'),
        );
      }
    }
  });

  test('console depth preserves detailed diagnostic history', () async {
    ClientLogger.setConsoleMinimumLevel(ClientLogLevel.info);
    final lines = captureConsole(() {
      ClientLogger.v(
        'retained-v6-policy',
        verboseLevel: VerboseLevel.level6,
        tag: 'LevelPolicy',
      );
    });
    expect(lines, isEmpty);
    expect(
      await ClientLogger.readText(),
      contains('V6/LevelPolicy: retained-v6-policy'),
    );
  });
}
