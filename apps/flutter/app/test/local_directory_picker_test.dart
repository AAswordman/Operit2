import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';

/// Exercises the real Material picker and channel adapter without a DocumentsUI activity.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/runtime');
  const documentChannel = MethodChannel('operit/folder_access');
  const captureKey = ValueKey('picker-preview');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  final originalPlatform = OperitFolderAccessPlatform.instance;
  late _Folders folders;
  FolderMountSource? selected;
  var completed = false;

  setUp(() {
    folders = _Folders();
    selected = null;
    completed = false;
    OperitFolderAccessAndroid.registerWith();
    messenger.setMockMethodCallHandler(channel, folders.handle);
    messenger.setMockMethodCallHandler(documentChannel, (call) async {
      throw StateError('Local selection must never launch the SAF picker');
    });
  });

  tearDown(() {
    OperitFolderAccessPlatform.instance = originalPlatform;
    messenger.setMockMethodCallHandler(channel, null);
    messenger.setMockMethodCallHandler(documentChannel, null);
  });

  /// Opens the actual Android-registered local picker beneath a localized Material application.
  Future<void> open(
    WidgetTester tester, {
    Brightness brightness = Brightness.light,
    double textScale = 1,
  }) async {
    await tester.pumpWidget(
      RepaintBoundary(
        key: captureKey,
        child: MaterialApp(
          debugShowCheckedModeBanner: false,
          locale: const Locale('zh'),
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          theme: ThemeData(
            useMaterial3: true,
            fontFamily: Platform.environment['PICKER_PREVIEW_FONT'] == null
                ? null
                : 'PickerPreview',
            colorScheme: ColorScheme.fromSeed(
              seedColor: const Color(0xff356859),
              brightness: brightness,
            ),
          ),
          builder: (context, child) => MediaQuery(
            data: MediaQuery.of(
              context,
            ).copyWith(textScaler: TextScaler.linear(textScale)),
            child: child!,
          ),
          home: LocalDirectoryPickerHost(
            child: Scaffold(
              body: Center(
                child: FilledButton(
                  onPressed: () async {
                    selected = await OperitFolderAccess.pickLocalDirectory();
                    completed = true;
                  },
                  child: const Text('Open picker'),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open picker'));
    await tester.pumpAndSettle();
  }

  testWidgets(
    'Android selects a native mount without opening a system directory picker',
    (tester) async {
      await open(tester);
      await tester.tap(find.text('Documents'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('Operit'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('使用此文件夹'));
      await tester.pumpAndSettle();
      expect(completed, isTrue);
      expect(selected!.backend, 'native');
      expect(selected!.root, '/storage/emulated/0/Documents/Operit');
      expect(selected!.namespace, '/virtual/local/folders');
      expect(folders.calls.first.method, 'localDirectoryRoots');
      expect(
        folders.calls
            .where((call) => call.method == 'localDirectoryList')
            .length,
        3,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'search filters folders and Android back navigates before dismissing',
    (tester) async {
      await open(tester);
      await tester.enterText(find.byType(TextField), 'doc');
      await tester.pump();
      expect(find.text('Documents'), findsOneWidget);
      expect(find.text('Download'), findsNothing);
      await tester.tap(find.text('Documents'));
      await tester.pumpAndSettle();
      expect(
        tester.widget<TextField>(find.byType(TextField)).controller!.text,
        isEmpty,
      );
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(find.text('Download'), findsOneWidget);
      expect(completed, isFalse);
      await tester.tap(find.byTooltip('关闭'));
      await tester.pumpAndSettle();
      expect(completed, isTrue);
      expect(selected, isNull);
    },
  );

  testWidgets(
    'new folder rejects traversal and opens the Host-created directory',
    (tester) async {
      await open(tester);
      await tester.tap(find.byTooltip('新建文件夹'));
      await tester.pumpAndSettle();
      final nameField = find.widgetWithText(TextField, '文件夹名称');
      await tester.enterText(nameField, '../bad');
      await tester.pump();
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, '创建'))
            .onPressed,
        isNull,
      );
      await tester.enterText(nameField, '我的备份');
      await tester.pump();
      await tester.tap(find.text('创建'));
      await tester.pumpAndSettle();
      expect(find.text('此文件夹为空'), findsOneWidget);
      expect(find.text('/storage/emulated/0/我的备份'), findsOneWidget);
      expect(folders.calls.last.arguments, {
        'path': '/storage/emulated/0',
        'name': '我的备份',
      });
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'read-only directories cannot be selected or receive new folders',
    (tester) async {
      folders.readOnly = '/storage/emulated/0/Documents';
      await open(tester);
      await tester.tap(find.text('Documents'));
      await tester.pumpAndSettle();
      expect(find.text('此位置为只读'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, '使用此文件夹'))
            .onPressed,
        isNull,
      );
      expect(
        tester
            .widget<IconButton>(
              find.widgetWithIcon(IconButton, Icons.create_new_folder_outlined),
            )
            .onPressed,
        isNull,
      );
    },
  );

  testWidgets('a duplicate folder reports its error inside the picker', (
    tester,
  ) async {
    await open(tester);
    await tester.tap(find.byTooltip('新建文件夹'));
    await tester.pumpAndSettle();
    await tester.enterText(
      find.widgetWithText(TextField, '文件夹名称'),
      'Documents',
    );
    await tester.pump();
    await tester.tap(find.text('创建'));
    await tester.pumpAndSettle();
    expect(find.text('Folder already exists'), findsOneWidget);
    expect(find.text('Download'), findsOneWidget);
    expect(completed, isFalse);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'denied authorization stays in the application and can be retried',
    (tester) async {
      folders.denyPermission = true;
      await open(tester);
      expect(find.text('无法访问此位置'), findsOneWidget);
      expect(find.text('Storage access denied'), findsOneWidget);
      folders.denyPermission = false;
      await tester.tap(find.text('重试'));
      await tester.pumpAndSettle();
      expect(find.text('Documents'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'listing errors disable selection and retry the exact failed path',
    (tester) async {
      await open(tester);
      folders.failListing = true;
      await tester.tap(find.text('Documents'));
      await tester.pumpAndSettle();
      expect(find.text('Cannot list directory'), findsOneWidget);
      expect(
        tester
            .widget<FilledButton>(find.widgetWithText(FilledButton, '使用此文件夹'))
            .onPressed,
        isNull,
      );
      folders.failListing = false;
      await tester.tap(find.text('重试'));
      await tester.pumpAndSettle();
      expect(find.text('Operit'), findsOneWidget);
      expect(folders.calls.last.arguments, {
        'path': '/storage/emulated/0/Documents',
      });
    },
  );

  testWidgets('closing during enumeration does not update a disposed dialog', (
    tester,
  ) async {
    final pending = Completer<Map<String, Object?>>();
    folders.pendingListing = pending;
    await tester.pumpWidget(
      MaterialApp(
        home: LocalDirectoryPickerHost(
          child: Builder(
            builder: (context) => TextButton(
              onPressed: LocalDirectoryPickerHost.select,
              child: const Text('Open'),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.text('Open'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 300));
    await tester.tap(find.byIcon(Icons.close_rounded));
    await tester.pumpAndSettle();
    pending.complete(folders.listing('/storage/emulated/0'));
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets('mounted volumes remain selectable through storage navigation', (
    tester,
  ) async {
    await open(tester);
    await tester.tap(find.byTooltip('存储位置'));
    await tester.pumpAndSettle();
    expect(find.text('SD 卡'), findsOneWidget);
    await tester.tap(find.text('SD 卡'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('使用此文件夹'));
    await tester.pumpAndSettle();
    expect(selected!.root, '/storage/ABCD-1234');
  });

  for (final brightness in Brightness.values) {
    testWidgets(
      'Material 3 picker fits a phone in $brightness and exports its rendered preview',
      (tester) async {
        tester.view.physicalSize = const Size(430, 932);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final fontPath = Platform.environment['PICKER_PREVIEW_FONT'];
        if (fontPath != null) {
          await tester.runAsync(() async {
            final loader = FontLoader('PickerPreview')
              ..addFont(
                Future.value(
                  ByteData.sublistView(await File(fontPath).readAsBytes()),
                ),
              );
            await loader.load();
          });
        }
        final iconsPath = Platform.environment['PICKER_PREVIEW_ICONS'];
        if (iconsPath != null) {
          await tester.runAsync(() async {
            final loader = FontLoader('MaterialIcons')
              ..addFont(
                Future.value(
                  ByteData.sublistView(await File(iconsPath).readAsBytes()),
                ),
              );
            await loader.load();
          });
        }
        await open(tester, brightness: brightness);
        await tester.tap(find.text('Documents'));
        await tester.pumpAndSettle();
        final theme = Theme.of(tester.element(find.text('选择文件夹')));
        expect(theme.useMaterial3, isTrue);
        expect(theme.brightness, brightness);
        expect(tester.takeException(), isNull);
        final output = Platform.environment['PICKER_PREVIEW_DIR'];
        if (output != null) {
          // Render the real widget with example directory contents, not a separate mockup.
          await tester.runAsync(() async {
            final boundary = tester.renderObject<RenderRepaintBoundary>(
              find.byKey(captureKey),
            );
            final image = await boundary.toImage(pixelRatio: 2);
            final bytes = await image.toByteData(
              format: ui.ImageByteFormat.png,
            );
            await Directory(output).create(recursive: true);
            await File(
              '$output/folder-picker-${brightness.name}.png',
            ).writeAsBytes(bytes!.buffer.asUint8List());
            image.dispose();
          });
        }
      },
    );
  }

  testWidgets('compact layout supports large system text', (tester) async {
    tester.view.physicalSize = const Size(360, 640);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    await open(tester, textScale: 1.5);
    expect(find.text('使用此文件夹'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('the selection action remains visible above the phone keyboard', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(390, 700);
    tester.view.devicePixelRatio = 1;
    tester.view.viewInsets = const FakeViewPadding(bottom: 300);
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetViewInsets);
    await open(tester);
    expect(find.text('使用此文件夹').hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });
}

/// Models native permission and directory responses while retaining the real channel codec.
class _Folders {
  final calls = <MethodCall>[];
  bool denyPermission = false;
  bool failListing = false;
  String? readOnly;
  Completer<Map<String, Object?>>? pendingListing;
  final children = <String, List<String>>{
    '/storage/emulated/0': [
      'DCIM',
      'Documents',
      'Download',
      'Movies',
      'Music',
      'Pictures',
    ],
    '/storage/emulated/0/Documents': [
      'Books',
      'Notes',
      'Operit',
      'Projects',
      '工作文档',
    ],
    '/storage/emulated/0/Documents/Operit': ['Backups'],
    '/storage/ABCD-1234': [],
  };

  /// Implements only native-directory operations; unrelated platform picker calls fail the test.
  Future<Object?> handle(MethodCall call) async {
    calls.add(call);
    final args = Map<String, Object?>.from(call.arguments as Map? ?? {});
    switch (call.method) {
      case 'localDirectoryRoots':
        if (denyPermission) {
          throw PlatformException(
            code: 'STORAGE_PERMISSION_REQUIRED',
            message: 'Storage access denied',
          );
        }
        return [
          {'path': '/storage/emulated/0', 'name': '内部存储空间'},
          {'path': '/storage/ABCD-1234', 'name': 'SD 卡'},
        ];
      case 'localDirectoryList':
        if (failListing) {
          throw PlatformException(
            code: 'DIRECTORY_ACCESS_ERROR',
            message: 'Cannot list directory',
          );
        }
        if (pendingListing != null) return pendingListing!.future;
        return listing(args['path'] as String);
      case 'localDirectoryCreate':
        final path = args['path'] as String;
        final name = args['name'] as String;
        if (children[path]!.contains(name)) {
          throw PlatformException(
            code: 'DIRECTORY_ACCESS_ERROR',
            message: 'Folder already exists',
          );
        }
        children[path]!.add(name);
        children['$path/$name'] = [];
        return listing('$path/$name');
      default:
        throw StateError('Unexpected Host operation: ${call.method}');
    }
  }

  /// Returns explicit breadcrumbs so Flutter never needs to interpret native paths.
  Map<String, Object?> listing(String path) {
    final root = path.startsWith('/storage/emulated/0')
        ? '/storage/emulated/0'
        : '/storage/ABCD-1234';
    final breadcrumbs = <Map<String, String>>[
      {'path': root, 'name': root == '/storage/emulated/0' ? '内部存储空间' : 'SD 卡'},
    ];
    var current = root;
    for (final part
        in path
            .substring(root.length)
            .split('/')
            .where((part) => part.isNotEmpty)) {
      current = '$current/$part';
      breadcrumbs.add({'path': current, 'name': part});
    }
    return {
      'path': path,
      'name': breadcrumbs.last['name'],
      'writable': path != readOnly,
      'breadcrumbs': breadcrumbs,
      'directories': [
        for (final name in children[path] ?? <String>[])
          {'path': '$path/$name', 'name': name},
      ],
    };
  }
}
