// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';
import 'dart:ui';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:webview_all/webview_all.dart';
import 'package:webview_all_windows/webview_all_windows.dart';

class RuntimeBrowserAutomationController {
  RuntimeBrowserAutomationController({
    required this.controller,
    required this.onSurfaceFrameRequested,
  });

  final WebViewController controller;
  final Future<void> Function() onSurfaceFrameRequested;
  final List<Map<String, Object?>> _consoleMessages = <Map<String, Object?>>[];
  final List<Map<String, Object?>> _networkRequests = <Map<String, Object?>>[];
  bool _encodedStreamAttached = false;
  Size? _surfaceSize;
  bool _surfaceFrameCaptureActive = false;
  bool _surfaceFrameCapturePending = false;
  final Map<int, WindowsBrowserSurfacePointerButton> _pressedPointerButtons =
      <int, WindowsBrowserSurfacePointerButton>{};

  /// Dispatches a compositor surface interaction into the real WebView.
  Future<void> dispatchSurfaceInteraction(String payloadJson) async {
    final payload = jsonDecode(payloadJson);
    if (payload is! Map<String, Object?>) {
      throw StateError('Browser surface interaction is not a JSON object');
    }
    final platform = _requireWindowsController();
    await _dispatchSurfaceInteraction(platform, payload);
    requestSurfaceFrame();
  }

  /// Dispatches one decoded compositor surface interaction.
  Future<void> _dispatchSurfaceInteraction(
    WindowsWebViewController platform,
    Map<String, Object?> payload,
  ) async {
    switch (payload['type'] as String?) {
      case 'batch':
        await _dispatchSurfaceInteractionBatch(platform, payload);
        return;
      case 'pointer':
        await _dispatchSurfacePointer(platform, payload);
        return;
      case 'resize':
        final size = Size(
          _readDouble(payload, 'width'),
          _readDouble(payload, 'height'),
        );
        await platform.resizeBrowserSurface(
          size,
          _readDouble(payload, 'scaleFactor'),
        );
        _surfaceSize = size;
        return;
      case 'cursor':
        await platform.moveBrowserSurfaceCursor(
          Offset(_readDouble(payload, 'x'), _readDouble(payload, 'y')),
        );
        return;
      case 'button':
        await platform.setBrowserSurfacePointerButton(
          _surfaceButton(payload['button']),
          isDown: payload['isDown'] as bool,
        );
        return;
      case 'scroll':
        await platform.scrollBrowserSurface(
          _readDouble(payload, 'dx'),
          _readDouble(payload, 'dy'),
        );
        return;
      case 'zoom':
        await platform.setZoomFactor(_readDouble(payload, 'value'));
        return;
      case 'key':
        await platform.dispatchBrowserSurfaceKeyEvent(
          type: _readString(payload, 'keyEventType'),
          key: _readString(payload, 'key'),
          code: _readString(payload, 'code'),
          windowsVirtualKeyCode: _readInt(payload, 'windowsVirtualKeyCode'),
          nativeVirtualKeyCode: _readInt(payload, 'nativeVirtualKeyCode'),
          modifiers: _readInt(payload, 'modifiers'),
          text: payload['text'] as String?,
          unmodifiedText: payload['unmodifiedText'] as String?,
        );
        return;
      default:
        throw StateError('Unknown browser surface interaction');
    }
  }

  /// Dispatches one raw Flutter pointer event into the compositor surface.
  Future<void> _dispatchSurfacePointer(
    WindowsWebViewController platform,
    Map<String, Object?> payload,
  ) async {
    await platform.moveBrowserSurfaceCursor(
      Offset(_readDouble(payload, 'x'), _readDouble(payload, 'y')),
    );
    final eventType = _readString(payload, 'eventType');
    final pointer = _readInt(payload, 'pointer');
    switch (eventType) {
      case 'move':
        return;
      case 'down':
        final button = _surfaceButtonForRawButtons(
          _readInt(payload, 'buttons'),
        );
        _pressedPointerButtons[pointer] = button;
        await platform.setBrowserSurfacePointerButton(button, isDown: true);
        return;
      case 'up':
      case 'cancel':
        final button = _pressedPointerButtons.remove(pointer);
        if (button == null) {
          throw StateError(
            'Browser surface pointer ended before a button was pressed: '
            '$pointer',
          );
        }
        await platform.setBrowserSurfacePointerButton(button, isDown: false);
        return;
      default:
        throw StateError('Unknown browser surface pointer event: $eventType');
    }
  }

