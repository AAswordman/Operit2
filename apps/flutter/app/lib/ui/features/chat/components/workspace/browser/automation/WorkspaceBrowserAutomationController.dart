// ignore_for_file: file_names

import 'dart:convert';

import 'package:webview_all/webview_all.dart';

class WorkspaceBrowserAutomationController {
  WorkspaceBrowserAutomationController({required this.controller});

  final WebViewController controller;
  final List<Map<String, Object?>> _consoleMessages = <Map<String, Object?>>[];
  final List<Map<String, Object?>> _networkRequests = <Map<String, Object?>>[];

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
}
