// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import 'package:operit_ui/operit_ui.dart';

const double navigationTabletBreakpoint = themeTabletBreakpoint;

bool useTabletLayoutForWidth(double width) {
  return width >= navigationTabletBreakpoint;
}

bool useTabletLayoutForContext(BuildContext context) {
  return useTabletLayoutForWidth(MediaQuery.sizeOf(context).width);
}