  /// Dispatches a decoded compositor surface interaction batch.
  Future<void> _dispatchSurfaceInteractionBatch(
    WindowsWebViewController platform,
    Map<String, Object?> payload,
  ) async {
    final interactions = payload['interactions'];
    if (interactions is! List<Object?>) {
      throw StateError('Browser surface interaction batch is not a list');
    }
    for (final item in interactions) {
      if (item is! Map<Object?, Object?>) {
        throw StateError('Browser surface batch item is not a JSON object');
      }
      final interaction = <String, Object?>{};
      for (final entry in item.entries) {
        final key = entry.key;
        if (key is! String) {
          throw StateError('Browser surface batch item key is not a string');
        }
        interaction[key] = entry.value;
      }
      await _dispatchSurfaceInteraction(platform, interaction);
    }
  }

  /// Captures the current owner compositor surface descriptor.
  Future<String> browserSurfaceDescriptor(String intentJson) async {
    final intent = _decodeSurfaceIntent(intentJson);
    final requestedTransport = _readString(intent, 'transport');
    switch (requestedTransport) {
      case 'localTexture':
        final platform = _requireWindowsController();
        final textureId = await platform.browserSurfaceTextureId();
        return jsonEncode(<String, Object?>{
          'transport': 'localTexture',
          'platform': 'windows',
          'textureId': textureId,
        });
      case 'encodedStream':
        _encodedStreamAttached = true;
        final size = _surfaceSize;
        requestSurfaceFrame();
        return jsonEncode(<String, Object?>{
          'transport': 'encodedStream',
          'platform': 'windows',
          'streamId': _surfaceStreamId,
          'codec': 'raw/bgra',
          'width': size?.width ?? 0,
          'height': size?.height ?? 0,
        });
      default:
        throw StateError('Invalid browser surface transport');
    }
  }

  Future<String> pageState() async {
    final url = await controller.currentUrl();
    final title = await controller.getTitle();
    return jsonEncode(<String, Object?>{'url': url, 'title': title});
  }

  Future<Object> evaluate(String expression) {
    return controller.runJavaScriptReturningResult(expression);
  }

  Future<Object> evaluateFunction(String function, {String? selector}) {
    final target = selector?.trim();
    if (target == null || target.isEmpty) {
      return controller.runJavaScriptReturningResult('($function)()');
    }
    return controller.runJavaScriptReturningResult(
      '($function)(${_resolverScript(target)})',
    );
  }

  Future<Object> runCode(String code) {
    return controller.runJavaScriptReturningResult(code);
  }

  Future<Object> snapshot() {
    return controller.runJavaScriptReturningResult(r'''
JSON.stringify((function() {
  const selector = 'a,button,input,textarea,select,[role]';
  const result = [];
  function collect(doc, framePath, originX, originY) {
    Array.from(doc.querySelectorAll(selector)).slice(0, 200).forEach(function(el, index) {
      const rect = el.getBoundingClientRect();
      const label = el.getAttribute('aria-label') || el.getAttribute('placeholder') || el.title || '';
      result.push({
        ref: framePath.concat([index]).join(':'),
        tag: el.tagName.toLowerCase(),
        role: el.getAttribute('role') || '',
        label: String(label).trim().slice(0, 160),
        text: (el.innerText || el.value || label || '').trim().slice(0, 160),
        x: Math.round(originX + rect.x),
        y: Math.round(originY + rect.y),
        width: Math.round(rect.width),
        height: Math.round(rect.height)
      });
    });
    Array.from(doc.querySelectorAll('iframe')).forEach(function(frame, frameIndex) {
      try {
        const rect = frame.getBoundingClientRect();
        if (frame.contentDocument) {
          collect(frame.contentDocument, framePath.concat(['f' + frameIndex]), originX + rect.x, originY + rect.y);
        }
      } catch (error) {}
    });
  }
  collect(document, ['el'], 0, 0);
  return result.slice(0, 300);
})())
''');
  }

  Future<void> click(String selector) {
    return controller.runJavaScript("${_resolverScript(selector)}?.click();");
  }

  Future<void> type(String selector, String text) {
    return controller.runJavaScript('''
var el = ${_resolverScript(selector)};
if (el) {
  el.focus();
  el.value = ${jsonEncode(text)};
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
}
''');
  }

  Future<void> pressKey(String key) {
    return controller.runJavaScript('''
document.activeElement && document.activeElement.dispatchEvent(new KeyboardEvent('keydown', {
  key: ${jsonEncode(key)},
  bubbles: true
}));
''');
  }

  Future<void> scrollBy(int x, int y) {
    return controller.scrollBy(x, y);
  }

