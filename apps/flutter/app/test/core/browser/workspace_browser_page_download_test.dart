import 'dart:convert';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserPageDownloads.dart';
import 'package:webview_all/webview_all.dart';

/// Exercises generated-file transfer ordering, acknowledgement, and teardown.
void main() {
  late List<WebViewDownloadEvent> events;
  late List<String> scripts;
  late RuntimeBrowserPageDownloads bridge;

  setUp(() {
    events = [];
    scripts = [];
    bridge = RuntimeBrowserPageDownloads(
      emit: (event) async {
        events.add(event);
      },
      runJavaScript: (script) async {
        scripts.add(script);
      },
    );
  });

  /// Sends the same begin message emitted by an injected blob download.
  Future<void> begin() => bridge.handle(
    jsonEncode({
      'type': 'begin',
      'id': 'file',
      'url': 'blob:https://example.test/id',
      'fileName': 'report.pdf',
      'totalBytes': null,
    }),
  );

  /// Sends one acknowledged binary chunk with its explicit sequence number.
  Future<void> chunk(int sequence, List<int> bytes) => bridge.handle(
    jsonEncode({
      'type': 'chunk',
      'id': 'file',
      'sequence': sequence,
      'data': base64Encode(bytes),
    }),
  );

  test(
    'generated files preserve binary bytes and acknowledge every chunk',
    () async {
      await begin();
      await chunk(0, [0, 128, 255]);
      await chunk(1, [1, 2]);
      await bridge.handle(
        jsonEncode({'type': 'end', 'id': 'file', 'receivedBytes': 5}),
      );
      expect(events.last.state, WebViewDownloadState.completed);
      expect(await events.last.readBytes!(), [0, 128, 255, 1, 2]);
      expect(events.last.receivedBytes, 5);
      expect(scripts, hasLength(3));
      expect(scripts.last, contains('"ack",1'));
    },
  );

  test('server length becomes known after the transfer starts', () async {
    await begin();
    await bridge.handle(
      jsonEncode({'type': 'metadata', 'id': 'file', 'totalBytes': 3}),
    );
    expect(events.last.totalBytes, 3);
    expect(scripts.last, contains('"ack",-2'));
  });

  test(
    'out-of-order chunks fail and abort the originating page request',
    () async {
      await begin();
      await chunk(1, [1]);
      expect(events.last.state, WebViewDownloadState.failed);
      expect(events.last.error, contains('Out-of-order'));
      expect(scripts.last, contains('"cancel"'));
    },
  );

  test(
    'oversized chunks are rejected before entering the file buffer',
    () async {
      await begin();
      await chunk(0, List.filled(65537, 1));
      expect(events.last.state, WebViewDownloadState.failed);
      expect(events.last.receivedBytes, 0);
    },
  );

  test('incomplete files cannot report completion', () async {
    await begin();
    await chunk(0, [1]);
    await bridge.handle(
      jsonEncode({'type': 'end', 'id': 'file', 'receivedBytes': 2}),
    );
    expect(events.last.state, WebViewDownloadState.failed);
    expect(events.last.error, contains('Incomplete'));
  });

  test('known content length must match the complete response', () async {
    await begin();
    await bridge.handle(
      jsonEncode({'type': 'metadata', 'id': 'file', 'totalBytes': 3}),
    );
    await chunk(0, [1]);
    await bridge.handle(
      jsonEncode({'type': 'end', 'id': 'file', 'receivedBytes': 1}),
    );
    expect(events.last.state, WebViewDownloadState.failed);
  });

  test(
    'closing the page interrupts transfers and rejects late chunks',
    () async {
      await begin();
      await bridge.close();
      expect(events.last.state, WebViewDownloadState.failed);
      final count = events.length;
      await chunk(0, [1]);
      expect(events, hasLength(count));
    },
  );

  test('cancellation releases buffers and aborts the page operation', () async {
    await begin();
    await events.single.cancel!();
    await chunk(0, [1]);
    expect(events, hasLength(1));
    expect(scripts.last, contains('"cancel"'));
  });
}
