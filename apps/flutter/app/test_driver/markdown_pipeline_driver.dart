import 'dart:io';

import 'package:integration_test/integration_test_driver.dart';

Future<void> main() => integrationDriver(
  timeout: const Duration(minutes: 10),
  writeResponseOnFailure: true,
  responseDataCallback: (data) => writeResponseData(
    data,
    destinationDirectory: Platform.environment['OPERIT_MARKDOWN_BENCH_OUTPUT'],
    testOutputFilename: 'markdown_pipeline',
  ),
);
