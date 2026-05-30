// ignore_for_file: file_names

import 'dart:convert';

import 'package:webview_all/webview_all.dart';

import 'WorkspaceUserscriptModels.dart';
import 'WorkspaceUserscriptStore.dart';

class WorkspaceUserscriptRuntime {
  WorkspaceUserscriptRuntime({required this.store});

  final WorkspaceUserscriptStore store;

  Future<void> injectForUrl(
    WebViewController controller,
    String url, {
    required WorkspaceUserscriptRunAt runAt,
  }) async {
    final scripts = store
        .scriptsForUrl(url)
        .where((item) => item.metadata.runAt == runAt)
        .toList(growable: false);
    for (final item in scripts) {
      final wrapped = _wrap(item, url);
      await controller.runJavaScript(wrapped);
      store.addLog(item.metadata.name, '已注入 $url');
    }
  }

  Future<List<WorkspaceUserscriptMenuCommand>> menuCommands(
    WebViewController controller,
  ) async {
    final result = await controller.runJavaScriptReturningResult(r'''
JSON.stringify((window.__operitUserscriptMenuCommands || []).map(function(item, index) {
  return {
    index: index,
    scriptName: String(item.scriptName || ''),
    caption: String(item.caption || '')
  };
}))
''');
    final decoded = jsonDecode(result as String) as List<Object?>;
    return decoded
        .map(
          (item) => WorkspaceUserscriptMenuCommand.fromJson(
            item as Map<String, Object?>,
          ),
        )
        .toList(growable: false);
  }

  Future<void> runMenuCommand(WebViewController controller, int index) async {
    await controller.runJavaScript('''
(function() {
  const commands = window.__operitUserscriptMenuCommands || [];
  const command = commands[$index];
  if (command && typeof command.command === 'function') {
    command.command();
  }
})();
''');
  }

