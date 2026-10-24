// ignore_for_file: file_names

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';

/// Retains page state without building or rendering an inactive page.
///
/// The element subtree stays mounted in its own build scope. Its render subtree
/// is removed from the live pipeline while inactive, including nested layout
/// callbacks and repaint boundaries. Pending widget updates are applied on wake.
/// A page-local Material keeps ink features inside the same render boundary;
/// live ancestors must not paint ink whose reference boxes have been parked.
/// A page-local Overlay also parks portal layout callbacks with their anchors.
/// Page-owned timers and subscriptions must also observe the ambient TickerMode
/// notifier; retaining a widget cannot suspend arbitrary asynchronous Dart work.
class RetainedPage extends StatefulWidget {
  /// Creates a visibility boundary for a state-preserving cached page.
  const RetainedPage({super.key, required this.active, required this.child});

  final bool active;
  final Widget child;

  /// Creates the lifecycle handoff between focus release and render suspension.
  @override
  State<RetainedPage> createState() => _RetainedPageState();
}

class _RetainedPageState extends State<RetainedPage> {
  late bool _renderActive;
  late final _RetainedBuildScheduler _buildScheduler;

  /// Leaves pages that start inactive unmounted until their first activation.
  @override
  void initState() {
    super.initState();
    _renderActive = widget.active;
    _buildScheduler = _RetainedBuildScheduler.acquire(
      (context as Element).owner!,
    );
  }

  /// Releases input connections before detaching their render geometry.
  @override
  void didUpdateWidget(covariant RetainedPage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (widget.active) {
      _renderActive = true;
    } else if (oldWidget.active) {
      // Focus changes are delivered asynchronously. Let EditableText finish its
      // current frame and close its input connection before parking the tree.
      WidgetsBinding.instance.addPostFrameCallback(_finishDeactivation);
    }
  }

  /// Parks the render subtree after the focus-release frame has completed.
  void _finishDeactivation(Duration timestamp) {
    if (mounted && !widget.active) {
      setState(() => _renderActive = false);
    }
  }

  /// Restores the build scheduler after the last retained page is removed.
  @override
  void dispose() {
    _buildScheduler.release();
    super.dispose();
  }

  /// Mutes tickers and focus before suspending the page's build and render work.
  @override
  Widget build(BuildContext context) {
    return TickerMode(
      enabled: widget.active,
      child: ExcludeFocus(
        excluding: !widget.active,
        child: _RetainedPageBody(
          active: _renderActive,
          child: Overlay.wrap(
            child: Material(
              type: MaterialType.transparency,
              child: widget.child,
            ),
          ),
        ),
      ),
    );
  }
}

/// Keeps normal frame scheduling live when only a dormant scope becomes dirty.
///
/// BuildOwner resets its scheduling flag when it flushes a build scope. A frame
/// containing only dormant dirty scopes otherwise leaves the root scope empty
/// and skips that reset. Queueing the RootElement guarantees the normal flush;
/// RootElement clears its own dirty flag without rebuilding its existing child.
class _RetainedBuildScheduler {
  /// Captures the owner's scheduler while at least one retained page is mounted.
  _RetainedBuildScheduler(this._owner)
    : _previousScheduler = _owner.onBuildScheduled! {
    _owner.onBuildScheduled = _scheduleBuild;
  }

  static final Expando<_RetainedBuildScheduler> _owners =
      Expando<_RetainedBuildScheduler>();
  final BuildOwner _owner;
  final VoidCallback _previousScheduler;
  int _references = 0;

  /// Shares one scheduler handoff among nested and sibling retained pages.
  static _RetainedBuildScheduler acquire(BuildOwner owner) {
    var scheduler = _owners[owner];
    if (scheduler == null) {
      scheduler = _RetainedBuildScheduler(owner);
      _owners[owner] = scheduler;
    }
    scheduler._references += 1;
    return scheduler;
  }

  /// Leaves an inexpensive root dirty entry without waking retained descendants.
  void _scheduleBuild() {
    _previousScheduler();
    WidgetsBinding.instance.rootElement!.markNeedsBuild();
  }

  /// Restores scheduler ownership when no retained boundaries remain mounted.
  void release() {
    _references -= 1;
    if (_references == 0) {
      assert(_owner.onBuildScheduled == _scheduleBuild);
      _owner.onBuildScheduled = _previousScheduler;
      _owners[_owner] = null;
    }
  }
}

class _RetainedPageBody extends RenderObjectWidget {
  /// Describes the latest child configuration without updating a sleeping page.
  const _RetainedPageBody({required this.active, required this.child});

  final bool active;
  final Widget child;

  /// Gives the page an independently scheduled build scope.
  @override
  RenderObjectElement createElement() => _RetainedPageElement(this);

  /// Creates the render boundary that parks inactive render subtrees.
  @override
  _RenderRetainedPage createRenderObject(BuildContext context) {
    return _RenderRetainedPage(active: active);
  }

  /// Applies activity changes without replacing the retained element subtree.
  @override
  void updateRenderObject(
    BuildContext context,
    _RenderRetainedPage renderObject,
  ) {
    renderObject.active = active;
  }
}

class _RetainedPageElement extends RenderObjectElement {
  /// Creates the element responsible for retaining the page's dirty queue.
  _RetainedPageElement(_RetainedPageBody super.widget);

