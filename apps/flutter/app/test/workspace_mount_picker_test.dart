import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceOverviewModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies the real workspace tab uses mount selection instead of URI-to-path conversion.
void main() {
  final original = OperitFolderAccessPlatform.instance;
  tearDown(() => OperitFolderAccessPlatform.instance = original);

  testWidgets('workspace tab binds the opaque document source unchanged', (
    tester,
  ) async {
    const source = FolderMountSource(
      backend: 'android_documents',
      root:
          'content://com.ai.assistance.operit.documents.ubuntu/tree/project%2Ftest',
      namespace: '/mnt/android/documents',
      name: '测试项目',
    );
    final picker = _WorkspaceMountPicker(source);
    OperitFolderAccessPlatform.instance = picker;
    final bindings = <String>[];
    await _mountWorkspacePicker(tester, bindings);
    await tester.tap(find.text('选择本机文件夹'));
    await tester.pumpAndSettle();
    expect(picker.calls, 1);
    expect(bindings, [source.encode()]);
    expect(tester.takeException(), isNull);
  });

  testWidgets('cancelled mount selection never invokes workspace binding', (
    tester,
  ) async {
    final picker = _WorkspaceMountPicker(null);
    OperitFolderAccessPlatform.instance = picker;
    final bindings = <String>[];
    await _mountWorkspacePicker(tester, bindings);
    await tester.tap(find.text('选择本机文件夹'));
    await tester.pumpAndSettle();
    expect(picker.calls, 1);
    expect(bindings, isEmpty);
    expect(tester.takeException(), isNull);
  });
}

/// Mounts the production workspace picker tab with inert unrelated capabilities.
Future<void> _mountWorkspacePicker(
  WidgetTester tester,
  List<String> bindings,
) async {
  final sessions = ValueNotifier<int>(0);
  addTearDown(sessions.dispose);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot:
          UserPreferencesManager.defaultThemePreferenceSnapshot,
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: MaterialApp(
        locale: const Locale('zh'),
        supportedLocales: AppLocalizations.supportedLocales,
        localizationsDelegates: AppLocalizations.localizationsDelegates,
        home: Scaffold(
          body: WorkspaceTabContent(
            tab: const WorkspaceTab(
              kind: WorkspaceTabKind.workspacePicker,
              title: '工作区',
              icon: Icons.folder_outlined,
            ),
            workspacePath: null,
            workspaceUsage: WorkspaceOverviewUsage.empty,
            terminalSessionCountListenable: sessions,
            browserSessionCountListenable: sessions,
            onListWorkspaceFiles: (_) async => const [],
            onListWorkspaceBindingDirectories: (_) async => const [],
            onReadWorkspaceTextFile: (_) async => '',
            onReadWorkspaceFileBytes: (_) async => Uint8List(0),
            onWriteWorkspaceFileBytes: (_, _) async {},
            onOpenWorkspaceFile: (_) async {},
            onOpenFile: (_) async {},
            onOpenFolder: (_) {},
            onAddFolder: () {},
            filesListingRevision: 0,
            onOpenTerminal: () {},
            onOpenTerminalSessions: () {},
            onOpenBrowserSessions: () {},
            onOpenBrowser: ({url, localFilePath, workspaceHtmlPath}) {},
            onFinishWebVisit: (_, _) {},
            onActivateCurrentTab: () {},
            onCloseCurrentTab: () {},
            onOpenWorkspaceCreator: () {},
            onBindWorkspace: (workspace) async => bindings.add(workspace),
            onChooseExistingWorkspace: () {},
            onUnbindWorkspace: () {},
            splitMarkdownContent: (_) async => const [],
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Rejects physical-path picking so accidental use of the old API fails the test.
class _WorkspaceMountPicker extends OperitFolderAccessPlatform {
  /// Supplies the authorized selection or an explicit user cancellation.
  _WorkspaceMountPicker(this.source);

  final FolderMountSource? source;
  int calls = 0;

  /// Returns the host-authorized source without inspecting its opaque URI.
  @override
  Future<FolderMountSource?> pickWorkspaceDirectory({
    String? initialDirectory,
  }) async {
    calls++;
    return source;
  }

  /// Rejects use of physical-path selection for a workspace mount.
  @override
  Future<String?> pickDirectory({String? initialDirectory}) =>
      throw StateError('Workspace selection must use the mount-source API');

  /// Rejects unrelated multi-directory selection in workspace tests.
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) =>
      throw StateError('Workspace selection must use the mount-source API');
}
