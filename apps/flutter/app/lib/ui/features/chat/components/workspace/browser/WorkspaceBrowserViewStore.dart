// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:operit2/core/browser/BrowserSessions.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/core/runtime/RuntimeConnectionManager.dart';

import 'WorkspaceBrowserStores.dart';
import 'WorkspaceBrowserUrlUtils.dart';
import 'downloads/WorkspaceBrowserDownloadStore.dart';
import 'permissions/WorkspaceBrowserPermissionStore.dart';
import 'tabs/WorkspaceBrowserTabModels.dart';
import 'userscripts/WorkspaceUserscriptModels.dart';

class WorkspaceBrowserViewDelegate {
  /// Creates workspace-only browser presentation callbacks.
  const WorkspaceBrowserViewDelegate({
    required this.onActivateRequested,
    required this.onCloseRequested,
  });

  final VoidCallback onActivateRequested;
  final VoidCallback onCloseRequested;
}

class _SurfaceInteractionBatch {
  /// Creates an empty browser surface interaction batch.
  _SurfaceInteractionBatch();

  Map<String, Object?>? _resize;
  Map<String, Object?>? _cursor;
  final List<Map<String, Object?>> _ordered = <Map<String, Object?>>[];
  double _scrollDx = 0;
  double _scrollDy = 0;
  bool _hasScroll = false;

  /// Records one browser surface interaction for the next frame.
  void add(Map<String, Object?> interaction) {
    switch (interaction['type'] as String?) {
      case 'resize':
        _resize = interaction;
        return;
      case 'cursor':
        _cursor = interaction;
        return;
      case 'scroll':
        _scrollDx += (interaction['dx'] as num).toDouble();
        _scrollDy += (interaction['dy'] as num).toDouble();
        _hasScroll = true;
        return;
      default:
        _ordered.add(interaction);
        return;
    }
  }

  /// Returns whether the batch contains no pending interactions.
  bool get isEmpty =>
      _resize == null && _cursor == null && !_hasScroll && _ordered.isEmpty;

  /// Serializes the pending interactions in compositor dispatch order.
  List<Map<String, Object?>> toList() {
    final interactions = <Map<String, Object?>>[];
    final resize = _resize;
    if (resize != null) {
      interactions.add(resize);
    }
    final cursor = _cursor;
    if (cursor != null) {
      interactions.add(cursor);
    }
    if (_hasScroll) {
      interactions.add(<String, Object?>{
        'type': 'scroll',
        'dx': _scrollDx,
        'dy': _scrollDy,
      });
    }
    interactions.addAll(_ordered);
    return interactions;
  }
}

class WorkspaceBrowserViewStore extends ChangeNotifier {
  /// Creates the workspace browser surface store.
  WorkspaceBrowserViewStore._();

  static final WorkspaceBrowserViewStore instance =
      WorkspaceBrowserViewStore._();

  static const String _homeUrl = 'https://www.bing.com';
  static const double _defaultZoomFactor = 1.0;
  static const double _minZoomFactor = 0.1;
  static const double _maxZoomFactor = 2.0;
  static const double _zoomStep = 0.1;
  static const String _mobileUserAgent =
      'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0.0.0 Mobile Safari/537.36';
  static const String _desktopUserAgent =
      'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 '
      '(KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36';
  static const String _localTextureTransport = 'localTexture';
  static const String _encodedStreamTransport = 'encodedStream';

  final BrowserSessions _sessions = BrowserSessions();
  final WorkspaceBrowserStores stores = WorkspaceBrowserStores();
  final WorkspaceBrowserPermissionStore permissions =
      WorkspaceBrowserPermissionStore();
  final List<WorkspaceBrowserTabState> _tabs = <WorkspaceBrowserTabState>[];
  final Map<String, StreamSubscription<BrowserSessionEvent>> _subscriptions =
      <String, StreamSubscription<BrowserSessionEvent>>{};
  final Map<String, _SurfaceInteractionBatch> _surfaceInteractionBatches =
      <String, _SurfaceInteractionBatch>{};
  final Set<String> _scheduledSurfaceInteractionBatches = <String>{};
  final ValueNotifier<int> sessionCount = ValueNotifier<int>(0);
  Future<void>? _loadFuture;
  WorkspaceBrowserViewDelegate? _delegate;
  int _selectedIndex = 0;
  int _openingCount = 0;

