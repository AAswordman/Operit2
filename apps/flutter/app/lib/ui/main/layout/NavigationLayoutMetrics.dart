// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

const double navigationTabletBreakpoint = 600;

bool useTabletLayoutForWidth(double width) {
  return width >= navigationTabletBreakpoint;
}

bool useTabletLayoutForContext(BuildContext context) {
  return useTabletLayoutForWidth(MediaQuery.sizeOf(context).width);
}
