// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import '../WorkspaceBrowserViewStore.dart';
import '../tabs/WorkspaceBrowserTabModels.dart';
import 'WorkspaceBrowserCompositorSurface.dart';
import 'WorkspaceBrowserNativeSurface.dart';

/// Shared session presentation for interactive browsing and temporary visits.
class WorkspaceBrowserSurface extends StatelessWidget {
  const WorkspaceBrowserSurface({
    super.key,
    required this.tab,
    required this.store,
  });

  final WorkspaceBrowserTabState tab;
  final WorkspaceBrowserViewStore store;

  @override
  Widget build(BuildContext context) => store.usesNativeSurface
      ? WorkspaceBrowserNativeSurface(sessionId: tab.id)
      : WorkspaceBrowserCompositorSurface(tab: tab, store: store);
}
