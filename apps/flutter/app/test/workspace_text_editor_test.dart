import 'dart:async';
import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFilePreviewContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabStrip.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceFilePreviewActionBar.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextCloseDialog.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextDocument.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextPreview.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceVirtualTextEditor.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceVirtualTextSurface.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextLineNumbers.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceTextZoomViewport.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/syntax/WorkspaceSyntaxLanguage.dart';
import 'package:operit2/ui/main/layout/SidebarDockController.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Wraps workspace widgets in the application's localized theme scope.
Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    locale: const Locale('zh'),
    supportedLocales: AppLocalizations.supportedLocales,
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    home: Scaffold(body: child),
  ),
);

/// Creates a text tab with a persistent document and workspace-relative path.
WorkspaceTab _textTab(
  WorkspaceTextDocument document, {
  WorkspaceFilePreviewKind kind = WorkspaceFilePreviewKind.text,
  String path = '.operit/config.json',
}) => WorkspaceTab(
  kind: WorkspaceTabKind.filePreview,
  title: 'config.json',
  icon: Icons.description_outlined,
  filePath: path,
  absolutePath: '/workspace/$path',
  previewKind: kind,
  textDocument: document,
);

/// Exercises the actual file-format router instead of mounting only the editor.
Widget _content(
  WorkspaceTab tab,
  Future<void> Function(String, Uint8List) write,
) => WorkspaceFilePreviewContent(
  tab: tab,
  onReadWorkspaceFileBytes: (_) async =>
      throw StateError('Unexpected byte read'),
  onWriteWorkspaceFileBytes: write,
  onOpenWorkspaceFile: (_) async =>
      throw StateError('Unexpected external open'),
  onOpenBrowser: ({url, localFilePath, workspaceHtmlPath}) =>
      throw StateError('Unexpected browser open'),
  splitMarkdownContent: (_) async => [],
);

