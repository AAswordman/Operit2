// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import 'package:operit_ui/operit_ui.dart';

const double navigationTabletBreakpoint = themeTabletBreakpoint;

// Keep the conversation list usable without crowding the primary content.
const double navigationSidebarDefaultWidth = 280;
const double navigationSidebarMinWidth = 200;
const double navigationSidebarMaxWidth = 480;
const double navigationMinContentWidth = 320;
const double navigationSidebarResizeHitWidth = 12;

bool useTabletLayoutForWidth(double width) {
  return width >= navigationTabletBreakpoint;
}

bool useTabletLayoutForContext(BuildContext context) {
  return useTabletLayoutForWidth(MediaQuery.sizeOf(context).width);
}
