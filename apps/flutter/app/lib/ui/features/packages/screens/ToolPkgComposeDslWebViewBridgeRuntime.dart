// ignore_for_file: file_names

import 'dart:convert';

const String composeDslWebViewInternalBridgeName =
    '__ComposeDslWebViewHostBridge__';
const String composeDslWebViewBridgeChannelName =
    '__ComposeDslWebViewHostBridgeChannel__';
const String _composeDslWebViewBridgeHtmlMarker =
    'data-operit-webview-bridge-runtime="1"';

/// Embeds the bridge for explicit HTML loads using the Kotlin host markup contract.
String injectComposeDslWebViewBridgeRuntimeIntoHtml(
  String html, {
  required Map<String, List<String>> javascriptInterfaces,
}) {
  if (html.contains(_composeDslWebViewBridgeHtmlMarker)) {
    return html;
  }
  final scriptTag = buildComposeDslWebViewBridgeRuntimeScriptTag(
    javascriptInterfaces: javascriptInterfaces,
  );
  final headClose = RegExp('</head>', caseSensitive: false);
  if (headClose.hasMatch(html)) {
    return html.replaceFirst(headClose, '$scriptTag</head>');
  }
  final headOpen = RegExp(r'<head[^>]*>', caseSensitive: false);
  final headOpenMatch = headOpen.firstMatch(html);
  if (headOpenMatch != null) {
    final headTag = headOpenMatch.group(0)!;
    return html.replaceFirst(headOpen, '$headTag$scriptTag');
  }
  final htmlOpen = RegExp(r'<html[^>]*>', caseSensitive: false);
  final htmlOpenMatch = htmlOpen.firstMatch(html);
  if (htmlOpenMatch != null) {
    final htmlTag = htmlOpenMatch.group(0)!;
    return html.replaceFirst(htmlOpen, '$htmlTag<head>$scriptTag</head>');
  }
  return '$scriptTag$html';
}

/// Escapes the generated runtime for use inside an HTML script element.
String buildComposeDslWebViewBridgeRuntimeScriptTag({
  required Map<String, List<String>> javascriptInterfaces,
}) {
  final scriptBody = buildComposeDslWebViewBridgeRuntimeScript(
    javascriptInterfaces: javascriptInterfaces,
  ).replaceAll('</script>', '<\\/script>');
  return '<script $_composeDslWebViewBridgeHtmlMarker>$scriptBody</script>';
}

/// Creates interfaces synchronously while keeping host invocations asynchronous.
String buildComposeDslWebViewBridgeRuntimeScript({
  required Map<String, List<String>> javascriptInterfaces,
}) {
  final interfacesJson = jsonEncode(
    javascriptInterfaces,
  ).replaceAll('<', r'\u003c');
  final hiddenBridgeNameJson = jsonEncode(composeDslWebViewInternalBridgeName);
  final channelNameJson = jsonEncode(composeDslWebViewBridgeChannelName);
  return '''
    (function() {
      var hiddenBridgeName = $hiddenBridgeNameJson;
      var channelName = $channelNameJson;
      var initialInterfaces = $interfacesJson;
      if (typeof window.__operitInstallComposeDslJavascriptInterfaces === 'function') {
        window.__operitInstallComposeDslJavascriptInterfaces(initialInterfaces);
        return;
      }
      var sequence = 0;
      var pending = {};
      var installed = {};
      var queued = [];
      // Drain startup calls when invoked by a call or the host's interface refresh.
      function flushQueuedMessages() {
        var channel = window[channelName];
        if (!channel || typeof channel.postMessage !== 'function') {
          return;
        }
        while (queued.length > 0) {
          var message = queued.shift();
          try {
            channel.postMessage(message.raw);
          } catch (error) {
            var failed = pending[message.id];
            delete pending[message.id];
            failed.reject(error);
          }
        }
      }
      // Correlate asynchronous host messages with their page-side promises.
      function send(type, payload) {
        sequence += 1;
        var id = String(Date.now()) + ':' + String(sequence);
        return new Promise(function(resolve, reject) {
          var raw = JSON.stringify({
            id: id,
            type: type,
            payload: payload === undefined ? null : payload
          });
          pending[id] = { resolve: resolve, reject: reject };
          queued.push({ id: id, raw: raw });
          flushQueuedMessages();
        });
      }
      // Resolve only the request identified by the native response envelope.
      window.__operitComposeDslWebViewHostReceive = function(message) {
        var envelope = typeof message === 'string' ? JSON.parse(message) : message;
        if (!envelope || !envelope.id || !pending[envelope.id]) {
          return;
        }
        var callbacks = pending[envelope.id];
        delete pending[envelope.id];
        if (envelope.success === false) {
          callbacks.reject(new Error(String(envelope.message || '')));
        } else {
          callbacks.resolve(envelope.data);
        }
      };
      // Expose host methods without allowing page-side reassignment.
      function defineReadonly(target, key, value) {
        Object.defineProperty(target, key, {
          configurable: true,
          enumerable: true,
          writable: false,
          value: value
        });
      }
      var hiddenBridge = {
        handleControllerCommand: function(payload) {
          return send('controllerCommand', payload);
        },
        listInterfaces: function() {
          return send('listInterfaces', {});
        },
        invoke: function(interfaceName, methodName, argsJson) {
          return send('invoke', {
            interfaceName: interfaceName,
            methodName: methodName,
            args: argsJson
          });
        },
        dispatchAction: function(actionId, payload) {
          return send('dispatchAction', {
            actionId: actionId,
            payload: payload === undefined ? null : payload
          });
        },
        pickFiles: function(options) {
          return send('pickFiles', options || {});
        }
      };
      defineReadonly(window, hiddenBridgeName, hiddenBridge);
      // Install the host snapshot before the page can attempt its first data read.
      function installInterfaces(descriptors) {
        for (var previousInterfaceName in installed) {
          if (
            Object.prototype.hasOwnProperty.call(installed, previousInterfaceName) &&
            !Object.prototype.hasOwnProperty.call(descriptors, previousInterfaceName)
          ) {
            try {
              delete window[previousInterfaceName];
            } catch (_deleteError) {
            }
          }
        }
        installed = {};
        window.__operitComposeDslInstalledJavascriptInterfaces = installed;
        for (var interfaceName in descriptors) {
          if (!Object.prototype.hasOwnProperty.call(descriptors, interfaceName)) {
            continue;
          }
          var methodNames = descriptors[interfaceName];
          var hostObject = {};
          window[interfaceName] = hostObject;
          for (var i = 0; i < methodNames.length; i += 1) {
            (function(targetObject, resolvedInterfaceName, resolvedMethodName) {
              defineReadonly(targetObject, resolvedMethodName, function() {
                var args = [];
                for (var argIndex = 0; argIndex < arguments.length; argIndex += 1) {
                  args.push(arguments[argIndex]);
                }
                return hiddenBridge.invoke(
                  resolvedInterfaceName,
                  resolvedMethodName,
                  JSON.stringify(args)
                );
              });
            })(hostObject, interfaceName, methodNames[i]);
          }
          window.__operitComposeDslInstalledJavascriptInterfaces[interfaceName] = true;
        }
        // Host lifecycle refreshes resume queued calls without timers or deadlines.
        flushQueuedMessages();
        window.dispatchEvent(new Event('operitComposeDslInterfacesReady'));
      }
      window.__operitInstallComposeDslJavascriptInterfaces = installInterfaces;
      installInterfaces(initialInterfaces);
    })();
  ''';
}
