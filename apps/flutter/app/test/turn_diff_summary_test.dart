import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart'
    hide WorkspaceFileEntry;
import 'package:operit2/ui/features/chat/components/WorkspaceShell.dart';
import 'package:operit2/ui/features/chat/components/part/TurnDiffSummary.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceDiffContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceGitChanges.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceDiffReview.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceInlineDiff.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceSession.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';
import 'package:operit2/ui/features/chat/viewmodel/WorkspaceFileModels.dart';

void main() {
  test('collects edit_file diffs for one completed turn', () {
    final review = reviewForMessage(
      _message(
        parts: <MessagePart>[
          MessagePart(
            partId: 'result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content:
                '<file-diff path="lib/a.dart" details="updated"><![CDATA['
                '@@ -1,1 +1,1 @@\n-old\n+fresh line\n]]></file-diff>',
            toolCallId: 'call-1',
            toolName: 'edit_file',
            attributes: const <String, String>{'status': 'success'},
          ),
        ],
      ),
    );

    expect(review.files, hasLength(1));
    expect(review.files.single.path, 'lib/a.dart');
    expect(review.additions, 1);
    expect(review.deletions, 1);
  });

  test('keeps only unified hunks and reads git status records', () {
    final hunks = parseUnifiedHunks(
      'file tail that is not a hunk\n'
      '@@ -1,1 +1,1 @@\n'
      '-old\n'
      '+fresh line\n',
    );
    expect(hunks, hasLength(1));
    expect(hunks.single.last.text, 'fresh line');

    final entries = parseGitPorcelain(' M a.dart\u0000?? new.txt\u0000');
    expect(entries.map((entry) => entry.status), <String>['M', 'U']);
    expect(entries.map((entry) => entry.path), <String>['a.dart', 'new.txt']);
  });

  test('computes intra-line segments for paired removed/added rows', () {
    final document = WorkspaceDiffDocument.parse(
      '@@ -10,3 +10,3 @@\n'
      ' context\n'
      '-final count = 1;\n'
      '+final count = 42;\n',
    );
    final removed = document.hunks.single.rows[1];
    final added = document.hunks.single.rows[2];
    expect(removed.kind, InlineDiffKind.remove);
    expect(added.kind, InlineDiffKind.add);
    expect(
      removed.text.substring(
        removed.segments.single.start,
        removed.segments.single.end,
      ),
      '1',
    );
    expect(
      added.text.substring(
        added.segments.single.start,
        added.segments.single.end,
      ),
      '42',
    );
  });

  testWidgets('folds unchanged regions between hunks', (tester) async {
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 800,
            height: 400,
            child: WorkspaceInlineDiff(
              diff:
                  '@@ -1,2 +1,2 @@\n'
                  ' a\n'
                  '-x\n'
                  '+y\n'
                  '@@ -30,2 +30,2 @@\n'
                  ' b\n'
                  '-p\n'
                  '+q\n',
            ),
          ),
        ),
      ),
    );
    expect(find.text('27 行未更改'), findsOneWidget);
    expect(find.text('x'), findsOneWidget);
    expect(find.text('q'), findsOneWidget);
  });

  testWidgets('opens the workspace diff from the turn summary', (tester) async {
    final review = reviewForMessage(
      _message(
        parts: <MessagePart>[
          MessagePart(
            partId: 'result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content:
                '<file-diff path="lib/a.dart" details="updated"><![CDATA['
                '@@ -1,1 +1,1 @@\n-old\n+fresh line\n]]></file-diff>',
            toolCallId: 'call-1',
            toolName: 'edit_file',
            attributes: const <String, String>{'status': 'success'},
          ),
        ],
      ),
    );
    var opened = false;
    final previous = WorkspaceShell.session.onOpenRequested;
    WorkspaceShell.session.onOpenRequested = () => opened = true;
    addTearDown(() {
      WorkspaceShell.session.onOpenRequested = previous;
      WorkspaceShell.session.tabs.removeWhere(
        (tab) => tab.kind == WorkspaceTabKind.diff,
      );
      if (WorkspaceShell.session.selectedIndex >=
          WorkspaceShell.session.tabs.length) {
        WorkspaceShell.session.selectedIndex = 0;
      }
    });

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: TurnDiffSummary(review: review, onUndo: () {}),
        ),
      ),
    );

    expect(find.text('已编辑 1 个文件'), findsOneWidget);
    expect(find.text('lib/a.dart'), findsOneWidget);

    await tester.tap(find.text('查看变更'));
    await tester.pump();

    expect(opened, isTrue);
    expect(
      WorkspaceShell.session.tabs.where(
        (tab) => tab.kind == WorkspaceTabKind.diff,
      ),
      hasLength(1),
    );
    expect(
      WorkspaceShell.session.tabs[WorkspaceShell.session.selectedIndex].kind,
      WorkspaceTabKind.diff,
    );
  });

  testWidgets('renders the selected file diff in the workspace pane', (
    tester,
  ) async {
    final review = reviewForMessage(
      _message(
        parts: <MessagePart>[
          MessagePart(
            partId: 'result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content:
                '<file-diff path="lib/a.dart" details="updated"><![CDATA['
                '@@ -1,1 +1,1 @@\n-old\n+fresh line\n]]></file-diff>',
            toolCallId: 'call-1',
            toolName: 'edit_file',
            attributes: const <String, String>{'status': 'success'},
          ),
        ],
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 800,
            height: 400,
            child: WorkspaceDiffContent(review: review),
          ),
        ),
      ),
    );

    expect(find.text('fresh line'), findsOneWidget);
    expect(find.text('上一轮改动'), findsOneWidget);
    expect(find.text('工作区文件'), findsOneWidget);
  });

  testWidgets('shows a file tree selection instead of the locked diff', (
    tester,
  ) async {
    final review = reviewForMessage(
      _message(
        parts: <MessagePart>[
          MessagePart(
            partId: 'result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content:
                '<file-diff path="lib/a.dart" details="updated"><![CDATA['
                '@@ -1,1 +1,1 @@\n-old\n+fresh line\n]]></file-diff>',
            toolCallId: 'call-1',
            toolName: 'edit_file',
            attributes: const <String, String>{'status': 'success'},
          ),
        ],
      ),
    );

    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 800,
            height: 500,
            child: WorkspaceDiffContent(
              review: review,
              onListWorkspaceFiles: (path) async {
                if (path.isEmpty) {
                  return <WorkspaceFileEntry>[
                    WorkspaceFileEntry(
                      name: 'notes',
                      path: r'D:\work\notes',
                      relativePath: 'notes',
                      isDirectory: true,
                      size: 0,
                      lastModified: '',
                    ),
                  ];
                }
                return <WorkspaceFileEntry>[
                  WorkspaceFileEntry(
                    name: 'readme.txt',
                    path: r'D:\work\notes\readme.txt',
                    relativePath: 'notes/readme.txt',
                    isDirectory: false,
                    size: 12,
                    lastModified: '',
                  ),
                ];
              },
              onReadWorkspaceTextFile: (path) async => 'tree file body',
              loadGitChanges: (_) async => const <WorkspaceGitChange>[],
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    await tester.tap(find.text('工作区文件'));
    await tester.pump();
    await tester.pump();

    await tester.tap(find.text('notes'));
    await tester.pump();
    await tester.pump();
    await tester.tap(find.text('readme.txt'));
    await tester.pump();
    await tester.pump();

    expect(find.text('tree file body'), findsOneWidget);
    expect(find.text('fresh line'), findsNothing);
  });

  test('openDiffTab uses latestDiffReview when opening diff from sidebar', () {
    final session = WorkspaceSession();
    final review = reviewForMessage(
      _message(
        parts: <MessagePart>[
          MessagePart(
            partId: 'result',
            sequence: 0,
            kind: MessagePartKind.toolResult,
            content:
                '<file-diff path="lib/b.dart" details="updated"><![CDATA['
                '@@ -1,1 +1,1 @@\n-old\n+new line\n]]></file-diff>',
            toolCallId: 'call-2',
            toolName: 'edit_file',
            attributes: const <String, String>{'status': 'success'},
          ),
        ],
      ),
    );
    session.latestDiffReview = review;
    session.openDiffTab();
    expect(session.tabs.last.kind, WorkspaceTabKind.diff);
    expect(session.tabs.last.diffReview, review);
  });

  testWidgets('loads git changes for workspacePath when diff review has no files', (
    tester,
  ) async {
    var scannedMounts = <({String name, String path})>[];
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 800,
            height: 500,
            child: WorkspaceDiffContent(
              review: const WorkspaceDiffReview(files: []),
              workspacePath: r'D:\projects\operit',
              onListWorkspaceFiles: (_) async => const <WorkspaceFileEntry>[],
              loadGitChanges: (mounts) async {
                scannedMounts = mounts.toList();
                return const <WorkspaceGitChange>[
                  WorkspaceGitChange(
                    root: r'D:\projects\operit',
                    status: 'M',
                    relativePath: 'lib/main.dart',
                    absolutePath: r'D:\projects\operit\lib\main.dart',
                    diff: '@@ -1,1 +1,1 @@\n-old\n+new\n',
                  ),
                ];
              },
            ),
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pump();

    expect(find.text('Git 改动'), findsOneWidget);
    expect(find.text('main.dart'), findsWidgets);
    expect(
      scannedMounts.any((mount) => mount.path == r'D:\projects\operit'),
      isTrue,
    );
  });
}

ChatUiMessage _message({required List<MessagePart> parts}) {
  return ChatMessage(
    pluginExtensions: const <String, Object?>{},
    sender: 'ai',
    parts: parts,
    timestamp: 1,
    roleName: 'ai',
    selectedVariantIndex: 0,
    variantCount: 1,
    provider: 'test',
    modelName: 'test',
    inputTokens: 0,
    outputTokens: 0,
    cachedInputTokens: 0,
    sentAt: 0,
    outputDurationMs: 0,
    waitDurationMs: 0,
    completedAt: 1,
    displayMode: ChatMessageDisplayMode.normal,
    isFavorite: false,
    contentStream: null,
  );
}
