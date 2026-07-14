// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../browser/BrowserAutomationBridge.dart';
import '../browser/BrowserAutomationModels.dart';
import '../logging/ClientLogger.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../web_visit/WebVisitBridge.dart';
import '../web_visit/WebVisitModels.dart';
import 'ComposeWebViewControllerBridge.dart';
import 'browser/RuntimeBrowserSessionRegistry.dart';

class _BrowserInteractHostTiming {
  const _BrowserInteractHostTiming({
    required this.sessionId,
    required this.acceptElapsedUs,
    required this.handleElapsedUs,
  });

  final String? sessionId;
  final int acceptElapsedUs;
  final int handleElapsedUs;
}

class _BrowserInteractHostStats {
  final Stopwatch windowStopwatch = Stopwatch()..start();
  int count = 0;
  int errorCount = 0;
  int totalAcceptElapsedUs = 0;
  int totalHandleElapsedUs = 0;
  int totalRespondElapsedUs = 0;
  int totalRoundTripElapsedUs = 0;
  int maxAcceptElapsedUs = 0;
  int maxHandleElapsedUs = 0;
  int maxRespondElapsedUs = 0;
  int maxRoundTripElapsedUs = 0;
  String? slowestSessionId;
}

class RuntimeHostInteractionSubscriber {
  RuntimeHostInteractionSubscriber._();

