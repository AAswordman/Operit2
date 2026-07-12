// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../link/CoreLinkProtocol.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';

class BrowserSessionEvent {
  /// Creates one decoded browser event delivered by Core Link.
  const BrowserSessionEvent({
    required this.sequence,
    required this.sessionId,
    required this.eventType,
    required this.session,
    required this.resultJson,
    required this.frameData,
    required this.frameCodec,
    required this.frameWidth,
    required this.frameHeight,
    required this.error,
  });

  /// Decodes one serialized RuntimeBrowser stream event.
  factory BrowserSessionEvent.fromRuntime(RuntimeBrowserStreamEvent event) {
    return BrowserSessionEvent(
      sequence: event.sequence,
      sessionId: event.sessionId,
      eventType: event.eventType,
      session: event.session,
      resultJson: event.resultJson,
      frameData: event.frameData,
      frameCodec: event.frameCodec,
      frameWidth: event.frameWidth,
      frameHeight: event.frameHeight,
      error: event.error,
    );
  }

  final int sequence;
  final String sessionId;
  final String eventType;
  final RuntimeBrowserSessionInfo? session;
  final String resultJson;
  final Uint8List frameData;
  final String? frameCodec;
  final int? frameWidth;
  final int? frameHeight;
  final String? error;
}

class BrowserSessions {
  /// Creates a Core-only browser session client.
  BrowserSessions({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  final GeneratedCoreProxyClients _clients;
  Future<CorePushSink>? _interactionPush;

  GeneratedServicesRuntimeBrowserServiceCoreProxy get _browser =>
      _clients.servicesRuntimeBrowserService;

  /// Lists sessions owned by the runtime app browser host.
  Future<List<RuntimeBrowserSessionInfo>> listSessions() {
    return _browser.listBrowserSessions();
  }

  /// Creates a session in the runtime app browser host.
  Future<RuntimeBrowserSessionInfo> createSession({
    required String initialUrl,
    String? userAgent,
    Map<String, String> headers = const <String, String>{},
  }) {
    return _browser.createBrowserSession(
      initialUrl: initialUrl,
      userAgent: userAgent,
      headers: headers,
    );
  }

  /// Updates request metadata for one owner browser session.
  Future<RuntimeBrowserSessionInfo> updateSession({
    required String sessionId,
    required String? userAgent,
    Map<String, String> headers = const <String, String>{},
  }) {
    return _browser.updateBrowserSession(
      sessionId: sessionId,
      userAgent: userAgent,
      headers: headers,
    );
  }

  /// Reads a compositor attach descriptor from the real owner WebView.
  Future<RuntimeBrowserCommandResult> getSnapshot(
    String sessionId, {
    required Map<String, Object?> displayIntent,
  }) {
    return _submit(
      action: 'snapshot',
      sessionId: sessionId,
      payloadJson: jsonEncode(displayIntent),
    );
  }

  /// Watches semantic state and surface events for one browser session.
  Stream<BrowserSessionEvent> watchEvents(String sessionId) {
    return _browser
        .browserSessionEventsChanges(sessionId: sessionId)
        .map(BrowserSessionEvent.fromRuntime);
  }

  /// Activates one owner browser session.
  Future<RuntimeBrowserCommandResult> activate(String sessionId) {
    return _submit(action: 'select', sessionId: sessionId);
  }

  /// Navigates one owner browser session.
  Future<RuntimeBrowserCommandResult> navigate(String sessionId, String url) {
    return _submit(action: 'navigate', sessionId: sessionId, url: url);
  }

  /// Navigates one owner browser session backward.
  Future<RuntimeBrowserCommandResult> goBack(String sessionId) {
    return _submit(action: 'back', sessionId: sessionId);
  }

  /// Navigates one owner browser session forward.
  Future<RuntimeBrowserCommandResult> goForward(String sessionId) {
    return _submit(action: 'forward', sessionId: sessionId);
  }

  /// Reloads one owner browser session.
  Future<RuntimeBrowserCommandResult> reload(String sessionId) {
    return _submit(action: 'reload', sessionId: sessionId);
  }

  /// Stops one owner browser session load.
  Future<RuntimeBrowserCommandResult> stop(String sessionId) {
    return _submit(action: 'stop', sessionId: sessionId);
  }

  /// Sends one compositor surface interaction to the real owner WebView.
  Future<void> interact(
    String sessionId,
    Map<String, Object?> payload,
  ) async {
    final sink = await _interactionSink();
    sink.add(<String, Object?>{
      'command': RuntimeBrowserCommand(
        action: 'interact',
        sessionId: sessionId,
        url: null,
        script: null,
        payloadJson: jsonEncode(payload),
        userAgent: null,
        headers: const <String, String>{},
      ).toJson(),
    });
  }

  /// Evaluates a script in the real owner WebView session.
  Future<RuntimeBrowserCommandResult> evaluate(
    String sessionId,
    String script,
  ) {
    return _submit(action: 'evaluate', sessionId: sessionId, script: script);
  }

  /// Clears cookies owned by the runtime app WebView implementation.
  Future<RuntimeBrowserCommandResult> clearCookies(String sessionId) {
    return _submit(action: 'clearCookies', sessionId: sessionId);
  }

  /// Closes one owner browser session through Core.
  Future<void> close(String sessionId) {
    return _browser.closeBrowserSession(sessionId: sessionId);
  }

  /// Submits a complete typed browser command through Core Link.
  Future<RuntimeBrowserCommandResult> _submit({
    required String action,
    String? sessionId,
    String? url,
    String? script,
    String payloadJson = '',
  }) {
    return _browser.submitBrowserCommand(
      command: RuntimeBrowserCommand(
        action: action,
        sessionId: sessionId,
        url: url,
        script: script,
        payloadJson: payloadJson,
        userAgent: null,
        headers: const <String, String>{},
      ),
    );
  }

  /// Opens the shared client-owned stream for compositor interactions.
  Future<CorePushSink> _interactionSink() {
    final current = _interactionPush;
    if (current != null) {
      return current;
    }
    final opened = _browser.bridge.push(
      CorePushRequest(
        requestId:
            'browser-input-${DateTime.now().microsecondsSinceEpoch}',
        targetPath: _browser.targetPath,
        methodName: 'submitBrowserCommand',
      ),
    );
    _interactionPush = opened;
    return opened;
  }
}
