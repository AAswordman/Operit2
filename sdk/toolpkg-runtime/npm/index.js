'use strict';

const { spawn } = require('node:child_process');
const { EventEmitter } = require('node:events');
const readline = require('node:readline');

function requireString(value, name) {
  if (typeof value !== 'string') {
    throw new TypeError(`${name} must be a string`);
  }
  return value;
}

function stringToolResult(toolName, value) {
  return {
    toolName: requireString(toolName, 'toolName'),
    success: true,
    result: {
      __type: 'StringResultData',
      value: requireString(value, 'value')
    },
    error: null
  };
}

function errorToolResult(toolName, message) {
  return {
    toolName: requireString(toolName, 'toolName'),
    success: false,
    result: {
      __type: 'StringResultData',
      value: ''
    },
    error: requireString(message, 'message')
  };
}

class ToolPkgRuntimeClient extends EventEmitter {
  constructor(options) {
    super();
    if (!options || typeof options.runtimeBin !== 'string' || !options.runtimeBin.trim()) {
      throw new Error('runtimeBin is required');
    }
    if (typeof options.languageCode !== 'string' || !options.languageCode.trim()) {
      throw new Error('languageCode is required');
    }

    this._nextId = 1;
    this._pending = new Map();
    this._queue = [];
    this._activeRequestId = null;
    this._host = options.host;
    this._closed = false;
    this._child = spawn(
      options.runtimeBin,
      ['serve', '--language', options.languageCode.trim()],
      {
        cwd: options.cwd,
        env: options.env || process.env,
        stdio: ['pipe', 'pipe', 'pipe']
      }
    );

    this._stdout = readline.createInterface({
      input: this._child.stdout,
      crlfDelay: Infinity
    });
    this._stdout.on('line', line => this._handleLine(line));
    this._child.stderr.on('data', chunk => {
      this.emit('stderr', chunk.toString('utf8'));
    });
    this._child.on('error', error => this._rejectAll(error));
    this._child.on('exit', (code, signal) => {
      this._closed = true;
      this._rejectAll(
        new Error(`toolpkg runtime exited: code=${code == null ? '' : code} signal=${signal || ''}`)
      );
    });
  }

  loadToolPkgFile(path) {
    return this._request('loadToolPkgFile', { path });
  }

  readToolPkgTextResource(containerPackageName, resourcePath) {
    return this._request('readToolPkgTextResource', {
      containerPackageName,
      resourcePath
    });
  }

  runFunction(call) {
    return this._request('runFunction', call);
  }

  runMainHook(call) {
    return this._request('runMainHook', call);
  }

  dispatchIpc(call) {
    return this._request('dispatchIpc', call);
  }

  destroyContext(contextKey) {
    return this._request('destroyContext', { contextKey });
  }

  destroy() {
    return this._request('destroy', {});
  }

  close() {
    this._closed = true;
    this._stdout.close();
    this._queue = [];
    this._activeRequestId = null;
    this._child.stdin.end();
    this._child.kill();
  }

  _request(method, params) {
    if (this._closed) {
      return Promise.reject(new Error('toolpkg runtime is closed'));
    }
    const id = String(this._nextId++);
    const payload = JSON.stringify({ id, method, params });
    return new Promise((resolve, reject) => {
      this._pending.set(id, { resolve, reject });
      this._queue.push({ id, payload });
      this._pump();
    });
  }

  _pump() {
    if (this._closed || this._activeRequestId || this._queue.length === 0) {
      return;
    }
    const item = this._queue.shift();
    this._activeRequestId = item.id;
    this._child.stdin.write(`${item.payload}\n`, error => {
      if (!error) {
        return;
      }
      this._activeRequestId = null;
      const pending = this._pending.get(item.id);
      if (pending) {
        this._pending.delete(item.id);
        pending.reject(error);
      }
      this._pump();
    });
  }

  _handleLine(line) {
    const response = JSON.parse(line);
    if (response && response.type === 'hostToolCall') {
      this._handleHostToolCall(response);
      return;
    }
    const pending = this._pending.get(String(response.id));
    if (!pending) {
      this.emit('unmatchedResponse', response);
      return;
    }
    this._pending.delete(String(response.id));
    if (this._activeRequestId === String(response.id)) {
      this._activeRequestId = null;
    }
    if (response.success) {
      pending.resolve(response.result);
    } else {
      const error = new Error(response.error || 'toolpkg runtime request failed');
      error.response = response;
      pending.reject(error);
    }
    this._pump();
  }

  _handleHostToolCall(message) {
    Promise.resolve()
      .then(() => {
        if (!this._host || typeof this._host.invokeTool !== 'function') {
          throw new Error('host.invokeTool is required');
        }
        return this._host.invokeTool(message.tool);
      })
      .then(results => {
        if (!Array.isArray(results)) {
          throw new Error('host.invokeTool must resolve to an array of ToolResult objects');
        }
        this._sendHostToolCallResult(message.id, results, null);
      })
      .catch(error => {
        const messageText = error instanceof Error ? error.message : String(error);
        this._sendHostToolCallResult(message.id, [], messageText);
      });
  }

  _sendHostToolCallResult(id, results, error) {
    const payload = {
      type: 'hostToolCallResult',
      id: String(id),
      results
    };
    if (error != null) {
      payload.error = String(error);
    }
    this._child.stdin.write(`${JSON.stringify(payload)}\n`, writeError => {
      if (writeError) {
        this._rejectAll(writeError);
      }
    });
  }

  _rejectAll(error) {
    this._activeRequestId = null;
    this._queue = [];
    for (const pending of this._pending.values()) {
      pending.reject(error);
    }
    this._pending.clear();
  }
}

module.exports = {
  ToolPkgRuntimeClient,
  errorToolResult,
  stringToolResult
};
