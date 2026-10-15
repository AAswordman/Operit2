/// Adapts required native capabilities to the public DSL context.
var OperitComposeHost = function () {
  function cloneObject(input) {
    if (!input || typeof input !== 'object' || Array.isArray(input)) {
      return {};
    }
    var out = {};
    for (var key in input) {
      if (Object.prototype.hasOwnProperty.call(input, key)) {
        out[key] = input[key];
      }
    }
    return out;
  }
  function createUserFacingError(message, detailData) {
    return {
      name: 'Error',
      message: String(message || '').trim(),
      data: detailData,
      toString: function () {
        return this.message;
      }
    };
  }
  function unwrapHostResult(result) {
    if (!result || typeof result !== 'object' || typeof result.success !== 'boolean') {
      throw new TypeError('Compose host returned an invalid result envelope');
    }
    if (!result.success) throw createUserFacingError(result.message, result);
    return result.data;
  }
  function resolvePackageName(value, runtime) {
    var name = String(value || runtime.packageName || '').trim();
    return name;
  }
  function create(runtime) {
    return {
      measureText: function (request) {
        var text = request && request.text != null ? String(request.text) : '';
        var fontSize = request && request.fontSize ? Number(request.fontSize) : 14;
        return {
          width: Math.min(request && request.maxWidth || 100000, text.length * fontSize * 0.56),
          height: Math.min(request && request.maxHeight || 100000, fontSize * 1.4)
        };
      },
      getEnv: function (key) {
        return runtime.callRuntime.getEnv(String(key));
      },
      setEnv: async function (key, value) {
        __operitNativeSetEnv(String(globalThis.__operitCurrentCallId), String(key), value == null ? '' : String(value));
      },
      setEnvs: async function (values) {
        __operitNativeSetEnvs(String(globalThis.__operitCurrentCallId), values);
      },
      callTool: function (toolName, params) {
        if (typeof toolCall === 'function') {
          return toolCall(String(toolName || ''), params || {});
        }
        throw createUserFacingError('Tool call bridge is unavailable');
      },
      navigate: function (route, args) {
        var routeId = String(route || '').trim();
        if (!routeId) {
          throw createUserFacingError('route is required');
        }
        var payload = args && typeof args === 'object' ? args : {};
        runtime.navigationCommands.push({
          route: routeId,
          args: OperitComposeNodes.serialize(payload, runtime, [])
        });
        return Promise.resolve();
      },
      showToast: function (message) {
        return toolCall('toast', {
          message: String(message || '')
        });
      },
      reportError: function (error) {
        console.error(error);
      },
      openFilePicker: function (options) {
        return __operitInvokeHostAsync(__operitNativeComposeFilePickerCommand, [{
          routeInstanceId: runtime.routeInstanceId || '',
          executionContextKey: runtime.executionContextKey || '',
          options: OperitComposeNodes.serialize(options === undefined ? {} : options, runtime, [])
        }]);
      },
      /** Saves UTF-8 content through the owning host's file selector. */
      saveFile: function (options) {
        return __operitInvokeHostAsync(__operitNativeComposeFileSaveCommand, [{
          routeInstanceId: runtime.routeInstanceId || '',
          executionContextKey: runtime.executionContextKey || '',
          options: OperitComposeNodes.serialize(options, runtime, [])
        }]);
      },
      getModuleSpec: function () {
        return runtime.moduleSpec;
      },
      getCurrentPackageName: function () {
        return runtime.packageName;
      },
      getCurrentToolPkgId: function () {
        return runtime.toolPkgId || runtime.packageName;
      },
      getCurrentUiModuleId: function () {
        return runtime.uiModuleId;
      },
      isPackageImported: function (packageName) {
        return __operitInvokeHostAsync(__operitNativeIsPackageImported, [resolvePackageName(packageName, runtime)]);
      },
      importPackage: function (packageName) {
        return __operitInvokeHostAsync(__operitNativeImportPackage, [resolvePackageName(packageName, runtime)]);
      },
      removePackage: function (packageName) {
        return __operitInvokeHostAsync(__operitNativeRemovePackage, [resolvePackageName(packageName, runtime)]);
      },
      usePackage: function (packageName) {
        return __operitInvokeHostAsync(__operitNativeUsePackage, [resolvePackageName(packageName, runtime)]);
      },
      listImportedPackages: function () {
        return __operitInvokeHostAsync(__operitNativeListImportedPackages, []);
      },
      resolveToolName: function (request) {
        var req = request === undefined ? {} : request;
        return __operitInvokeHostAsync(__operitNativeResolveToolName, [String(req.packageName || runtime.packageName || ''), String(req.subpackageId || ''), String(req.toolName || ''), req.preferImported !== false]);
      },
      formatTemplate: function (template, values) {
        var result = String(template || '');
        var source = values && typeof values === 'object' ? values : {};
        for (var key in source) {
          if (Object.prototype.hasOwnProperty.call(source, key)) {
            result = result.split('{' + key + '}').join(source[key] == null ? '' : String(source[key]));
          }
        }
        return result;
      }
    };
  }
  return {
    create: create,
    copy: cloneObject,
    error: createUserFacingError,
    unwrap: unwrapHostResult
  };
}();
