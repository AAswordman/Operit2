// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'WorkspaceTabModels.dart';
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
      title: presentation.route.title,
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
