// ignore_for_file: file_names

import 'package:flutter/widgets.dart';
import 'RuntimeBrowserSurfaceRegistry.dart';

/// Keeps an owner surface mounted while optional presentation geometry changes.
/// It belongs to the global host, outside retained pages and workspace tabs.
class RuntimeBrowserSurface extends StatefulWidget {
  const RuntimeBrowserSurface({
    super.key,
    required this.sessionId,
    required this.child,
    this.registry,
  });

  final String sessionId;
  final Widget child;
  final RuntimeBrowserSurfaceRegistry? registry;

  @override
  State<RuntimeBrowserSurface> createState() => _RuntimeBrowserSurfaceState();
}

class _RuntimeBrowserSurfaceState extends State<RuntimeBrowserSurface> {
  final LayerLink _backgroundLink = LayerLink();
  Size? _lastPresentedSize;

  @override
  Widget build(BuildContext context) {
    final registry = widget.registry ?? RuntimeBrowserSurfaceRegistry.instance;
    return AnimatedBuilder(
      animation: registry,
      child: widget.child,
      builder: (context, child) {
        final presentation = registry.presentationFor(widget.sessionId);
        if (presentation != null) _lastPresentedSize = presentation.size;
        // Preserve the last viewport while hidden; responsive page layout must
        // not change simply because the user switched a workspace tab.
        final size =
            presentation?.size ??
            _lastPresentedSize ??
            MediaQuery.sizeOf(context);
        return Positioned.fill(
          child: IgnorePointer(
            ignoring: presentation == null,
            child: ExcludeSemantics(
              excluding: presentation == null,
              child: Opacity(
                opacity: presentation == null ? 0 : 1,
                // The follower must occupy the host viewport for hit testing;
                // its browser child may be translated far outside its origin.
                child: CompositedTransformFollower(
                  link: presentation?.link ?? _backgroundLink,
                  showWhenUnlinked: presentation == null,
                  child: Align(
                    alignment: Alignment.topLeft,
                    child: SizedBox(
                      width: size.width,
                      height: size.height,
                      child: ClipRect(child: child),
                    ),
                  ),
                ),
              ),
            ),
          ),
        );
      },
    );
  }
}
