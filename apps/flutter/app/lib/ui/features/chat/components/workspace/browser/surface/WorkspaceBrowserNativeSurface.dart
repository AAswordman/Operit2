// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSurfaceRegistry.dart';

/// Presents the globally owned session without taking over its platform view.
class WorkspaceBrowserNativeSurface extends StatelessWidget {
  const WorkspaceBrowserNativeSurface({super.key, required this.sessionId});

  final String sessionId;

  @override
  Widget build(BuildContext context) =>
      RuntimeBrowserSurfaceTarget(sessionId: sessionId);
}
