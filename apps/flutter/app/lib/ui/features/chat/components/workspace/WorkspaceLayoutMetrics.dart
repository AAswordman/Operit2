// ignore_for_file: file_names

import '../../../../main/layout/NavigationLayoutMetrics.dart';

// Navigation and tab chrome still share the application's navigation breakpoint.
const double workspaceTabletBreakpoint = navigationTabletBreakpoint;

// Leave enough room for both chat and workspace before placing them side by side.
const double workspaceSideBySideBreakpoint = 1024;
const double workspaceDefaultTabletWidth = 360;
const double workspaceDefaultWidthFraction = 0.45;
const double workspaceMinWidth = 280;
const double workspaceMinTabletChatWidth = 320;
const double workspaceResizeHandleHitWidth = 24;
const double workspaceResizeHandleTrackWidth =
    workspaceResizeHandleHitWidth / 3;
const double workspaceResizeHandleVisualWidth = 3;
const double workspaceResizeHandleHeight = 56;
