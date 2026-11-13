import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/markdown/MarkdownFileScope.dart';
import 'package:operit2/ui/common/markdown/MarkdownInlineSpannable.dart';
import 'package:operit2/ui/common/markdown/MarkdownLink.dart';
import 'package:operit2/ui/common/markdown/MarkdownNodeGrouper.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceSession.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';

void main() {
  test('file destinations support relative paths and virtual file URIs', () {
    for (final (source, expected) in [
      ('xxx.txt', 'xxx.txt'),
      ('../src/main.dart#L12', '../src/main.dart'),
      ('./中文%20文件.txt', '中文 文件.txt'),
      ('file:///mnt/project/中文%20文件.txt', '/mnt/project/中文 文件.txt'),
      ('vfs:///mnt/project/report.pdf', '/mnt/project/report.pdf'),
      ('operit-vfs://mnt/project/archive.zip', '/mnt/project/archive.zip'),
      (
        'origin:/operitapp/outputs/report.txt',
        'origin:/operitapp/outputs/report.txt',
      ),
    ]) {
      expect(markdownFileLinkPath(source), expected, reason: source);
    }
    for (final source in [
      '',
      '#section',
      '?query=only',
      'https://example.org/a.txt',
      'http://example.org/a.pdf',
      '//example.org/file.txt',
      'mailto:test@example.org',
      'javascript:alert(1)',
      'data:text/plain,test',
      'file://remote-host/file.txt',
    ]) {
      expect(markdownFileLinkPath(source), isNull, reason: source);
    }
  });

  for (final (source, expected, kind) in [
    (
      'xxx.txt',
      '/operitapp/outputs/old/xxx.txt',
      WorkspaceFilePreviewKind.text,
    ),
    (
      '../src/main.dart',
      '/operitapp/outputs/src/main.dart',
      WorkspaceFilePreviewKind.text,
    ),
    (
      './中文%20文件.txt',
      '/operitapp/outputs/old/中文 文件.txt',
      WorkspaceFilePreviewKind.text,
    ),
    (
      './literal%2520%23%3F.txt',
      '/operitapp/outputs/old/literal%20#?.txt',
      WorkspaceFilePreviewKind.text,
    ),
    (
      '%23notes.txt',
      '/operitapp/outputs/old/#notes.txt',
      WorkspaceFilePreviewKind.text,
    ),
    (
      '/mnt/project/README.md',
      '/mnt/project/README.md',
      WorkspaceFilePreviewKind.markdown,
    ),
    (
      'file:///mnt/project/report.pdf',
      '/mnt/project/report.pdf',
      WorkspaceFilePreviewKind.pdf,
    ),
    (
      'vfs:///mnt/project/archive.zip',
      '/mnt/project/archive.zip',
      WorkspaceFilePreviewKind.binary,
    ),
  ]) {
    testWidgets('clicking $source opens its workspace preview', (tester) async {
      final session = WorkspaceSession();
      addTearDown(session.dispose);
      final gestures = MarkdownLinkGestureOwner();
      addTearDown(gestures.dispose);
      final reads = <String>[];
      final opened = <String>[];
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MarkdownLinkScope(
              onOpenFile: (path) async {
                opened.add(path);
                await session.openFile(
                  path: path,
                  relativePath: path,
                  readText: (path) async {
                    reads.add(path);
                    return 'original contents';
                  },
                );
              },
              child: MarkdownFileScope(
                basePath: '/operitapp/outputs/old',
                nodeId: 'origin',
                clients: GeneratedCoreProxyClients(_NoCallsBridge()),
                child: Builder(
                  builder: (context) => Text.rich(
                    buildMarkdownInlineSpannableFromMarkdownNodes(
                      context: context,
                      children: [
                        MarkdownNodeStable(
                          type: MarkdownNodeType.link,
                          content: '',
                          isStreaming: false,
                          linkUrl: source,
                          children: const [
                            MarkdownNodeStable(
                              type: MarkdownNodeType.bold,
                              content: '打开文件',
                              isStreaming: false,
                            ),
                          ],
                        ),
                      ],
                      textColor: Colors.black,
                      gestures: gestures,
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.tap(find.text('打开文件', findRichText: true));
      await tester.pumpAndSettle();
      expect(opened, [expected]);
      expect(session.tabs, hasLength(2));
      final tab = session.tabs.last;
      expect(tab.absolutePath, expected);
      expect(tab.filePath, expected);
      expect(tab.previewKind, kind);
      if (tab.textDocument != null) {
        expect(reads, [expected]);
        tab.textDocument!.updateText('unsaved draft');
      } else {
        expect(reads, isEmpty);
      }
      // Reopening a preview must keep its unsaved draft and avoid another read.
      session.selectedIndex = 0;
      await tester.tap(find.text('打开文件', findRichText: true));
      await tester.pumpAndSettle();
      expect(session.tabs, hasLength(2));
      expect(session.selectedIndex, 1);
      expect(session.tabs.last, same(tab));
      expect(
        tab.textDocument?.text,
        tab.textDocument == null ? null : 'unsaved draft',
      );
      expect(reads.length, tab.textDocument == null ? 0 : 1);
      expect(tester.takeException(), isNull);
    });
  }

  test(
    'reopening a file in the secondary pane retains that pane and draft',
    () async {
      final session = WorkspaceSession();
      addTearDown(session.dispose);
      await session.openFile(
        path: '/mnt/project/file.txt',
        relativePath: 'project/file.txt',
        readText: (_) async => 'draft',
      );
      final tab = session.tabs.removeLast();
      session.selectedIndex = 0;
      session.secondaryTabs.add(tab);
      session.secondarySelectedIndex = -1;
      await session.openFile(
        path: tab.absolutePath!,
        relativePath: tab.absolutePath!,
        readText: (_) async => throw StateError('must reuse the draft'),
      );
      expect(session.tabs, hasLength(1));
      expect(session.secondarySelectedIndex, 0);
      expect(session.secondaryTabs.single, same(tab));
    },
  );

  test('concurrent file reads create only one preview tab', () async {
    final session = WorkspaceSession();
    addTearDown(session.dispose);
    final contents = Completer<String>();
    final opens = List.generate(
      2,
      (_) => session.openFile(
        path: '/mnt/project/file.txt',
        relativePath: 'project/file.txt',
        readText: (_) => contents.future,
      ),
    );
    contents.complete('contents');
    await Future.wait(opens);
    expect(session.tabs, hasLength(2));
    expect(session.tabs.last.textDocument!.text, 'contents');
  });
}

/// A file click routed to the workspace must never invoke an external tool.
class _NoCallsBridge extends OperitRuntimeBridge {
  @override
  dynamic noSuchMethod(Invocation invocation) =>
      throw StateError('Unexpected Core call: ${invocation.memberName}');
}
