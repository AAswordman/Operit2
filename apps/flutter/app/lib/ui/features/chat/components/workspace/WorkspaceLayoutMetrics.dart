// ignore_for_file: file_names

import '../../../../common/components/AdaptiveSidePanel.dart';
import '../../../../main/layout/NavigationLayoutMetrics.dart';
import '../../../../window/OperitWindowPlatform.dart';

// This breakpoint controls navigation attachment and tab chrome, not panel layout.
const double workspaceTabletBreakpoint = navigationTabletBreakpoint;

/// Desktop workspaces always split; phone and tablet workspaces always cover.
SidePanelLayout get workspaceSidePanelLayout =>
    operitUsesDesktopLayout ? SidePanelLayout.split : SidePanelLayout.overlay;

const double workspaceDefaultWidth = 360;
const double workspaceDefaultWidthFraction = 0.45;
const double workspaceMinWidth = 280;
const double workspaceMinChatWidth = 320;
const double workspaceResizeHandleHitWidth = 24;
const double workspaceResizeHandleTrackWidth =
    workspaceResizeHandleHitWidth / 3;
const double workspaceResizeHandleVisualWidth = 3;
const double workspaceResizeHandleHeight = 56;
