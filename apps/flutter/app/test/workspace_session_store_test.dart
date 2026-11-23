import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceSession.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceSessionStore.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';

/// Covers local workspace switching, retained drafts, and asynchronous ownership.
void main() {
  test(
    'different workspaces restore their own visibility and navigation',
    () async {
      final store = WorkspaceSessionStore();
      final first = store.select(
        workspacePath: '/workspaces/a',
        chatId: 'chat-a',
      );
      first.isOpen = true;
      await first.openFile(
        path: '/workspaces/a/readme.txt',
        relativePath: 'readme.txt',
        readText: (_) async => 'saved text',
      );
      final document = first.tabs.last.textDocument!;
      document.updateText('unsaved draft');
      first.secondaryTabs.add(
        const WorkspaceTab(
          kind: WorkspaceTabKind.browser,
          title: 'Browser A',
          icon: Icons.public,
          browserSessionId: 'browser-a',
        ),
      );
      first.splitAxis = WorkspaceSplitAxis.vertical;
      first.splitRatio = 0.7;
      first.secondaryPaneFirst = true;

      final second = store.select(
        workspacePath: '/workspaces/b',
        chatId: 'chat-b',
      );
      expect(store.current, same(second));
      expect(second.isOpen, isFalse);
      expect(second.tabs.single.kind, WorkspaceTabKind.home);
      expect(second.secondaryTabs, isEmpty);
      expect(second.splitAxis, isNull);
      second.tabs.add(
        const WorkspaceTab(
          kind: WorkspaceTabKind.terminal,
          title: 'Terminal B',
          icon: Icons.terminal,
          terminalSessionId: 'terminal-b',
        ),
      );
      second.selectedIndex = 1;

      final restored = store.select(
        workspacePath: '/workspaces/a',
        chatId: 'chat-a',
      );
      expect(restored, same(first));
      expect(restored.isOpen, isTrue);
      expect(restored.selectedIndex, 1);
      expect(restored.tabs.last.textDocument, same(document));
      expect(document.text, 'unsaved draft');
      expect(document.isDirty, isTrue);
      expect(restored.secondaryTabs.single.browserSessionId, 'browser-a');
      expect(restored.splitAxis, WorkspaceSplitAxis.vertical);
      expect(restored.splitRatio, 0.7);
      expect(restored.secondaryPaneFirst, isTrue);
      restored.isOpen = false;

      expect(
        store.select(workspacePath: '/workspaces/b', chatId: 'chat-b'),
        same(second),
      );
      expect(store.current.isOpen, isFalse);
      expect(store.current.tabs.last.terminalSessionId, 'terminal-b');
      expect(store.current.selectedIndex, 1);
      expect(
        store.select(workspacePath: '/workspaces/a', chatId: 'chat-a').isOpen,
        isFalse,
      );
    },
  );

  test('conversations bound to the same workspace share the same session', () {
    final store = WorkspaceSessionStore();
    final first = store.select(
      workspacePath: '/workspaces/shared',
      chatId: 'one',
    );
    first.isOpen = true;
    first.selectedIndex = 0;
    expect(
      store.select(workspacePath: '/workspaces/shared', chatId: 'two'),
      same(first),
    );
    expect(store.current.isOpen, isTrue);
  });

  test(
    'unbound chats are isolated and rebinding restores the bound workspace',
    () {
      final store = WorkspaceSessionStore();
      final unbound = store.select(workspacePath: null, chatId: 'one');
      unbound.isOpen = true;
      final second = store.select(workspacePath: null, chatId: 'two');
      expect(second, isNot(same(unbound)));
      expect(second.isOpen, isFalse);
      final bound = store.select(workspacePath: '/workspaces/a', chatId: 'one');
      expect(bound, isNot(same(unbound)));
      expect(bound.isOpen, isFalse);
      expect(store.select(workspacePath: '', chatId: 'one'), same(unbound));
      expect(store.current.isOpen, isTrue);
      expect(
        store.select(workspacePath: '/workspaces/a', chatId: 'one'),
        same(bound),
      );
    },
  );

  test('another frontend store starts without the current frontend state', () {
    final first = WorkspaceSessionStore();
    first.select(workspacePath: '/workspaces/a', chatId: 'one').isOpen = true;
    final independent = WorkspaceSessionStore();
    final session = independent.select(
      workspacePath: '/workspaces/a',
      chatId: 'one',
    );
    expect(session.isOpen, isFalse);
    expect(session.tabs.single.kind, WorkspaceTabKind.home);
    expect(session, isNot(same(first.current)));
  });

  test(
    'a file read finishing after switching stays in its owning workspace',
    () async {
      final store = WorkspaceSessionStore();
      final first = store.select(workspacePath: '/workspaces/a', chatId: 'one');
      final read = Completer<String>();
      final opening = first.openFile(
        path: '/workspaces/a/delayed.txt',
        relativePath: 'delayed.txt',
        readText: (_) => read.future,
      );
      final second = store.select(
        workspacePath: '/workspaces/b',
        chatId: 'two',
      );
      read.complete('workspace a contents');
      await opening;
      expect(store.current, same(second));
      expect(second.tabs.single.kind, WorkspaceTabKind.home);
      expect(first.tabs.last.textDocument!.text, 'workspace a contents');
    },
  );
}