  Element? _child;
  bool _frameCallbackScheduled = false;
  late final BuildScope _pageBuildScope = BuildScope(
    scheduleRebuild: _scheduleRebuild,
  );

  /// Isolates dirty descendants from the application's normal build pass.
  @override
  BuildScope get buildScope => _pageBuildScope;

  /// Exposes the render boundary used for layout-time rebuilding.
  @override
  _RenderRetainedPage get renderObject =>
      super.renderObject as _RenderRetainedPage;

  /// Installs the layout callback without eagerly mounting an inactive page.
  @override
  void mount(Element? parent, Object? newSlot) {
    super.mount(parent, newSlot);
    renderObject.rebuildPage = _rebuildPage;
  }

  /// Stores the newest page configuration and schedules only active pages.
  @override
  void update(covariant _RetainedPageBody newWidget) {
    super.update(newWidget);
    _scheduleRebuild();
  }

  /// Defers inherited changes to this page's next active layout pass.
  @override
  void markNeedsBuild() {
    _scheduleRebuild();
  }

  /// Schedules layout-time building without touching a sleeping render subtree.
  void _scheduleRebuild() {
    if (!(widget as _RetainedPageBody).active || _frameCallbackScheduled) {
      return;
    }
    if (!renderObject.attached) {
      // Nested active pages can be parked by an ancestor. Mark their callback
      // dirty for reattachment without requesting a frame in the live pipeline.
      renderObject.scheduleLayoutCallback();
      return;
    }
    switch (SchedulerBinding.instance.schedulerPhase) {
      case SchedulerPhase.idle:
      case SchedulerPhase.postFrameCallbacks:
        _frameCallbackScheduled = true;
        SchedulerBinding.instance.scheduleFrameCallback(_rebuildNextFrame);
      case SchedulerPhase.transientCallbacks:
      case SchedulerPhase.midFrameMicrotasks:
      case SchedulerPhase.persistentCallbacks:
        renderObject.scheduleLayoutCallback();
    }
  }

  /// Rechecks activity after a deferred request before scheduling layout.
  void _rebuildNextFrame(Duration timestamp) {
    _frameCallbackScheduled = false;
    if (mounted && (widget as _RetainedPageBody).active) {
      renderObject.scheduleLayoutCallback();
    }
  }

  /// Flushes pending updates against the current inherited data on activation.
  void _rebuildPage() {
    owner!.buildScope(this, _updatePageChild);
  }

  /// Updates the retained child once with the latest widget configuration.
  void _updatePageChild() {
    _child = updateChild(_child, (widget as _RetainedPageBody).child, null);
  }

  /// Keeps retained elements visible to framework lifecycle and disposal walks.
  @override
  void visitChildren(ElementVisitor visitor) {
    final child = _child;
    if (child != null) {
      visitor(child);
    }
  }

  /// Excludes inactive pages from onstage element discovery.
  @override
  void debugVisitOnstageChildren(ElementVisitor visitor) {
    if ((widget as _RetainedPageBody).active) {
      visitChildren(visitor);
    }
  }

  /// Releases an element moved elsewhere through a global key.
  @override
  void forgetChild(Element child) {
    assert(child == _child);
    _child = null;
    super.forgetChild(child);
  }

  /// Retains the child's render root even while it is outside the pipeline.
  @override
  void insertRenderObjectChild(RenderObject child, Object? slot) {
    assert(slot == null);
    renderObject.retainedChild = child as RenderBox;
  }

  /// Rejects moves because this boundary has exactly one render child slot.
  @override
  void moveRenderObjectChild(
    RenderObject child,
    Object? oldSlot,
    Object? newSlot,
  ) {
    throw StateError('A retained page has only one render child slot.');
  }

  /// Removes a disposed or reparented render root from the retained page.
  @override
  void removeRenderObjectChild(RenderObject child, Object? slot) {
    assert(renderObject.retainedChild == child);
    renderObject.retainedChild = null;
  }

  /// Releases the element callback before the render boundary is disposed.
  @override
  void unmount() {
    renderObject.rebuildPage = null;
    super.unmount();
  }
}

class _RenderRetainedPage extends RenderProxyBox
    with RenderObjectWithLayoutCallbackMixin {
  /// Creates a pipeline boundary with an initially empty retained page.
  _RenderRetainedPage({required bool active}) : _active = active;

  bool _active;
  RenderBox? _retainedChild;
  VoidCallback? rebuildPage;

  /// Returns the render root owned by the retained element subtree.
  RenderBox? get retainedChild => _retainedChild;

  /// Attaches only active render roots to the live rendering pipeline.
  set retainedChild(RenderBox? value) {
    _retainedChild = value;
    child = _active ? value : null;
  }

  /// Parks or restores the render root without disposing any page state.
  set active(bool value) {
    if (_active == value) {
      return;
    }
    _active = value;
    child = value ? _retainedChild : null;
    markNeedsLayout();
    markNeedsPaint();
    markNeedsSemanticsUpdate();
  }

  /// Prevents page repaints from repainting neighboring page slots.
  @override
  bool get isRepaintBoundary => true;

  /// Flushes the isolated build scope only while the page is active.
  @override
  void layoutCallback() {
    if (_active) {
      rebuildPage!();
    }
  }

  /// Lays out active content and gives sleeping slots no child work.
  @override
  void performLayout() {
    runLayoutCallback();
    super.performLayout();
  }
}
