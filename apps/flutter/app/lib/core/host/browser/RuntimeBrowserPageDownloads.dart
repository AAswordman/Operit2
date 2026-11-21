// ignore_for_file: file_names
import 'dart:convert';
import 'dart:typed_data';
import 'package:webview_all/webview_all.dart';

/// Receives page-created files through a bounded, acknowledged chunk protocol.
class RuntimeBrowserPageDownloads {
  /// Binds page messages to the same engine download lifecycle as native files.
  RuntimeBrowserPageDownloads({
    required this.emit,
    required this.runJavaScript,
  });
  final Future<void> Function(WebViewDownloadEvent) emit;
  final Future<void> Function(String) runJavaScript;
  final _transfers = <String, _PageTransfer>{};

  /// Validates message ordering and acknowledges each chunk before the next is sent.
  Future<void> handle(String message) async {
    final value = jsonDecode(message) as Map<String, dynamic>;
    final id = value['id'] as String;
    try {
      switch (value['type']) {
        case 'begin':
          if (_transfers.containsKey(id)) {
            throw StateError('Duplicate page transfer');
          }
          final transfer = _PageTransfer(
            value['url'] as String,
            value['fileName'] as String,
            (value['totalBytes'] as num?)?.toInt(),
          );
          _transfers[id] = transfer;
          await emit(_event(id, transfer, WebViewDownloadState.running));
          await _control(id, 'ack', -1);
        case 'metadata':
          final transfer = _transfers[id];
          if (transfer == null) return;
          transfer.total = (value['totalBytes'] as num?)?.toInt();
          await emit(_event(id, transfer, WebViewDownloadState.running));
          await _control(id, 'ack', -2);
        case 'chunk':
          final transfer = _transfers[id];
          if (transfer == null) return;
          final sequence = value['sequence'] as int;
          if (sequence != transfer.sequence++) {
            throw StateError('Out-of-order download chunk');
          }
          final bytes = base64Decode(value['data'] as String);
          if (bytes.length > 65536) {
            throw StateError('Download chunk exceeds protocol limit');
          }
          transfer.bytes.add(bytes);
          await emit(_event(id, transfer, WebViewDownloadState.running));
          await _control(id, 'ack', sequence);
        case 'end':
          final transfer = _transfers[id];
          if (transfer == null) return;
          if (value['receivedBytes'] != transfer.bytes.length) {
            throw StateError('Incomplete page download');
          }
          if (transfer.total != null &&
              transfer.total != transfer.bytes.length) {
            throw StateError(
              'The page download ended before its declared content length',
            );
          }
          _transfers.remove(id);
          await emit(_event(id, transfer, WebViewDownloadState.completed));
        case 'error':
          final transfer = _transfers.remove(id);
          if (transfer != null) {
            await emit(
              _event(
                id,
                transfer,
                WebViewDownloadState.failed,
                error: value['error'] as String,
              ),
            );
          }
        default:
          throw StateError('Unknown page download message');
      }
    } catch (error) {
      final transfer = _transfers.remove(id);
      if (transfer != null) {
        await emit(
          _event(
            id,
            transfer,
            WebViewDownloadState.failed,
            error: error.toString(),
          ),
        );
      }
      await _control(id, 'cancel', -1);
    }
  }

  /// Reports navigation or session closure as an interrupted page transfer.
  Future<void> close() async {
    final pending = Map.of(_transfers);
    _transfers.clear();
    for (final entry in pending.entries) {
      await emit(
        _event(
          entry.key,
          entry.value,
          WebViewDownloadState.failed,
          error: 'The originating page closed before the download completed',
        ),
      );
    }
  }

  /// Produces a transfer update while keeping byte buffers private to the owner.
  WebViewDownloadEvent _event(
    String id,
    _PageTransfer transfer,
    WebViewDownloadState state, {
    String? error,
  }) => WebViewDownloadEvent(
    id: 'page:$id',
    url: transfer.url,
    fileName: transfer.name,
    state: state,
    receivedBytes: transfer.bytes.length,
    totalBytes: transfer.total,
    error: error,
    cancel: () async {
      _transfers.remove(id);
      await _control(id, 'cancel', -1);
    },
    readBytes: state == WebViewDownloadState.completed
        ? () async => transfer.bytes.takeBytes()
        : null,
  );