  static const MethodChannel _channel = MethodChannel('operit/runtime');
  static const String _browserInputLogTag = 'RuntimeBrowserInput';
  static const int _browserInteractHostSummaryIntervalMs = 500;
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );
  static const List<RuntimeHostInteractionKind> _ownerKinds =
      <RuntimeHostInteractionKind>[
        RuntimeHostInteractionKind.browserAutomation,
        RuntimeHostInteractionKind.browserSession,
        RuntimeHostInteractionKind.webVisit,
        RuntimeHostInteractionKind.composeWebViewController,
        RuntimeHostInteractionKind.systemCaptureScreenshot,
        RuntimeHostInteractionKind.systemLanguageCode,
        RuntimeHostInteractionKind.systemRecognizeText,
        RuntimeHostInteractionKind.systemOperation,
        RuntimeHostInteractionKind.fileOpen,
        RuntimeHostInteractionKind.fileShare,
        RuntimeHostInteractionKind.audioPlay,
        RuntimeHostInteractionKind.musicPlayback,
        RuntimeHostInteractionKind.bluetooth,
        RuntimeHostInteractionKind.ttsSynthesis,
        RuntimeHostInteractionKind.ttsPlayback,
        RuntimeHostInteractionKind.localInference,
      ];

  static StreamSubscription<RuntimeHostInteractionRequest>? _subscription;
  static int _ttsPlaybackOwnerSequence = 0;
  static final Map<String, _BrowserInteractHostTiming>
  _browserInteractHostTimings = <String, _BrowserInteractHostTiming>{};
  static _BrowserInteractHostStats _browserInteractHostStats =
      _BrowserInteractHostStats();

  /// Installs the owner host interaction stream listener.
  static void install() {
    if (_subscription != null) {
      return;
    }
    _subscription = _clients.servicesRuntimeHostInteractionService
        .ownerHostInteractionEventsChanges(kinds: _ownerKinds)
        .listen(
          (event) => unawaited(_handleEvent(event)),
          onError: (Object error, StackTrace stackTrace) {
            FlutterError.reportError(
              FlutterErrorDetails(
                exception: error,
                stack: stackTrace,
                library: 'runtime host interaction subscriber',
                context: ErrorDescription(
                  'listening owner host interaction stream',
                ),
              ),
            );
          },
        );
  }

  /// Stops the owner host interaction stream listener.
  static Future<void> uninstall() async {
    final subscription = _subscription;
    _subscription = null;
    await subscription?.cancel();
  }

  static Future<void> _handleEvent(
    RuntimeHostInteractionRequest request,
  ) async {
    final eventStopwatch = Stopwatch()..start();
    try {
      final response = await _dispatch(request);
      final respondStopwatch = Stopwatch()..start();
      await _respondOwnerHostInteraction(request.requestId, response);
      respondStopwatch.stop();
      _recordBrowserInteractHostResponseTiming(
        request.requestId,
        respondElapsedUs: respondStopwatch.elapsedMicroseconds,
        roundTripElapsedUs: eventStopwatch.elapsedMicroseconds,
        failed: false,
      );
    } catch (error, stackTrace) {
      final response = _errorResponse(error);
      try {
        await _respondOwnerHostInteraction(request.requestId, response);
      } catch (respondError, respondStackTrace) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: respondError,
            stack: respondStackTrace,
            library: 'runtime host interaction subscriber',
            context: ErrorDescription(
              'responding owner host interaction error',
            ),
          ),
        );
      }
      _recordBrowserInteractHostResponseTiming(
        request.requestId,
        respondElapsedUs: 0,
        roundTripElapsedUs: eventStopwatch.elapsedMicroseconds,
        failed: true,
      );
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'runtime host interaction subscriber',
          context: ErrorDescription('handling owner host interaction event'),
        ),
      );
    }
  }

  /// Sends one owner host interaction response through the runtime service.
  static Future<void> _respondOwnerHostInteraction(
    String requestId,
    RuntimeHostInteractionResponse response,
  ) {
    return _clients.servicesRuntimeHostInteractionService
        .respondOwnerHostInteraction(requestId: requestId, response: response);
  }

  /// Builds a typed error response for one failed owner interaction.
  static RuntimeHostInteractionResponse _errorResponse(Object error) {
    return _response(error: error.toString());
  }

  /// Dispatches one owner host interaction request by kind.
  static Future<RuntimeHostInteractionResponse> _dispatch(
    RuntimeHostInteractionRequest request,
  ) {
    return switch (request.kind) {
      RuntimeHostInteractionKind.browserAutomation => _handleBrowserAutomation(
        _requirePayload(request.browserAutomation, request.kind),
      ),
      RuntimeHostInteractionKind.browserSession => _handleBrowserSession(
        _requirePayload(request.browserSession, request.kind),
        request.requestId,
      ),
      RuntimeHostInteractionKind.webVisit => _handleWebVisit(
        _requirePayload(request.webVisit, request.kind),
      ),
      RuntimeHostInteractionKind.composeWebViewController =>
        _handleComposeWebViewController(
          _requirePayload(request.composeWebViewController, request.kind),
        ),
      RuntimeHostInteractionKind.systemCaptureScreenshot =>
        _handleSystemCaptureScreenshot(),
      RuntimeHostInteractionKind.systemLanguageCode =>
        _handleSystemLanguageCode(),
      RuntimeHostInteractionKind.systemRecognizeText =>
        _handleSystemRecognizeText(
          _requirePayload(request.systemRecognizeText, request.kind),
        ),
      RuntimeHostInteractionKind.systemOperation => _handleSystemOperation(
        _requirePayload(request.systemOperation, request.kind),
      ),
      RuntimeHostInteractionKind.fileOpen => _handleFileOpen(
        _requirePayload(request.fileOpen, request.kind),
      ),
      RuntimeHostInteractionKind.fileShare => _handleFileShare(
        _requirePayload(request.fileShare, request.kind),
      ),
      RuntimeHostInteractionKind.audioPlay => _handleAudioPlay(
        _requirePayload(request.audioPlay, request.kind),
      ),
      RuntimeHostInteractionKind.musicPlayback => _handleMusicPlayback(
        _requirePayload(request.musicPlayback, request.kind),
      ),
      RuntimeHostInteractionKind.bluetooth => _handleBluetooth(
        _requirePayload(request.bluetooth, request.kind),
      ),
      RuntimeHostInteractionKind.ttsSynthesis => _handleTtsSynthesis(
        _requirePayload(request.ttsSynthesis, request.kind),
      ),
      RuntimeHostInteractionKind.ttsPlayback => _handleTtsPlayback(
        _requirePayload(request.ttsPlayback, request.kind),
      ),
      RuntimeHostInteractionKind.localInference => _handleLocalInference(
        _requirePayload(request.localInference, request.kind),
      ),
      RuntimeHostInteractionKind.toolPermission => throw StateError(
        'tool permission is handled by the approval bridge',
      ),
    };
  }

  /// Handles one browser automation owner request.
  static Future<RuntimeHostInteractionResponse> _handleBrowserAutomation(
    RuntimeHostInteractionBrowserAutomationPayload payload,
  ) async {
    final request = BrowserAutomationRequest.fromHostPayload(payload);
    try {
      final response = await BrowserAutomationBridge.handle(request);
      return _response(
        browserAutomation: RuntimeHostInteractionBrowserAutomationResponse(
          requestId: response.requestId,
          success: response.success,
          result: response.result,
          error: response.error,
        ),
      );
    } catch (error) {
      return _response(
        browserAutomation: RuntimeHostInteractionBrowserAutomationResponse(
          requestId: request.requestId,
          success: false,
          result: '',
          error: error.toString(),
        ),
      );
    }
  }

  /// Handles one browser session owner request.
  static Future<RuntimeHostInteractionResponse> _handleBrowserSession(
    RuntimeHostInteractionBrowserSessionPayload payload,
    String requestId,
  ) async {
    final handleStopwatch = Stopwatch()..start();
    final command = RuntimeBrowserCommand.fromJson(
      jsonDecode(payload.commandJson) as Map<String, Object?>,
    );
    final RuntimeBrowserCommandResult result;
    if (command.action == 'interact') {
      final acceptStopwatch = Stopwatch()..start();
      result = RuntimeBrowserSessionRegistry.instance
          .acceptRuntimeBrowserInteraction(command);
      acceptStopwatch.stop();
      final response = _response(
        browserSession: RuntimeHostInteractionBrowserSessionResponse(
          resultJson: jsonEncode(result.toJson()),
        ),
      );
      handleStopwatch.stop();
      _browserInteractHostTimings[requestId] = _BrowserInteractHostTiming(
        sessionId: result.session?.sessionId,
        acceptElapsedUs: acceptStopwatch.elapsedMicroseconds,
        handleElapsedUs: handleStopwatch.elapsedMicroseconds,
      );
      return response;
    }
    result = await RuntimeBrowserSessionRegistry.instance
        .handleRuntimeBrowserCommand(command);
    return _response(
      browserSession: RuntimeHostInteractionBrowserSessionResponse(
        resultJson: jsonEncode(result.toJson()),
      ),
    );
  }

  /// Records one browser interaction host response timing.
  static void _recordBrowserInteractHostResponseTiming(
    String requestId, {
    required int respondElapsedUs,
    required int roundTripElapsedUs,
    required bool failed,
  }) {
    final timing = _browserInteractHostTimings.remove(requestId);
    if (timing == null) {
      return;
    }
    final stats = _browserInteractHostStats;
    stats.count += 1;
    if (failed) {
      stats.errorCount += 1;
    }
    stats.totalAcceptElapsedUs += timing.acceptElapsedUs;
    stats.totalHandleElapsedUs += timing.handleElapsedUs;
    stats.totalRespondElapsedUs += respondElapsedUs;
    stats.totalRoundTripElapsedUs += roundTripElapsedUs;
    if (timing.acceptElapsedUs > stats.maxAcceptElapsedUs) {
      stats.maxAcceptElapsedUs = timing.acceptElapsedUs;
    }
    if (timing.handleElapsedUs > stats.maxHandleElapsedUs) {
      stats.maxHandleElapsedUs = timing.handleElapsedUs;
    }
    if (respondElapsedUs > stats.maxRespondElapsedUs) {
      stats.maxRespondElapsedUs = respondElapsedUs;
    }
    if (roundTripElapsedUs > stats.maxRoundTripElapsedUs) {
      stats.maxRoundTripElapsedUs = roundTripElapsedUs;
      stats.slowestSessionId = timing.sessionId;
    }
    if (stats.windowStopwatch.elapsedMilliseconds >=
        _browserInteractHostSummaryIntervalMs) {
      _logBrowserInteractHostSummary(stats);
      _browserInteractHostStats = _BrowserInteractHostStats();
    }
  }

  /// Logs one compact browser interaction host timing summary.
  static void _logBrowserInteractHostSummary(_BrowserInteractHostStats stats) {
    ClientLogger.d(
      'host summary elapsedMs=${stats.windowStopwatch.elapsedMilliseconds} '
      'accepted=${stats.count} errors=${stats.errorCount} '
      'avgAcceptUs=${_averageMicroseconds(stats.totalAcceptElapsedUs, stats.count)} '
      'maxAcceptUs=${stats.maxAcceptElapsedUs} '
      'avgHandleUs=${_averageMicroseconds(stats.totalHandleElapsedUs, stats.count)} '
      'maxHandleUs=${stats.maxHandleElapsedUs} '
      'avgRespondUs=${_averageMicroseconds(stats.totalRespondElapsedUs, stats.count)} '
      'maxRespondUs=${stats.maxRespondElapsedUs} '
      'avgRoundTripUs=${_averageMicroseconds(stats.totalRoundTripElapsedUs, stats.count)} '
      'maxRoundTripUs=${stats.maxRoundTripElapsedUs} '
      'slowestSession=${stats.slowestSessionId}',
      tag: _browserInputLogTag,
    );
  }

  /// Formats an average duration in microseconds.
  static String _averageMicroseconds(int totalElapsedUs, int count) {
    return (totalElapsedUs / count).toStringAsFixed(1);
  }

  /// Handles one web visit owner request.
  static Future<RuntimeHostInteractionResponse> _handleWebVisit(
    RuntimeHostInteractionWebVisitPayload payload,
  ) async {
    final request = WebVisitRequest.fromHostPayload(payload);
    try {
      final response = await WebVisitBridge.handle(request);
      return _response(webVisit: _webVisitResponse(response));
    } catch (error) {
      return _response(
        webVisit: RuntimeHostInteractionWebVisitResponse(
          requestId: request.requestId,
          success: false,
          result: null,
          error: error.toString(),
        ),
      );
    }
  }

  static Future<RuntimeHostInteractionResponse> _handleComposeWebViewController(
    RuntimeHostInteractionComposeWebViewControllerPayload payload,
  ) async {
    final result = await ComposeWebViewControllerBridge.handle(
      payload.commandJson,
    );
    return _response(
      composeWebViewController:
          RuntimeHostInteractionComposeWebViewControllerResponse(
            result: result,
          ),
    );
  }

  static Future<RuntimeHostInteractionResponse>
  _handleSystemCaptureScreenshot() async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerSystemCaptureScreenshot',
    );
    final response =
        RuntimeHostInteractionSystemCaptureScreenshotResponse.fromJson(
          _requireMethodResponseMap(
            rawResponse,
            'ownerSystemCaptureScreenshot',
          ),
        );
    return _response(systemCaptureScreenshot: response);
  }

  static Future<RuntimeHostInteractionResponse>
  _handleSystemLanguageCode() async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerSystemLanguageCode',
    );
    final response = RuntimeHostInteractionSystemLanguageCodeResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerSystemLanguageCode'),
    );
    return _response(systemLanguageCode: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleSystemRecognizeText(
    RuntimeHostInteractionSystemRecognizeTextPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerSystemRecognizeText',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionSystemRecognizeTextResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerSystemRecognizeText'),
    );
    return _response(systemRecognizeText: response);
  }

  /// Handles a generic system-operation request through the owner MethodChannel.
  static Future<RuntimeHostInteractionResponse> _handleSystemOperation(
    RuntimeHostInteractionSystemOperationPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerSystemOperation',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionSystemOperationResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerSystemOperation'),
    );
    return _response(systemOperation: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleAudioPlay(
    RuntimeHostInteractionAudioPlayPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerAudioPlay',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionAudioPlayResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerAudioPlay'),
    );
    return _response(audioPlay: response);
  }

  /// Handles a file-open request through the owner MethodChannel.
  static Future<RuntimeHostInteractionResponse> _handleFileOpen(
    RuntimeHostInteractionFileOpenPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerFileOpen',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionFileOperationResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerFileOpen'),
    );
    return _response(fileOpen: response);
  }

  /// Handles a file-share request through the owner MethodChannel.
  static Future<RuntimeHostInteractionResponse> _handleFileShare(
    RuntimeHostInteractionFileSharePayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerFileShare',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionFileOperationResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerFileShare'),
    );
    return _response(fileShare: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleMusicPlayback(
    RuntimeHostInteractionMusicPlaybackPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerMusicPlayback',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionMusicPlaybackResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerMusicPlayback'),
    );
    return _response(musicPlayback: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleBluetooth(
    RuntimeHostInteractionBluetoothPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerBluetooth',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionBluetoothResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerBluetooth'),
    );
    return _response(bluetooth: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleTtsSynthesis(
    RuntimeHostInteractionTtsSynthesisPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerTtsSynthesize',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionTtsSynthesisResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerTtsSynthesize'),
    );
    return _response(ttsSynthesis: response);
  }

  static T _requirePayload<T>(T? payload, RuntimeHostInteractionKind kind) {
    if (payload == null) {
      throw StateError('$kind payload is missing');
    }
    return payload;
  }

  static Map<String, Object?> _requireMethodResponseMap(
    Object? rawResponse,
    String method,
  ) {
    if (rawResponse is! Map<Object?, Object?>) {
      throw StateError('$method response must be a map');
    }
    return rawResponse.map((key, value) {
      if (key is! String) {
        throw StateError('$method response key must be a string: $key');
      }
      return MapEntry(key, value);
    });
  }

  static RuntimeHostInteractionWebVisitResponse _webVisitResponse(
    WebVisitResponse response,
  ) {
    return RuntimeHostInteractionWebVisitResponse(
      requestId: response.requestId,
      success: response.success,
      result: response.result == null
          ? null
          : _webVisitResult(response.result!),
      error: response.error,
    );
  }

  /// Forwards one insertion-ordered TTS command with an owner sequence.
  static Future<RuntimeHostInteractionResponse> _handleTtsPlayback(
    RuntimeHostInteractionTtsPlaybackPayload payload,
  ) async {
    final methodPayload = payload.toJson();
    methodPayload['ownerSequence'] = ++_ttsPlaybackOwnerSequence;
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerTtsPlayback',
      methodPayload,
    );
    final response = RuntimeHostInteractionTtsPlaybackResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerTtsPlayback'),
    );
    return _response(ttsPlayback: response);
  }

  /// Handles one owner-host local inference command.
  static Future<RuntimeHostInteractionResponse> _handleLocalInference(
    RuntimeHostInteractionLocalInferencePayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerLocalInference',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionLocalInferenceResponse.fromJson(
      _requireMethodResponseMap(rawResponse, 'ownerLocalInference'),
    );
    return _response(localInference: response);
  }

  static RuntimeHostInteractionWebVisitResult _webVisitResult(
    WebVisitResult result,
  ) {
    return RuntimeHostInteractionWebVisitResult(
      url: result.url,
      title: result.title,
      content: result.content,
      metadata: result.metadata.entries
          .map(
            (entry) => RuntimeHostInteractionWebVisitMetadataEntry(
              name: entry.key,
              value: entry.value,
            ),
          )
          .toList(growable: false),
      links: result.links
          .map(
            (link) => RuntimeHostInteractionWebVisitLink(
              url: link.url,
              text: link.text,
            ),
          )
          .toList(growable: false),
      imageLinks: result.imageLinks,
    );
  }

  static RuntimeHostInteractionResponse _response({
    String? error,
    RuntimeHostInteractionBrowserAutomationResponse? browserAutomation,
    RuntimeHostInteractionBrowserSessionResponse? browserSession,
    RuntimeHostInteractionWebVisitResponse? webVisit,
    RuntimeHostInteractionComposeWebViewControllerResponse?
    composeWebViewController,
    RuntimeHostInteractionSystemCaptureScreenshotResponse?
    systemCaptureScreenshot,
    RuntimeHostInteractionSystemLanguageCodeResponse? systemLanguageCode,
    RuntimeHostInteractionSystemRecognizeTextResponse? systemRecognizeText,
    RuntimeHostInteractionSystemOperationResponse? systemOperation,
    RuntimeHostInteractionFileOperationResponse? fileOpen,
    RuntimeHostInteractionFileOperationResponse? fileShare,
    RuntimeHostInteractionAudioPlayResponse? audioPlay,
    RuntimeHostInteractionMusicPlaybackResponse? musicPlayback,
    RuntimeHostInteractionBluetoothResponse? bluetooth,
    RuntimeHostInteractionTtsSynthesisResponse? ttsSynthesis,
    RuntimeHostInteractionTtsPlaybackResponse? ttsPlayback,
    RuntimeHostInteractionLocalInferenceResponse? localInference,
    RuntimeHostInteractionToolPermissionResponse? toolPermission,
  }) {
    return RuntimeHostInteractionResponse(
      error: error,
      browserAutomation: browserAutomation,
      browserSession: browserSession,
      webVisit: webVisit,
      composeWebViewController: composeWebViewController,
      systemCaptureScreenshot: systemCaptureScreenshot,
      systemLanguageCode: systemLanguageCode,
      systemRecognizeText: systemRecognizeText,
      systemOperation: systemOperation,
      fileOpen: fileOpen,
      fileShare: fileShare,
      audioPlay: audioPlay,
      musicPlayback: musicPlayback,
      bluetooth: bluetooth,
      ttsSynthesis: ttsSynthesis,
      ttsPlayback: ttsPlayback,
      localInference: localInference,
      toolPermission: toolPermission,
    );
  }
}