  String _wrap(WorkspaceUserscriptItem item, String pageUrl) {
    final name = jsonEncode(item.metadata.name);
    final id = jsonEncode(item.id);
    final encodedPageUrl = jsonEncode(pageUrl);
    final encodedConnects = jsonEncode(item.metadata.connects);
    final encodedValues = jsonEncode(item.values);
    final encodedResourceTexts = jsonEncode(item.resourceTexts);
    final encodedResourceUrls = jsonEncode(item.resourceUrls);
    final encodedMetadata = jsonEncode(item.metadata.toJson());
    final requireSources = item.metadata.requires
        .map((url) => item.requireSources[url])
        .whereType<String>()
        .join('\n;\n');
    final source = item.source;
    return '''
(function() {
  const scriptName = $name;
  const scriptId = $id;
  const pageUrl = $encodedPageUrl;
  const connectRules = $encodedConnects;
  const valuesRoot = window.__operitUserscriptValues || (window.__operitUserscriptValues = {});
  const values = valuesRoot[scriptId] || (valuesRoot[scriptId] = $encodedValues);
  const resourceTexts = $encodedResourceTexts;
  const resourceUrls = $encodedResourceUrls;
  const scriptMetadata = $encodedMetadata;
  const menuCommands = window.__operitUserscriptMenuCommands || (window.__operitUserscriptMenuCommands = []);
  const tabStoreRoot = window.__operitUserscriptTabs || (window.__operitUserscriptTabs = {});
  const tabStore = tabStoreRoot[scriptId] || (tabStoreRoot[scriptId] = {});
  function persistValue(action, key, value) {
    if (!window.OperitUserscriptStorage || !window.OperitUserscriptStorage.postMessage) return;
    window.OperitUserscriptStorage.postMessage(JSON.stringify({
      action: action,
      id: scriptId,
      key: String(key),
      value: value
    }));
  }
  function reportRuntime(status, message) {
    if (!window.OperitUserscriptRuntime || !window.OperitUserscriptRuntime.postMessage) return;
    window.OperitUserscriptRuntime.postMessage(JSON.stringify({
      id: scriptId,
      name: scriptName,
      url: pageUrl,
      status: status,
      message: String(message)
    }));
  }
  function originOf(rawUrl) {
    try {
      const url = new URL(rawUrl, pageUrl);
      const port = url.port && !((url.protocol === 'http:' && url.port === '80') || (url.protocol === 'https:' && url.port === '443')) ? ':' + url.port : '';
      return url.protocol + '//' + url.hostname.toLowerCase() + port;
    } catch (error) {
      return null;
    }
  }
  function connectAllowed(targetRawUrl) {
    const pageOrigin = originOf(pageUrl);
    const targetOrigin = originOf(targetRawUrl);
    if (pageOrigin && targetOrigin && pageOrigin === targetOrigin) {
      return true;
    }
    let targetHost = '';
    try {
      targetHost = new URL(targetRawUrl, pageUrl).hostname.toLowerCase();
    } catch (error) {
      return false;
    }
    return connectRules.some(function(rule) {
      const normalized = String(rule).trim().toLowerCase();
      if (normalized === '*') return true;
      if (normalized === 'self') return pageOrigin && pageOrigin === targetOrigin;
      if (normalized === targetHost) return true;
      if (normalized.startsWith('*.')) {
        const suffix = normalized.slice(2);
        return targetHost === suffix || targetHost.endsWith('.' + suffix);
      }
      return false;
    });
  }
  window.GM_getValue = function(key, defaultValue) {
    return Object.prototype.hasOwnProperty.call(values, key) ? values[key] : defaultValue;
  };
  window.GM_setValue = function(key, value) {
    values[key] = value;
    persistValue('set', key, value);
  };
  window.GM_deleteValue = function(key) {
    delete values[key];
    persistValue('delete', key, null);
  };
  window.GM_listValues = function() {
    return Object.keys(values);
  };
  window.GM_getValues = function(keys) {
    const result = {};
    (keys || Object.keys(values)).forEach(function(key) {
      result[key] = values[key];
    });
    return result;
  };
  window.GM_setValues = function(nextValues) {
    Object.keys(nextValues || {}).forEach(function(key) {
      window.GM_setValue(key, nextValues[key]);
    });
  };
  window.GM_deleteValues = function(keys) {
    (keys || []).forEach(function(key) {
      window.GM_deleteValue(key);
    });
  };
  window.GM_getResourceText = function(name) {
    return resourceTexts[String(name)] || '';
  };
  window.GM_getResourceURL = function(name) {
    return resourceUrls[String(name)] || '';
  };
  window.GM_info = {
    script: scriptMetadata,
    scriptHandler: 'Operit',
    version: 'workspace-browser'
  };
  window.GM_log = function() {
    console.log.apply(console, arguments);
  };
  window.GM_addStyle = function(css) {
    const style = document.createElement('style');
    style.textContent = String(css);
    (document.head || document.documentElement).appendChild(style);
    return style;
  };
  window.GM_addElement = function(parentOrTag, tagOrAttrs, attrs) {
    let parent = document.body || document.documentElement;
    let tagName = parentOrTag;
    let attributes = tagOrAttrs || {};
    if (parentOrTag && parentOrTag.nodeType === 1) {
      parent = parentOrTag;
      tagName = tagOrAttrs;
      attributes = attrs || {};
    }
    const el = document.createElement(String(tagName));
    Object.keys(attributes || {}).forEach(function(key) {
      if (key === 'textContent') {
        el.textContent = attributes[key];
      } else {
        el.setAttribute(key, attributes[key]);
      }
    });
    parent.appendChild(el);
    return el;
  };
  window.GM_getTab = function(callback) {
    if (typeof callback === 'function') callback(tabStore);
    return tabStore;
  };
  window.GM_saveTab = function(tab) {
    Object.keys(tabStore).forEach(function(key) { delete tabStore[key]; });
    Object.assign(tabStore, tab || {});
  };
  window.GM_getTabs = function(callback) {
    if (typeof callback === 'function') callback(tabStoreRoot);
    return tabStoreRoot;
  };
  window.GM_setClipboard = function(text) {
    if (navigator.clipboard && navigator.clipboard.writeText) {
      navigator.clipboard.writeText(String(text));
    }
  };
  window.GM_openInTab = function(url) {
    if (window.OperitBrowserPopup && window.OperitBrowserPopup.postMessage) {
      window.OperitBrowserPopup.postMessage(JSON.stringify({
        action: 'open',
        url: String(url)
      }));
    }
  };
  window.GM_notification = function(details) {
    const text = typeof details === 'string' ? details : (details && (details.text || details.title)) || '';
    if (text) console.log('[Operit userscript notification] ' + text);
  };
  window.GM_registerMenuCommand = function(caption, command) {
    menuCommands.push({ scriptName: scriptName, caption: String(caption), command: command });
    return menuCommands.length - 1;
  };
  window.GM_xmlhttpRequest = function(details) {
    if (!details || !details.url || !connectAllowed(details.url)) {
      if (details && details.onerror) {
        details.onerror({ error: 'Blocked by @connect' });
      }
      return { abort: function() {} };
    }
    const controller = new AbortController();
    fetch(details.url, {
      method: details.method || 'GET',
      headers: details.headers || {},
      body: details.data,
      signal: controller.signal
    }).then(function(response) {
      return response.text().then(function(text) {
        if (details.onload) {
          details.onload({
            status: response.status,
            statusText: response.statusText,
            responseText: text,
            finalUrl: response.url
          });
        }
      });
    }).catch(function(error) {
      if (details.onerror) details.onerror({ error: String(error) });
    });
    return { abort: function() { controller.abort(); } };
  };
  try {
$requireSources
$source
    reportRuntime('ran', '已运行 ' + pageUrl);
  } catch (error) {
    reportRuntime('error', String(error));
    console.error('[Operit userscript] ' + scriptName + ': ' + error);
  }
})();
''';
  }
}