/// Covers workspace editing, persistence, zoom, and the tab gesture contract.
void main() {
  testWidgets(
    'workspace file paths select grammars without replacing the draft',
    (tester) async {
      final document = WorkspaceTextDocument('final value = "中文";');
      const files = <String, WorkspaceSyntaxLanguage>{
        'main.dart': WorkspaceSyntaxLanguage.dart,
        'main.rs': WorkspaceSyntaxLanguage.rust,
        'app.yaml': WorkspaceSyntaxLanguage.yaml,
        'README.md': WorkspaceSyntaxLanguage.markdown,
        'plain.txt': WorkspaceSyntaxLanguage.plainText,
      };
      TextEditingController? original;
      for (final file in files.entries) {
        await tester.pumpWidget(
          _app(_content(_textTab(document, path: file.key), (_, _) async {})),
        );
        await tester.pumpAndSettle();
        final editor = tester.widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        );
        final state = tester.state<WorkspaceVirtualTextEditorState>(
          find.byType(WorkspaceVirtualTextEditor),
        );
        original ??= editor.controller;
        expect(identical(editor.controller, original), isTrue);
        expect(editor.language, file.value);
        expect(state.renderEditable.textLayout.syntax.language, file.value);
        expect(editor.controller.text, document.text);
        expect(tester.takeException(), isNull);
      }
    },
  );

  testWidgets('performance workload isolates zoom from editor controls', (
    tester,
  ) async {
    final document = WorkspaceTextDocument(
      List.generate(
        2000,
        (i) => 'line $i 中文 e\u0301 👩‍💻 ${'wrapping content ' * 6}',
      ).join('\n'),
    );
    await tester.pumpWidget(
      _app(_content(_textTab(document), (_, _) async {})),
    );
    await tester.pumpAndSettle();
    final scroll = tester
        .widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .scrollController;
    scroll.jumpTo(12000);
    await tester.pump();
    final focal = tester.getCenter(find.byType(WorkspaceTextZoomViewport));
    tester.binding.handlePointerEvent(
      PointerPanZoomStartEvent(pointer: 190, position: focal),
    );
    await tester.pump();
    final previous = debugOnRebuildDirtyWidget;
    var symbolBuilds = 0;
    var toolbarBuilds = 0;
    var editorBuilds = 0;
    debugOnRebuildDirtyWidget = (element, builtOnce) {
      previous?.call(element, builtOnce);
      if (element.widget is TextButton) symbolBuilds++;
      if (element.widget is IconButton) toolbarBuilds++;
      if (element.widget is WorkspaceVirtualTextEditor) editorBuilds++;
    };
    addTearDown(() => debugOnRebuildDirtyWidget = previous);
    final samples = <int>[];
    for (var frame = 0; frame < 60; frame++) {
      final timer = Stopwatch()..start();
      for (var sample = 0; sample < 8; sample++) {
        final progress = (frame * 8 + sample) / (60 * 8 - 1);
        tester.binding.handlePointerEvent(
          PointerPanZoomUpdateEvent(
            pointer: 190,
            position: focal,
            scale: 0.8 + progress * 1.2,
            pan: Offset(0, progress * 15),
          ),
        );
      }
      await tester.pump(const Duration(milliseconds: 16));
      samples.add(timer.elapsedMicroseconds);
    }
    debugOnRebuildDirtyWidget = previous;
    expect(symbolBuilds, 0, reason: 'Zoom must not rebuild the shortcut bar');
    expect(toolbarBuilds, 0, reason: 'Zoom must not rebuild unchanged actions');
    expect(
      editorBuilds,
      0,
      reason:
          'Zoom changes layout, not WorkspaceVirtualTextEditor configuration',
    );
    samples.sort();
    debugPrint(
      'EDITOR_PERF zoom frames=60 events=480 symbols=$symbolBuilds toolbar=$toolbarBuilds editor=$editorBuilds median_us=${samples[30]} p95_us=${samples[56]}',
    );
    tester.binding.handlePointerEvent(
      PointerPanZoomEndEvent(pointer: 190, position: focal),
    );
    await tester.pumpAndSettle();
    final scrollTimer = Stopwatch()..start();
    for (var frame = 0; frame < 60; frame++) {
      scroll.jumpTo(scroll.offset + 8);
      await tester.pump(const Duration(milliseconds: 16));
    }
    debugPrint(
      'EDITOR_PERF scroll frames=60 total_us=${scrollTimer.elapsedMicroseconds}',
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets('line number paragraphs are reused and bounded while scrolling', (
    tester,
  ) async {
    final document = WorkspaceTextDocument(
      List.generate(1200, (i) => 'line $i').join('\n'),
    );
    await tester.pumpWidget(
      _app(_content(_textTab(document), (_, _) async {})),
    );
    await tester.pumpAndSettle();
    final gutter = tester.renderObject<RenderWorkspaceTextLineNumbers>(
      find.byType(WorkspaceTextLineNumbers),
    );
    final scroll = tester
        .widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .scrollController;
    final layouts = gutter.debugLabelLayouts;
    expect(layouts, greaterThan(0));
    for (var frame = 0; frame < 12; frame++) {
      gutter.markNeedsPaint();
      await tester.pump();
    }
    expect(
      gutter.debugLabelLayouts,
      layouts,
      reason: 'Repainting unchanged numbers must not shape fresh paragraphs',
    );
    for (var page = 1; page <= 30; page++) {
      scroll.jumpTo(scroll.position.maxScrollExtent * page / 30);
      await tester.pump();
      expect(gutter.debugCachedLabels, lessThanOrEqualTo(128));
    }
    final finalLayouts = gutter.debugLabelLayouts;
    for (var frame = 0; frame < 6; frame++) {
      gutter.markNeedsPaint();
      await tester.pump();
    }
    expect(gutter.debugLabelLayouts, finalLayouts);
    final widget = tester.widget<WorkspaceTextLineNumbers>(
      find.byType(WorkspaceTextLineNumbers),
    );
    gutter.update(
      enabled: widget.enabled,
      scale: widget.scale,
      text: widget.text,
      scrollController: widget.scrollController,
      color: const Color(0xff123456),
      dividerColor: widget.dividerColor,
    );
    expect(
      gutter.debugCachedLabels,
      0,
      reason: 'A color change must invalidate old labels',
    );
    await tester.pump();
    expect(gutter.debugLabelLayouts, greaterThan(finalLayouts));
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(800, 3200);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    await tester.pumpAndSettle();
    await _pinchZoom(tester, 0.6);
    scroll.jumpTo(0);
    await tester.pump();
    expect(gutter.visibleLines().length, greaterThan(128));
    final tallLayouts = gutter.debugLabelLayouts;
    for (var frame = 0; frame < 6; frame++) {
      gutter.markNeedsPaint();
      await tester.pump();
    }
    expect(
      gutter.debugLabelLayouts,
      tallLayouts,
      reason:
          'A tall viewport must cache its whole visible page without thrashing',
    );
    tester.view.physicalSize = const Size(800, 600);
    await tester.pumpAndSettle();
    expect(gutter.debugCachedLabels, lessThanOrEqualTo(128));
    expect(tester.takeException(), isNull);
  });

  test(
    'save retains edits made during the in-flight workspace write',
    () async {
      final document = WorkspaceTextDocument('original');
      addTearDown(document.dispose);
      document.updateText('first draft');
      final pending = Completer<void>();
      final writes = <String>[];
      final save = document.save((text) {
        writes.add(text);
        return pending.future;
      });
      expect(document.isSaving, isTrue);
      document.updateText('second draft');
      await document.save((text) async => writes.add(text));
      expect(writes, ['first draft']);
      pending.complete();
      await save;
      expect(document.text, 'second draft');
      expect(document.isDirty, isTrue);
      expect(document.isSaving, isFalse);
      await document.save((text) async => writes.add(text));
      expect(writes, ['first draft', 'second draft']);
      expect(document.isDirty, isFalse);
    },
  );

  test(
    'cached dirty state tracks edits restored during an in-flight save',
    () async {
      final document = WorkspaceTextDocument('original');
      addTearDown(document.dispose);
      document.updateText('modified');
      final pending = Completer<void>();
      final save = document.save((_) => pending.future);
      document.updateText('original');
      expect(document.isDirty, isFalse);
      document.updateScale(1.8);
      expect(document.isDirty, isFalse);
      pending.complete();
      await save;
      expect(document.isDirty, isTrue);
      document.updateText('modified');
      expect(document.isDirty, isFalse);
    },
  );

  test(
    'failed workspace writes preserve the draft and expose the real error',
    () async {
      final document = WorkspaceTextDocument('original');
      addTearDown(document.dispose);
      document.updateText('draft');
      final error = StateError('write denied');
      await document.save((_) async => throw error);
      expect(document.text, 'draft');
      expect(document.isDirty, isTrue);
      expect(document.isSaving, isFalse);
      expect(document.saveError, same(error));
    },
  );

  testWidgets(
    'text file has an editor, no browser bar, and saves UTF-8 to its relative path',
    (tester) async {
      final document = WorkspaceTextDocument('{"title": "original"}');
      final writes = <(String, String)>[];
      await tester.pumpWidget(
        _app(
          _content(_textTab(document), (path, bytes) async {
            writes.add((path, utf8.decode(bytes)));
          }),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(WorkspaceVirtualTextEditor), findsOneWidget);
      expect(find.byType(WorkspaceFilePreviewActionBar), findsNothing);
      expect(find.byIcon(Icons.open_in_browser), findsNothing);
      expect(
        tester
            .widget<IconButton>(
              find.widgetWithIcon(IconButton, Icons.save_outlined),
            )
            .onPressed,
        isNull,
      );
      await _enterText(
        tester,
        find.byType(WorkspaceVirtualTextEditor),
        '{"title": "中文修改"}',
      );
      await tester.pump();
      await tester.tap(find.byTooltip('保存'));
      await tester.pumpAndSettle();
      expect(writes, [('.operit/config.json', '{"title": "中文修改"}')]);
      expect(document.isDirty, isFalse);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Ctrl+S saves the focused editor', (tester) async {
    final document = WorkspaceTextDocument('initial');
    var saved = '';
    await tester.pumpWidget(
      _app(
        _content(_textTab(document), (_, bytes) async {
          saved = utf8.decode(bytes);
        }),
      ),
    );
    await tester.pumpAndSettle();
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'keyboard draft',
    );
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyS);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    expect(saved, 'keyboard draft');
    expect(document.isDirty, isFalse);
  });

  for (final useMaterial3 in <bool>[false, true]) {
    testWidgets(
      'editor controls stay compact under padded themes (Material3: $useMaterial3)',
      (tester) async {
        tester.view.devicePixelRatio = 1;
        tester.view.physicalSize = const Size(390, 700);
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.view.resetPhysicalSize);
        final document = WorkspaceTextDocument('initial');
        final pendingWrite = Completer<void>();
        await tester.pumpWidget(
          _app(
            Builder(
              builder: (context) => Theme(
                data: ThemeData(
                  useMaterial3: useMaterial3,
                  colorScheme: Theme.of(context).colorScheme,
                  materialTapTargetSize: MaterialTapTargetSize.padded,
                  visualDensity: VisualDensity.comfortable,
                  iconButtonTheme: IconButtonThemeData(
                    style: IconButton.styleFrom(
                      minimumSize: const Size.square(64),
                      padding: const EdgeInsets.all(16),
                      tapTargetSize: MaterialTapTargetSize.padded,
                    ),
                  ),
                  textButtonTheme: TextButtonThemeData(
                    style: TextButton.styleFrom(
                      minimumSize: const Size(96, 64),
                      padding: const EdgeInsets.all(16),
                      tapTargetSize: MaterialTapTargetSize.padded,
                    ),
                  ),
                ),
                child: _content(
                  _textTab(document, kind: WorkspaceFilePreviewKind.markdown),
                  (_, _) => pendingWrite.future,
                ),
              ),
            ),
          ),
        );
        await tester.pumpAndSettle();
        for (final icon in <IconData>[
          Icons.undo,
          Icons.redo,
          Icons.save_outlined,
          Icons.visibility_outlined,
        ]) {
          final button = find.widgetWithIcon(IconButton, icon);
          expect(tester.getSize(button), const Size.square(40));
          expect(tester.widget<IconButton>(button).iconSize, 24);
          expect(
            tester.widget<IconButton>(button).style!.tapTargetSize,
            MaterialTapTargetSize.shrinkWrap,
          );
        }
        final open = find.byKey(const ValueKey('workspace-insert-{'));
        final close = find.byKey(const ValueKey('workspace-insert-}'));
        final row = find.byKey(const ValueKey('workspace-quick-input'));
        expect(tester.getSize(open), const Size(36, 32));
        expect(tester.getSize(close), const Size(36, 32));
        expect(tester.getSize(row).height, 32);
        expect(tester.getCenter(close).dx - tester.getCenter(open).dx, 36);
        expect(
          tester.widget<TextButton>(open).style!.tapTargetSize,
          MaterialTapTargetSize.shrinkWrap,
        );
        await tester.tap(open);
        await tester.pumpAndSettle();
        expect(document.text, '{initial');
        final editorTop = tester
            .getTopLeft(find.byType(WorkspaceVirtualTextEditor))
            .dy;
        await tester.tap(find.byTooltip('保存'));
        await tester.pump();
        expect(find.byType(CircularProgressIndicator), findsOneWidget);
        expect(
          tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)).dy,
          editorTop,
          reason: 'Saving must not increase toolbar height',
        );
        pendingWrite.complete();
        await tester.pumpAndSettle();
        expect(document.isDirty, isFalse);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('toolbar undo redo and save share the native editor history', (
    tester,
  ) async {
    final document = WorkspaceTextDocument('initial');
    final writes = <String>[];
    await tester.pumpWidget(
      _app(
        _content(_textTab(document), (_, bytes) async {
          writes.add(utf8.decode(bytes));
        }),
      ),
    );
    await tester.pump(const Duration(milliseconds: 600));
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.undo))
          .onPressed,
      isNull,
    );
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.redo))
          .onPressed,
      isNull,
    );
    expect(find.byIcon(Icons.zoom_in), findsNothing);
    expect(find.byIcon(Icons.zoom_out), findsNothing);
    expect(find.text('100%'), findsNothing);
    await _enterText(tester, find.byType(WorkspaceVirtualTextEditor), 'draft');
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.undo))
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byTooltip('撤销'));
    await tester.pumpAndSettle();
    expect(document.text, 'initial');
    expect(document.isDirty, isFalse);
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.redo))
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byTooltip('重做'));
    await tester.pumpAndSettle();
    expect(document.text, 'draft');
    expect(
      tester
          .state<WorkspaceVirtualTextEditorState>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .widget
          .focusNode
          .hasFocus,
      isTrue,
    );
    await tester.tap(find.byTooltip('保存'));
    await tester.pumpAndSettle();
    expect(writes, ['draft']);
    expect(document.isDirty, isFalse);
    await tester.tap(find.byTooltip('撤销'));
    await tester.pumpAndSettle();
    expect(document.text, 'initial');
    expect(document.isDirty, isTrue);
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'different edit',
    );
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.redo))
          .onPressed,
      isNull,
    );
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'quick symbols replace selection, preserve focus and support undo redo',
    (tester) async {
      final document = WorkspaceTextDocument('a👩‍💻z');
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pump(const Duration(milliseconds: 600));
      await _showKeyboard(tester, find.byType(WorkspaceVirtualTextEditor));
      final state = tester.state<WorkspaceVirtualTextEditorState>(
        find.byType(WorkspaceVirtualTextEditor),
      );
      state.widget.controller.selection = TextSelection(
        baseOffset: document.text.length - 1,
        extentOffset: 1,
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('workspace-insert-{')));
      await tester.pump(const Duration(milliseconds: 600));
      await tester.pump();
      expect(document.text, 'a{z');
      expect(
        state.widget.controller.selection,
        const TextSelection.collapsed(offset: 2),
      );
      expect(state.widget.focusNode.hasFocus, isTrue);
      expect(tester.testTextInput.isVisible, isTrue);
      await tester.tap(find.byKey(const ValueKey('workspace-insert-}')));
      await tester.pump(const Duration(milliseconds: 600));
      await tester.pump();
      expect(document.text, 'a{}z');
      expect(state.widget.controller.selection.extentOffset, 3);
      await tester.tap(find.byTooltip('撤销'));
      await tester.pumpAndSettle();
      expect(document.text, 'a{z');
      await tester.tap(find.byTooltip('重做'));
      await tester.pumpAndSettle();
      expect(document.text, 'a{}z');
      expect(state.widget.focusNode.hasFocus, isTrue);
      expect(tester.takeException(), isNull);
    },
    variant: TargetPlatformVariant({
      TargetPlatform.android,
      TargetPlatform.iOS,
      TargetPlatform.windows,
    }),
  );

  testWidgets(
    'quick symbol row scrolls on phones and starts editing an unfocused file',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(390, 700);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final document = WorkspaceTextDocument('');
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('workspace-insert-{')));
      await tester.pumpAndSettle();
      expect(document.text, '{');
      expect(
        tester
            .state<WorkspaceVirtualTextEditorState>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .widget
            .focusNode
            .hasFocus,
        isTrue,
      );
      final row = find.byKey(const ValueKey('workspace-quick-input'));
      final scrollable = tester.state<ScrollableState>(
        find.descendant(of: row, matching: find.byType(Scrollable)),
      );
      await tester.drag(row, const Offset(-350, 0));
      await tester.pumpAndSettle();
      expect(scrollable.position.pixels, greaterThan(0));
      expect(document.text, '{');
      scrollable.position.jumpTo(scrollable.position.maxScrollExtent);
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey(r'workspace-insert-$')));
      await tester.pumpAndSettle();
      expect(document.text, r'{$');
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('Markdown preview hides quick input and retains undo history', (
    tester,
  ) async {
    final document = WorkspaceTextDocument('# original');
    await tester.pumpWidget(
      _app(
        _content(
          _textTab(document, kind: WorkspaceFilePreviewKind.markdown),
          (_, _) async {},
        ),
      ),
    );
    await tester.pump(const Duration(milliseconds: 600));
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      '# edited',
    );
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    await tester.tap(find.byTooltip('Markdown 预览'));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('workspace-quick-input')), findsNothing);
    final dormantEditor = tester.state<WorkspaceVirtualTextEditorState>(
      find.byType(WorkspaceVirtualTextEditor, skipOffstage: false),
    );
    expect(
      dormantEditor.renderEditable.attached,
      isFalse,
      reason:
          'The hidden editor must leave the rendering pipeline without losing history',
    );
    await _pinchZoom(tester, 1.2);
    expect(dormantEditor.renderEditable.attached, isFalse);
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.undo))
          .onPressed,
      isNull,
    );
    await tester.tap(find.byTooltip('编辑'));
    await tester.pumpAndSettle();
    expect(find.byKey(const ValueKey('workspace-quick-input')), findsOneWidget);
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.undo))
          .onPressed,
      isNotNull,
    );
    await tester.tap(find.byTooltip('撤销'));
    await tester.pumpAndSettle();
    expect(document.text, '# original');
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'symbol input commits composing text through the editable input',
    (tester) async {
      final document = WorkspaceTextDocument('');
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      await _showKeyboard(tester, find.byType(WorkspaceVirtualTextEditor));
      tester.testTextInput.updateEditingValue(
        const TextEditingValue(
          text: '中文',
          selection: TextSelection.collapsed(offset: 2),
          composing: TextRange(start: 0, end: 2),
        ),
      );
      await tester.pump();
      await tester.tap(find.byKey(const ValueKey('workspace-insert-{')));
      await tester.pumpAndSettle();
      final state = tester.state<WorkspaceVirtualTextEditorState>(
        find.byType(WorkspaceVirtualTextEditor),
      );
      expect(document.text, '中文{');
      expect(state.widget.controller.value.composing, TextRange.empty);
      expect(state.widget.controller.selection.extentOffset, 3);
      expect(tester.testTextInput.isVisible, isTrue);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('switching documents starts a separate undo history', (
    tester,
  ) async {
    final first = WorkspaceTextDocument('first');
    final second = WorkspaceTextDocument('second');
    await tester.pumpWidget(_app(_content(_textTab(first), (_, _) async {})));
    await tester.pump(const Duration(milliseconds: 600));
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'first draft',
    );
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    await tester.pumpWidget(_app(_content(_textTab(second), (_, _) async {})));
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    expect(
      tester
          .widget<IconButton>(find.widgetWithIcon(IconButton, Icons.undo))
          .onPressed,
      isNull,
    );
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'second draft',
    );
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pump();
    await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
    await tester.sendKeyEvent(LogicalKeyboardKey.keyZ);
    await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
    await tester.pumpAndSettle();
    expect(second.text, 'second');
    expect(first.text, 'first draft');
    await tester.tap(find.byTooltip('重做'));
    await tester.pumpAndSettle();
    expect(second.text, 'second draft');
    expect(tester.takeException(), isNull);
  });

  testWidgets('editor reports write errors without losing editable text', (
    tester,
  ) async {
    final document = WorkspaceTextDocument('initial');
    await tester.pumpWidget(
      _app(
        _content(_textTab(document), (_, _) async {
          throw StateError('permission denied');
        }),
      ),
    );
    await tester.pumpAndSettle();
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'keep this draft',
    );
    await tester.pump();
    await tester.tap(find.byTooltip('保存'));
    await tester.pumpAndSettle();
    expect(find.text('Bad state: permission denied'), findsOneWidget);
    expect(document.text, 'keep this draft');
    expect(document.isDirty, isTrue);
    expect(
      tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .controller
          .text,
      'keep this draft',
    );
  });

  testWidgets(
    'keyboard zoom and two-finger pinch work without toolbar zoom controls',
    (tester) async {
      final document = WorkspaceTextDocument('pinch this text');
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      expect(find.byIcon(Icons.zoom_in), findsNothing);
      expect(find.byIcon(Icons.zoom_out), findsNothing);
      expect(find.text('100%'), findsNothing);
      await _showKeyboard(tester, find.byType(WorkspaceVirtualTextEditor));
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.equal);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pump();
      expect(document.scale, closeTo(1.1, 0.001));
      expect(_visualFontSize(tester), closeTo(15.4, 0.01));
      await tester.sendKeyDownEvent(LogicalKeyboardKey.controlLeft);
      await tester.sendKeyEvent(LogicalKeyboardKey.digit0);
      await tester.sendKeyUpEvent(LogicalKeyboardKey.controlLeft);
      await tester.pump();
      expect(document.scale, 1);
      final center = tester.getCenter(find.byType(WorkspaceVirtualTextEditor));
      final first = await tester.createGesture(
        pointer: 1,
        kind: PointerDeviceKind.touch,
      );
      final second = await tester.createGesture(
        pointer: 2,
        kind: PointerDeviceKind.touch,
      );
      await first.down(center - const Offset(30, 0));
      await second.down(center + const Offset(30, 0));
      await second.moveTo(center + const Offset(90, 0));
      await tester.pump();
      expect(document.scale, closeTo(2, 0.01));
      expect(_visualFontSize(tester), closeTo(28, 0.01));
      await second.up();
      await first.up();
      expect(document.text, 'pinch this text');
      expect(document.isDirty, isFalse);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'trackpad pinch changes text zoom through unified pointer events',
    (tester) async {
      final document = WorkspaceTextDocument('trackpad text');
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      final position = tester.getCenter(
        find.byType(WorkspaceVirtualTextEditor),
      );
      tester.binding.handlePointerEvent(
        PointerPanZoomStartEvent(pointer: 5, position: position),
      );
      tester.binding.handlePointerEvent(
        PointerPanZoomUpdateEvent(pointer: 5, position: position, scale: 1.5),
      );
      await tester.pump();
      expect(document.scale, closeTo(1.5, 0.001));
      tester.binding.handlePointerEvent(
        PointerPanZoomEndEvent(pointer: 5, position: position),
      );
      expect(document.text, 'trackpad text');
      expect(document.isDirty, isFalse);
    },
  );

  testWidgets(
    'pinch anchor is stable in the painted frame, not corrected a frame later',
    (tester) async {
      final document = WorkspaceTextDocument(
        List.generate(160, (i) => 'line $i').join('\n'),
      );
      late RenderEditable render;
      late TextPosition anchor;
      late double lineFraction;
      var sampling = false;
      final painted = <double>[];
      await tester.pumpWidget(
        _app(
          _PaintProbe(
            onPaint: () {
              if (!sampling) return;
              painted.add(_anchorPoint(render, anchor, lineFraction).dy);
            },
            child: _content(_textTab(document), (_, _) async {}),
          ),
        ),
      );
      await tester.pumpAndSettle();
      final scroll = tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .scrollController;
      scroll.jumpTo(900);
      await tester.pump();
      render = tester
          .state<WorkspaceVirtualTextEditorState>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .renderEditable;
      final focal =
          tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
          const Offset(100, 220);
      anchor = render.getPositionForPoint(focal);
      lineFraction =
          (render.globalToLocal(focal).dy - _lineCenter(render, anchor).dy) /
          render.preferredLineHeight;
      final first = await tester.createGesture(
        pointer: 31,
        kind: PointerDeviceKind.touch,
      );
      final second = await tester.createGesture(
        pointer: 32,
        kind: PointerDeviceKind.touch,
      );
      await first.down(focal - const Offset(30, 0));
      await second.down(focal + const Offset(30, 0));
      await tester.pump();
      sampling = true;
      await first.moveTo(focal - const Offset(45, 0));
      await second.moveTo(focal + const Offset(45, 0));
      await tester.pump();
      expect(painted, isNotEmpty);
      for (final y in painted) {
        expect(
          y,
          closeTo(focal.dy, 1),
          reason:
              'Every painted frame must already contain the corrected scroll offset',
        );
      }
      sampling = false;
      await second.up();
      await first.up();
      await tester.pumpAndSettle();
    },
  );

  for (final width in <double>[390, 800]) {
    testWidgets(
      'continuous wrapped zoom preserves its painted text anchor at width $width',
      (tester) async {
        tester.view.devicePixelRatio = 3;
        tester.view.physicalSize = Size(width * 3, 844 * 3);
        addTearDown(tester.view.resetDevicePixelRatio);
        addTearDown(tester.view.resetPhysicalSize);
        final text = List.generate(
          140,
          (i) => '行 $i  👩‍💻 e\u0301 中英文 ${'wrapping text ' * 14}',
        ).join('\r\n');
        final document = WorkspaceTextDocument(text);
        late RenderEditable render;
        late ScrollController scroll;
        late TextPosition anchor;
        late double fraction;
        var sampling = false;
        final painted = <Offset>[];
        await tester.pumpWidget(
          _app(
            _PaintProbe(
              onPaint: () {
                if (!sampling) return;
                painted.add(_anchorPoint(render, anchor, fraction));
              },
              child: _content(_textTab(document), (_, _) async {}),
            ),
          ),
        );
        await tester.pumpAndSettle();
        await _showKeyboard(tester, find.byType(WorkspaceVirtualTextEditor));
        await tester.pumpAndSettle();
        scroll = tester
            .widget<WorkspaceVirtualTextEditor>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .scrollController;
        scroll.jumpTo(1600);
        await tester.pump();
        render = tester
            .state<WorkspaceVirtualTextEditorState>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .renderEditable;
        final gutter = tester.renderObject<RenderWorkspaceTextLineNumbers>(
          find.byType(WorkspaceTextLineNumbers),
        );
        final baseGutterWidth = gutter.gutterWidth;
        final baseLabelHeight = gutter.lineNumberHeight;
        final gutterLeft = gutter.localToGlobal(Offset.zero).dx;
        final anchorRowHeights = <double>[];
        final focal =
            tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
            const Offset(160, 250);
        anchor = render.getPositionForPoint(focal);
        fraction =
            (render.globalToLocal(focal).dy - _lineCenter(render, anchor).dy) /
            render.preferredLineHeight;
        final left = _textInsetAfterGutter(tester, render);
        final first = await tester.createGesture(
          pointer: 41,
          kind: PointerDeviceKind.touch,
        );
        final second = await tester.createGesture(
          pointer: 42,
          kind: PointerDeviceKind.touch,
        );
        await first.down(focal - const Offset(35, 0));
        await second.down(focal + const Offset(35, 0));
        await tester.pump();
        sampling = true;
        for (var frame = 0; frame < 100; frame++) {
          final progress = frame < 50 ? frame / 49 : (99 - frame) / 49;
          final scale = 0.6 + progress * 1.9;
          // Horizontal finger movement must never displace the wrapped page.
          // The fixed text anchor follows the vertical center through reflow.
          final center =
              focal + Offset(progress * 18 * (scale - 1), progress * 35);
          painted.clear();
          await first.moveTo(center - Offset(35 * scale, 0));
          await second.moveTo(center + Offset(35 * scale, 0));
          tester.renderObject(find.byType(_PaintProbe)).markNeedsPaint();
          await tester.pump(const Duration(milliseconds: 16));
          expect(painted, isNotEmpty);
          for (final point in painted) {
            expect(
              (point.dy - center.dy).abs(),
              lessThan(0.01),
              reason: 'Frame $frame must be anchored before painting',
            );
          }
          final virtualRender = render as RenderWorkspaceVirtualText;
          anchorRowHeights.add(
            virtualRender
                .textLayout
                .lines[virtualRender.textLayout.lineAtOffset(anchor.offset)]
                .height!,
          );
          expect(gutter.gutterWidth, closeTo(baseGutterWidth * scale, 0.01));
          expect(
            gutter.lineNumberHeight,
            closeTo(baseLabelHeight * scale, 0.01),
          );
          expect(
            gutter.localToGlobal(Offset.zero).dx,
            closeTo(gutterLeft, 0.01),
          );
          final labels = gutter.visibleLines();
          for (var i = 1; i < labels.length; i++) {
            expect(
              labels[i].centerY - labels[i - 1].centerY,
              greaterThanOrEqualTo(gutter.lineNumberHeight),
              reason: 'Scaled line numbers must not overlap vertically',
            );
          }
          expect(
            _textInsetAfterGutter(tester, render),
            closeTo(left, 0.01),
            reason:
                'The editor must stay flush with the scaled gutter despite horizontal pinch movement',
          );
          final zoom = tester.renderObject<RenderBox>(
            find.byType(WorkspaceTextZoomViewport),
          );
          final fieldBox = tester.renderObject<RenderBox>(
            find.byType(WorkspaceVirtualTextEditor),
          );
          expect(
            fieldBox.localToGlobal(Offset(fieldBox.size.width, 0)).dx,
            closeTo(
              zoom.localToGlobal(Offset(zoom.size.width - 12, 0)).dx,
              0.01,
            ),
            reason: 'Wrapped text must fit the visible width at every scale',
          );
        }
        expect(
          anchorRowHeights[49],
          greaterThan(anchorRowHeights[0] * 2),
          reason: 'Long lines must rewrap as the visible font size increases',
        );
        final releasePoint = painted.last;
        sampling = false;
        await second.up();
        await first.up();
        await tester.pump(const Duration(milliseconds: 16));
        expect(
          (_anchorPoint(render, anchor, fraction) - releasePoint).distance,
          lessThan(0.01),
        );
        await tester.pump(const Duration(milliseconds: 300));
        expect(
          (_anchorPoint(render, anchor, fraction) - releasePoint).distance,
          lessThan(0.01),
        );
        expect(document.text, text);
        expect(document.isDirty, isFalse);
        expect(tester.takeException(), isNull);
      },
      variant: TargetPlatformVariant({
        TargetPlatform.android,
        TargetPlatform.iOS,
      }),
    );
  }

  testWidgets(
    'geometric zoom keeps text hit testing and single-finger scrolling aligned',
    (tester) async {
      final document = WorkspaceTextDocument(
        List.generate(100, (i) => 'line $i with editable text').join('\n'),
      );
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      final scroll = tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .scrollController;
      scroll.jumpTo(650);
      await tester.pump();
      final focal =
          tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
          const Offset(100, 180);
      tester.binding.handlePointerEvent(
        PointerPanZoomStartEvent(pointer: 51, position: focal),
      );
      tester.binding.handlePointerEvent(
        PointerPanZoomUpdateEvent(pointer: 51, position: focal, scale: 1.8),
      );
      tester.binding.handlePointerEvent(
        PointerPanZoomEndEvent(pointer: 51, position: focal),
      );
      await tester.pump();
      final state = tester.state<WorkspaceVirtualTextEditorState>(
        find.byType(WorkspaceVirtualTextEditor),
      );
      final expected = state.renderEditable.getPositionForPoint(focal);
      await tester.tapAt(focal);
      await tester.pumpAndSettle();
      expect(state.widget.focusNode.hasFocus, isTrue);
      expect(state.widget.controller.selection.extentOffset, expected.offset);
      final before = scroll.offset;
      final swipe = await tester.createGesture(
        pointer: 52,
        kind: PointerDeviceKind.touch,
      );
      await swipe.down(focal);
      await swipe.moveBy(const Offset(0, -35));
      await swipe.moveBy(const Offset(0, -70));
      await swipe.up();
      await tester.pumpAndSettle();
      expect(scroll.offset, greaterThan(before));
      expect(document.scale, closeTo(1.8, 0.001));
      expect(document.isDirty, isFalse);
      expect(tester.takeException(), isNull);
    },
  );

  for (final focalY in <double>[90, 330]) {
    testWidgets(
      'pinch keeps the text line under focal y=$focalY through reflow',
      (tester) async {
        final text = List.generate(
          120,
          (i) => 'line $i: ${'content ' * 20}',
        ).join('\n');
        final document = WorkspaceTextDocument(text);
        await tester.pumpWidget(
          _app(_content(_textTab(document), (_, _) async {})),
        );
        await tester.pumpAndSettle();
        final field = tester.widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        );
        final scroll = field.scrollController;
        scroll.jumpTo(1800);
        await tester.pump();
        final render = tester
            .state<WorkspaceVirtualTextEditorState>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .renderEditable;
        final focal =
            tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
            Offset(100, focalY);
        final position = render.getPositionForPoint(focal);
        final fraction =
            (render.globalToLocal(focal).dy -
                _lineCenter(render, position).dy) /
            render.preferredLineHeight;
        final first = await tester.createGesture(
          pointer: 11,
          kind: PointerDeviceKind.touch,
        );
        final second = await tester.createGesture(
          pointer: 12,
          kind: PointerDeviceKind.touch,
        );
        await first.down(focal - const Offset(30, 0));
        await second.down(focal + const Offset(30, 0));
        await tester.pump();
        final selection = field.controller.selection;
        await first.moveTo(focal - const Offset(45, 0));
        await second.moveTo(focal + const Offset(45, 0));
        await tester.pump();
        await tester.pump();
        expect(document.scale, closeTo(1.5, 0.001));
        var anchorY = _anchorPoint(render, position, fraction).dy;
        expect(anchorY, closeTo(focal.dy, 1));
        expect(
          scroll.offset,
          inInclusiveRange(0, scroll.position.maxScrollExtent),
          reason:
              'Virtual extents discover wrapped heights; source geometry is the anchor',
        );
        final movedFocal = focal + const Offset(0, 40);
        await first.moveTo(movedFocal - const Offset(45, 0));
        await second.moveTo(movedFocal + const Offset(45, 0));
        await tester.pump();
        await tester.pump();
        anchorY = _anchorPoint(render, position, fraction).dy;
        expect(anchorY, closeTo(movedFocal.dy, 1));
        await first.moveTo(movedFocal - const Offset(24, 0));
        await second.moveTo(movedFocal + const Offset(24, 0));
        await tester.pump();
        await tester.pump();
        expect(document.scale, closeTo(0.8, 0.001));
        anchorY = _anchorPoint(render, position, fraction).dy;
        expect(anchorY, closeTo(movedFocal.dy, 1));
        expect(field.controller.selection, selection);
        await second.up();
        await first.up();
        await tester.pumpAndSettle();
        expect(document.text, text);
        expect(document.isDirty, isFalse);
        expect(tester.takeException(), isNull);
      },
    );
  }

  testWidgets('trackpad zoom preserves its text anchor while the center pans', (
    tester,
  ) async {
    final document = WorkspaceTextDocument(
      List.generate(120, (i) => 'line $i').join('\n'),
    );
    await tester.pumpWidget(
      _app(_content(_textTab(document), (_, _) async {})),
    );
    await tester.pumpAndSettle();
    final scroll = tester
        .widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .scrollController;
    scroll.jumpTo(650);
    await tester.pump();
    final render = tester
        .state<WorkspaceVirtualTextEditorState>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .renderEditable;
    final focal =
        tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
        const Offset(60, 180);
    final position = render.getPositionForPoint(focal);
    final fraction =
        (render.globalToLocal(focal).dy - _lineCenter(render, position).dy) /
        render.preferredLineHeight;
    tester.binding.handlePointerEvent(
      PointerPanZoomStartEvent(pointer: 20, position: focal),
    );
    tester.binding.handlePointerEvent(
      PointerPanZoomUpdateEvent(
        pointer: 20,
        position: focal,
        scale: 1.7,
        pan: const Offset(0, 35),
      ),
    );
    await tester.pump();
    await tester.pump();
    final anchorY = _anchorPoint(render, position, fraction).dy;
    expect(anchorY, closeTo(focal.dy + 35, 1));
    tester.binding.handlePointerEvent(
      PointerPanZoomEndEvent(pointer: 20, position: focal),
    );
    await tester.pumpAndSettle();
    expect(document.scale, closeTo(1.7, 0.001));
    expect(document.isDirty, isFalse);
    expect(tester.takeException(), isNull);
  });

  testWidgets('single-finger scrolling does not zoom the text', (tester) async {
    final document = WorkspaceTextDocument(
      List.generate(100, (i) => 'line $i').join('\n'),
    );
    await tester.pumpWidget(
      _app(_content(_textTab(document), (_, _) async {})),
    );
    await tester.pumpAndSettle();
    final scrollable = tester.state<ScrollableState>(
      find
          .descendant(
            of: find.byType(WorkspaceVirtualTextEditor),
            matching: find.byType(Scrollable),
          )
          .first,
    );
    final before = scrollable.position.pixels;
    await tester.drag(
      find.byType(WorkspaceVirtualTextEditor),
      const Offset(0, -180),
    );
    await tester.pumpAndSettle();
    expect(scrollable.position.pixels, greaterThan(before));
    expect(document.scale, 1);
    expect(document.isDirty, isFalse);
  });

  testWidgets('draft and zoom survive editor unmount and remount', (
    tester,
  ) async {
    final document = WorkspaceTextDocument('initial');
    final tab = _textTab(document);
    await tester.pumpWidget(_app(_content(tab, (_, _) async {})));
    await tester.pumpAndSettle();
    await _enterText(
      tester,
      find.byType(WorkspaceVirtualTextEditor),
      'persistent draft',
    );
    await _pinchZoom(tester, 1.1);
    await tester.pumpWidget(_app(const SizedBox.shrink()));
    await tester.pumpAndSettle();
    await tester.pumpWidget(_app(_content(tab, (_, _) async {})));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .controller
          .text,
      'persistent draft',
    );
    expect(document.scale, closeTo(1.1, 0.001));
    expect(_visualFontSize(tester), closeTo(15.4, 0.01));
    expect(document.isDirty, isTrue);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'Markdown opens editable and preview toggling retains the draft',
    (tester) async {
      final document = WorkspaceTextDocument('# Heading');
      await tester.pumpWidget(
        _app(
          _content(
            _textTab(document, kind: WorkspaceFilePreviewKind.markdown),
            (_, _) async {},
          ),
        ),
      );
      await tester.pumpAndSettle();
      await _enterText(
        tester,
        find.byType(WorkspaceVirtualTextEditor),
        '# Edited heading',
      );
      await tester.tap(find.byTooltip('Markdown 预览'));
      await tester.pumpAndSettle();
      expect(find.byType(StreamMarkdownRenderer), findsOneWidget);
      expect(find.byIcon(Icons.open_in_browser), findsNothing);
      await tester.tap(find.byTooltip('编辑'));
      await tester.pumpAndSettle();
      expect(
        tester
            .widget<WorkspaceVirtualTextEditor>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .controller
            .text,
        '# Edited heading',
      );
      expect(document.isDirty, isTrue);
    },
  );

  testWidgets('Markdown zoom corrects the scroll viewport before painting', (
    tester,
  ) async {
    final paragraphs = List.generate(
      90,
      (i) => 'Markdown paragraph $i 中文 👩‍💻 ${'fixed wrapping ' * 8}',
    );
    final document = WorkspaceTextDocument(paragraphs.join('\n\n'));

    /// Creates a paragraph event using the real shared Markdown protocol.
    core_proxy.MarkdownStreamEvent event(
      String type, {
      int? blockId,
      int? inlineId,
      String? value,
    }) => core_proxy.MarkdownStreamEvent(
      chatId: 'workspace-zoom-test',
      eventType: type,
      value: value,
      id: null,
      blockId: blockId,
      inlineId: inlineId,
      parentBlockId: null,
      nodeType: null,
      headerLevel: null,
      xml: null,
    );

    late RenderBox contentBox;
    late Offset scenePoint;
    var sampling = false;
    final painted = <Offset>[];
    await tester.pumpWidget(
      _app(
        _PaintProbe(
          onPaint: () {
            if (sampling) painted.add(contentBox.localToGlobal(scenePoint));
          },
          child: WorkspaceTextPreview(
            tab: _textTab(document, kind: WorkspaceFilePreviewKind.markdown),
            onWriteWorkspaceFileBytes: (_, _) async {},
            onOpenBrowser: ({url, localFilePath, workspaceHtmlPath}) =>
                throw StateError('Unexpected browser open'),
            splitMarkdownContent: (_) async => [
              for (var i = 0; i < paragraphs.length; i++) ...[
                event('markdownBlockStart', blockId: i),
                event('markdownInlineStart', blockId: i, inlineId: i),
                event(
                  'markdownInlineChunk',
                  blockId: i,
                  inlineId: i,
                  value: paragraphs[i],
                ),
                event('markdownBlockEnd', blockId: i),
              ],
              event('completed'),
            ],
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.byTooltip('Markdown 预览'));
    await tester.pumpAndSettle();
    final scrollView = tester.widget<SingleChildScrollView>(
      find.ancestor(
        of: find.byType(StreamMarkdownRenderer),
        matching: find.byType(SingleChildScrollView),
      ),
    );
    final scroll = scrollView.controller!;
    expect(scroll.position.maxScrollExtent, greaterThan(1000));
    scroll.jumpTo(800);
    await tester.pump();
    contentBox = tester.renderObject(find.byType(StreamMarkdownRenderer));
    final extent =
        scroll.position.maxScrollExtent + scroll.position.viewportDimension;
    final focal =
        tester.getTopLeft(find.byWidget(scrollView)) + const Offset(160, 230);
    scenePoint = contentBox.globalToLocal(focal);
    tester.binding.handlePointerEvent(
      PointerPanZoomStartEvent(pointer: 60, position: focal),
    );
    await tester.pump();
    sampling = true;
    for (var frame = 0; frame < 40; frame++) {
      final progress = frame < 20 ? frame / 19 : (39 - frame) / 19;
      final scale = 0.7 + progress * 1.5;
      final pan = Offset(progress * 18 * (scale - 1), progress * 25);
      painted.clear();
      tester.binding.handlePointerEvent(
        PointerPanZoomUpdateEvent(
          pointer: 60,
          position: focal,
          scale: scale,
          pan: pan,
        ),
      );
      tester.renderObject(find.byType(_PaintProbe)).markNeedsPaint();
      await tester.pump(const Duration(milliseconds: 16));
      expect(painted, isNotEmpty);
      for (final point in painted) {
        expect(
          (point.dy - (focal.dy + pan.dy)).abs(),
          lessThan(0.01),
          reason: 'Markdown frame $frame must be corrected before painting',
        );
      }
      expect(
        scroll.position.maxScrollExtent + scroll.position.viewportDimension,
        closeTo(extent, 0.01),
      );
    }
    final releasePoint = painted.last;
    sampling = false;
    tester.binding.handlePointerEvent(
      PointerPanZoomEndEvent(pointer: 60, position: focal),
    );
    await tester.pumpAndSettle();
    expect(
      (contentBox.localToGlobal(scenePoint) - releasePoint).distance,
      lessThan(0.01),
    );
    await tester.tap(find.byTooltip('编辑'));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .controller
          .text,
      document.text,
    );
    expect(document.isDirty, isFalse);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'horizontal zoom bounds do not disturb the painted vertical anchor',
    (tester) async {
      tester.view.devicePixelRatio = 3;
      tester.view.physicalSize = const Size(1170, 2532);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final document = WorkspaceTextDocument(
        List.generate(140, (i) => 'boundaries line $i').join('\n'),
      );
      late RenderEditable render;
      late ScrollController scroll;
      late TextPosition anchor;
      late double fraction;
      var sampling = false;
      final painted = <Offset>[];
      await tester.pumpWidget(
        _app(
          _PaintProbe(
            onPaint: () {
              if (sampling) {
                painted.add(_anchorPoint(render, anchor, fraction));
              }
            },
            child: _content(_textTab(document), (_, _) async {}),
          ),
        ),
      );
      await tester.pumpAndSettle();
      scroll = tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .scrollController;
      scroll.jumpTo(800);
      await tester.pump();
      render = tester
          .state<WorkspaceVirtualTextEditorState>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .renderEditable;
      final viewport = tester.getRect(find.byType(WorkspaceVirtualTextEditor));
      final focal = viewport.topLeft + const Offset(160, 250);
      anchor = render.getPositionForPoint(focal);
      fraction =
          (render.globalToLocal(focal).dy - _lineCenter(render, anchor).dy) /
          render.preferredLineHeight;
      final left = _textInsetAfterGutter(tester, render);
      tester.binding.handlePointerEvent(
        PointerPanZoomStartEvent(pointer: 62, position: focal),
      );
      await tester.pump();
      sampling = true;
      for (final scale in <double>[
        0.6,
        0.99,
        1,
        1.01,
        2.5,
        1.01,
        1,
        0.99,
        0.6,
      ]) {
        for (final panX in <double>[-160, 160]) {
          painted.clear();
          tester.binding.handlePointerEvent(
            PointerPanZoomUpdateEvent(
              pointer: 62,
              position: focal,
              scale: scale,
              pan: Offset(panX, 25),
            ),
          );
          tester.renderObject(find.byType(_PaintProbe)).markNeedsPaint();
          await tester.pump(const Duration(milliseconds: 16));
          expect(_textInsetAfterGutter(tester, render), closeTo(left, 0.01));
          expect(painted, isNotEmpty);
          for (final point in painted) {
            expect(point.dy, closeTo(focal.dy + 25, 0.01));
          }
        }
      }
      final releasePoint = painted.last;
      sampling = false;
      tester.binding.handlePointerEvent(
        PointerPanZoomEndEvent(pointer: 62, position: focal),
      );
      await tester.pumpAndSettle();
      expect(
        (_anchorPoint(render, anchor, fraction) - releasePoint).distance,
        lessThan(0.01),
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('zoom layout composes with ancestor fitted application zoom', (
    tester,
  ) async {
    final document = WorkspaceTextDocument(
      List.generate(120, (i) => 'app zoom line $i').join('\n'),
    );
    late RenderEditable render;
    late ScrollController scroll;
    late TextPosition anchor;
    late double fraction;
    var sampling = false;
    final painted = <Offset>[];
    await tester.pumpWidget(
      _app(
        FittedBox(
          fit: BoxFit.fill,
          alignment: Alignment.topLeft,
          child: SizedBox(
            width: 1000,
            height: 750,
            child: _PaintProbe(
              onPaint: () {
                if (sampling) {
                  painted.add(_anchorPoint(render, anchor, fraction));
                }
              },
              child: _content(_textTab(document), (_, _) async {}),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    scroll = tester
        .widget<WorkspaceVirtualTextEditor>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .scrollController;
    scroll.jumpTo(800);
    await tester.pump();
    render = tester
        .state<WorkspaceVirtualTextEditorState>(
          find.byType(WorkspaceVirtualTextEditor),
        )
        .renderEditable;
    final focal =
        tester.getTopLeft(find.byType(WorkspaceVirtualTextEditor)) +
        const Offset(170, 200);
    anchor = render.getPositionForPoint(focal);
    fraction =
        (render.globalToLocal(focal).dy - _lineCenter(render, anchor).dy) /
        render.preferredLineHeight;
    final left = _textInsetAfterGutter(tester, render);
    tester.binding.handlePointerEvent(
      PointerPanZoomStartEvent(pointer: 61, position: focal),
    );
    await tester.pump();
    sampling = true;
    tester.binding.handlePointerEvent(
      PointerPanZoomUpdateEvent(
        pointer: 61,
        position: focal,
        scale: 1.7,
        pan: const Offset(15, -20),
      ),
    );
    tester.renderObject(find.byType(_PaintProbe)).markNeedsPaint();
    await tester.pump();
    expect(_textInsetAfterGutter(tester, render), closeTo(left, 0.01));
    expect(painted, isNotEmpty);
    for (final point in painted) {
      expect((point.dy - (focal.dy - 20)).abs(), lessThan(0.01));
    }
    sampling = false;
    tester.binding.handlePointerEvent(
      PointerPanZoomEndEvent(pointer: 61, position: focal),
    );
    await tester.pumpAndSettle();
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'line numbers follow logical lines through wrapping, scrolling and edits',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(390, 844);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final lines = <String>[
        'long first line ${'中文 e\u0301 👩‍💻 ' * 18}',
        '',
        for (var i = 2; i < 120; i++) 'source line $i',
        '',
      ];
      final document = WorkspaceTextDocument(lines.join('\r\n'));
      await tester.pumpWidget(
        _app(_content(_textTab(document), (_, _) async {})),
      );
      await tester.pumpAndSettle();
      final gutter = tester.renderObject<RenderWorkspaceTextLineNumbers>(
        find.byType(WorkspaceTextLineNumbers),
      );
      final render = tester
          .state<WorkspaceVirtualTextEditorState>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .renderEditable;
      final scroll = tester
          .widget<WorkspaceVirtualTextEditor>(
            find.byType(WorkspaceVirtualTextEditor),
          )
          .scrollController;
      final starts = <int>[
        0,
        for (final match in RegExp(r'\r\n').allMatches(document.text))
          match.end,
      ];
      final gutterWidth = gutter.gutterWidth;
      final labelHeight = gutter.lineNumberHeight;

      /// Checks each rendered label against the editor's first visual row.
      void checkLabels() {
        expect(gutter.lineCount, lines.length);
        expect(gutter.gutterWidth, closeTo(gutterWidth * document.scale, 0.01));
        expect(
          gutter.lineNumberHeight,
          closeTo(labelHeight * document.scale, 0.01),
        );
        final visible = gutter.visibleLines();
        expect(visible, isNotEmpty);
        for (final line in visible) {
          final actual = _lineCenter(
            render,
            TextPosition(offset: starts[line.number - 1]),
          );
          final center = gutter.globalToLocal(render.localToGlobal(actual));
          expect(line.centerY, closeTo(center.dy, 0.01));
        }
      }

      checkLabels();
      final first = gutter.visibleLines();
      expect(first[0].number, 1);
      expect(first[1].number, 2);
      expect(
        first[1].centerY - first[0].centerY,
        greaterThan(render.preferredLineHeight * 3),
        reason: 'Soft wraps must not create extra source line numbers',
      );
      scroll.jumpTo(600);
      await tester.pump();
      checkLabels();
      expect(gutter.visibleLines().first.number, greaterThan(1));
      await _pinchZoom(tester, 1.1);
      await tester.pumpAndSettle();
      checkLabels();
      await _pinchZoom(tester, 1 / 1.1);
      await tester.pumpAndSettle();
      checkLabels();
      await _enterText(
        tester,
        find.byType(WorkspaceVirtualTextEditor),
        'one\n\nthree\n',
      );
      await tester.pumpAndSettle();
      expect(gutter.lineCount, 4);
      expect(gutter.visibleLines().map((line) => line.number), [1, 2, 3, 4]);
      await _enterText(tester, find.byType(WorkspaceVirtualTextEditor), '');
      await tester.pumpAndSettle();
      expect(gutter.lineCount, 1);
      expect(gutter.visibleLines().single.number, 1);
      expect(tester.takeException(), isNull);
    },
  );

  for (final text in <String>[
    '',
    'one line',
    '${'long content ' * 40}\n' * 100,
  ]) {
    testWidgets(
      'wrapped zoom clamps scale and document edges for ${text.length} characters',
      (tester) async {
        final document = WorkspaceTextDocument(text);
        await tester.pumpWidget(
          _app(_content(_textTab(document), (_, _) async {})),
        );
        await tester.pumpAndSettle();
        final render = tester
            .state<WorkspaceVirtualTextEditorState>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .renderEditable;
        final scroll = tester
            .widget<WorkspaceVirtualTextEditor>(
              find.byType(WorkspaceVirtualTextEditor),
            )
            .scrollController;
        final left = _textInsetAfterGutter(tester, render);
        var pointer = 90;
        for (final bottom in <bool>[false, true]) {
          scroll.jumpTo(bottom ? scroll.position.maxScrollExtent : 0);
          await tester.pump();
          final focal = tester.getCenter(
            find.byType(WorkspaceTextZoomViewport),
          );
          final startScale = document.scale;
          tester.binding.handlePointerEvent(
            PointerPanZoomStartEvent(pointer: pointer, position: focal),
          );
          for (final scale in <double>[0.01, 100, 0.01, 1 / startScale]) {
            tester.binding.handlePointerEvent(
              PointerPanZoomUpdateEvent(
                pointer: pointer,
                position: focal,
                scale: scale,
                pan: const Offset(300, 100),
              ),
            );
            await tester.pump();
            expect(
              document.scale,
              closeTo((startScale * scale).clamp(0.6, 2.5), 0.001),
            );
            expect(_textInsetAfterGutter(tester, render), closeTo(left, 0.01));
            expect(
              scroll.offset,
              inInclusiveRange(0, scroll.position.maxScrollExtent),
            );
          }
          final offset = scroll.offset;
          tester.binding.handlePointerEvent(
            PointerPanZoomEndEvent(pointer: pointer, position: focal),
          );
          await tester.pumpAndSettle();
          expect(scroll.offset, closeTo(offset, 0.01));
          pointer++;
        }
        expect(document.text, text);
        expect(document.isDirty, isFalse);
        expect(tester.takeException(), isNull);
      },
    );
  }

  for (final width in <double>[390, 1024]) {
    testWidgets('tab swipes scroll and long presses dock at width $width', (
      tester,
    ) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = Size(width, 600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final tabs = <WorkspaceTab>[
        const WorkspaceTab(
          kind: WorkspaceTabKind.home,
          title: 'Home',
          icon: Icons.home,
          closable: false,
        ),
        for (var i = 0; i < 12; i++)
          WorkspaceTab(
            kind: WorkspaceTabKind.filePreview,
            title: 'config-file-$i.json',
            icon: Icons.description_outlined,
          ),
        const WorkspaceTab(
          kind: WorkspaceTabKind.plugin,
          title: 'Plugin',
          icon: Icons.extension,
          pluginEntryId: 'plugin-test',
        ),
      ];
      WorkspaceTabDragPayload? dropped;
      await tester.pumpWidget(
        _app(
          Column(
            children: <Widget>[
              WorkspaceTabStrip(
                tabs: tabs,
                selectedIndex: 1,
                onSelected: (_) {},
                onClosed: (_) {},
              ),
              Expanded(
                child: DragTarget<WorkspaceTabDragPayload>(
                  onAcceptWithDetails: (details) => dropped = details.data,
                  builder: (_, _, _) =>
                      const SizedBox.expand(key: ValueKey('drop-target')),
                ),
              ),
            ],
          ),
        ),
      );
      await tester.pumpAndSettle();
      final strip = find.byType(SingleChildScrollView);
      final scrollable = tester.state<ScrollableState>(
        find.descendant(of: strip, matching: find.byType(Scrollable)),
      );
      final start = tester.getCenter(find.text('config-file-0.json'));
      final swipe = await tester.createGesture(kind: PointerDeviceKind.touch);
      await swipe.down(start);
      await swipe.moveBy(const Offset(-60, 0));
      await tester.pump();
      await swipe.moveBy(const Offset(-60, 0));
      await tester.pump();
      expect(scrollable.position.pixels, greaterThan(0));
      expect(dropped, isNull);
      expect(find.text('config-file-0.json'), findsOneWidget);
      await swipe.up();
      await tester.pumpAndSettle();
      scrollable.position.jumpTo(0);
      await tester.pumpAndSettle();
      final hold = await tester.createGesture(kind: PointerDeviceKind.touch);
      await hold.down(tester.getCenter(find.text('config-file-0.json')));
      await tester.pump(kLongPressTimeout + const Duration(milliseconds: 20));
      await hold.moveTo(
        tester.getCenter(find.byKey(const ValueKey('drop-target'))),
      );
      await tester.pump();
      await hold.up();
      await tester.pumpAndSettle();
      expect(dropped!.tab, same(tabs[1]));
      scrollable.position.jumpTo(scrollable.position.maxScrollExtent);
      await tester.pumpAndSettle();
      expect(
        find.byType(LongPressDraggable<SidebarDockDragPayload>),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets(
    'long titles cap tab width and selecting an offscreen tab reveals it',
    (tester) async {
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = const Size(1024, 600);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      final tabs = <WorkspaceTab>[
        const WorkspaceTab(
          kind: WorkspaceTabKind.home,
          title: 'Home',
          icon: Icons.home,
          closable: false,
        ),
        for (var i = 0; i < 6; i++)
          WorkspaceTab(
            kind: WorkspaceTabKind.terminal,
            title: 'super_admin_default_session_bash_20368a0$i',
            icon: Icons.terminal,
          ),
      ];
      Widget strip({required int selectedIndex}) => SizedBox(
        width: 360,
        child: WorkspaceTabStrip(
          tabs: tabs,
          selectedIndex: selectedIndex,
          onSelected: (_) {},
          onClosed: (_) {},
        ),
      );
      await tester.pumpWidget(_app(strip(selectedIndex: 0)));
      await tester.pumpAndSettle();
      final lastTitle = tabs.last.title;
      final lastButton = find.ancestor(
        of: find.text(lastTitle),
        matching: find.byType(LongPressDraggable<WorkspaceTabDragPayload>),
      );
      expect(tester.getSize(lastButton).width, lessThanOrEqualTo(220));

      await tester.pumpWidget(_app(strip(selectedIndex: tabs.length - 1)));
      await tester.pumpAndSettle();
      final stripRect = tester.getRect(find.byType(SingleChildScrollView));
      final revealedTitle = tester.getRect(find.text(lastTitle));
      expect(revealedTitle.left, greaterThanOrEqualTo(stripRect.left));
      expect(revealedTitle.right, lessThanOrEqualTo(stripRect.right));
    },
  );

  for (final action in <String>['保存', '放弃修改']) {
    testWidgets('dirty close requires an explicit $action decision', (
      tester,
    ) async {
      final document = WorkspaceTextDocument('initial')..updateText('draft');
      bool? confirmed;
      final writes = <String>[];
      await tester.pumpWidget(
        _app(
          Builder(
            builder: (context) => TextButton(
              onPressed: () async {
                confirmed = await confirmWorkspaceTextClose(
                  context,
                  document: document,
                  write: (text) async => writes.add(text),
                );
              },
              child: const Text('Close file'),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.text('Close file'));
      await tester.pumpAndSettle();
      expect(confirmed, isNull);
      await tester.tap(find.text(action));
      await tester.pumpAndSettle();
      expect(confirmed, isTrue);
      expect(find.byType(AlertDialog), findsNothing);
      expect(writes, action == '保存' ? ['draft'] : isEmpty);
      expect(document.isDirty, action != '保存');
    });
  }

  testWidgets('dirty close supports cancel and keeps a failed save open', (
    tester,
  ) async {
    final document = WorkspaceTextDocument('initial')..updateText('draft');
    bool? confirmed;
    await tester.pumpWidget(
      _app(
        Builder(
          builder: (context) => TextButton(
            onPressed: () async {
              confirmed = await confirmWorkspaceTextClose(
                context,
                document: document,
                write: (_) async => throw StateError('write denied'),
              );
            },
            child: const Text('Close file'),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    await tester.tap(find.text('Close file'));
    await tester.pumpAndSettle();
    await tester.tap(find.text('保存'));
    await tester.pumpAndSettle();
    expect(find.byType(AlertDialog), findsOneWidget);
    expect(find.text('Bad state: write denied'), findsOneWidget);
    expect(confirmed, isNull);
    await tester.tap(find.text('取消'));
    await tester.pumpAndSettle();
    expect(confirmed, isFalse);
    expect(document.isDirty, isTrue);
  });
}

/// Samples text geometry during paint rather than after post-frame callbacks.
class _PaintProbe extends SingleChildRenderObjectWidget {
  /// Creates a paint observer around the actual editor.
  const _PaintProbe({required this.onPaint, required super.child});

  final VoidCallback onPaint;

  /// Installs the geometry observer in the rendering pipeline.
  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderPaintProbe(onPaint);

  /// Updates the observer without remounting the editor.
  @override
  void updateRenderObject(
    BuildContext context,
    covariant _RenderPaintProbe renderObject,
  ) {
    renderObject.onPaint = onPaint;
  }
}

class _RenderPaintProbe extends RenderProxyBox {
  /// Creates a probe that samples geometry before its child is painted.
  _RenderPaintProbe(this.onPaint);

  VoidCallback onPaint;

  /// Records exactly the geometry the user will see in this frame.
  @override
  void paint(PaintingContext context, Offset offset) {
    onPaint();
    super.paint(context, offset);
  }
}

/// Measures the displayed font size through the same transform used for input.
double _visualFontSize(WidgetTester tester) {
  final field = tester.widget<WorkspaceVirtualTextEditor>(
    find.byType(WorkspaceVirtualTextEditor),
  );
  final render = tester
      .state<WorkspaceVirtualTextEditorState>(
        find.byType(WorkspaceVirtualTextEditor),
      )
      .renderEditable;
  return field.style.fontSize! *
      render.getTransformTo(null).getMaxScaleOnAxis();
}

/// Measures an anchored sub-line point in the actual painted coordinate space.
Offset _anchorPoint(
  RenderEditable render,
  TextPosition position,
  double fraction,
) {
  return render.localToGlobal(
    _lineCenter(render, position) +
        Offset(0, fraction * render.preferredLineHeight),
  );
}

/// Reads a line center without pixel snapping or font-dependent caret heights.
Offset _lineCenter(RenderEditable render, TextPosition position) =>
    render
        .getEndpointsForSelection(TextSelection.fromPosition(position))
        .single
        .point -
    Offset(0, render.preferredLineHeight / 2);

/// Measures text padding after the gutter in unzoomed application coordinates.
double _textInsetAfterGutter(WidgetTester tester, RenderEditable render) {
  final gutter = tester.renderObject<RenderWorkspaceTextLineNumbers>(
    find.byType(WorkspaceTextLineNumbers),
  );
  return gutter.globalToLocal(render.localToGlobal(Offset.zero)).dx -
      gutter.gutterWidth;
}

/// Exercises gesture zoom independently of toolbar actions.
Future<void> _pinchZoom(WidgetTester tester, double factor) async {
  final focal = tester.getCenter(find.byType(WorkspaceTextZoomViewport));
  tester.binding.handlePointerEvent(
    PointerPanZoomStartEvent(pointer: 180, position: focal),
  );
  tester.binding.handlePointerEvent(
    PointerPanZoomUpdateEvent(pointer: 180, position: focal, scale: factor),
  );
  await tester.pump();
  tester.binding.handlePointerEvent(
    PointerPanZoomEndEvent(pointer: 180, position: focal),
  );
  await tester.pump();
}

/// Opens the full-document virtual editor input connection for widget tests.
Future<void> _showKeyboard(WidgetTester tester, Finder finder) async {
  tester.state<WorkspaceVirtualTextEditorState>(finder).requestKeyboard();
  await tester.pump();
}

/// Sends native full-document input without relying on an invisible TextField.
Future<void> _enterText(WidgetTester tester, Finder finder, String text) async {
  await _showKeyboard(tester, finder);
  tester.testTextInput.enterText(text);
  await tester.pump();
}
