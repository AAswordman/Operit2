@TestOn('browser')
library;

import 'dart:convert';
import 'dart:js_interop';
import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:file_selector_web/file_selector_web.dart';
import 'package:file_selector_web/src/dom_helper.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/ComposeDslFilePickerService.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';
import 'package:web/web.dart' as web;

import 'support/MemoryFilePickerStorage.dart';

/// Exercises browser File selection through the production Web adapter and bounded content reader.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late MemoryFilePickerStorage storage;
  setUp(() => storage = MemoryFilePickerStorage());
  final original = FileSelectorPlatform.instance;
  tearDown(() => FileSelectorPlatform.instance = original);
  final request = jsonEncode({
    'picker': 'image',
    'allowMultiple': false,
    'mimeTypes': ['image/*'],
    'maxContentBytes': 8 * 1024 * 1024,
  });

  test(
    'Web selection publishes browser bytes at a readable VFS path',
    () async {
      final bytes = Uint8List(selectedFileChunkBytes + 7);
      for (var i = 0; i < bytes.length; i++) {
        bytes[i] = i % 251;
      }
      final selection = _BrowserSelection(
        web.File(
          [bytes.toJS].toJS,
          'avatar.png',
          web.FilePropertyBag(type: 'image/png'),
        ),
      );
      FileSelectorPlatform.instance = FileSelectorWeb(domHelper: selection);
      final result =
          jsonDecode(
                await ComposeDslFilePickerService.open(
                  request,
                  storage: storage,
                ),
              )
              as Map;
      final selected = (result['files'] as List).single as Map;
      final uri = selected['uri'] as String;
      expect(result['cancelled'], false);
      expect(selected['name'], 'avatar.png');
      expect(selected['mimeType'], 'image/png');
      expect(uri, startsWith('/app/data/temp/clean_on_exit/file_picker/'));
      expect(storage.files[uri], bytes);
      expect(selected['path'], uri);
      expect(base64Decode(selected['contentBase64'] as String), bytes);
      expect(selection.accept, contains('image/*'));
      expect(selection.multiple, false);
    },
  );

  test('Web cancellation completes the same owner contract', () async {
    FileSelectorPlatform.instance = FileSelectorWeb(
      domHelper: _BrowserSelection(null),
    );
    final result = jsonDecode(
      await ComposeDslFilePickerService.open(request, storage: storage),
    );
    expect(result, {'cancelled': true, 'files': []});
  });
}

/// Supplies browser selection events while retaining DomHelper and BlobXFile production behavior.
class _BrowserSelection extends DomHelper {
  /// Defines the browser File selected by the user, or an explicit cancellation.
  _BrowserSelection(this.file);
  final web.File? file;
  String? accept;
  bool? multiple;

  /// Delivers a DOM event to the real input handler without opening an interactive dialog.
  @override
  Future<List<XFile>> getFiles({
    String accept = '',
    bool multiple = false,
    web.HTMLInputElement? input,
  }) {
    this.accept = accept;
    this.multiple = multiple;
    final element = web.HTMLInputElement()..type = 'file';
    final pending = super.getFiles(
      accept: accept,
      multiple: multiple,
      input: element,
    );
    final selected = file;
    if (selected == null) {
      element.dispatchEvent(web.Event('cancel'));
    } else {
      final transfer = web.DataTransfer();
      transfer.items.add(selected);
      element.files = transfer.files;
      element.dispatchEvent(web.Event('change'));
    }
    return pending;
  }
}
