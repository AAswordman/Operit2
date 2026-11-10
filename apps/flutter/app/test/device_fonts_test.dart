import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_system_fonts/operit_system_fonts.dart';
import 'package:operit_system_fonts/src/font_face.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/appearance/DeviceFontPicker.dart';
import 'package:operit2/ui/theme/DeviceFontService.dart';

/// Verifies native catalog ownership, exact font faces, and the searchable selection flow.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/system_fonts');

  test(
    'native catalog is unique and sorted and face reads use the selected family',
    () async {
      final calls = <MethodCall>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            calls.add(call);
            return switch (call.method) {
              'listFamilies' => ['微软雅黑', 'Arial', 'Arial'],
              'readFamily' => {
                'bytes': Uint8List.fromList([0, 1, 0, 0, ...List.filled(8, 0)]),
                'index': 0,
              },
              _ => throw StateError('Unexpected font method'),
            };
          });
      addTearDown(
        () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null),
      );
      final platform = MethodChannelSystemFonts();
      expect(await platform.listFamilies(), ['Arial', '微软雅黑']);
      expect(await platform.readFamily('微软雅黑'), hasLength(12));
      expect(calls.last.arguments, '微软雅黑');
    },
  );

  test(
    'native errors stay visible instead of fabricating a font list',
    () async {
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(channel, (call) async {
            throw PlatformException(
              code: 'permission_denied',
              message: 'Font access denied',
            );
          });
      addTearDown(
        () => TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(channel, null),
      );
      await expectLater(
        MethodChannelSystemFonts().listFamilies(),
        throwsA(isA<PlatformException>()),
      );
    },
  );

  test(
    'TTC extraction selects the requested face and recalculates SFNT checksums',
    () {
      final collection = _collection();
      final first = extractFontFace(collection, index: 0);
      final second = extractFontFace(collection, index: 1);
      expect(first, isNot(equals(second)));
      final data = ByteData.sublistView(second);
      expect(data.getUint32(0), 0x00010000);
      expect(data.getUint32(12), 0x68656164);
      final head = data.getUint32(20);
      expect(data.getUint32(head + 12), 0x22222222);
      var sum = 0;
      for (var i = 0; i < second.length; i += 4) {
        sum = (sum + data.getUint32(i)) & 0xffffffff;
      }
      expect(sum, 0xb1b0afba);
      expect(
        () => extractFontFace(collection, index: 2),
        throwsFormatException,
      );
      expect(() => extractFontFace(collection), throwsFormatException);
    },
  );

  testWidgets(
    'searches installed fonts and returns the exact registered preview bytes',
    (tester) async {
      final platform = _Fonts();
      final service = DeviceFontService(
        platform: platform,
        registry: ThemeFontRegistry(registration: (_, _) async {}),
      );
      PreparedDeviceFont? selected;
      await tester.pumpWidget(
        MaterialApp(
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          home: Builder(
            builder: (context) => Scaffold(
              body: TextButton(
                onPressed: () async {
                  selected = await showDeviceFontPicker(
                    context,
                    service: service,
                  );
                },
                child: const Text('Fonts'),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('Fonts'));
      await tester.pumpAndSettle();
      expect(find.text('Arial'), findsOneWidget);
      expect(find.text('微软雅黑'), findsOneWidget);
      await tester.enterText(find.byType(TextField), '雅黑');
      await tester.pumpAndSettle();
      expect(find.text('Arial'), findsNothing);
      await tester.tap(find.text('微软雅黑'));
      await tester.pumpAndSettle();
      expect(platform.reads, ['微软雅黑']);
      final preview = tester.widget<Text>(
        find.text('The quick brown fox · 0123456789\n字体预览：你好，世界'),
      );
      expect(preview.style!.fontFamily, startsWith('OperitThemeFont_'));
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(selected!.name, '微软雅黑');
      expect(selected!.bytes, platform.bytes);
      expect(selected!.family, preview.style!.fontFamily);
    },
  );

  testWidgets('shows device access failure and retries only on user request', (
    tester,
  ) async {
    final platform = _Fonts()..catalogError = StateError('Font access denied');
    final service = DeviceFontService(platform: platform);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: Builder(
          builder: (context) => TextButton(
            onPressed: () => showDeviceFontPicker(context, service: service),
            child: const Text('Fonts'),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Fonts'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Font access denied'), findsOneWidget);
    expect(find.text('Arial'), findsNothing);
    platform.catalogError = null;
    await tester.tap(find.text('Retry'));
    await tester.pumpAndSettle();
    expect(find.text('Arial'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('font read errors keep save disabled and remain visible', (
    tester,
  ) async {
    final platform = _Fonts()..readError = StateError('Unreadable font');
    final service = DeviceFontService(platform: platform);
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: DeviceFontPicker(
          service: service,
          catalog: platform.listFamilies(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Arial'));
    await tester.pumpAndSettle();
    expect(find.textContaining('Unreadable font'), findsOneWidget);
    expect(
      tester
          .widget<FilledButton>(find.widgetWithText(FilledButton, 'Save'))
          .onPressed,
      isNull,
    );
  });

  testWidgets('ignores an earlier face read after a newer selection', (
    tester,
  ) async {
    final platform = _Fonts();
    final first = Completer<Uint8List>();
    platform.pending = first;
    final service = DeviceFontService(
      platform: platform,
      registry: ThemeFontRegistry(registration: (_, _) async {}),
    );
    await tester.pumpWidget(
      MaterialApp(
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        supportedLocales: AppLocalizations.supportedLocales,
        home: DeviceFontPicker(
          service: service,
          catalog: platform.listFamilies(),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Arial'));
    await tester.pump();
    await tester.tap(find.text('微软雅黑'));
    await tester.pumpAndSettle();
    first.complete(Uint8List.fromList([9, 8, 7]));
    await tester.pumpAndSettle();
    final save = tester.widget<FilledButton>(
      find.widgetWithText(FilledButton, 'Save'),
    );
    expect(save.onPressed, isNotNull);
    final labels = tester.widgetList<Text>(find.text('微软雅黑'));
    expect(labels, hasLength(2));
    expect(tester.takeException(), isNull);
  });
}

/// Makes two collection faces with different head metadata and absolute table offsets.
Uint8List _collection() {
  final bytes = Uint8List(108);
  final data = ByteData.sublistView(bytes);
  data.setUint32(0, 0x74746366);
  data.setUint32(4, 0x00010000);
  data.setUint32(8, 2);
  data.setUint32(12, 20);
  data.setUint32(16, 48);
  for (var i = 0; i < 2; i++) {
    final face = 20 + i * 28;
    final head = 76 + i * 16;
    data.setUint32(face, 0x00010000);
    data.setUint16(face + 4, 1);
    data.setUint32(face + 12, 0x68656164);
    data.setUint32(face + 20, head);
    data.setUint32(face + 24, 16);
    data.setUint32(head + 12, i == 0 ? 0x11111111 : 0x22222222);
  }
  return bytes;
}

class _Fonts implements SystemFontsPlatform {
  final bytes = Uint8List.fromList([1, 2, 3]);
  final reads = <String>[];
  Completer<Uint8List>? pending;
  Object? catalogError;
  Object? readError;

  /// Returns an explicit catalog independently of the test machine.
  @override
  Future<List<String>> listFamilies() async {
    if (catalogError != null) throw catalogError!;
    return ['Arial', '微软雅黑'];
  }

  /// Allows deterministic out-of-order reads while retaining selected family identity.
  @override
  Future<Uint8List> readFamily(String family) async {
    reads.add(family);
    if (readError != null) throw readError!;
    if (family == 'Arial' && pending != null) return pending!.future;
    return bytes;
  }
}
