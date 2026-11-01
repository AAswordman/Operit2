import 'dart:async';

import 'package:flutter/material.dart';
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
  }) async {
    await tester.pumpWidget(
      _app(
        WorkspaceFileBrowserContent(
          rootLabel: '/workspace/demo',
          rootRelativePath: '',
          onListWorkspaceFiles: list ?? (path) async => fixtures[path] ?? [],
          onOpenFile: open ?? (_) async {},
        ),
        textScale: textScale,
      ),
    );
    await tester.pumpAndSettle();
  }

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
