import 'dart:async';
import 'dart:convert';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:share_plus_platform_interface/share_plus_platform_interface.dart'
    show SharePlatform, ShareParams, ShareResult, ShareResultStatus;
import 'package:operit2/ui/features/chat/components/share/ChatShareImageGenerator.dart';
import 'package:operit2/ui/features/chat/components/share/ChatShareImagePreviewDialog.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final originalSelector = FileSelectorPlatform.instance;
  final png = base64Decode(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAACklEQVR4nGMAAQAABQABDQottAAAAABJRU5ErkJggg==',
  );
  final originalShare = SharePlatform.instance;
  final share = _SharePlatform();

  setUpAll(() => SharePlatform.instance = share);
  tearDownAll(() => SharePlatform.instance = originalShare);
  late _SavePlatform selector;

  setUp(() {
    selector = _SavePlatform();
    FileSelectorPlatform.instance = selector;
    share.params = null;
    share.error = null;
  });

  tearDown(() {
    FileSelectorPlatform.instance = originalSelector;
  });

  Future<void> showPreview(WidgetTester tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: ChatShareImagePreviewDialog(
            image: ChatShareImage(
              storagePath: '/private/runtime/share/cached.png',
              bytes: png,
            ),
            onDismiss: () {},
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets(
    'save opens the selector with the exact PNG and reports success',
    (tester) async {
      await showPreview(tester);
      expect(find.text('/private/runtime/share/cached.png'), findsNothing);
      expect(find.text('分享图片'), findsOneWidget);
      await tester.tap(find.text('保存到文件'));
      await tester.pumpAndSettle();

      expect(selector.calls, 1);
      expect(selector.bytes, png);
      expect(selector.mimeType, 'image/png');
      expect(selector.name, matches(r'^operit_share_\d+\.png$'));
      expect(selector.groups!.single.extensions, <String>['png']);
      expect(selector.groups!.single.mimeTypes, <String>['image/png']);
      expect(selector.groups!.single.uniformTypeIdentifiers, <String>[
        'public.png',
      ]);
      expect(find.text('图片已保存'), findsOneWidget);
    },
  );

  testWidgets(
    'cancelled save leaves the preview open without a success message',
    (tester) async {
      selector.destination = null;
      await showPreview(tester);
      await tester.tap(find.text('保存到文件'));
      await tester.pumpAndSettle();

      expect(selector.calls, 1);
      expect(find.text('图片已保存'), findsNothing);
      expect(find.text('长图预览'), findsOneWidget);
      expect(
        tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
        isNotNull,
      );
    },
  );

  testWidgets('save errors are visible and allow retrying', (tester) async {
    selector.error = PlatformException(code: 'denied', message: '保存被拒绝');
    await showPreview(tester);
    await tester.tap(find.text('保存到文件'));
    await tester.pumpAndSettle();

    expect(find.textContaining('保存失败'), findsOneWidget);
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).onPressed,
      isNotNull,
    );
    expect(find.text('图片已保存'), findsNothing);
  });

  testWidgets('pending save prevents duplicate exports and sharing', (
    tester,
  ) async {
    final pending = Completer<FileSaveLocation?>();
    selector.pending = pending.future;
    await showPreview(tester);
    await tester.tap(find.text('保存到文件'));
    await tester.tap(find.text('保存到文件'));
    await tester.pump();

    expect(selector.calls, 1);
    expect(
      tester.widget<OutlinedButton>(find.byType(OutlinedButton)).onPressed,
      isNull,
    );
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNull,
    );
    pending.complete(null);
    await tester.pumpAndSettle();
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
    );
  });

  testWidgets('share sends the exact PNG and an iPad popover anchor', (
    tester,
  ) async {
    await showPreview(tester);
    final buttonRect = tester.getRect(find.byType(FilledButton));
    await tester.tap(find.text('分享图片'));
    await tester.pumpAndSettle();

    final params = share.params!;
    expect(await params.files!.single.readAsBytes(), png);
    expect(params.files!.single.mimeType, 'image/png');
    expect(
      params.fileNameOverrides!.single,
      matches(r'^operit_share_\d+\.png$'),
    );
    expect(params.sharePositionOrigin, buttonRect);
    expect(selector.calls, 0);
    expect(find.text('图片已保存'), findsNothing);
    expect(tester.takeException(), isNull);
  });

  testWidgets('share errors are visible and re-enable the actions', (
    tester,
  ) async {
    share.error = PlatformException(code: 'unavailable', message: '没有可用的分享应用');
    await showPreview(tester);
    await tester.tap(find.text('分享图片'));
    await tester.pumpAndSettle();
    expect(find.textContaining('分享失败'), findsOneWidget);
    expect(
      tester.widget<FilledButton>(find.byType(FilledButton)).onPressed,
      isNotNull,
    );
  });
}

class _SavePlatform extends FileSelectorPlatform {
  String? destination = '/chosen/chat.png';
  Object? error;
  Future<FileSaveLocation?>? pending;
  int calls = 0;
  Uint8List? bytes;
  String? name;
  String? mimeType;
  List<XTypeGroup>? groups;

  @override
  Future<FileSaveLocation?> saveFile({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    calls++;
    bytes = await file.readAsBytes();
    name = options.suggestedName;
    mimeType = file.mimeType;
    groups = acceptedTypeGroups;
    if (error != null) throw error!;
    if (pending != null) return pending!;
    return destination == null ? null : FileSaveLocation(destination!);
  }
}

class _SharePlatform extends SharePlatform {
  ShareParams? params;
  Object? error;

  @override
  Future<ShareResult> share(ShareParams params) async {
    this.params = params;
    if (error != null) throw error!;
    return const ShareResult('', ShareResultStatus.dismissed);
  }
}
