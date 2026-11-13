// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'WorkspaceTabModels.dart';
import 'file_preview/WorkspaceTextDocument.dart';
import '../../../../common/contributions/PluginWorkspacePresentation.dart';
import '../../../../common/icons/MaterialIconNameResolver.dart';

/// Describes the orientation of the two workspace panes.
enum WorkspaceSplitAxis { horizontal, vertical }

/// Keeps workspace navigation alive independently of mounted panel widgets.
class WorkspaceSession extends ChangeNotifier {
  /// Creates the main workspace with its permanent home tab selected.
  WorkspaceSession();

  /// Opens the side workspace when a turn review is requested.
  VoidCallback? onOpenRequested;

  final List<WorkspaceTab> tabs = <WorkspaceTab>[
    const WorkspaceTab(
      kind: WorkspaceTabKind.home,
      title: '',
      icon: Icons.home_outlined,
      closable: false,
    ),
  ];
  final List<WorkspaceTab> secondaryTabs = <WorkspaceTab>[];
  int selectedIndex = 0;
  int secondarySelectedIndex = 0;
  WorkspaceSplitAxis? splitAxis;
  double splitRatio = 0.5;
  bool secondaryPaneFirst = false;
  int tabIdentitySequence = 0;

  /// Reuses file previews and their drafts across Markdown links and the file tree.
  Future<void> openFile({
    required String path,
    required String relativePath,
    required Future<String> Function(String path) readText,
  }) async {
    bool activate() {
      for (final pane in [tabs, secondaryTabs]) {
        final index = pane.indexWhere(
          (tab) =>
              tab.kind == WorkspaceTabKind.filePreview &&
              tab.absolutePath == path,
        );
        if (index < 0) continue;
        if (identical(pane, tabs)) {
          selectedIndex = index;
        } else {
          secondarySelectedIndex = index;
        }
        notifyListeners();
        return true;
      }
      return false;
    }

    if (activate()) return;
    final kind = workspacePreviewKindForPath(path);
    final document =
        kind == WorkspaceFilePreviewKind.text ||
            kind == WorkspaceFilePreviewKind.markdown
        ? WorkspaceTextDocument(await readText(relativePath))
        : null;
    // A second request may have opened this file while its contents loaded.
    if (activate()) return;
    tabs.add(
      WorkspaceTab(
        kind: WorkspaceTabKind.filePreview,
        title: path.split('/').last,
        icon: workspacePreviewIconForKind(kind),
        filePath: relativePath,
        absolutePath: path,
        textDocument: document,
        previewKind: kind,
      ),
    );
    selectedIndex = tabs.length - 1;
    notifyListeners();
  }

  /// Semantic conversation context forwarded opaquely to independently owned plugin pages.
  Map<String, Object?> _pluginHostState = const {};

  /// Exposes the latest opaque conversation context to all mounted plugin tabs.
  Map<String, Object?> get pluginHostState => _pluginHostState;

  /// Updates live plugin input independently of the navigation that opened a tab.
  set pluginHostState(Map<String, Object?> value) {
    _pluginHostState = Map<String, Object?>.unmodifiable(value);
    notifyListeners();
  }

  /// Opens or updates a plugin-owned page without duplicating a docked sidebar view.
  void openPluginPage(PluginWorkspacePresentation presentation) {
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.plugin,
      title: presentation.title,
      icon: presentation.icon == null
          ? Icons.extension_outlined
          : MaterialIconNameResolver.resolve(presentation.icon!),
      pluginPresentation: presentation,
      identityToken: presentation.identity,
    );
    final secondaryIndex = secondaryTabs.indexWhere(
      (item) => item.pluginPresentation?.identity == presentation.identity,
    );
    if (secondaryIndex >= 0) {
      secondaryTabs[secondaryIndex] = tab;
      secondarySelectedIndex = secondaryIndex;
    } else {
      final index = tabs.indexWhere(
        (item) => item.pluginPresentation?.identity == presentation.identity,
      );
      if (index >= 0) {
        tabs[index] = tab;
        selectedIndex = index;
      } else {
        tabs.add(tab);
        selectedIndex = tabs.length - 1;
      }
    }
    notifyListeners();
    onOpenRequested?.call();
  }
}
