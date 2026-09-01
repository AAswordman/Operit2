import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/l10n/generated/app_localizations_zh.dart';
import 'package:operit2/ui/features/chat/components/style/input/common/InputProcessingStatusLane.dart';

void main() {
  test('maps summary processing keys to the Kotlin-era Chinese text', () {
    final l10n = AppLocalizationsZh();

    expect(
      inputProcessingStatusText(
        l10n,
        core_proxy.InputProcessingState.summarizing(
          message: 'chat_compressing_history',
        ),
      ),
      '正在压缩历史记录...',
    );
    expect(
      inputProcessingStatusText(
        l10n,
        core_proxy.InputProcessingState.summarizing(
          message: 'message_summarizing',
        ),
      ),
      '正在总结记忆...',
    );
  });

  testWidgets('reserves the same status lane height when status is hidden', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: InputProcessingStatusLane(
            visible: false,
            status: '',
            textStyle: null,
          ),
        ),
      ),
    );
    final hiddenSize = tester.getSize(find.byType(InputProcessingStatusLane));

    await tester.pumpWidget(
      const MaterialApp(
        home: Scaffold(
          body: InputProcessingStatusLane(
            visible: true,
            status: 'Receiving response',
            textStyle: null,
          ),
        ),
      ),
    );
    final visibleSize = tester.getSize(find.byType(InputProcessingStatusLane));

    expect(hiddenSize.height, inputProcessingStatusLaneHeight);
    expect(visibleSize, hiddenSize);
    expect(find.text('Receiving response'), findsOneWidget);
  });
}
