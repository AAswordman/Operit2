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
          var depsJson = JSON.stringify(deps || []);
          if (!current || current.depsJson !== depsJson) {
            current = {
              depsJson: depsJson,
              value: factory()
            };
            runtime.memoStore[memoKey] = current;
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
