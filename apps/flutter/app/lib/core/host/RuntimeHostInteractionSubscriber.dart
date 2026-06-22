// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import '../../ui/features/packages/screens/ToolPkgComposeDslWebView.dart';
import '../bridge/PlatformCoreProxy.dart';
import '../bridge/ProxyCoreRuntimeBridge.dart';
import '../browser/BrowserAutomationBridge.dart';
import '../browser/BrowserAutomationModels.dart';
import '../proxy/generated/CoreProxyClients.g.dart';
import '../proxy/generated/CoreProxyModels.g.dart';
import '../web_visit/WebVisitBridge.dart';
import '../web_visit/WebVisitModels.dart';

class RuntimeHostInteractionSubscriber {
  RuntimeHostInteractionSubscriber._();

  static const MethodChannel _channel = MethodChannel('operit/runtime');
  static const GeneratedCoreProxyClients _clients = GeneratedCoreProxyClients(
    ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
  );
  static const List<RuntimeHostInteractionKind> _ownerKinds =
      <RuntimeHostInteractionKind>[
    RuntimeHostInteractionKind.browserAutomation,
    RuntimeHostInteractionKind.webVisit,
    RuntimeHostInteractionKind.composeWebViewController,
    RuntimeHostInteractionKind.systemCaptureScreenshot,
    RuntimeHostInteractionKind.systemRecognizeText,
    RuntimeHostInteractionKind.audioPlay,
    RuntimeHostInteractionKind.ttsSynthesis,
  ];

  static StreamSubscription<RuntimeHostInteractionRequest>? _subscription;

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

  static Future<void> _handleEvent(RuntimeHostInteractionRequest request) async {
    try {
      final response = await _dispatch(request);
      await _clients.servicesRuntimeHostInteractionService
          .respondOwnerHostInteraction(
            requestId: request.requestId,
            response: response,
          );
    } catch (error, stackTrace) {
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

  static Future<RuntimeHostInteractionResponse> _dispatch(
    RuntimeHostInteractionRequest request,
  ) {
    return switch (request.kind) {
      RuntimeHostInteractionKind.browserAutomation => _handleBrowserAutomation(
          _requirePayload(request.browserAutomation, request.kind),
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
      RuntimeHostInteractionKind.systemRecognizeText => _handleSystemRecognizeText(
          _requirePayload(request.systemRecognizeText, request.kind),
        ),
      RuntimeHostInteractionKind.audioPlay => _handleAudioPlay(
          _requirePayload(request.audioPlay, request.kind),
        ),
      RuntimeHostInteractionKind.ttsSynthesis => _handleTtsSynthesis(
          _requirePayload(request.ttsSynthesis, request.kind),
        ),
      RuntimeHostInteractionKind.toolPermission => throw StateError(
          'tool permission is handled by the approval bridge',
        ),
    };
  }

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
    final result = await ComposeDslWebViewHostRegistry.handleControllerCommand(
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
    final response = RuntimeHostInteractionSystemCaptureScreenshotResponse
        .fromJson(rawResponse as Map<String, Object?>);
    return _response(systemCaptureScreenshot: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleSystemRecognizeText(
    RuntimeHostInteractionSystemRecognizeTextPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerSystemRecognizeText',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionSystemRecognizeTextResponse.fromJson(
      rawResponse as Map<String, Object?>,
    );
    return _response(systemRecognizeText: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleAudioPlay(
    RuntimeHostInteractionAudioPlayPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerAudioPlay',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionAudioPlayResponse.fromJson(
      rawResponse as Map<String, Object?>,
    );
    return _response(audioPlay: response);
  }

  static Future<RuntimeHostInteractionResponse> _handleTtsSynthesis(
    RuntimeHostInteractionTtsSynthesisPayload payload,
  ) async {
    final rawResponse = await _channel.invokeMethod<Object?>(
      'ownerTtsSynthesize',
      payload.toJson(),
    );
    final response = RuntimeHostInteractionTtsSynthesisResponse.fromJson(
      rawResponse as Map<String, Object?>,
    );
    return _response(ttsSynthesis: response);
  }

  static T _requirePayload<T>(T? payload, RuntimeHostInteractionKind kind) {
    if (payload == null) {
      throw StateError('$kind payload is missing');
    }
    return payload;
  }

  static RuntimeHostInteractionWebVisitResponse _webVisitResponse(
    WebVisitResponse response,
  ) {
    return RuntimeHostInteractionWebVisitResponse(
      requestId: response.requestId,
      success: response.success,
      result: response.result == null ? null : _webVisitResult(response.result!),
      error: response.error,
    );
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
    RuntimeHostInteractionBrowserAutomationResponse? browserAutomation,
    RuntimeHostInteractionWebVisitResponse? webVisit,
    RuntimeHostInteractionComposeWebViewControllerResponse?
        composeWebViewController,
    RuntimeHostInteractionSystemCaptureScreenshotResponse?
        systemCaptureScreenshot,
    RuntimeHostInteractionSystemRecognizeTextResponse? systemRecognizeText,
    RuntimeHostInteractionAudioPlayResponse? audioPlay,
    RuntimeHostInteractionTtsSynthesisResponse? ttsSynthesis,
    RuntimeHostInteractionToolPermissionResponse? toolPermission,
  }) {
    return RuntimeHostInteractionResponse(
      browserAutomation: browserAutomation,
      webVisit: webVisit,
      composeWebViewController: composeWebViewController,
      systemCaptureScreenshot: systemCaptureScreenshot,
      systemRecognizeText: systemRecognizeText,
      audioPlay: audioPlay,
      ttsSynthesis: ttsSynthesis,
      toolPermission: toolPermission,
    );
  }
}