  void addConsoleMessage(JavaScriptConsoleMessage message) {
    _consoleMessages.insert(0, <String, Object?>{
      'level': message.level.name,
      'message': message.message,
      'createdAt': DateTime.now().toIso8601String(),
    });
    if (_consoleMessages.length > 300) {
      _consoleMessages.removeRange(300, _consoleMessages.length);
    }
  }

  String consoleMessages({String? level}) {
    final messages = level == null
        ? _consoleMessages
        : _consoleMessages
              .where((item) => item['level'] == level)
              .toList(growable: false);
    return jsonEncode(messages);
  }

  void addNetworkRequest(String rawMessage) {
    final message = jsonDecode(rawMessage) as Map<String, Object?>;
    _networkRequests.insert(0, <String, Object?>{
      ...message,
      'createdAt': DateTime.now().toIso8601String(),
    });
    if (_networkRequests.length > 300) {
      _networkRequests.removeRange(300, _networkRequests.length);
    }
  }

  String networkRequests() {
    return jsonEncode(_networkRequests);
  }

  String networkRequest(int index) {
    return jsonEncode(_networkRequests[index]);
  }

  Future<void> selectOption(String selector, List<String> values) {
    return controller.runJavaScript('''
var el = ${_resolverScript(selector)};
if (el) {
  const values = ${jsonEncode(values)};
  Array.from(el.options || []).forEach(function(option) {
    option.selected = values.indexOf(option.value) >= 0 || values.indexOf(option.text) >= 0;
  });
  el.dispatchEvent(new Event('input', { bubbles: true }));
  el.dispatchEvent(new Event('change', { bubbles: true }));
}
''');
  }

  Future<void> hover(String selector) {
    return controller.runJavaScript('''
var el = ${_resolverScript(selector)};
if (el) {
  const rect = el.getBoundingClientRect();
  el.dispatchEvent(new MouseEvent('mouseover', {
    bubbles: true,
    clientX: rect.left + rect.width / 2,
    clientY: rect.top + rect.height / 2
  }));
  el.dispatchEvent(new MouseEvent('mousemove', {
    bubbles: true,
    clientX: rect.left + rect.width / 2,
    clientY: rect.top + rect.height / 2
  }));
}
''');
  }

  Future<void> drag(String startSelector, String endSelector) {
    return controller.runJavaScript('''
var start = ${_resolverScript(startSelector)};
var end = ${_resolverScript(endSelector)};
if (start && end) {
  const startRect = start.getBoundingClientRect();
  const endRect = end.getBoundingClientRect();
  const startX = startRect.left + startRect.width / 2;
  const startY = startRect.top + startRect.height / 2;
  const endX = endRect.left + endRect.width / 2;
  const endY = endRect.top + endRect.height / 2;
  start.dispatchEvent(new MouseEvent('mousedown', {
    bubbles: true,
    clientX: startX,
    clientY: startY
  }));
  document.dispatchEvent(new MouseEvent('mousemove', {
    bubbles: true,
    clientX: endX,
    clientY: endY
  }));
  end.dispatchEvent(new MouseEvent('mouseup', {
    bubbles: true,
    clientX: endX,
    clientY: endY
  }));
  end.dispatchEvent(new DragEvent('drop', { bubbles: true }));
}
''');
  }

  Future<void> fillForm(Map<String, String> fields) async {
    for (final entry in fields.entries) {
      await type(entry.key, entry.value);
    }
  }

  Future<Object> waitForText(String text) {
    return controller.runJavaScriptReturningResult('''
new Promise(function(resolve) {
  const target = ${jsonEncode(text)};
  const startedAt = Date.now();
  const timer = setInterval(function() {
    if (document.body && document.body.innerText.indexOf(target) >= 0) {
      clearInterval(timer);
      resolve(true);
    }
    if (Date.now() - startedAt > 10000) {
      clearInterval(timer);
      resolve(false);
    }
  }, 100);
})
''');
  }

  Future<Object> waitForTextGone(String text) {
    return controller.runJavaScriptReturningResult('''
new Promise(function(resolve) {
  const target = ${jsonEncode(text)};
  const startedAt = Date.now();
  const timer = setInterval(function() {
    if (!document.body || document.body.innerText.indexOf(target) < 0) {
      clearInterval(timer);
      resolve(true);
    }
    if (Date.now() - startedAt > 10000) {
      clearInterval(timer);
      resolve(false);
    }
  }, 100);
})
''');
  }

