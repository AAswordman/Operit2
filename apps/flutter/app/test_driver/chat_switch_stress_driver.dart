import 'dart:io';
import 'package:integration_test/integration_test_driver.dart';

/// Saves the actual profile application report outside the measured frame regions.
Future<void> main() => integrationDriver(
  timeout: const Duration(minutes: 30),
  writeResponseOnFailure: true,
  responseDataCallback: (data) => writeResponseData(
    data,
    destinationDirectory: Platform.environment['OPERIT_CHAT_STRESS_OUTPUT'],
    testOutputFilename: 'chat_switch_stress',
  ),
);
