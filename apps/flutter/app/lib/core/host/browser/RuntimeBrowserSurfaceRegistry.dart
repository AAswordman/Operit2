// ignore_for_file: file_names

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

/// Connects workspace presentation anchors to permanently mounted owner views.
/// A detached retained page releases its anchor without disposing the session.
class RuntimeBrowserSurfaceRegistry extends ChangeNotifier {
  RuntimeBrowserSurfaceRegistry();

  static final instance = RuntimeBrowserSurfaceRegistry();
  final Map<String, Map<Object, RuntimeBrowserSurfacePresentation>>
  _presentations = {};
  bool _notificationScheduled = false;

  RuntimeBrowserSurfacePresentation? presentationFor(String sessionId) {
    final anchors = _presentations[sessionId];
    return anchors == null || anchors.isEmpty ? null : anchors.values.last;
  }

  void attach(
    String sessionId,
    RuntimeBrowserSurfacePresentation presentation,
  ) {
    final anchors = _presentations.putIfAbsent(sessionId, () => {});
    final previous = anchors[presentation.owner];
    if (previous?.owner == presentation.owner &&
        previous?.size == presentation.size) {
      return;
    }
    anchors[presentation.owner] = presentation;
    _scheduleNotification();
  }

  void detach(String sessionId, Object owner) {
    final anchors = _presentations[sessionId];
    if (anchors == null || anchors.remove(owner) == null) return;
    if (anchors.isEmpty) _presentations.remove(sessionId);
    _scheduleNotification();
  }

  // Render attachment and layout can happen during a build. Publish only after
  // that frame, and explicitly schedule the frame that applies the new geometry.
  void _scheduleNotification() {
    if (_notificationScheduled) return;
    _notificationScheduled = true;
    SchedulerBinding.instance.addPostFrameCallback((_) {
      _notificationScheduled = false;
      notifyListeners();
    });
    SchedulerBinding.instance.ensureVisualUpdate();
  }
}

class RuntimeBrowserSurfacePresentation {
  const RuntimeBrowserSurfacePresentation({
    required this.owner,
    required this.link,
    required this.size,
  });

  final Object owner;
  final LayerLink link;
  final Size size;
}

/// Reserves presentation geometry; it never creates or transfers a WebView.
class RuntimeBrowserSurfaceTarget extends SingleChildRenderObjectWidget {
  const RuntimeBrowserSurfaceTarget({
    super.key,
    required this.sessionId,
    this.registry,
  }) : super(child: const SizedBox.expand());

  final String sessionId;
  final RuntimeBrowserSurfaceRegistry? registry;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderBrowserSurfaceTarget(
        sessionId,
        registry ?? RuntimeBrowserSurfaceRegistry.instance,
      );

  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderLeaderLayer renderObject,
  ) {
    (renderObject as _RenderBrowserSurfaceTarget).configure(
      sessionId,
      registry ?? RuntimeBrowserSurfaceRegistry.instance,
    );
  }
}

class _RenderBrowserSurfaceTarget extends RenderLeaderLayer {
  _RenderBrowserSurfaceTarget(this._sessionId, this._registry)
    : super(link: LayerLink());

  RuntimeBrowserSurfaceRegistry _registry;
  String _sessionId;

  void configure(String sessionId, RuntimeBrowserSurfaceRegistry registry) {
    if (sessionId == _sessionId && identical(registry, _registry)) return;
    _registry.detach(_sessionId, this);
    _sessionId = sessionId;
    _registry = registry;
    if (attached && hasSize) _publish();
  }

  void _publish() {
    _registry.attach(
      _sessionId,
      RuntimeBrowserSurfacePresentation(owner: this, link: link, size: size),
    );
  }

  @override
  void attach(PipelineOwner owner) {
    super.attach(owner);
    if (hasSize) _publish();
  }

  @override
  void performLayout() {
    super.performLayout();
    _publish();
  }

  @override
  void detach() {
    _registry.detach(_sessionId, this);
    super.detach();
  }
}