  List<WorkspaceBrowserTabState> get tabs =>
      List<WorkspaceBrowserTabState>.unmodifiable(_tabs);

  int get selectedIndex => _selectedIndex;

  WorkspaceBrowserTabState? get currentTab {
    if (_tabs.isEmpty) {
      return null;
    }
    return _tabs[_selectedIndex];
  }

  int get activeDownloadCount => stores.downloads.items
      .where(
        (item) =>
            item.state == WorkspaceBrowserDownloadState.pending ||
            item.state == WorkspaceBrowserDownloadState.running,
      )
      .length;

  /// Attaches workspace-only presentation callbacks.
  void attachUi({
    required WorkspaceBrowserViewDelegate delegate,
    required Future<void> Function(String path, Uint8List bytes)
    onWriteWorkspaceFileBytes,
  }) {
    _delegate = delegate;
    stores.downloads.setWorkspaceSaver(onWriteWorkspaceFileBytes);
  }

  /// Detaches workspace presentation callbacks.
  void detachUi() {
    _delegate = null;
  }

  /// Loads local browser chrome stores and existing Core sessions.
  Future<void> ensureLoaded() {
    final current = _loadFuture;
    if (current != null) {
      return current;
    }
    final next = _load();
    _loadFuture = next;
    return next;
  }

  /// Initializes the first visible browser surface.
  Future<void> openInitialTab({
    String? initialUrl,
    String? initialUserAgent,
    Map<String, String>? initialHeaders,
    String? initialFilePath,
    String? initialWorkspaceHtmlPath,
  }) async {
    await ensureLoaded();
    if (_tabs.isNotEmpty || _openingCount > 0) {
      return;
    }
    final target = initialFilePath ?? initialWorkspaceHtmlPath;
    if (target != null && target.trim().isNotEmpty) {
      await openTab(url: Uri.file(target.trim()).toString());
      return;
    }
    await openTab(
      url: initialUrl,
      userAgent: initialUserAgent,
      headers: initialHeaders,
    );
  }

  /// Creates and attaches a real owner browser session through Core.
  Future<WorkspaceBrowserTabState> openTab({
    String? url,
    String? userAgent,
    Map<String, String>? headers,
  }) async {
    _openingCount += 1;
    try {
      await ensureLoaded();
      final raw = url?.trim();
      final normalized = normalizeWorkspaceBrowserUrl(
        raw == null || raw.isEmpty ? _homeUrl : raw,
      );
      final info = await _sessions.createSession(
        initialUrl: normalized,
        userAgent: userAgent,
        headers: headers ?? const <String, String>{},
      );
      return await _attachSession(info, select: true);
    } finally {
      _openingCount -= 1;
    }
  }

  /// Attaches an existing Core browser session to the workspace view.
  Future<WorkspaceBrowserTabState> attachSession(
    RuntimeBrowserSessionInfo session,
  ) {
    return _attachSession(session, select: true);
  }

  /// Opens a runtime-owner local file session through Core.
  Future<WorkspaceBrowserTabState> openLocalFileTab(String absolutePath) {
    return openTab(url: Uri.file(absolutePath).toString());
  }

  /// Opens a runtime-owner workspace HTML session through Core.
  Future<WorkspaceBrowserTabState> openWorkspaceHtmlTab(
    String path, {
    String? initialUrl,
  }) {
    return openTab(
      url: initialUrl?.trim().isNotEmpty == true
          ? initialUrl!.trim()
          : Uri.file(path).toString(),
    );
  }

