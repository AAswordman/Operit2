/// Owns keyed hooks and coalesces state notifications.
var OperitComposeState = function () {
  function create(runtime) {
    function notifyStateChanged() {
      runtime.stateDirty = true;
      if (runtime.stateChangeScheduled) {
        return;
      }
      runtime.stateChangeScheduled = true;
      runtime.pendingStateChangePromise = Promise.resolve().then(function () {
        try {
          runtime.stateChangeScheduled = false;
          if (!runtime.stateDirty) {
            return;
          }
          runtime.stateDirty = false;
          flushStateChangeListeners();
        } finally {
          runtime.pendingStateChangePromise = null;
        }
      });
    }
    function flushStateChangeListeners() {
      if (!runtime.stateChangeListeners || runtime.stateChangeListeners.length <= 0) {
        return;
      }
      var listeners = runtime.stateChangeListeners.slice();
      for (var i = 0; i < listeners.length; i += 1) {
        listeners[i]();
      }
    }
    function subscribeStateChange(listener) {
      if (typeof listener !== 'function') {
        throw new TypeError('State listener must be a function');
      }
      runtime.stateChangeListeners.push(listener);
      var active = true;
      return function () {
        if (!active) {
          return;
        }
        active = false;
        var index = runtime.stateChangeListeners.indexOf(listener);
        if (index >= 0) {
          runtime.stateChangeListeners.splice(index, 1);
        }
      };
    }
    function flushPendingStateChanges() {
      if (runtime.pendingStateChangePromise && typeof runtime.pendingStateChangePromise.then === 'function') {
        return runtime.pendingStateChangePromise;
      }
      return Promise.resolve();
    }
    return {
      api: {
        /// Registers cancellable work with composition-owned dependency and disposal tracking.
        useEffect: function (key, setup, dependencies) {
          OperitComposeReactive.effect(runtime.ctx, key, setup, dependencies);
        },
        useState: function (key, initialValue) {
          var stateKey = String(key || '').trim();
          if (!stateKey) {
            throw new Error('useState key is required');
          }
          if (!Object.prototype.hasOwnProperty.call(runtime.stateStore, stateKey)) {
            runtime.stateStore[stateKey] = initialValue;
          }
          return [runtime.stateStore[stateKey], function (nextValue) {
            runtime.stateStore[stateKey] = nextValue;
            OperitComposeReactive.stateChanged(runtime.ctx, stateKey);
            notifyStateChanged();
          }];
        },
        useMutable: function (key, initialValue) {
          var stateKey = String(key || '').trim();
          if (!stateKey) {
            throw new Error('useMutable key is required');
          }
          if (!Object.prototype.hasOwnProperty.call(runtime.memoStore, stateKey)) {
            runtime.memoStore[stateKey] = initialValue;
          }
          return [runtime.memoStore[stateKey], function (nextValue) {
            runtime.memoStore[stateKey] = nextValue;
          }];
        },
        useRef: function (key, initialValue) {
          var stateKey = String(key || '').trim();
          if (!stateKey) {
            throw new Error('useRef key is required');
          }
          if (!Object.prototype.hasOwnProperty.call(runtime.memoStore, stateKey)) {
            runtime.memoStore[stateKey] = {
              current: initialValue
            };
          }
          return runtime.memoStore[stateKey];
        },
        useMemo: function (key, factory, deps) {
          var memoKey = 'memo:' + String(key || '');
          var current = runtime.memoStore[memoKey];
          var values = Array.isArray(deps) ? deps : [];
          var changed = !current;
          var previous = current && current.__operitDeps;
          if (!changed && Array.isArray(previous) && previous.length === values.length) {
            changed = false;
            for (var index = 0; index < values.length; index += 1) {
              if (!Object.is(previous[index], values[index])) {
                changed = true;
                break;
              }
            }
            // Same-session renders keep the exact dependency references. Avoid
            // serializing large catalogs and node windows when those references
            // are unchanged; cross-session stores still use the old structural
            // representation below.
            if (!changed) {
              var generated = OperitComposeReactive.memoValue(runtime.ctx, memoKey,
                current.generated, false, factory);
              if (current.generated !== generated) {
                var refreshed = { depsJson: current.depsJson, value: generated.value, generated: generated };
                Object.defineProperty(refreshed, '__operitDeps', {
                  value: previous.slice(),
                  writable: true,
                  configurable: true,
                  enumerable: false
                });
                runtime.memoStore[memoKey] = refreshed;
                current = refreshed;
              }
              return current.value;
            }
          }
          var depsJson = JSON.stringify(values);
          changed = !current || current.depsJson !== depsJson;
          var generated = OperitComposeReactive.memoValue(runtime.ctx, memoKey,
            current && current.generated, changed, factory);
          if (!current || current.generated !== generated || current.depsJson !== depsJson) {
            current = { depsJson: depsJson, value: generated.value, generated: generated };
            runtime.memoStore[memoKey] = current;
          }
          if (current && current.__operitDeps !== values) {
            // Keep the identity cache private to the live JS session. It is
            // deliberately non-enumerable so persisted memo output remains a
            // plain JSON value and can still cross a fresh runtime boundary.
            Object.defineProperty(current, '__operitDeps', {
              value: values.slice(),
              writable: true,
              configurable: true,
              enumerable: false
            });
          }
          return current.value;
        }
      },
      subscribe: subscribeStateChange,
      flush: flushPendingStateChanges
    };
  }
  return {
    create: create
  };
}();
