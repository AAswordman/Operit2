import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/gestures.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextDocument.dart';
import 'package:flutter/services.dart';
import 'package:flutter_svg/flutter_svg.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFileBrowserContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFileIcon.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspacePathBar.dart';
import 'package:operit2/ui/features/chat/viewmodel/WorkspaceFileModels.dart';

WorkspaceFileEntry _entry(String path, {bool folder = false}) =>
    WorkspaceFileEntry(
      name: path.split('/').last,
      path: '/workspace/demo/$path',
      relativePath: path,
      isDirectory: folder,
      size: 0,
      lastModified: '',
    );

Widget _app(Widget child, {double textScale = 1}) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    locale: const Locale('zh'),
    supportedLocales: AppLocalizations.supportedLocales,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    builder: (context, child) => MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(textScaler: TextScaler.linear(textScale)),
      child: child!,
    ),
    home: Scaffold(body: child),
  ),
);

void main() {
  final fixtures = <String, List<WorkspaceFileEntry>>{
    '': [
      _entry('README.md'),
      _entry('src', folder: true),
      _entry('assets', folder: true),
      _entry('package.json'),
    ],
    'src': [_entry('src/main.dart'), _entry('src/lib', folder: true)],
    'src/lib': [_entry('src/lib/tree.ts')],
    'assets': [_entry('assets/logo.png'), _entry('assets/guide.pdf')],
  };

  Future<void> mount(
    WidgetTester tester, {
    Future<List<WorkspaceFileEntry>> Function(String)? list,
    Future<void> Function(WorkspaceFileEntry)? open,
    double textScale = 1,
    WorkspaceFileMutation? mutate,
    WorkspaceFileClipboard? clipboard,
    bool protectWorkspaceRoots = false,
  }) async {
    await tester.pumpWidget(
      _app(
        WorkspaceFileBrowserContent(
          rootLabel: '/workspace/demo',
          rootRelativePath: '',
          onListWorkspaceFiles: list ?? (path) async => fixtures[path] ?? [],
          onOpenFile: open ?? (_) async {},
          onMutateWorkspaceFile: mutate,
          fileClipboard: clipboard,
          protectWorkspaceRoots: protectWorkspaceRoots,
        ),
        textScale: textScale,
      ),
    );
    await tester.pumpAndSettle();
  }

  testWidgets('long press opens compact actions without opening the file', (
    tester,
  ) async {
    var opened = false;
    await mount(
      tester,
      open: (_) async => opened = true,
      mutate: (_, _, _) async {},
    );
    await tester.longPress(find.text('README.md'));
    await tester.pumpAndSettle();
    for (final label in ['重命名', '复制', '剪切', '粘贴', '删除']) {
      expect(find.text(label), findsOneWidget);
    }
    expect(opened, isFalse);
    final material = find
        .ancestor(of: find.text('复制'), matching: find.byType(Material))
        .first;
    expect(tester.getSize(material).width, lessThanOrEqualTo(156));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'secondary click deletes files immediately and refreshes the tree',
    (tester) async {
      final entries = [_entry('README.md')];
      final calls = <String>[];
      await mount(
        tester,
        list: (_) async => List.of(entries),
        mutate: (operation, path, destination) async {
          calls.add('${operation.name}:$path');
          entries.clear();
        },
      );
      await tester.tap(find.text('README.md'), buttons: kSecondaryMouseButton);
      await tester.pumpAndSettle();
      await tester.tap(find.text('删除'));
      await tester.pumpAndSettle();
      expect(find.byType(AlertDialog), findsNothing);
      expect(calls, ['delete:README.md']);
      expect(find.text('README.md'), findsNothing);
    },
  );

  testWidgets('folder deletion requires confirmation and cancel preserves it', (
    tester,
  ) async {
    var deleted = false;
    await mount(tester, mutate: (_, _, _) async => deleted = true);
    Future<void> requestDelete() async {
      await tester.longPress(find.text('src'));
      await tester.pumpAndSettle();
      await tester.tap(find.text('删除'));
      await tester.pumpAndSettle();
    }

    await requestDelete();
    expect(find.text('删除文件夹？'), findsOneWidget);
    expect(deleted, isFalse);
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(deleted, isFalse);
    await requestDelete();
    await tester.tap(find.widgetWithText(FilledButton, '删除'));
    await tester.pumpAndSettle();
    expect(deleted, isTrue);
  });

  testWidgets('rename validates leaf names and submits the new sibling path', (
    tester,
  ) async {
    final calls = <String>[];
    await mount(
      tester,
      mutate: (operation, path, destination) async {
        calls.add('${operation.name}:$path:$destination');
      },
    );
    await tester.longPress(find.text('README.md'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('重命名'));
    await tester.pumpAndSettle();
    await tester.enterText(find.byType(TextField), '../oops');
    await tester.tap(find.widgetWithText(FilledButton, '重命名'));
    await tester.pumpAndSettle();
    expect(calls, isEmpty);
    expect(find.text('请输入有效名称，不能包含路径分隔符。'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'guide.md');
    await tester.tap(find.widgetWithText(FilledButton, '重命名'));
    await tester.pumpAndSettle();
    expect(calls, ['move:README.md:guide.md']);
    expect(tester.takeException(), isNull);
  });

  testWidgets('copy beside original chooses a free name and preserves suffix', (
    tester,
  ) async {
    final calls = <String>[];
    await mount(
      tester,
      list: (_) async => [_entry('README.md'), _entry('README（副本 1）.md')],
      mutate: (operation, path, destination) async =>
          calls.add('${operation.name}:$path:$destination'),
    );
    await tester.longPress(find.text('README.md'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('复制'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('demo'), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('粘贴'));
    await tester.pumpAndSettle();
    expect(calls, ['copy:README.md:README（副本 2）.md']);
  });

  testWidgets('cut pastes into folder and clears shared clipboard', (
    tester,
  ) async {
    final clipboard = WorkspaceFileClipboard();
    addTearDown(clipboard.dispose);
    final calls = <String>[];
    await mount(
      tester,
      clipboard: clipboard,
      mutate: (operation, path, destination) async {
        calls.add('${operation.name}:$path:$destination');
      },
    );
    await tester.longPress(find.text('README.md'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('剪切'));
    await tester.pumpAndSettle();
    expect(clipboard.value?.cut, isTrue);
    await tester.tap(find.text('src'), buttons: kSecondaryMouseButton);
    await tester.pumpAndSettle();
    await tester.tap(find.text('粘贴'));
    await tester.pumpAndSettle();
    expect(calls, ['move:README.md:src/README.md']);
    expect(clipboard.value, isNull);
  });

  testWidgets('failed paste keeps cut data and displays failure', (
    tester,
  ) async {
    final clipboard = WorkspaceFileClipboard()
      ..value = WorkspaceFileClipboardEntry(_entry('README.md'), cut: true);
    addTearDown(clipboard.dispose);
    await mount(
      tester,
      clipboard: clipboard,
      mutate: (_, _, _) async {
        throw StateError('Name already exists');
      },
    );
    await tester.longPress(find.text('src'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('粘贴'));
    await tester.pumpAndSettle();
    expect(clipboard.value, isNotNull);
    expect(find.textContaining('Name already exists'), findsOneWidget);
  });

  testWidgets('folders cannot be pasted into their own descendants', (
    tester,
  ) async {
    final clipboard = WorkspaceFileClipboard()
      ..value = WorkspaceFileClipboardEntry(
        _entry('src', folder: true),
        cut: true,
      );
    addTearDown(clipboard.dispose);
    await mount(tester, clipboard: clipboard, mutate: (_, _, _) async {});
    await tester.tap(find.text('src'));
    await tester.pumpAndSettle();
    await tester.longPress(find.text('lib'));
    await tester.pumpAndSettle();
    final paste = tester.widget<PopupMenuItem<dynamic>>(
      find.ancestor(
        of: find.text('粘贴'),
        matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
      ),
    );
    expect(paste.enabled, isFalse);
  });

  test(
    'moving an open preview preserves unsaved text and zoom on the new path',
    () {
      final document = WorkspaceTextDocument('original')
        ..updateText('unsaved edit')
        ..updateScale(1.5);
      addTearDown(document.dispose);
      final original = WorkspaceTab(
        kind: WorkspaceTabKind.filePreview,
        title: 'old.md',
        icon: Icons.description,
        filePath: 'src/old.md',
        absolutePath: '/workspace/src/old.md',
        textDocument: document,
        previewKind: WorkspaceFilePreviewKind.markdown,
      );
      final moved = original.withFileLocation(
        'docs/new.md',
        '/workspace/docs/new.md',
      );
      expect(moved.filePath, 'docs/new.md');
      expect(moved.absolutePath, '/workspace/docs/new.md');
      expect(moved.title, 'new.md');
      expect(moved.textDocument, same(document));
      expect(moved.textDocument!.text, 'unsaved edit');
      expect(moved.textDocument!.isDirty, isTrue);
      expect(moved.textDocument!.scale, 1.5);
      expect(moved.previewKind, WorkspaceFilePreviewKind.markdown);
    },
  );

  testWidgets('mounted roots allow copying but disable destructive actions', (
    tester,
  ) async {
    await mount(
      tester,
      protectWorkspaceRoots: true,
      list: (_) async => [_entry('project', folder: true)],
      mutate: (_, _, _) async {},
    );
    await tester.longPress(find.text('project'));
    await tester.pumpAndSettle();
    for (final label in ['重命名', '剪切', '删除', '粘贴']) {
      final item = tester.widget<PopupMenuItem<dynamic>>(
        find.ancestor(
          of: find.text(label),
          matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
        ),
      );
      expect(item.enabled, isFalse);
    }
    final copy = tester.widget<PopupMenuItem<dynamic>>(
      find.ancestor(
        of: find.text('复制'),
        matching: find.byWidgetPredicate((widget) => widget is PopupMenuItem),
      ),
    );
    expect(copy.enabled, isTrue);
  });

  for (final scale in [1.0, 2.0]) {
    testWidgets('selected row centers its text and icons at scale $scale', (
      tester,
    ) async {
      await mount(tester, textScale: scale);
      Future<void> checkRow(String name, Finder icon) async {
        await tester.tap(find.text(name));
        await tester.pumpAndSettle();
        final row = find
            .ancestor(of: find.text(name), matching: find.byType(InkWell))
            .first;
        final center = tester.getRect(row).center.dy;
        expect(
          tester.getRect(find.text(name)).center.dy,
          closeTo(center, 0.01),
        );
        expect(
          tester.getRect(find.descendant(of: row, matching: icon)).center.dy,
          closeTo(center, 0.01),
        );
      }

      await checkRow('src', find.byIcon(Icons.folder_open));
      await checkRow('main.dart', find.byType(SvgPicture));
      await checkRow('assets', find.byIcon(Icons.folder_open));
      await checkRow('logo.png', find.byIcon(Icons.image_outlined));
    });
  }

  testWidgets(
    'expands in place, sorts folders first, caches and opens relative files',
    (tester) async {
      final listed = <String>[];
      final opened = <WorkspaceFileEntry>[];
      await mount(
        tester,
        list: (path) async {
          listed.add(path);
          return fixtures[path] ?? [];
        },
        open: (entry) async => opened.add(entry),
      );
      expect(listed, ['']);
      expect(
        tester.getTopLeft(find.text('assets')).dy,
        lessThan(tester.getTopLeft(find.text('src')).dy),
      );
      expect(
        tester.getTopLeft(find.text('src')).dy,
        lessThan(tester.getTopLeft(find.text('package.json')).dy),
      );
      await tester.tap(find.text('src'));
      await tester.pumpAndSettle();
      expect(find.text('assets'), findsOneWidget);
      expect(find.text('main.dart'), findsOneWidget);
      expect(
        tester.widget<WorkspacePathBar>(find.byType(WorkspacePathBar)).path,
        '/workspace/demo',
      );
      await tester.tap(find.text('lib'));
      await tester.pumpAndSettle();
      expect(
        tester.getTopLeft(find.text('tree.ts')).dx,
        greaterThan(tester.getTopLeft(find.text('main.dart')).dx),
      );
      await tester.tap(find.text('tree.ts'));
      await tester.pumpAndSettle();
      expect(opened.single.relativePath, 'src/lib/tree.ts');
      await tester.tap(find.text('src'));
      await tester.pumpAndSettle();
      expect(find.text('tree.ts'), findsNothing);
      await tester.tap(find.text('src'));
      await tester.pumpAndSettle();
      expect(find.text('tree.ts'), findsOneWidget);
      expect(listed, ['', 'src', 'src/lib']);
    },
  );

  testWidgets('refresh keeps expanded branches and replaces cached children', (
    tester,
  ) async {
    var refreshed = false;
    final listed = <String>[];
    await mount(
      tester,
      list: (path) async {
        listed.add(path);
        if (refreshed && path == 'src') return [_entry('src/updated.dart')];
        return fixtures[path] ?? [];
      },
    );
    await tester.tap(find.text('src'));
    await tester.pumpAndSettle();
    refreshed = true;
    await tester.tap(find.byTooltip('刷新'));
    await tester.pumpAndSettle();
    expect(find.text('updated.dart'), findsOneWidget);
    expect(find.text('main.dart'), findsNothing);
    expect(listed, ['', 'src', '', 'src']);
  });

  testWidgets(
    'refresh discards stale child results even when they finish last',
    (tester) async {
      final old = Completer<List<WorkspaceFileEntry>>();
      var requests = 0;
      await mount(
        tester,
        list: (path) async {
          if (path != 'src') return fixtures[path] ?? [];
          requests++;
          return requests == 1 ? old.future : [_entry('src/new.dart')];
        },
      );
      await tester.tap(find.text('src'));
      await tester.pump();
      expect(find.byType(CircularProgressIndicator), findsOneWidget);
      await tester.tap(find.byTooltip('刷新'));
      await tester.pumpAndSettle();
      expect(find.text('new.dart'), findsOneWidget);
      old.complete([_entry('src/stale.dart')]);
      await tester.pumpAndSettle();
      expect(find.text('stale.dart'), findsNothing);
      expect(find.text('new.dart'), findsOneWidget);
    },
  );

  testWidgets(
    'branch failures can be retried and empty folders stay in their branch',
    (tester) async {
      var attempts = 0;
      await mount(
        tester,
        list: (path) async {
          if (path == 'src') {
            if (++attempts == 1) throw StateError('Permission denied');
            return [];
          }
          return fixtures[path] ?? [];
        },
      );
      await tester.tap(find.text('src'));
      await tester.pumpAndSettle();
      expect(find.byTooltip('Bad state: Permission denied'), findsOneWidget);
      await tester.tap(find.text('重试'));
      await tester.pumpAndSettle();
      expect(
        find.text(
          AppLocalizations.of(
            tester.element(find.byType(WorkspaceFileBrowserContent)),
          )!.emptyFolder,
        ),
        findsOneWidget,
      );
      expect(find.text('README.md'), findsOneWidget);
      expect(attempts, 2);
    },
  );

  testWidgets(
    'arrow keys traverse the tree, collapse parents and Enter opens files',
    (tester) async {
      final opened = <String>[];
      await mount(
        tester,
        open: (entry) async => opened.add(entry.relativePath),
      );
      await tester.tap(find.text('src'));
      await tester.pumpAndSettle();
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowRight);
      await tester.pumpAndSettle();
      expect(find.text('tree.ts'), findsOneWidget);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowDown);
      await tester.sendKeyEvent(LogicalKeyboardKey.enter);
      await tester.pumpAndSettle();
      expect(opened, ['src/lib/tree.ts']);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.arrowLeft);
      await tester.pumpAndSettle();
      expect(find.text('tree.ts'), findsNothing);
      expect(find.text('main.dart'), findsOneWidget);
    },
  );

  testWidgets('narrow panels support deep paths at large text scales', (
    tester,
  ) async {
    await tester.binding.setSurfaceSize(const Size(240, 650));
    addTearDown(() => tester.binding.setSurfaceSize(null));
    await mount(
      tester,
      textScale: 2,
      list: (path) async {
        final next = path.isEmpty ? 'nested' : '$path/nested';
        return [_entry(next, folder: true)];
      },
    );
    for (var i = 0; i < 7; i++) {
      await tester.tap(find.text('nested').last);
      await tester.pumpAndSettle();
    }
    expect(find.text('nested'), findsNWidgets(8));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'root errors allow retry and late loads are safe after disposal',
    (tester) async {
      final pending = Completer<List<WorkspaceFileEntry>>();
      var requests = 0;
      await mount(
        tester,
        list: (path) async {
          if (++requests == 1) throw StateError('Offline');
          return pending.future;
        },
      );
      expect(find.text('重试'), findsOneWidget);
      await tester.tap(find.text('重试'));
      await tester.pump();
      await tester.pumpWidget(_app(const SizedBox()));
      pending.completeError(StateError('Late error'));
      await tester.pumpAndSettle();
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('code and non-code files use distinct type icons', (
    tester,
  ) async {
    await tester.pumpWidget(
      _app(
        Row(
          children: [
            buildWorkspaceFileIcon('main.dart'),
            buildWorkspaceFileIcon('index.TS'),
            buildWorkspaceFileIcon('photo.png'),
            buildWorkspaceFileIcon('manual.pdf'),
            buildWorkspaceFileIcon('data.csv'),
            buildWorkspaceFileIcon('backup.zip'),
            buildWorkspaceFileIcon('.env.production'),
          ],
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(find.byType(SvgPicture), findsNWidgets(2));
    expect(find.byIcon(Icons.image_outlined), findsOneWidget);
    expect(find.byIcon(Icons.picture_as_pdf_outlined), findsOneWidget);
    expect(find.byIcon(Icons.table_chart_outlined), findsOneWidget);
    expect(find.byIcon(Icons.folder_zip_outlined), findsOneWidget);
    expect(find.byIcon(Icons.tune), findsOneWidget);
  });
}
