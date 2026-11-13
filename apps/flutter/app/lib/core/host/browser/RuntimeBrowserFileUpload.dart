// ignore_for_file: file_names

import 'dart:convert';

import 'package:mime/mime.dart';
import 'package:webview_all/webview_all.dart';

/// Owns file chooser interception and uploads for one browser automation session.
class RuntimeBrowserFileUpload {
  /// Binds the upload lifecycle to the session's existing WebView controller.
  RuntimeBrowserFileUpload(this._controller);

  final WebViewController _controller;
  int _nextCaptureId = 0;

  /// Runs an automation action while capturing the file chooser it opens.
  Future<void> run(String script) {
    return _capture(() => _controller.runJavaScript(script));
  }

  /// Evaluates an automation action and preserves its JavaScript return value.
  Future<Object?> evaluate(String script) {
    return _capture(() => _controller.runJavaScriptReturningResult(script));
  }

  /// Applies host-provided files, clears the selection, or cancels the chooser.
  Future<void> upload(String filesJson) {
    final files = _decodeFiles(filesJson);
    final payload = jsonEncode(files?.map((file) => file.toJson()).toList());
    return _controller.runJavaScript('''
$_fileChooserRuntimeScript
window.__operitBrowserFileChooser.run(() =>
  window.__operitBrowserFileChooser.upload($payload));
''');
  }

  /// Keeps chooser capture active while the WebView executes unmodified code.
  ///
  /// Page-owned eval is subject to CSP. Separate native calls preserve script
  /// completion values and global declarations without evaluating a string in
  /// the page. Unique scopes keep overlapping actions and navigation independent.
  Future<T> _capture<T>(Future<T> Function() action) async {
    final captureId = _nextCaptureId++;
    await _controller.runJavaScript('''
$_fileChooserRuntimeScript
window.__operitBrowserFileChooser.beginCapture($captureId);
''');
    var actionFailed = false;
    try {
      return await action();
    } catch (_) {
      actionFailed = true;
      rethrow;
    } finally {
      try {
        await _controller.runJavaScript(
          'window.__operitBrowserFileChooser?.endCapture($captureId);',
        );
      } catch (_) {
        // A navigation or disposed WebView must not hide the action's error.
        if (!actionFailed) rethrow;
      }
    }
  }

  /// Decodes the internal host payload while preserving explicit cancellation.
  List<_BrowserUploadFile>? _decodeFiles(String filesJson) {
    final decoded = jsonDecode(filesJson);
    if (decoded == null) return null;
    if (decoded is! List<Object?>) {
      throw StateError('Browser upload files must be an array or null');
    }
    return decoded.map(_BrowserUploadFile.fromJson).toList(growable: false);
  }
}

/// Represents one validated host file without exposing file-system APIs to Dart.
class _BrowserUploadFile {
  /// Stores the name, encoded bytes, and detected media type of one file.
  const _BrowserUploadFile(this.name, this.base64, this.mimeType);

  /// Validates one host file and derives its media type from its name and bytes.
  factory _BrowserUploadFile.fromJson(Object? value) {
    if (value is! Map<String, Object?>) {
      throw StateError('Browser upload file must be an object');
    }
    final name = value['name'];
    final encoded = value['base64'];
    if (name is! String || name.isEmpty || encoded is! String) {
      throw StateError('Browser upload file requires name and base64');
    }
    final mimeType = lookupMimeType(name, headerBytes: base64Decode(encoded));
    return _BrowserUploadFile(name, encoded, mimeType);
  }

  final String name;
  final String base64;
  final String? mimeType;

  /// Encodes the browser File payload, omitting an unknown optional media type.
  Map<String, Object?> toJson() {
    final result = <String, Object?>{'name': name, 'base64': base64};
    if (mimeType != null) result['type'] = mimeType;
    return result;
  }
}

