import 'dart:convert';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';

import 'package:operit2/core/host/ComposeDslFilePickerService.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/folder_access');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  late OperitFolderAccessPlatform original;
  final calls = <MethodCall>[];
  Object? response;
  PlatformException? error;

  setUp(() {
    original = OperitFolderAccessPlatform.instance;
    OperitFolderAccessMacOS.registerWith();
    calls.clear();
    response = null;
    error = null;
    messenger.setMockMethodCallHandler(channel, (call) async {
      calls.add(call);
      if (error != null) throw error!;
      return response;
    });
  });

  tearDown(() {
    OperitFolderAccessPlatform.instance = original;
    messenger.setMockMethodCallHandler(channel, null);
  });

  Future<Map<String, dynamic>> open({required bool multiple}) async {
    return jsonDecode(
          await ComposeDslFilePickerService.open(
            jsonEncode({
              'picker': 'directory',
              'allowMultiple': multiple,
              'mimeTypes': <String>[],
            }),
          ),
        )
        as Map<String, dynamic>;
  }

  test(
    'default delegate routes both selections through file_selector',
    () async {
      final selector = FileSelectorPlatform.instance;
      final fake = _FolderSelector();
      FileSelectorPlatform.instance = fake;
      OperitFolderAccessPlatform.instance = original;
      try {
        expect(
          await OperitFolderAccess.pickDirectory(initialDirectory: '/start'),
          '/one',
        );
        expect(
          await OperitFolderAccess.pickDirectories(initialDirectory: '/start'),
          ['/one', '/two'],
        );
        expect(fake.initialDirectories, ['/start', '/start']);
        expect(calls, isEmpty);
      } finally {
        FileSelectorPlatform.instance = selector;
      }
    },
  );

  test(
    'single directory uses unified package and preserves result shape',
    () async {
      response = '/tmp/folder';
      final result = await open(multiple: false);
      expect(calls.single.method, 'pickDirectory');
      expect(result['cancelled'], false);
      expect((result['files'] as List).single['path'], '/tmp/folder');
    },
  );

  test('multiple directories use unified package', () async {
    response = ['/tmp/one', '/tmp/two'];
    final result = await open(multiple: true);
    expect(calls.single.method, 'pickDirectories');
    expect(result['cancelled'], false);
    expect((result['files'] as List).map((f) => f['path']), response);
  });

  test('single cancellation remains cancelled', () async {
    final result = await open(multiple: false);
    expect(result, {'cancelled': true, 'files': []});
  });

  test('multiple cancellation remains cancelled', () async {
    response = <String>[];
    expect(await open(multiple: true), {'cancelled': true, 'files': []});
  });

  test('permission failures are not disguised as cancellation', () async {
    error = PlatformException(code: 'FOLDER_ACCESS_ERROR');
    for (final multiple in [false, true]) {
      await expectLater(
        open(multiple: multiple),
        throwsA(isA<PlatformException>()),
      );
    }
  });

  test('both APIs forward initial directory', () async {
    await OperitFolderAccess.pickDirectory(initialDirectory: '/tmp/start');
    response = <String>[];
    await OperitFolderAccess.pickDirectories(initialDirectory: '/tmp/start');
    expect(calls.map((c) => c.arguments), [
      {'initialDirectory': '/tmp/start'},
      {'initialDirectory': '/tmp/start'},
    ]);
  });
}

class _FolderSelector extends FileSelectorPlatform {
  final initialDirectories = <String?>[];

  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    initialDirectories.add(initialDirectory);
    return '/one';
  }

  @override
  Future<List<String>> getDirectoryPaths({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    initialDirectories.add(initialDirectory);
    return ['/one', '/two'];
  }
}