  /// Selects one attached browser surface and activates its owner session.
  void selectSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      throw StateError('Browser session is not attached');
    }
    _selectedIndex = index;
    _delegate?.onActivateRequested();
    notifyListeners();
    unawaited(_sessions.activate(sessionId));
  }

  /// Closes one browser session through Core.
  void closeSession(String sessionId) {
    unawaited(_sessions.close(sessionId));
    _removeSession(sessionId);
  }

  /// Closes the current browser session through Core.
  void closeCurrentTab() {
    final tab = _requireCurrentTab();
    closeSession(tab.id);
  }

  /// Navigates the current real owner WebView.
  void navigateCurrent(String rawUrl) {
    final tab = _requireCurrentTab();
    final url = normalizeWorkspaceBrowserUrl(rawUrl);
    tab.update(url: url, addressText: url, errorText: null);
    unawaited(_sessions.navigate(tab.id, url));
  }

  /// Navigates the current real owner WebView backward.
  void goBack() {
    unawaited(_sessions.goBack(_requireCurrentTab().id));
  }

  /// Navigates the current real owner WebView forward.
  void goForward() {
    unawaited(_sessions.goForward(_requireCurrentTab().id));
  }

  /// Reloads or stops the current real owner WebView.
  void refreshOrStop() {
    final tab = _requireCurrentTab();
    if (tab.isLoading) {
      unawaited(_sessions.stop(tab.id));
      return;
    }
    unawaited(_sessions.reload(tab.id));
  }

  /// Toggles the current URL in the controller app bookmark store.
  void toggleBookmark() {
    final tab = _requireCurrentTab();
    stores.bookmarks.toggle(url: tab.url, title: tab.title);
    notifyListeners();
  }

  /// Changes the user agent used by the real owner WebView.
  Future<void> setDesktopMode(bool enabled) async {
    final tab = _requireCurrentTab();
    tab.update(desktopMode: enabled);
    await _sessions.updateSession(
      sessionId: tab.id,
      userAgent: enabled ? _desktopUserAgent : _mobileUserAgent,
    );
    await _sessions.reload(tab.id);
  }

  /// Increases the owner WebView zoom factor.
  void zoomIn() {
    unawaited(_setZoomFactor(_requireCurrentTab().zoomFactor + _zoomStep));
  }

  /// Decreases the owner WebView zoom factor.
  void zoomOut() {
    unawaited(_setZoomFactor(_requireCurrentTab().zoomFactor - _zoomStep));
  }

  /// Restores the owner WebView zoom factor to the workspace default.
  void resetZoom() {
    unawaited(_setZoomFactor(_defaultZoomFactor));
  }

  /// Reads owner-side userscript menu commands through Core.
  Future<List<WorkspaceUserscriptMenuCommand>> loadMenuCommands() async {
    final result = await _sessions.evaluate(
      _requireCurrentTab().id,
      r'''JSON.stringify((window.__operitUserscriptMenuCommands || []).map(function(item, index) { return { index: index, scriptName: String(item.scriptName || ''), caption: String(item.caption || '') }; }))''',
    );
    final encoded = jsonDecode(result.resultJson);
    if (encoded is! String) {
      throw StateError('Userscript menu result is not a string');
    }
    final decoded = jsonDecode(encoded) as List<Object?>;
    return decoded
        .map(
          (item) => WorkspaceUserscriptMenuCommand.fromJson(
            item as Map<String, Object?>,
          ),
        )
        .toList(growable: false);
  }

  /// Runs one owner-side userscript menu command through Core.
  Future<void> runMenuCommand(int index) async {
    await _sessions.evaluate(
      _requireCurrentTab().id,
      '(function(){const commands=window.__operitUserscriptMenuCommands||[];const command=commands[$index];if(command&&typeof command.command==="function")command.command();})();',
    );
  }

  /// Evaluates one browser script in the real owner WebView.
  Future<String> evaluate(String script) async {
    final result = await _sessions.evaluate(_requireCurrentTab().id, script);
    return result.resultJson;
  }

  /// Decodes one JavaScript result returned by the real owner WebView.
  Future<Object?> evaluateValue(String script) async {
    return jsonDecode(await evaluate(script));
  }

  /// Clears cookies in the real owner WebView through Core.
  Future<void> clearCookies() async {
    await _sessions.clearCookies(_requireCurrentTab().id);
  }

  /// Resizes the owner compositor surface to the workspace viewport.
  Future<void> resizeSurface(String sessionId, Size size, double scaleFactor) {
    _queueSurfaceInteraction(sessionId, <String, Object?>{
      'type': 'resize',
      'width': size.width,
      'height': size.height,
      'scaleFactor': scaleFactor,
    });
    return Future<void>.value();
  }

  /// Moves the owner compositor cursor to a workspace-local position.
  void moveSurfaceCursor(String sessionId, Offset position) {
    _queueSurfaceInteraction(sessionId, <String, Object?>{
      'type': 'cursor',
      'x': position.dx,
      'y': position.dy,
    });
  }

  /// Changes one owner compositor pointer button state.
  void setSurfacePointerButton(
    String sessionId, {
    required int button,
    required bool isDown,
  }) {
    _queueSurfaceInteraction(sessionId, <String, Object?>{
      'type': 'button',
      'button': button,
      'isDown': isDown,
    });
  }

  /// Sends a workspace scroll delta to the owner compositor surface.
  void scrollSurface(String sessionId, double dx, double dy) {
    _queueSurfaceInteraction(sessionId, <String, Object?>{
      'type': 'scroll',
      'dx': dx,
      'dy': dy,
    });
  }

  /// Sends a keyboard event to the owner compositor surface.
  void keySurface(String sessionId, Map<String, Object?> event) {
    _queueSurfaceInteraction(sessionId, <String, Object?>{
      'type': 'key',
      ...event,
    });
  }

  /// Queues one compositor interaction for a frame-batched dispatch.
  void _queueSurfaceInteraction(
    String sessionId,
    Map<String, Object?> interaction,
  ) {
    final batch = _surfaceInteractionBatches.putIfAbsent(
      sessionId,
      _SurfaceInteractionBatch.new,
    );
    batch.add(interaction);
    if (_scheduledSurfaceInteractionBatches.add(sessionId)) {
      SchedulerBinding.instance.scheduleFrameCallback((_) {
        _flushSurfaceInteractionBatch(sessionId);
      });
    }
  }

  /// Sends all pending compositor interactions for one browser session.
  void _flushSurfaceInteractionBatch(String sessionId) {
    _scheduledSurfaceInteractionBatches.remove(sessionId);
    final batch = _surfaceInteractionBatches.remove(sessionId);
    if (batch == null || batch.isEmpty) {
      return;
    }
    unawaited(
      _sessions.interact(sessionId, <String, Object?>{
        'type': 'batch',
        'interactions': batch.toList(),
      }),
    );
  }

  /// Loads local stores and attaches every existing Core browser session.
  Future<void> _load() async {
    await stores.load();
    final sessions = await _sessions.listSessions();
    for (final session in sessions) {
      await _attachSession(session, select: session.active);
    }
  }

  /// Creates a shared compositor surface attachment for one session.
  Future<WorkspaceBrowserTabState> _attachSession(
    RuntimeBrowserSessionInfo info, {
    required bool select,
  }) async {
    final existing = _tabById(info.sessionId);
    if (existing != null) {
      _applySessionInfo(existing, info);
      if (select) {
        _selectedIndex = _tabs.indexOf(existing);
      }
      return existing;
    }
    final tab = WorkspaceBrowserTabState(
      id: info.sessionId,
      initialUrl: info.currentUrl,
      title: info.title,
      capabilities: const WorkspaceBrowserSessionCapabilities(
        pageJavaScript: false,
        pageHooks: false,
        permissionRequests: false,
        javaScriptDialogs: false,
      ),
      preferredUserAgent: info.userAgent,
    );
    _applySessionInfo(tab, info);
    _tabs.add(tab);
    sessionCount.value = _tabs.length;
    _subscriptions[tab.id] = _sessions
        .watchEvents(tab.id)
        .listen(
          (event) => _handleEvent(tab.id, event),
          onError: (Object error, StackTrace stackTrace) {
            tab.update(errorText: error.toString(), isLoading: false);
          },
        );
    final snapshot = await _sessions.getSnapshot(
      tab.id,
      displayIntent: _surfaceDisplayIntent(),
    );
    _applySurfaceDescriptor(tab, snapshot.resultJson);
    if (select) {
      _selectedIndex = _tabs.length - 1;
    }
    notifyListeners();
    return tab;
  }

  /// Applies one Core browser event to an attached surface.
  void _handleEvent(String sessionId, BrowserSessionEvent event) {
    final tab = _tabById(sessionId);
    if (tab == null) {
      return;
    }
    final info = event.session;
    if (info != null) {
      _applySessionInfo(tab, info);
    }
    if (event.eventType == 'surface' && event.resultJson.isNotEmpty) {
      _applySurfaceDescriptor(tab, event.resultJson);
    }
    if (event.eventType == 'surfaceFrame' && event.frameData.isNotEmpty) {
      tab.updateSurfaceFrame(
        WorkspaceBrowserSurfaceFrame(
          data: event.frameData,
          codec: event.frameCodec ?? '',
          width: event.frameWidth ?? 0,
          height: event.frameHeight ?? 0,
        ),
      );
    }
    if (event.eventType == 'popupCreated') {
      final decoded = jsonDecode(event.resultJson);
      if (decoded is! Map<String, Object?>) {
        throw StateError('Browser popup session is not a JSON object');
      }
      unawaited(
        _attachSession(
          RuntimeBrowserSessionInfo.fromJson(decoded),
          select: true,
        ),
      );
    }
    if (event.eventType == 'closed') {
      _removeSession(sessionId);
      return;
    }
    if (event.error != null) {
      tab.update(errorText: event.error, isLoading: false);
    }
    notifyListeners();
  }

  /// Copies typed Core session metadata into one view tab.
  void _applySessionInfo(
    WorkspaceBrowserTabState tab,
    RuntimeBrowserSessionInfo info,
  ) {
    tab.update(
      url: info.currentUrl,
      addressText: info.currentUrl,
      title: info.title.isEmpty ? info.currentUrl : info.title,
      isLoading: info.isLoading,
      canGoBack: info.canGoBack,
      canGoForward: info.canGoForward,
      progress: info.progress,
    );
  }

  /// Removes one surface attachment and its Core stream subscription.
  void _removeSession(String sessionId) {
    final index = _tabs.indexWhere((tab) => tab.id == sessionId);
    if (index < 0) {
      return;
    }
    final tab = _tabs.removeAt(index);
    sessionCount.value = _tabs.length;
    unawaited(_subscriptions.remove(sessionId)?.cancel());
    _surfaceInteractionBatches.remove(sessionId);
    _scheduledSurfaceInteractionBatches.remove(sessionId);
    tab.dispose();
    if (_tabs.isEmpty) {
      _selectedIndex = 0;
      _delegate?.onCloseRequested();
    } else if (_selectedIndex >= _tabs.length) {
      _selectedIndex = _tabs.length - 1;
    }
    notifyListeners();
  }

  /// Changes the zoom factor of the owner WebView.
  Future<void> _setZoomFactor(double value) async {
    final tab = _requireCurrentTab();
    final next = value.clamp(_minZoomFactor, _maxZoomFactor).toDouble();
    tab.update(zoomFactor: next);
    await _sessions.interact(tab.id, <String, Object?>{
      'type': 'zoom',
      'value': next,
    });
  }

  /// Applies one serialized compositor descriptor to a workspace tab.
  void _applySurfaceDescriptor(
    WorkspaceBrowserTabState tab,
    String descriptorJson,
  ) {
    final decoded = jsonDecode(descriptorJson);
    if (decoded is! Map<String, Object?>) {
      throw StateError('Browser surface descriptor is not a JSON object');
    }
    tab.updateSurfaceDescriptor(
      WorkspaceBrowserSurfaceDescriptor.fromJson(decoded),
    );
  }

  /// Builds the compositor transport requested by the current viewer.
  Map<String, Object?> _surfaceDisplayIntent() {
    final platform = defaultTargetPlatform;
    final sameProcessWindows =
        !kIsWeb &&
        RuntimeConnectionManager.instance.config.mode ==
            RuntimeConnectionMode.local &&
        platform == TargetPlatform.windows &&
        WidgetsBinding.instance.platformDispatcher.views.isNotEmpty;
    return <String, Object?>{
      'transport': sameProcessWindows
          ? _localTextureTransport
          : _encodedStreamTransport,
      'acceptedCodecs': const <String>['raw/bgra'],
    };
  }

  /// Returns the attached tab with the requested session identifier.
  WorkspaceBrowserTabState? _tabById(String sessionId) {
    for (final tab in _tabs) {
      if (tab.id == sessionId) {
        return tab;
      }
    }
    return null;
  }

  /// Returns the current tab or reports a missing attachment.
  WorkspaceBrowserTabState _requireCurrentTab() {
    final tab = currentTab;
    if (tab == null) {
      throw StateError('No browser session is attached');
    }
    return tab;
  }
}
