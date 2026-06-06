// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

const double settingsWideLayoutBreakpoint = 760;

bool settingsUseWideLayout(BuildContext context) {
  return MediaQuery.sizeOf(context).width >= settingsWideLayoutBreakpoint;
}
