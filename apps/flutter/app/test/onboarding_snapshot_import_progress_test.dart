import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/ui/features/onboarding/OnboardingStartupRoute.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

void main() {
  testWidgets('upload progress is visible before the preview is available', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      reading: true,
      progress: _progress('upload', '上传快照', 0.42),
    );

    expect(find.text('上传快照'), findsOneWidget);
    expect(find.text('42%'), findsOneWidget);
    expect(find.text('snapshot.zip'), findsOneWidget);
    expect(find.text('检测到可迁移内容'), findsNothing);
    expect(_indicator(tester).value, 0.42);
    expect(_pickButton(tester).onPressed, isNull);
    expect(tester.takeException(), isNull);
  });

  testWidgets('upload progress updates as more chunks are sent', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      reading: true,
      progress: _progress('upload', '上传快照', 0),
    );
    expect(_indicator(tester).value, 0);
    expect(find.text('0%'), findsOneWidget);

    await _pumpImportPage(
      tester,
      reading: true,
      progress: _progress('upload', '上传快照', 0.85),
    );
    expect(_indicator(tester).value, 0.85);
    expect(find.text('85%'), findsOneWidget);
  });

  testWidgets('inspection shows activity rather than an invented percentage', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      reading: true,
      progress: _progress('inspect', '检查快照', 0),
    );

    expect(find.text('检查快照'), findsOneWidget);
    expect(_indicator(tester).value, isNull);
    expect(find.text('0%'), findsNothing);
  });

  testWidgets('import shows an immediate state before the first native event', (
    tester,
  ) async {
    await _pumpImportPage(tester, importing: true, snapshot: _preview);

    expect(find.text('准备导入'), findsOneWidget);
    expect(_indicator(tester).value, isNull);
    expect(_pickButton(tester).onPressed, isNull);
  });

  testWidgets('native migration progress appears above the snapshot metrics', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      importing: true,
      snapshot: _preview,
      progress: _progress('chats', '迁移聊天', 0.64),
    );

    expect(find.text('迁移聊天'), findsOneWidget);
    expect(find.text('64%'), findsOneWidget);
    expect(_indicator(tester).value, 0.64);
    expect(
      tester.getTopLeft(find.byType(LinearProgressIndicator)).dy,
      lessThan(tester.getTopLeft(find.text('检测到可迁移内容')).dy),
    );
  });

  testWidgets('idle preview hides stale progress and enables file selection', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      snapshot: _preview,
      progress: _progress('upload', '上传快照', 1),
    );

    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.text('检测到可迁移内容'), findsOneWidget);
    expect(_pickButton(tester).onPressed, isNotNull);
  });

  testWidgets('failure hides the busy indicator and allows another selection', (
    tester,
  ) async {
    await _pumpImportPage(
      tester,
      progress: _progress('inspect', '检查快照', 0),
      errorText: 'Invalid snapshot',
    );

    expect(find.byType(LinearProgressIndicator), findsNothing);
    expect(find.text('快照导入失败\nInvalid snapshot'), findsOneWidget);
    expect(_pickButton(tester).onPressed, isNotNull);
  });
}

Future<void> _pumpImportPage(
  WidgetTester tester, {
  bool reading = false,
  bool importing = false,
  core.Operit1SnapshotPreview? snapshot,
  core.Operit1SnapshotImportProgress? progress,
  String? errorText,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      home: Scaffold(
        body: OnboardingSnapshotImportPage(
          snapshot: snapshot,
          fileName: 'snapshot.zip',
          reading: reading,
          importing: importing,
          progress: progress,
          onPickSnapshot: () {},
          errorText: errorText,
        ),
      ),
    ),
  );
  // Indeterminate progress intentionally keeps animating, so do not settle.
  await tester.pump(const Duration(milliseconds: 200));
}

LinearProgressIndicator _indicator(WidgetTester tester) => tester
    .widget<LinearProgressIndicator>(find.byType(LinearProgressIndicator));

FilledButton _pickButton(WidgetTester tester) =>
    tester.widget<FilledButton>(find.byType(FilledButton));

core.Operit1SnapshotImportProgress _progress(
  String stage,
  String title,
  double value,
) => core.Operit1SnapshotImportProgress(
  stage: stage,
  title: title,
  detail: 'Snapshot progress details',
  progress: value,
  active: true,
);

const _preview = core.Operit1SnapshotPreview(
  formatVersion: 1,
  packageName: 'operit1',
  createdAt: 0,
  modelConfig: core.Operit1ModelConfigSnapshotPreview(
    formatVersion: 1,
    packageName: 'operit1',
    createdAt: 0,
    configs: [],
    chatConfigId: null,
    chatModelId: null,
    chatModelIndex: null,
  ),
  datastoreFiles: [],
  chatCount: 2,
  messageCount: 10,
  importedFileCount: 3,
  importedExternalFileCount: 1,
  detectedDomains: ['chat'],
);
