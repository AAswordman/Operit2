/// Owns controller commands and accepted interface registrations.
var OperitComposeWebView = function () {
  function create(runtime, key) {
    var controllerKey = String(key || '').trim();
    if (!controllerKey) {
      throw new Error('webview controller key is required');
    }
    var descriptor = {
      __composeWebViewController: true,
      key: controllerKey,
      routeInstanceId: runtime.routeInstanceId || '',
      executionContextKey: runtime.executionContextKey || ''
    };
    /** Builds one structured controller request with its owning Compose context. */
    function controllerRequest(command, payload) {
      return {
        command: String(command),
        key: controllerKey,
        routeInstanceId: runtime.routeInstanceId || '',
        executionContextKey: runtime.executionContextKey || '',
        payload: OperitComposeNodes.serialize(payload, runtime, [])
      };
    }
    /** Executes a synchronous controller command through the structured host binding. */
    function invokeControllerCommand(command, payload) {
      return OperitComposeHost.unwrap(__operitNativeComposeWebViewControllerCommand(controllerRequest(command, payload)));
    }
    /** Executes an asynchronous controller command through its owning host Promise. */
    function invokeControllerCommandSuspend(command, payload) {
      return __operitInvokeHostAsync(__operitNativeComposeWebViewControllerCommandSuspend, [controllerRequest(command, payload)], OperitComposeHost.unwrap);
    }
    function defineMethod(target, name, handler) {
      Object.defineProperty(target, name, {
        configurable: false,
        enumerable: false,
        writable: false,
        value: handler
      });
    }
    var controller = OperitComposeHost.copy(descriptor);
    defineMethod(controller, 'toJSON', function () {
      return OperitComposeHost.copy(descriptor);
    });
    defineMethod(controller, 'loadUrl', function (url, headers) {
      var finalUrl = String(url || '').trim();
      if (!finalUrl) {
        throw new Error('webview controller loadUrl requires a non-empty url');
      }
      invokeControllerCommand('loadUrl', {
        url: finalUrl,
        headers: headers && typeof headers === 'object' ? headers : {}
      });
    });
    defineMethod(controller, 'loadHtml', function (html, options) {
      invokeControllerCommand('loadHtml', {
        html: html == null ? '' : String(html),
        options: options && typeof options === 'object' ? options : {}
      });
    });
    defineMethod(controller, 'reload', function () {
      invokeControllerCommand('reload', {});
    });
    defineMethod(controller, 'stopLoading', function () {
      invokeControllerCommand('stopLoading', {});
    });
    defineMethod(controller, 'goBack', function () {
      invokeControllerCommand('goBack', {});
    });
    defineMethod(controller, 'goForward', function () {
      invokeControllerCommand('goForward', {});
    });
    defineMethod(controller, 'clearHistory', function () {
      invokeControllerCommand('clearHistory', {});
    });
    defineMethod(controller, 'evaluateJavascript', function (script) {
      return Promise.resolve(invokeControllerCommandSuspend('evaluateJavascript', {
        script: script == null ? '' : String(script)
      }));
    });
    defineMethod(controller, 'getState', function () {
      return invokeControllerCommand('getState', {});
    });
    defineMethod(controller, 'addJavascriptInterface', function (name, object) {
      var interfaceName = String(name || '').trim();
      if (!interfaceName) {
        throw new Error('webview controller addJavascriptInterface requires a non-empty name');
      }
      if (!object || typeof object !== 'object' || Array.isArray(object)) {
        throw new Error('webview controller addJavascriptInterface requires an object');
      }
      var interfaces = runtime.webViewInterfaces.get(controllerKey);
      if (!interfaces) {
        interfaces = new Map();
        runtime.webViewInterfaces.set(controllerKey, interfaces);
      }
      var previous = interfaces.get(interfaceName);
      var methods = new Map();
      var references = Object.create(null);
      Object.keys(object).forEach(/** Gives interface methods controller ownership, not transient UI-node ownership. */
      function (method) {
        var handler = object[method];
        if (typeof handler !== 'function') return;
        var id = previous && previous.get(method);
        if (!id) {
          id = runtime.actions.allocate();
        }
        methods.set(method, {
          id: id,
          handler: handler
        });
        references[method] = {
          __actionId: id
        };
      });
      invokeControllerCommand('addJavascriptInterface', {
        name: interfaceName,
        object: references
      });
      var owned = new Map();
      methods.forEach(/** Publishes callbacks after the Host has accepted the interface descriptor. */
      function (method, name) {
        runtime.actions.set(method.id, method.handler);
        owned.set(name, method.id);
      });
      if (previous) previous.forEach(/** Releases only methods removed by this explicit interface replacement. */
      function (id, name) {
        if (!owned.has(name)) runtime.actions.release(id);
      });
      interfaces.set(interfaceName, owned);
    });
    defineMethod(controller, 'removeJavascriptInterface', function (name) {
      var interfaceName = String(name || '').trim();
      if (!interfaceName) {
        throw new Error('webview controller removeJavascriptInterface requires a non-empty name');
      }
      invokeControllerCommand('removeJavascriptInterface', {
        name: interfaceName
      });
      var interfaces = runtime.webViewInterfaces.get(controllerKey);
      var methods = interfaces && interfaces.get(interfaceName);
      if (methods) {
        methods.forEach(/** Releases interface-owned actions when the plugin explicitly unregisters it. */
        function (id) {
          runtime.actions.release(id);
        });
        interfaces.delete(interfaceName);
        if (interfaces.size === 0) runtime.webViewInterfaces.delete(controllerKey);
      }
    });
    return controller;
  }
  return {
    create: create
  };
}();