  /// Routes acknowledgements to the originating frame without crossing origin boundaries.
  Future<void> _control(
    String id,
    String action,
    int sequence,
  ) => runJavaScript(
    'globalThis.__operitPageDownloads.control(${jsonEncode(id)},${jsonEncode(action)},$sequence);',
  );
}

class _PageTransfer {
  /// Retains the metadata and ordered chunks belonging to one generated file.
  _PageTransfer(this.url, this.name, this.total);
  final String url;
  final String name;
  int? total;
  final bytes = BytesBuilder(copy: false);
  int sequence = 0;
}

/// Captures explicit blob/data downloads while preserving normal native navigation.
const runtimeBrowserPageDownloadScript = r'''
(() => {
  if (globalThis.__operitPageDownloads) return;
  const transfers = new Map();
  const post = (message) => globalThis.OperitBrowserDownloads.postMessage(JSON.stringify(message));
  const control = (id, action, sequence) => {
    const transfer = transfers.get(id);
    if (transfer) {
      if (action === 'cancel') {
        transfer.controller.abort();
        if (transfer.reject) transfer.reject(new Error('Download cancelled'));
      } else if (transfer.sequence === sequence && transfer.resolve) {
        transfer.resolve();
      }
      return;
    }
    for (let index = 0; index < frames.length; index++) {
      frames[index].postMessage({operitDownloadControl: true, id, action, sequence}, '*');
    }
  };
  addEventListener('message', (event) => {
    if (event.source !== parent || !event.data || event.data.operitDownloadControl !== true) return;
    const {id, action, sequence} = event.data;
    control(id, action, sequence);
  });
  const start = async (url, fileName = 'download') => {
    const id = 'file-' + Array.from(crypto.getRandomValues(new Uint8Array(16)), (byte) => byte.toString(16).padStart(2, '0')).join('');
    const transfer = {controller: new AbortController(), sequence: -1};
    transfers.set(id, transfer);
    const send = (message, sequence) => new Promise((resolve, reject) => {
      transfer.sequence = sequence;
      const timeout = setTimeout(() => reject(new Error('Download owner stopped acknowledging data')), 30000);
      transfer.resolve = () => { clearTimeout(timeout); resolve(); };
      transfer.reject = (error) => { clearTimeout(timeout); reject(error); };
      post({...message, id});
    });
    let received = 0;
    let reader;
    try {
      const responsePromise = fetch(url, {signal: transfer.controller.signal, credentials: 'include'});
      responsePromise.catch(() => {});
      await send({type:'begin', url, fileName, totalBytes:null}, -1);
      const response = await responsePromise;
      if (!response.ok) throw new Error('HTTP ' + response.status);
      const encoding = response.headers.get('content-encoding');
      const length = encoding === null || encoding === 'identity' ? response.headers.get('content-length') : null;
      await send({type:'metadata', totalBytes: length === null ? null : Number(length)}, -2);
      reader = response.body.getReader();
      let sequence = 0;
      while (true) {
        const {value, done} = await reader.read();
        if (done) break;
        for (let offset = 0; offset < value.length; offset += 65536) {
          const chunk = value.subarray(offset, offset + 65536);
          let binary = '';
          for (let index = 0; index < chunk.length; index += 8192) {
            binary += String.fromCharCode(...chunk.subarray(index, index + 8192));
          }
          await send({type:'chunk', sequence, data:btoa(binary)}, sequence++);
          received += chunk.length;
        }
      }
      post({type:'end', id, receivedBytes:received});
    } catch (error) {
      post({type:'error', id, error:String(error)});
    } finally {
      if (reader) await reader.cancel().catch(() => {});
      transfers.delete(id);
    }
  };
  globalThis.__operitPageDownloads = {start, control};
  addEventListener('click', (event) => {
    const anchor = event.composedPath().find((node) => node && node.tagName === 'A');
    if (!anchor || !anchor.hasAttribute('download')) return;
    const url = new URL(anchor.href, location.href);
    if (url.protocol !== 'blob:' && url.protocol !== 'data:') return;
    event.preventDefault();
    event.stopImmediatePropagation();
    void start(url.href, anchor.download || 'download');
  }, true);
})();
''';
