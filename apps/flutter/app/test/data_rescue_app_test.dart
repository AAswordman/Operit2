import 'dart:convert';
import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:file_selector_android/file_selector_android.dart';
import 'package:flutter/services.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/rescue/DataRescueApp.dart';

/// Verifies the independent launch contract and maintenance-only request sequence.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  test('matches only explicit maintenance entry contracts', () {
    expect(
      isDataRescueLaunch(
        arguments: const [],
        route: '/',
        uri: Uri.parse('https://operit.test/'),
      ),
      isFalse,
    );
    expect(
      isDataRescueLaunch(
        arguments: const ['--data-rescue'],
        route: '/',
        uri: Uri(),
      ),
      isTrue,
    );
    expect(
      isDataRescueLaunch(
        arguments: const [],
        route: '/data-rescue',
        uri: Uri(),
      ),
      isTrue,
    );
    expect(
      isDataRescueLaunch(
        arguments: const [],
        route: '/',
        uri: Uri.parse('https://operit.test/?mode=data-rescue'),
      ),
      isTrue,
    );
    expect(
      isDataRescueLaunch(
        arguments: const [],
        route: '/',
        uri: Uri(),
        hostRequested: true,
      ),
      isTrue,
    );
  });

  testWidgets('prepares host data without showing path or password inputs', (
    tester,
  ) async {
    final host = _MaintenanceHost();
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    expect(host.operations, ['open', 'configureAutomatic']);
    expect(find.byType(TextField), findsNothing);
    expect(find.textContaining('备份密码'), findsNothing);
    expect(find.text('确认数据目录'), findsNothing);
    expect(find.text('备份与恢复'), findsOneWidget);
    final export = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '导出全部数据'),
    );
    expect(export.onPressed, isNotNull);
    expect(find.text('已准备就绪，可以导出全部数据或选择备份恢复。'), findsOneWidget);
  });

  /// Returns the maintenance lease when the owning screen is removed without pressing its close button.
  testWidgets('disposal closes the maintenance session', (tester) async {
    final host = _MaintenanceHost();
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pumpAndSettle();
    expect(host.operations, ['open', 'configureAutomatic', 'close']);
  });

  testWidgets('pending import is ready to resume without path confirmation', (
    tester,
  ) async {
    final host = _MaintenanceHost(pending: true);
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    expect(host.operations, ['open', 'configureAutomatic']);
    expect(find.byType(TextField), findsNothing);
    final export = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, '导出全部数据'),
    );
    expect(export.onPressed, isNull);
    await tester.ensureVisible(find.text('完成中断的导入'));
    await tester.tap(find.text('完成中断的导入'));
    await tester.pumpAndSettle();
    expect(host.operations, ['open', 'configureAutomatic', 'resumeRestore']);
    expect(find.textContaining('中断的导入已完成'), findsOneWidget);
  });

  testWidgets(
    'shows location errors and retries Host preparation without requesting paths',
    (tester) async {
      final host = _MaintenanceHost(prepareFailures: 1);
      await tester.pumpWidget(DataRescueApp(request: host.request));
      await tester.pumpAndSettle();
      expect(find.textContaining('无法自动定位数据'), findsOneWidget);
      expect(find.byType(TextField), findsNothing);
      final export = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, '导出全部数据'),
      );
      expect(export.onPressed, isNull);
      await tester.ensureVisible(find.text('重新打开维护会话'));
      await tester.tap(find.text('重新打开维护会话'));
      await tester.pumpAndSettle();
      expect(host.operations, [
        'open',
        'configureAutomatic',
        'open',
        'configureAutomatic',
      ]);
      expect(find.textContaining('无法自动定位数据'), findsNothing);
      final readyExport = tester.widget<FilledButton>(
        find.widgetWithText(FilledButton, '导出全部数据'),
      );
      expect(readyExport.onPressed, isNotNull);
    },
  );
  testWidgets('shows measured restore progress while the task remains active', (
    tester,
  ) async {
    final host = _MaintenanceHost(pending: true)..measureResume = true;
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('完成中断的导入'));
    await tester.tap(find.text('完成中断的导入'));
    await tester.pump();
    expect(find.text('正在复核恢复数据'), findsOneWidget);
    expect(find.text('64 / 256 字节（25.0%）'), findsOneWidget);
    expect(
      tester
          .widget<LinearProgressIndicator>(find.byType(LinearProgressIndicator))
          .value,
      .25,
    );
    await tester.pump(const Duration(milliseconds: 80));
    expect(find.text('128 / 256 字节（50.0%）'), findsOneWidget);
    await tester.pumpAndSettle();
    expect(find.textContaining('中断的导入已完成'), findsOneWidget);
    expect(find.byType(LinearProgressIndicator), findsNothing);
  });

  testWidgets(
    'reports a task failure after progress without showing completion',
    (tester) async {
      final host = _MaintenanceHost(pending: true)
        ..measureResume = true
        ..failResume = true;
      await tester.pumpWidget(DataRescueApp(request: host.request));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('完成中断的导入'));
      await tester.tap(find.text('完成中断的导入'));
      await tester.pumpAndSettle();
      expect(find.textContaining('备份完整性检查失败'), findsOneWidget);
      expect(find.textContaining('中断的导入已完成'), findsNothing);
      expect(find.byType(LinearProgressIndicator), findsNothing);
    },
  );
  testWidgets(
    'reads a selected backup in bounded ranges before integrity inspection',
    (tester) async {
      final original = FileSelectorPlatform.instance;
      final file = _PickedBackup();
      final picker = _BackupPicker(file);
      FileSelectorPlatform.instance = picker;
      addTearDown(() => FileSelectorPlatform.instance = original);
      final host = _MaintenanceHost();
      await tester.pumpWidget(DataRescueApp(request: host.request));
      await tester.pumpAndSettle();
      await tester.ensureVisible(find.text('从备份恢复'));
      await tester.tap(find.text('从备份恢复'));
      await tester.pumpAndSettle();
      expect(file.ranges, [1048576, 1048576, 402848]);
      expect(
        host.operations.where((operation) => operation == 'uploadChunk').length,
        39,
      );
      expect(picker.released, [file]);
      expect(host.operations.last, 'inspect');
      expect(host.uploaded, 2500000);
      expect(find.textContaining('完整性校验通过'), findsOneWidget);
    },
  );

  /// Releases the selected source even when the Host rejects its first upload chunk.
  testWidgets('releases picker staging after upload failure', (tester) async {
    final original = FileSelectorPlatform.instance;
    final file = _PickedBackup();
    final picker = _BackupPicker(file);
    FileSelectorPlatform.instance = picker;
    addTearDown(() => FileSelectorPlatform.instance = original);
    final host = _MaintenanceHost()..failUpload = true;
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('从备份恢复'));
    await tester.tap(find.text('从备份恢复'));
    await tester.pumpAndSettle();
    expect(picker.released, [file]);
    expect(host.operations, isNot(contains('inspect')));
    expect(find.textContaining('Upload storage is full'), findsOneWidget);
  });

  /// Releases picker staging when the source fails before the upload is created.
  testWidgets('releases picker staging after source metadata failure', (
    tester,
  ) async {
    final original = FileSelectorPlatform.instance;
    final file = _PickedBackup(failLength: true);
    final picker = _BackupPicker(file);
    FileSelectorPlatform.instance = picker;
    addTearDown(() => FileSelectorPlatform.instance = original);
    final host = _MaintenanceHost();
    await tester.pumpWidget(DataRescueApp(request: host.request));
    await tester.pumpAndSettle();
    await tester.ensureVisible(find.text('从备份恢复'));
    await tester.tap(find.text('从备份恢复'));
    await tester.pumpAndSettle();
    expect(picker.released, [file]);
    expect(host.operations, isNot(contains('beginUpload')));
    expect(
      find.textContaining('Source metadata is unavailable'),
      findsOneWidget,
    );
  });

  /// Sends Android staging ownership back to the picker channel exactly once.
  test(
    'Android releases its selected source through the owning channel',
    () async {
      const channel = MethodChannel(
        'dev.flutter.packages.file_selector_android/save',
      );
      final messenger =
          TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
      final calls = <MethodCall>[];
      messenger.setMockMethodCallHandler(channel, (call) async {
        calls.add(call);
        return null;
      });
      addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
      await FileSelectorAndroid().releaseFile(
        XFile('/cache/operit-selected-owned.backup'),
      );
      expect(calls.single.method, 'releaseSelectedFile');
      expect(calls.single.arguments, {
        'path': '/cache/operit-selected-owned.backup',
      });
    },
  );

  /// Preserves a native release failure instead of declaring its resources released.
  test('Android propagates selected source release errors', () async {
    const channel = MethodChannel(
      'dev.flutter.packages.file_selector_android/save',
    );
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    messenger.setMockMethodCallHandler(channel, (call) async {
      throw PlatformException(
        code: 'SOURCE_RELEASE_FAILED',
        message: 'Cannot remove staging',
      );
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    await expectLater(
      FileSelectorAndroid().releaseFile(
        XFile('/cache/operit-selected-owned.backup'),
      ),
      throwsA(
        isA<PlatformException>().having(
          (error) => error.code,
          'code',
          'SOURCE_RELEASE_FAILED',
        ),
      ),
    );
  });
}

class _MaintenanceHost {
  /// Creates an offline Host with explicit pending-import and preparation-error states.
  _MaintenanceHost({this.pending = false, this.prepareFailures = 0});
  final bool pending;
  int prepareFailures;
  final operations = <String>[];
  Map<String, Object?>? task;
  int jobId = 0;
  int resumePoll = 0;
  bool measureResume = false;
  bool failResume = false;
  int uploaded = 0;
  bool failUpload = false;

  /// Accepts only automatic preparation and the maintenance operations exercised by these tests.
  Future<Object?> request(Map<String, Object?> request) async {
    final operation = request['operation']! as String;
    if (operation == 'start') {
      expect(task, isNull);
      task = Map<String, Object?>.from(request['request']! as Map);
      return <String, Object?>{'jobId': ++jobId};
    }
    if (operation == 'poll') {
      expect(request['jobId'], jobId);
      final active = task!;
      if (measureResume &&
          active['operation'] == 'resumeRestore' &&
          resumePoll++ < 3) {
        return <String, Object?>{
          'done': false,
          'progress': <String, Object?>{
            'phase': 'verifyRestored',
            'completed': resumePoll * 64,
            'total': 256,
            'unit': 'bytes',
          },
        };
      }
      task = null;
      return <String, Object?>{
        'done': true,
        'value': await this.request(active),
      };
    }
    operations.add(operation);
    switch (operation) {
      case 'close':
        return null;
      case 'open':
        return <String, Object?>{'commitPending': pending};
      case 'configureAutomatic':
        expect(request.keys, ['operation']);
        if (prepareFailures > 0) {
          prepareFailures -= 1;
          throw StateError('应用的数据位置记录损坏，无法自动定位数据。');
        }
        return <String, Object?>{'commitPending': pending};
      case 'beginUpload':
        return <String, Object?>{'archiveId': 'selected-archive'};
      case 'uploadChunk':
        if (failUpload) throw StateError('Upload storage is full');
        final bytes = base64Decode(request['bytes']! as String);
        expect(bytes.length, lessThanOrEqualTo(65536));
        uploaded += bytes.length;
        return null;
      case 'sealUpload':
        return <String, Object?>{'byteLength': 2500000};
      case 'inspect':
        return <String, Object?>{
          'files': <Object?>[
            <String, Object?>{'path': 'runtime/data.bin'},
          ],
        };
      case 'describe':
        return <String, Object?>{'commitPending': pending};
      case 'resumeRestore':
        if (failResume) throw StateError('备份完整性检查失败');
        return <String, Object?>{'restored': true};
      default:
        throw StateError(
          'Unexpected operation in standalone maintenance UI: $operation',
        );
    }
  }
}

/// Supplies an already selected backup through the portable picker contract.
class _BackupPicker extends FileSelectorPlatform {
  /// Retains a bounded-read test source.
  _BackupPicker(this.file);
  final XFile file;
  final released = <XFile>[];

  /// Records the release of picker-owned staging after source upload completes.
  @override
  Future<void> releaseFile(XFile file) async {
    expect(identical(file, this.file), isTrue);
    released.add(file);
  }

  /// Models a provider whose length becomes known after staging.
  @override
  Future<XFile?> openFileWithProgress({
    required void Function(int completed, int? total) onProgress,
    List<XTypeGroup>? acceptedTypeGroups,
  }) async {
    onProgress(65536, null);
    return file;
  }
}

/// Rejects unbounded archive reads so browser imports cannot allocate the whole backup.
class _PickedBackup extends XFile {
  /// Creates a declared 2500000-byte source without disk storage.
  _PickedBackup({this.failLength = false}) : super('/selected/backup.zip');
  final bool failLength;
  final ranges = <int>[];

  /// Supplies the selected source size independently of any read.
  @override
  Future<int> length() async {
    if (failLength) throw StateError('Source metadata is unavailable');
    return 2500000;
  }

  /// Allows only the bounded ranges used by the rescue upload loop.
  @override
  Stream<Uint8List> openRead([int? start, int? end]) async* {
    expect(start, isNotNull);
    expect(end, isNotNull);
    expect(end! - start!, lessThanOrEqualTo(1024 * 1024));
    ranges.add(end - start);
    yield Uint8List(end - start);
  }
}