  String _resolverScript(String selectorOrRef) {
    final encoded = jsonEncode(selectorOrRef);
    return '''
(function() {
  const target = $encoded;
  const selector = 'a,button,input,textarea,select,[role]';
  const refParts = target.split(':');
  if (refParts[0] === 'el') {
    let doc = document;
    for (let index = 1; index < refParts.length - 1; index++) {
      const frameMatch = /^f(\\d+)\$/.exec(refParts[index]);
      if (!frameMatch) return null;
      const frame = Array.from(doc.querySelectorAll('iframe'))[Number(frameMatch[1])];
      if (!frame || !frame.contentDocument) return null;
      doc = frame.contentDocument;
    }
    return Array.from(doc.querySelectorAll(selector))[Number(refParts[refParts.length - 1])] || null;
  }
  return document.querySelector(target);
})()
''';
  }

  /// Returns the Windows controller required by compositor surface operations.
  WindowsWebViewController _requireWindowsController() {
    final platform = controller.platform;
    if (platform is! WindowsWebViewController) {
      throw StateError('Browser compositor surface requires Windows WebView');
    }
    return platform;
  }

  /// Decodes a browser surface display intent payload.
  Map<String, Object?> _decodeSurfaceIntent(String intentJson) {
    final payload = jsonDecode(intentJson);
    if (payload is! Map<String, Object?>) {
      throw StateError('Browser surface intent is not a JSON object');
    }
    return payload;
  }

  /// Returns the stable compositor stream identifier for this controller.
  String get _surfaceStreamId => 'browser-surface-${identityHashCode(this)}';

  /// Queues one owner WebView frame publication after prior captures complete.
  void requestSurfaceFrame() {
    if (!_encodedStreamAttached || _surfaceSize == null) {
      return;
    }
    if (_surfaceFrameCaptureActive) {
      _surfaceFrameCapturePending = true;
      return;
    }
    _surfaceFrameCaptureActive = true;
    unawaited(_drainSurfaceFrameCaptures());
  }

  /// Publishes at most one active frame and one coalesced pending refresh.
  Future<void> _drainSurfaceFrameCaptures() async {
    do {
      _surfaceFrameCapturePending = false;
      try {
        await onSurfaceFrameRequested();
      } catch (error, stackTrace) {
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: error,
            stack: stackTrace,
            library: 'runtime browser surface',
            context: ErrorDescription('capturing an encoded browser frame'),
          ),
        );
      }
    } while (_surfaceFrameCapturePending);
    _surfaceFrameCaptureActive = false;
  }

  /// Captures one frame from the real owner WebView compositor.
  Future<WindowsBrowserSurfaceFrame> captureSurfaceFrame() {
    if (!_encodedStreamAttached) {
      throw StateError('Encoded browser surface is not attached');
    }
    return _requireWindowsController().captureBrowserSurfaceFrame();
  }

  /// Reads a required numeric surface interaction field.
  double _readDouble(Map<String, Object?> payload, String key) {
    final value = payload[key];
    if (value is! num) {
      throw StateError(
        'Browser surface interaction field is not numeric: $key',
      );
    }
    return value.toDouble();
  }

  /// Reads a required integer surface interaction field.
  int _readInt(Map<String, Object?> payload, String key) {
    final value = payload[key];
    if (value is! num) {
      throw StateError(
        'Browser surface interaction field is not numeric: $key',
      );
    }
    return value.toInt();
  }

  /// Reads a required string surface interaction field.
  String _readString(Map<String, Object?> payload, String key) {
    final value = payload[key];
    if (value is! String) {
      throw StateError(
        'Browser surface interaction field is not a string: $key',
      );
    }
    return value;
  }

  /// Converts a serialized surface button index into the Windows enum.
  WindowsBrowserSurfacePointerButton _surfaceButton(Object? value) {
    if (value is! int ||
        value < 0 ||
        value >= WindowsBrowserSurfacePointerButton.values.length) {
      throw StateError('Browser surface pointer button is invalid');
    }
    return WindowsBrowserSurfacePointerButton.values[value];
  }

  /// Converts a raw Flutter pointer button mask into the Windows enum.
  WindowsBrowserSurfacePointerButton _surfaceButtonForRawButtons(int buttons) {
    switch (buttons) {
      case kPrimaryButton:
        return WindowsBrowserSurfacePointerButton.primary;
      case kSecondaryButton:
        return WindowsBrowserSurfacePointerButton.secondary;
      case kTertiaryButton:
        return WindowsBrowserSurfacePointerButton.tertiary;
      default:
        throw StateError('Invalid browser surface raw buttons: $buttons');
    }
  }
}