/// Encapsulates chooser state, interception, and selection in one page runtime.
const _fileChooserRuntimeScript = r'''
(function() {
  if (window.__operitBrowserFileChooser) return;

  class BrowserFileChooser {
    /** Creates the lifecycle state for one page and its accessible frames. */
    constructor() {
      this.captureScopes = new Set();
      this.pendingInput = null;
      this.documents = new WeakSet();
      this.frames = new WeakSet();
      this.onClick = this.capture.bind(this);
    }

    /** Counts only the capture scopes still active in this document. */
    get captureDepth() {
      return this.captureScopes.size;
    }

    /** Opens an independent scope for a native or in-page automation action. */
    beginCapture(scope = Symbol()) {
      this.installDocument(document);
      this.captureScopes.add(scope);
      return scope;
    }

    /** Ignores scopes belonging to an earlier document after navigation. */
    endCapture(scope) {
      this.captureScopes.delete(scope);
    }

    /** Captures chooser requests only for the duration of an automation action. */
    run(action) {
      const scope = this.beginCapture();
      let asynchronous = false;
      try {
        const result = action();
        if (result && typeof result.then === 'function') {
          const pending = Promise.resolve(result).finally(() => { this.endCapture(scope); });
          asynchronous = true;
          return pending;
        }
        return result;
      } finally {
        if (!asynchronous) this.endCapture(scope);
      }
    }

    /** Applies a selection atomically or cancels without changing existing files. */
    upload(files) {
      const input = this.requireInput();
      if (files === null) {
        this.pendingInput = null;
        this.dispatch(input, 'cancel');
        return;
      }
      this.validateSelection(input, files);
      input.files = this.createFileList(input.ownerDocument.defaultView, files);
      this.pendingInput = null;
      this.dispatch(input, 'input');
      this.dispatch(input, 'change');
    }

    /** Requires the original file input to belong to a live page document. */
    requireInput() {
      const input = this.pendingInput;
      if (!input) throw new Error('No active browser file chooser');
      const view = input.ownerDocument.defaultView;
      if (!view || view.document !== input.ownerDocument || input.type !== 'file') {
        throw new Error('Browser file chooser is no longer valid');
      }
      return input;
    }

    /** Validates chooser constraints before replacing its selected files. */
    validateSelection(input, files) {
      if (input.disabled) throw new Error('Browser file input is disabled');
      if (input.webkitdirectory) throw new Error('Browser upload does not support directory choosers');
      if (!input.multiple && files.length > 1) {
        throw new Error('Browser file chooser does not allow multiple files');
      }
    }

    /** Builds browser File objects using the selected input's own JavaScript realm. */
    createFileList(view, files) {
      const transfer = new view.DataTransfer();
      for (const file of files) {
        const binary = view.atob(file.base64);
        const bytes = view.Uint8Array.from(binary, character => character.charCodeAt(0));
        const options = {};
        if (file.type !== undefined) options.type = file.type;
        transfer.items.add(new view.File([bytes], file.name, options));
      }
      return transfer.files;
    }

    /** Notifies page listeners after the chooser transition has committed. */
    dispatch(input, type) {
      const Event = input.ownerDocument.defaultView.Event;
      input.dispatchEvent(new Event(type, { bubbles: true, composed: type !== 'cancel' }));
    }

    /** Records file input clicks without suppressing ordinary manual selection. */
    capture(event) {
      if (this.captureDepth === 0) return;
      const input = event.composedPath().find(node =>
        node && node.nodeType === 1 && node.tagName === 'INPUT' && node.type === 'file');
      if (!input || input.disabled) return;
      event.preventDefault();
      this.pendingInput = input;
    }

    /** Installs interception once for each accessible document and its frames. */
    installDocument(doc) {
      if (!this.documents.has(doc)) {
        this.documents.add(doc);
        doc.addEventListener('click', this.onClick, true);
        this.installInputMethods(doc.defaultView);
        const observer = new doc.defaultView.MutationObserver(() => this.installFrames(doc));
        observer.observe(doc, { childList: true, subtree: true });
      }
      this.installFrames(doc);
    }

    /** Keeps same-origin frame interception synchronized with frame loads. */
    installFrames(doc) {
      for (const frame of doc.querySelectorAll('iframe')) {
        if (!this.frames.has(frame)) {
          this.frames.add(frame);
          frame.addEventListener('load', () => this.installFrame(frame));
        }
        this.installFrame(frame);
      }
    }

    /** Installs hooks only in frames whose document is accessible to the page. */
    installFrame(frame) {
      const doc = frame.contentDocument;
      if (doc) this.installDocument(doc);
    }

    /** Captures detached input clicks while preserving native event dispatch. */
    installInputMethods(view) {
      const runtime = this;
      const prototype = view.HTMLInputElement.prototype;
      const originalClick = prototype.click;
      /** Wraps a file input click for the active automation action. */
      prototype.click = function() {
        const intercept = runtime.captureDepth > 0 && this.type === 'file';
        if (intercept) this.addEventListener('click', runtime.onClick, true);
        try {
          return originalClick.call(this);
        } finally {
          if (intercept) this.removeEventListener('click', runtime.onClick, true);
        }
      };
      this.installShowPicker(view, prototype);
    }

    /** Preserves the browser's picker API while intercepting automated file requests. */
    installShowPicker(view, prototype) {
      if (typeof prototype.showPicker !== 'function') return;
      const runtime = this;
      const originalShowPicker = prototype.showPicker;
      /** Captures an explicit file picker request during automation. */
      prototype.showPicker = function() {
        if (runtime.captureDepth === 0 || this.type !== 'file') {
          return originalShowPicker.call(this);
        }
        if (this.disabled) {
          throw new view.DOMException('The file input is disabled', 'InvalidStateError');
        }
        runtime.pendingInput = this;
      };
    }
  }

  window.__operitBrowserFileChooser = new BrowserFileChooser();
})()
''';
