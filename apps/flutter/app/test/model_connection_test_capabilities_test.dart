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
}
