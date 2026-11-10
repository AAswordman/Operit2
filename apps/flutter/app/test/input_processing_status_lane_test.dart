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

  test('summary stages expose localized labels and increasing estimates', () {
    final l10n = AppLocalizationsZh();
    expect(
      inputProcessingStatusText(
        l10n,
        core_proxy.InputProcessingState.summarizing(
          message: 'summary_key_files',
        ),
      ),
      '正在整理关键文件与代码...',
    );
    expect(
      summaryStageProgress('summary_key_files'),
      greaterThan(summaryStageProgress('summary_core_task')),
    );
    expect(
      summaryStageProgress('summary_formatting'),
      greaterThan(summaryStageProgress('summary_next_steps')),
    );
  });

  testWidgets(
    'summary activity follows live state and stops before the next reply',
    (tester) async {
      final l10n = AppLocalizationsZh();
      Future<void> show(
        core_proxy.InputProcessingState state, {
        bool visible = true,
      }) => tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: 200,
              child: InputProcessingStatusLane(
                visible: visible,
                status: inputProcessingStatusText(l10n, state),
                textStyle: null,
                isSummarizing: state.tag == 'Summarizing',
              ),
            ),
          ),
        ),
      );
      final indicator = find.byKey(
        const ValueKey('summary-processing-indicator'),
      );
      await show(
        core_proxy.InputProcessingState.summarizing(
          message: 'chat_compressing_history',
        ),
      );
      expect(indicator, findsOneWidget);
      expect(find.text('正在压缩历史记录...'), findsOneWidget);
      await tester.pump(const Duration(seconds: 3));
      expect(indicator, findsOneWidget);

      await show(
        core_proxy.InputProcessingState.summarizing(
          message: 'summary_key_files',
        ),
      );
      expect(indicator, findsOneWidget);
      expect(find.text('正在整理关键文件与代码...'), findsOneWidget);

      await show(core_proxy.InputProcessingState.idle(), visible: false);
      expect(indicator, findsNothing);
      await show(
        core_proxy.InputProcessingState.receiving(
          message: 'enhanced_receiving_response',
        ),
      );
      expect(indicator, findsNothing);
      await tester.pump(const Duration(seconds: 25));
      expect(indicator, findsNothing);

      // A later real summary starts activity again; hidden status stays hidden.
      await show(
        core_proxy.InputProcessingState.summarizing(
          message: 'message_summarizing',
        ),
      );
      expect(indicator, findsOneWidget);
      await show(
        core_proxy.InputProcessingState.summarizing(
          message: 'message_summarizing',
        ),
        visible: false,
      );
      expect(indicator, findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

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
