import 'dart:async';
import 'dart:convert';

import 'package:operit_folder_access/operit_folder_access.dart'
    show FolderMountSource, OperitFolderAccessMacOS, OperitFolderAccessPlatform;

import 'package:file_selector_ohos/src/file_selector_api.g.dart';
import 'package:file_selector_ohos/src/file_selector_ohos.dart';
import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/SelectedFileInput.dart';

class OhosMetadataApi extends FileSelectorApi {
  OhosMetadataApi(this.response);
  final FileResponse response;
  @override
  Future<FileResponse?> openFile(String? dir, FileTypes types) async =>
      response;
  @override
  Future<List<FileResponse>> openFiles(String? dir, FileTypes types) async => [
    response,
  ];
}

/// Like Web cross_file: a read without an explicit range would allocate the file.
class RangeOnlyFile extends XFile {
  RangeOnlyFile(this.size, {String path = '/not-read/large.apk'}) : super(path);
  final int size;
  final ranges = <(int, int)>[];
  @override
  Future<int> length() async => size;
  @override
  Stream<Uint8List> openRead([int? start, int? end]) async* {
    if (start == null || end == null) throw StateError('Unbounded source read');
    ranges.add((start, end));
    if (end - start > selectedFileChunkBytes) {
      throw StateError('Oversized read');
    }
    final bytes = Uint8List(end - start);
    for (var i = 0; i < bytes.length; i++) {
      bytes[i] = (start + i) % 251;
    }
    yield bytes;
  }
}

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final originalFileAccess = OperitFolderAccessPlatform.instance;
  tearDown(() => OperitFolderAccessPlatform.instance = originalFileAccess);

  test('143 MB input is read lazily in bounded source ranges', () async {
    final source = RangeOnlyFile(143388128);
    final file = await SelectedFileInput.fromXFile(source);
    expect(source.ranges, isEmpty);
    var offset = 0;
    await for (final bytes in file.chunks()) {
      expect(bytes.length, lessThanOrEqualTo(selectedFileChunkBytes));
      expect(bytes.first, offset % 251);
      expect(bytes.last, (offset + bytes.length - 1) % 251);
      offset += bytes.length;
    }
    expect(offset, source.size);
    expect(source.ranges.length, (source.size / selectedFileChunkBytes).ceil());
    await file.close();
  });

  test(
    'Windows native sources retain their exact path without an eager read',
    () async {
      final source = RangeOnlyFile(143388128, path: r'D:\Downloads\large.apk');
      final input = await SelectedFileInput.fromXFile(
        source,
        name: 'large.apk',
      );
      expect(source.ranges, isEmpty);
      expect(jsonDecode(input.sourceJson!), {
        'backend': 'native',
        'root': source.path,
        'namespace': '/virtual/local/attachments',
        'name': 'large.apk',
      });
      await input.close();
      expect(source.ranges, isEmpty);
    },
  );

  test('unused and cancelled inputs never read the remaining file', () async {
    final unusedSource = RangeOnlyFile(143388128);
    final unused = await SelectedFileInput.fromXFile(unusedSource);
    await unused.close();
    expect(unusedSource.ranges, isEmpty);
    final source = RangeOnlyFile(143388128);
    final file = await SelectedFileInput.fromXFile(source);
    await file.readChunk();
    await file.close();
    expect(source.ranges, [(0, selectedFileChunkBytes)]);
  });

  test(
    'registered adapter supplies sources without app platform selection',
    () async {
      final adapter = _CustomFileAccess();
      OperitFolderAccessPlatform.instance = adapter;
      final source = RangeOnlyFile(143388128);
      final input = await SelectedFileInput.fromXFile(
        source,
        name: 'selected.apk',
      );
      expect(source.ranges, isEmpty);
      expect(jsonDecode(input.sourceJson!), {
        'backend': 'custom-resource',
        'root': 'authorized-resource',
        'namespace': '/virtual/custom/attachments',
        'name': 'selected.apk',
      });
      expect(adapter.files, [source]);
      await input.close();
      expect(source.ranges, isEmpty);
    },
  );

  test(
    'macOS adapter retains the grant before exposing a source without reading',
    () async {
      const accessChannel = MethodChannel('operit/folder_access');
      final retained = <String>[];
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
          .setMockMethodCallHandler(accessChannel, (call) async {
            expect(call.method, 'retainFile');
            retained.add(call.arguments['path'] as String);
            return null;
          });
      OperitFolderAccessMacOS.registerWith();
      final source = RangeOnlyFile(143388128);
      try {
        final input = await SelectedFileInput.fromXFile(source);
        expect(retained, [source.path]);
        expect(jsonDecode(input.sourceJson!)['root'], source.path);
        expect(source.ranges, isEmpty);
        await input.close();
      } finally {
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
            .setMockMethodCallHandler(accessChannel, null);
      }
    },
  );

  const channel = MethodChannel(
    'dev.flutter.packages.file_selector_ohos/input',
  );
  final calls = <MethodCall>[];
  final positions = <String, int>{};
  var nextToken = 0;
  var failRead = false;
  var shortRead = false;
  var oversizedRead = false;
  final api = OhosMetadataApi(
    FileResponse(
      path: 'file://docs/storage/large.apk',
      name: '文档.apk',
      mimeType: 'application/vnd.android.package-archive',
      size: 143388128,
    ),
  );

  setUp(() {
    calls.clear();
    positions.clear();
    failRead = shortRead = oversizedRead = false;
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, (call) async {
          calls.add(call);
          switch (call.method) {
            case 'open':
              final token = '${++nextToken}';
              positions[token] = call.arguments['start'] as int;
              return token;
            case 'readChunk':
              if (failRead) throw PlatformException(code: 'READ_FAILED');
              if (shortRead) return Uint8List(0);
              final token = call.arguments['token'] as String;
              final max = call.arguments['maxBytes'] as int;
              expect(max, lessThanOrEqualTo(selectedFileChunkBytes));
              final offset = positions[token]!;
              final bytes = Uint8List(max + (oversizedRead ? 1 : 0));
              for (var i = 0; i < bytes.length; i++) {
                bytes[i] = (offset + i) % 251;
              }
              positions[token] = offset + bytes.length;
              return bytes;
            case 'close':
              positions.remove(call.arguments['token']);
              return null;
          }
          throw StateError('Unexpected call ${call.method}');
        });
  });

  tearDown(() {
    expect(positions, isEmpty, reason: 'All native descriptors must be closed');
    TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger
        .setMockMethodCallHandler(channel, null);
  });

  test(
    'OHOS selection returns only metadata and opens no descriptor',
    () async {
      expect(api.response.encode() as List<Object?>, hasLength(4));
      final file = (await FileSelectorOhos(api: api).openFiles()).single;
      expect(file.path, api.response.path);
      expect(file.name, '文档.apk');
      expect(await file.length(), 143388128);
      expect(calls, isEmpty);
      final input = await SelectedFileInput.fromXFile(file);
      await input.close();
      expect(calls, isEmpty);
    },
  );

  test('OHOS direct openRead is bounded and closes on cancellation', () async {
    final file = (await FileSelectorOhos(api: api).openFile())!;
    final iterator = StreamIterator<Uint8List>(file.openRead());
    expect(await iterator.moveNext(), isTrue);
    expect(iterator.current.length, selectedFileChunkBytes);
    expect(calls.map((c) => c.method), ['open', 'readChunk']);
    await iterator.cancel();
    expect(calls.map((c) => c.method), ['open', 'readChunk', 'close']);
  });

  test('OHOS ranged read respects its start and exclusive end', () async {
    final file = (await FileSelectorOhos(api: api).openFile())!;
    final chunks = await file.openRead(107, 113).toList();
    expect(chunks.single, [107, 108, 109, 110, 111, 112]);
    expect(calls[0].arguments['start'], 107);
    expect(calls[1].arguments['maxBytes'], 6);
    expect(calls.last.method, 'close');
    await expectLater(file.openRead(-1, 10).toList(), throwsRangeError);
    expect(await file.openRead(0, 0).toList(), isEmpty);
  });

  for (final failure in ['read error', 'early EOF', 'oversized chunk']) {
    test('OHOS closes URI input after $failure', () async {
      failRead = failure == 'read error';
      shortRead = failure == 'early EOF';
      oversizedRead = failure == 'oversized chunk';
      final file = (await FileSelectorOhos(api: api).openFile())!;
      await expectLater(file.openRead(0, 10).toList(), throwsA(anything));
      expect(calls.last.method, 'close');
    });
  }
}

class _CustomFileAccess extends OperitFolderAccessPlatform {
  final files = <XFile>[];
  @override
  Future<FolderMountSource?> fileSource(XFile file, {String? name}) async {
    files.add(file);
    return FolderMountSource(
      backend: 'custom-resource',
      root: 'authorized-resource',
      namespace: '/virtual/custom/attachments',
      name: name ?? file.name,
    );
  }

  @override
  Future<String?> pickDirectory({String? initialDirectory}) async => null;
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async => [];
}
