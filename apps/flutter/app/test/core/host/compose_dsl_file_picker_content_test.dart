import 'dart:convert';
import 'dart:typed_data';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/ComposeDslFilePickerService.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';

import 'support/MemoryFilePickerStorage.dart';

/// Verifies bounded content reads independently of the test runner platform.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late MemoryFilePickerStorage storage;
  setUp(() => storage = MemoryFilePickerStorage());
  final original = FileSelectorPlatform.instance;
  tearDown(() => FileSelectorPlatform.instance = original);

  /// Requests the same content contract used by character avatars.
  Future<Map<String, dynamic>> pick({int? limit}) async =>
      jsonDecode(
            await ComposeDslFilePickerService.open(
              jsonEncode({
                'picker': 'image',
                'allowMultiple': false,
                'mimeTypes': ['image/*'],
                'maxContentBytes': ?limit,
              }),
              storage: storage,
            ),
          )
          as Map<String, dynamic>;

  test(
    'iOS selection returns a readable VFS file and bounded content',
    () async {
      final file = _SelectedImage(selectedFileChunkBytes + 7);
      FileSelectorPlatform.instance = _Selector(file);
      final result = await pick(limit: 8 * 1024 * 1024);
      final entry = (result['files'] as List).single as Map;
      expect(result['cancelled'], false);
      expect(entry['path'], storage.allocated.single);
      expect(entry['uri'], entry['path']);
      expect(storage.files[entry['path']], isNotNull);
      expect(entry['path'], isNot(file.path));
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

  test(
    'selection without inline content still publishes a readable VFS file',
    () async {
      final file = _SelectedImage(100);
      FileSelectorPlatform.instance = _Selector(file);
      final result = await pick();
      expect(
        (result['files'] as List).single,
        isNot(contains('contentBase64')),
      );
      expect(file.ranges, [(0, file.size)]);
      final entry = (result['files'] as List).single as Map;
      expect(storage.files[entry['path']], hasLength(file.size));
      expect(entry['uri'], entry['path']);
    },
  );

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
    expect(storage.files, isEmpty);
    expect(storage.deleted, storage.allocated);
  });

  test(
    'failed multi-file selection removes every committed destination',
    () async {
      FileSelectorPlatform.instance = _MultipleSelector([
        _SelectedImage(10),
        _SelectedImage(10, failRead: true),
      ]);
      await expectLater(
        ComposeDslFilePickerService.open(
          jsonEncode({
            'picker': 'document',
            'allowMultiple': true,
            'mimeTypes': [],
          }),
          storage: storage,
        ),
        throwsStateError,
      );
      expect(storage.allocated, hasLength(2));
      expect(storage.files, isEmpty);
      expect(storage.deleted, unorderedEquals(storage.allocated));
    },
  );

  test('storage failures never return a physical source path', () async {
    FileSelectorPlatform.instance = _Selector(_SelectedImage(10));
    storage.failWrite = true;
    await expectLater(pick(), throwsStateError);
    expect(storage.files, isEmpty);
    expect(storage.deleted, storage.allocated);
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

  /// Supplies iOS display metadata independently of the test runner's path separators.
  @override
  String get name => 'avatar.png';

  /// Reports the native selection size without reading it.
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

/// Supplies multiple sources so one failed read can exercise whole-request cleanup.
class _MultipleSelector extends FileSelectorPlatform {
  /// Sets the exact ordered sources selected by the user.
  _MultipleSelector(this.files);
  final List<XFile> files;

  /// Returns the selected sources through the same multi-file adapter contract.
  @override
  Future<List<XFile>> openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async => files;
}
