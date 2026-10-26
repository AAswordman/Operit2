// Regression tests for protocol-markup recovery.
//
// On device a chat block whose opening tag was cut by a stream boundary used to render its raw XML
// into the transcript (`<tool_result name=… status="success"><content>…`). The renderer's fallback
// now refuses to show markup: it shows the `<content>` payload, or nothing.
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/part/ProtocolMarkupRecovery.dart';

void main() {
  group('looksLikeProtocolMarkup', () {
    test('recognizes the shapes seen in the real transcript', () {
      const samples = <String>[
        'name="tool_name">super_admin:shell</param></tool>',
        '<tool_result name="super_admin:shell" status="success"><content>{"ok":true}</content></tool_result>',
        '<tool name="package_proxy" call_id="part-177"><param name="params">{"sessionId":"x"}</param>',
        '<tool_call name="sleep"><parameter name="duration_ms">90000</parameter>',
      ];
      for (final sample in samples) {
        expect(looksLikeProtocolMarkup(sample), isTrue, reason: sample);
      }
    });

    test('leaves ordinary prose alone', () {
      const samples = <String>[
        '这一局只有 46 帧，我直接分析。',
        'if (a < b) then return;',
        '表格里 `tools` 和 `status` 都是普通单词。',
        '',
      ];
      for (final sample in samples) {
        expect(looksLikeProtocolMarkup(sample), isFalse, reason: sample);
      }
    });
  });

  group('protocolMarkupPayload', () {
    test('extracts and unescapes the content payload', () {
      const block =
          '<tool_result name="bash" status="success"><content>{"background":true,&quot;command&quot;:&quot;id&quot;}</content></tool_result>';
      final payload = protocolMarkupPayload(block);
      expect(payload, isNotNull);
      expect(payload, contains('"command":"id"'));
      expect(payload, isNot(contains('&quot;')));
    });

    test('returns null when the block has no readable payload', () {
      expect(protocolMarkupPayload('name="tool_name">super_admin:shell</param></tool>'), isNull);
      expect(protocolMarkupPayload('<content></content>'), isNull);
    });

    test('returns null for ordinary prose so the caller keeps its own rendering', () {
      expect(protocolMarkupPayload('普通的一句话'), isNull);
    });
  });

  group('file-diff block (seen leaking on device 2026-10-26 12:33)', () {
    const block = '<file-diff path="/mnt/android/sdcard/mod/smali/com/endfield/hook/AppAttachHook.smali" '
        'details="Successfully applied AI code to file: /mnt/android/sdcard/mod/smali/com/endfield/hook/AppAttachHook.smali">'
        '<![CDATA[Changes: +42 -2 lines\n@@ -51,10 +51,50 @@\n 51 |     const-string v1, "g6.b"]]></file-diff>';

    test('is recognized as protocol markup even though it is not in the tag whitelist', () {
      expect(looksLikeProtocolMarkup(block), isTrue);
    });

    test('exposes the CDATA payload instead of the raw markup', () {
      final payload = protocolMarkupPayload(block);
      expect(payload, isNotNull);
      expect(payload, contains('Changes: +42 -2 lines'));
      expect(payload, isNot(contains('<![CDATA[')));
      expect(payload, isNot(contains('file-diff')));
    });

    test('an attributed tag is enough even for an unknown tag name', () {
      expect(looksLikeProtocolMarkup('<some-future-block id="7">body</some-future-block>'), isTrue);
    });
  });
}
