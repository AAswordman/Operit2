import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/ComposeDslFilePickerService.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final original = FileSelectorPlatform.instance;
  tearDown(() => FileSelectorPlatform.instance = original);

  Future<Map<String, dynamic>> pick({int? limit}) async =>
      jsonDecode(
            await ComposeDslFilePickerService.open(
              jsonEncode({
                'picker': 'image',
                'allowMultiple': false,
                'mimeTypes': ['image/*'],
                'maxContentBytes': ?limit,
              }),
            ),
          )
          as Map<String, dynamic>;

  test(
    'iOS picker bytes are returned without mounting the native path',
    () async {
      final file = _SelectedImage(selectedFileChunkBytes + 7);
      FileSelectorPlatform.instance = _Selector(file);
      final result = await pick(limit: 8 * 1024 * 1024);
      final entry = (result['files'] as List).single as Map;
      expect(result['cancelled'], false);
      expect(entry['path'], file.path);
      expect(entry['name'], 'avatar.png');
      expect(entry['size'], file.size);
      final bytes = base64Decode(entry['contentBase64'] as String);
      expect(bytes.length, file.size);
      for (var i = 0; i < bytes.length; i++) {
        if (bytes[i] != i % 251) fail('Selected content changed at byte $i');
      }
      expect(file.ranges, [
        (0, selectedFileChunkBytes),
        (selectedFileChunkBytes, file.size),
      ]);
    },
  );

  test('oversized images are rejected before reading any bytes', () async {
    final file = _SelectedImage(8 * 1024 * 1024 + 1);
    FileSelectorPlatform.instance = _Selector(file);
    await expectLater(pick(limit: 8 * 1024 * 1024), throwsStateError);
    expect(file.ranges, isEmpty);
  });

  test('ordinary metadata selection does not read file contents', () async {
    final file = _SelectedImage(100);
    FileSelectorPlatform.instance = _Selector(file);
    final result = await pick();
    expect((result['files'] as List).single, isNot(contains('contentBase64')));
    expect(file.ranges, isEmpty);
  });

  test(
    'cancellation returns no content and invalid limits fail before selection',
    () async {
      final selector = _Selector(null);
      FileSelectorPlatform.instance = selector;
      expect(await pick(limit: 1024), {'cancelled': true, 'files': []});
      for (final limit in [0, -1]) {
        await expectLater(pick(limit: limit), throwsFormatException);
      }
      expect(selector.calls, 1);
    },
  );

  test('read failures propagate without a successful import result', () async {
    final file = _SelectedImage(10, failRead: true);
    FileSelectorPlatform.instance = _Selector(file);
    await expectLater(pick(limit: 1024), throwsStateError);
  });
}

class _Selector extends FileSelectorPlatform {
  _Selector(this.file);
  final XFile? file;
  int calls = 0;

  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    calls++;
    return file;
  }
}

/// Models a selected native file whose unmounted path cannot be read by plugins.
class _SelectedImage extends XFile {
  _SelectedImage(this.size, {this.failRead = false})
    : super('/private/var/mobile/tmp/selector/avatar.png', name: 'avatar.png');
  final int size;
  final bool failRead;
  final ranges = <(int, int)>[];

  @override
  Future<int> length() async => size;

  @override
  Future<Uint8List> readAsBytes() async =>
      throw StateError('Unbounded source read');

  @override
  Stream<Uint8List> openRead([int? start, int? end]) async* {
    if (failRead) throw StateError('Selected image is no longer readable');
    if (start == null || end == null || end - start > selectedFileChunkBytes) {
      throw StateError('Selected image must be read in bounded ranges');
    }
    ranges.add((start, end));
    yield Uint8List.fromList(
      List.generate(end - start, (i) => (start + i) % 251),
    );
  }
}
