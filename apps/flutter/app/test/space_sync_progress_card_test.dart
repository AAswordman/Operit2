import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as generated;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/runtime/SpaceSyncProgressCard.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

void main() {
  generated.SpaceSyncProgress progress({
    generated.SpaceSyncStatus status = generated.SpaceSyncStatus.syncing,
    generated.SpaceSyncStage stage = generated.SpaceSyncStage.records,
    int fileBytes = 0,
    int fileSize = 0,
    String spaceId = 'space',
    int? updatedAt,
  }) {
    final now = DateTime.now().millisecondsSinceEpoch;
    return generated.SpaceSyncProgress(
      spaceId: spaceId,
      peerNodeId: 'tablet',
      startedAt: now - 10000,
      updatedAt: updatedAt ?? now,
      status: status,
      stage: stage,
      exchangedRecords: 1200,
      transferredBytes: 2097152,
      fileBytes: fileBytes,
      fileSize: fileSize,
      retryAt: status == generated.SpaceSyncStatus.waiting ? now + 2000 : null,
      error: status == generated.SpaceSyncStatus.waiting ? 'offline' : null,
    );
  }

  Future<void> mount(
    WidgetTester tester,
    List<generated.SpaceSyncProgress> items, {
    bool active = true,
  }) async {
    tester.binding.platformDispatcher.localesTestValue = const [Locale('zh')];
    addTearDown(tester.binding.platformDispatcher.clearLocalesTestValue);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot:
            UserPreferencesManager.defaultThemePreferenceSnapshot,
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: Scaffold(
          body: TickerMode(
            enabled: active,
            child: SpaceSyncProgressCard(
              progress: items,
              spaceId: 'space',
              deviceNames: const {'tablet': '平板'},
            ),
          ),
        ),
      ),
    );
    await tester.pump();
  }

  testWidgets('no card or progress animation without a large sync', (
    tester,
  ) async {
    await mount(tester, []);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.textContaining('同步'), findsNothing);
  });

  testWidgets(
    'records have no invented percentage and files use actual bytes',
    (tester) async {
      await mount(tester, [progress()]);
      expect(find.text('正在与 平板 同步'), findsOneWidget);
      expect(
        tester
            .widget<LinearProgressIndicator>(
              find.byType(LinearProgressIndicator),
            )
            .value,
        isNull,
      );
      expect(find.text('已交换 1200 条记录 · 已传输 2.0 MB'), findsOneWidget);

      await mount(tester, [
        progress(
          stage: generated.SpaceSyncStage.files,
          fileBytes: 1048576,
          fileSize: 4194304,
        ),
      ]);
      expect(
        tester
            .widget<LinearProgressIndicator>(
              find.byType(LinearProgressIndicator),
            )
            .value,
        0.25,
      );
      expect(find.text('当前文件：1.0 MB / 4.0 MB'), findsOneWidget);
    },
  );

  testWidgets('waiting stays visible without a running progress animation', (
    tester,
  ) async {
    await mount(tester, [progress(status: generated.SpaceSyncStatus.waiting)]);
    expect(find.text('同步暂时中断，稍后自动重试'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
    await tester.pump(const Duration(seconds: 30));
    expect(find.text('正在与 平板 同步'), findsOneWidget);
  });

  testWidgets('completion disappears and stale space progress is hidden', (
    tester,
  ) async {
    await mount(tester, [
      progress(status: generated.SpaceSyncStatus.completed),
    ]);
    expect(find.text('与 平板 同步完成'), findsOneWidget);
    await tester.pump(const Duration(seconds: 3));
    expect(find.text('与 平板 同步完成'), findsNothing);
    await mount(tester, [progress(spaceId: 'previous-space')]);
    expect(find.textContaining('同步'), findsNothing);
  });

  testWidgets('retained page cancels its dismissal timer while inactive', (
    tester,
  ) async {
    final item = progress(status: generated.SpaceSyncStatus.completed);
    await mount(tester, [item]);
    await mount(tester, [item], active: false);
    await tester.pump(const Duration(seconds: 3));
    // Returning to the page reads the current completion age.
    await mount(tester, [
      progress(
        status: generated.SpaceSyncStatus.completed,
        updatedAt: DateTime.now().millisecondsSinceEpoch - 4000,
      ),
    ]);
    expect(find.textContaining('同步完成'), findsNothing);
    expect(tester.takeException(), isNull);
  });
}
