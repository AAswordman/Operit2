// ignore_for_file: file_names

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/ui/features/settings/model/ModelConnectionTestCapabilities.dart';

typedef _TestItem =
    core_proxy.CoreOperitProvidersChatLlmproviderModelConfigConnectionTesterModelConnectionTestItem;

void main() {
  const current = core_proxy.ModelCapabilities(
    directImage: true,
    directAudio: false,
    directVideo: false,
    toolCall: true,
  );

  core_proxy.ModelConnectionTestReport reportFor(List<_TestItem> items) {
    return core_proxy.ModelConnectionTestReport(
      providerId: 'provider',
      modelId: 'model',
      providerName: 'Provider',
      providerType: 'OPENAI',
      success: items.every((item) => item.success),
      items: items,
    );
  }

  _TestItem item(
    core_proxy.ModelConnectionTestType type, {
    required bool success,
  }) {
    return _TestItem(
      type: type,
      success: success,
      error: success ? null : 'failed',
    );
  }

  test(
    'chat-only report keeps the currently selected tool and image flags',
    () {
      final next = capabilitiesFromConnectionTest(
        reportFor(<_TestItem>[
          item(core_proxy.ModelConnectionTestType.chat, success: true),
        ]),
        current,
      );

      expect(next.toolCall, isTrue);
      expect(next.directImage, isTrue);
      expect(next.directAudio, isFalse);
      expect(next.directVideo, isFalse);
    },
  );

  test('failed capability checks turn the matching flags off', () {
    final next = capabilitiesFromConnectionTest(
      reportFor(<_TestItem>[
        item(core_proxy.ModelConnectionTestType.chat, success: true),
        item(core_proxy.ModelConnectionTestType.toolCall, success: false),
        item(core_proxy.ModelConnectionTestType.image, success: true),
      ]),
      current,
    );

    expect(next.toolCall, isFalse);
    expect(next.directImage, isTrue);
  });

  for (final (error, restriction) in [
    (
      'request failed: 402 Payment Required: insufficient credits',
      ModelConnectionTestRestriction.credits,
    ),
    (
      'request failed: 402 Payment Required (out of its allowance: not asked again before 00:37 UTC+8)',
      ModelConnectionTestRestriction.credits,
    ),
    (
      'request failed: 429 Too Many Requests: cooldown',
      ModelConnectionTestRestriction.rateLimit,
    ),
    ('HTTP 429: rate limit', ModelConnectionTestRestriction.rateLimit),
  ]) {
    test('restricted checks keep capability selections: $error', () {
      final report = reportFor([
        item(core_proxy.ModelConnectionTestType.chat, success: true),
        _TestItem(
          type: core_proxy.ModelConnectionTestType.image,
          success: false,
          error: error,
        ),
        _TestItem(
          type: core_proxy.ModelConnectionTestType.toolCall,
          success: false,
          error: error,
        ),
      ]);
      final next = capabilitiesFromConnectionTest(report, current);
      expect(connectionTestRestriction(error), restriction);
      expect(connectionTestHasRestrictions(report), isTrue);
      expect(connectionTestHasFailures(report), isFalse);
      expect(next.directImage, isTrue);
      expect(next.toolCall, isTrue);
    });
  }

  test('completed checks still apply alongside restricted checks', () {
    final report = reportFor([
      item(core_proxy.ModelConnectionTestType.chat, success: true),
      _TestItem(
        type: core_proxy.ModelConnectionTestType.image,
        success: false,
        error: 'request failed: 402 Payment Required',
      ),
      item(core_proxy.ModelConnectionTestType.toolCall, success: false),
      item(core_proxy.ModelConnectionTestType.audio, success: true),
    ]);
    final next = capabilitiesFromConnectionTest(report, current);
    expect(connectionTestHasRestrictions(report), isTrue);
    expect(connectionTestHasFailures(report), isTrue);
    expect(next.directImage, isTrue);
    expect(next.toolCall, isFalse);
    expect(next.directAudio, isTrue);
  });

  test('numbers inside an unrelated error do not establish a restriction', () {
    expect(
      connectionTestRestriction('request failed: 400 Bad Request: 402 tokens'),
      isNull,
    );
    expect(
      connectionTestRestriction('request failed: 500 Internal Server Error'),
      isNull,
    );
  });
}
