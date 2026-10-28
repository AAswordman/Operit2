// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'WorkspaceTabModels.dart';
import 'WorkspaceDiffReview.dart';

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

  /// The most recently seen diff review from conversation messages.
  WorkspaceDiffReview? latestDiffReview;

  /// Replaces the workspace diff tab with one turn and asks the shell to open.
  void showDiffReview(WorkspaceDiffReview review) {
    latestDiffReview = review;
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.diff,
      title: '变更',
      icon: Icons.difference_outlined,
      diffReview: review,
    );
    final index = tabs.indexWhere((item) => item.kind == WorkspaceTabKind.diff);
    if (index >= 0) {
      tabs[index] = tab;
      selectedIndex = index;
    } else {
      tabs.add(tab);
      selectedIndex = tabs.length - 1;
    }
    notifyListeners();
    onOpenRequested?.call();
  }

  /// Opens or activates the workspace diff tab.
  void openDiffTab({WorkspaceDiffReview? review}) {
    final index = tabs.indexWhere((item) => item.kind == WorkspaceTabKind.diff);
    final effectiveReview = review ??
        (index >= 0 ? tabs[index].diffReview : null) ??
        latestDiffReview ??
        const WorkspaceDiffReview(files: <WorkspaceDiffFile>[]);
    final tab = WorkspaceTab(
      kind: WorkspaceTabKind.diff,
      title: '变更',
      icon: Icons.difference_outlined,
      diffReview: effectiveReview,
    );
    if (index >= 0) {
      tabs[index] = tab;
      selectedIndex = index;
    } else {
      tabs.add(tab);
      selectedIndex = tabs.length - 1;
    }
    notifyListeners();
    onOpenRequested?.call();
  }
}
