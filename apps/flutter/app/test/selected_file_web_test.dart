@TestOn('browser')
library;

import 'dart:js_interop';
import 'dart:typed_data';

import 'package:file_selector_web/src/blob_xfile.dart';
import 'package:file_selector_web/src/dom_helper.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';
import 'package:operit2/ui/features/chat/viewmodel/ChatViewModel.dart';

import 'streamed_file_attachment_test.dart' show UploadBridge;
import 'package:web/web.dart' as web;

void main() {
  test(
    'Web selection retains browser File with no complete-content read',
    () async {
      final input = web.HTMLInputElement()..type = 'file';
      final transfer = web.DataTransfer();
      transfer.items.add(
        web.File(
          [
            Uint8List.fromList([0, 1, 255]).toJS,
          ].toJS,
          '文档.apk',
        ),
      );
      final selection = DomHelper().getFiles(input: input);
      input.files = transfer.files;
      input.dispatchEvent(web.Event('change'));
      final files = await selection;
      expect(files.single, isA<BlobXFile>());
      expect(files.single.name, '文档.apk');
      expect(await files.single.length(), 3);
      expect(await files.single.openRead(1, 3).single, [1, 255]);
      web.URL.revokeObjectURL(files.single.path);
    },
  );

  test(
    '143 MB browser Blob streams bounded slices without object URL fetches',
    () async {
      const size = 143388128;
      final block = Uint8List(selectedFileChunkBytes);
      for (var i = 0; i < block.length; i++) {
        block[i] = i % 251;
      }
      final fullBlocks = size ~/ block.length;
      final tail = size % block.length;
      final blob = web.File(
        [
          for (var i = 0; i < fullBlocks; i++) block.toJS,
          Uint8List.sublistView(block, 0, tail).toJS,
        ].toJS,
        'large.apk',
      );
      final file = BlobXFile(blob);
      // Reads must use the original browser File; re-fetching this URL must fail.
      web.URL.revokeObjectURL(file.path);
      var total = 0;
      await for (final bytes in file.openRead()) {
        expect(bytes.length, lessThanOrEqualTo(selectedFileChunkBytes));
        expect(bytes.first, 0);
        expect(bytes.last, (bytes.length - 1) % 251);
        total += bytes.length;
      }
      expect(total, size);
      final selected = await SelectedFileInput.fromXFile(file, name: '文档.apk');
      expect(selected.sourceJson, isNull);
      final bridge = UploadBridge();
      await ChatViewModel(
        bridge: bridge,
        attachmentSourceBridge: bridge,
      ).attachSelectedFile(selected, expectedChatId: 'browser-chat');
      expect(bridge.received, size);
      expect(bridge.completed, isTrue);
      expect(bridge.methods, [
        'selectedAttachmentPolicy',
        'beginAttachmentUpload',
        'writeAttachmentSource',
        'completeAttachmentSource',
        'createReferenceAttachment',
        'attachUploadedFile',
      ]);
      await expectLater(selected.readChunk(), throwsStateError);
      await expectLater(file.openRead(-1, 10).toList(), throwsRangeError);
      expect(await file.openRead(0, 0).toList(), isEmpty);
    },
  );
}
