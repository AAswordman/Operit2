import 'dart:convert';
import 'dart:io';

import 'package:file_selector_ios/file_selector_ios.dart';
import 'package:file_selector_ios/src/messages.g.dart' as documents;
import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:image_picker_ios/src/messages.g.dart' as photos;
import 'package:operit_folder_access/operit_folder_access.dart';
import 'package:operit2/core/host/ComposeDslFilePickerService.dart';

import 'support/MemoryFilePickerStorage.dart';

/// Exercises the shared Rust owner protocol and real iOS adapters at their native channels.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  late MemoryFilePickerStorage storage;
  setUp(() => storage = MemoryFilePickerStorage());
  final selector = FileSelectorPlatform.instance;
  final folders = OperitFolderAccessPlatform.instance;
  final contract =
      jsonDecode(
            File(
              '../../../core/crates/foundation/host-api/tests/fixtures/compose_file_picker.json',
            ).readAsStringSync(),
          )
          as Map<String, dynamic>;
  final cases = (contract['valid'] as List).cast<Map<String, dynamic>>();
  final avatar =
      cases.singleWhere(
            (entry) => entry['name'] == 'bounded character avatar',
          )['ownerRequest']
          as Map<String, dynamic>;
  tearDown(() {
    FileSelectorPlatform.instance = selector;
    OperitFolderAccessPlatform.instance = folders;
  });

  for (final entry in cases) {
    test(
      'Flutter accepts serialized Rust owner request: ${entry['name']}',
      () async {
        final files = _ContractSelector();
        FileSelectorPlatform.instance = files;
        OperitFolderAccessPlatform.instance = _ContractFolders();
        final request = entry['ownerRequest'] as Map<String, dynamic>;
        final result =
            jsonDecode(
                  await ComposeDslFilePickerService.open(
                    jsonEncode(request),
                    storage: storage,
                  ),
                )
                as Map<String, dynamic>;
        expect(result['cancelled'], false);
        final selected = (result['files'] as List).single as Map;
        if (request['picker'] == 'directory') {
          expect(selected['path'], '/mnt/local/folders/selected-1');
          expect(selected['uri'], selected['path']);
          expect(storage.directories.single.root, '/selected/folder');
          expect(files.calls, 0);
        } else {
          expect(files.calls, 1);
          expect(files.multiple, request['allowMultiple']);
          for (final group in files.filters) {
            expect(
              group.extensions,
              isNotEmpty,
              reason: 'Windows and OHOS require extension filters',
            );
            expect(
              group.mimeTypes,
              isNotEmpty,
              reason: 'Android and Linux consume MIME filters',
            );
            expect(
              group.webWildCards,
              isNotEmpty,
              reason: 'Web consumes accept filters',
            );
          }
          if (request['maxContentBytes'] != null) {
            expect(base64Decode(selected['contentBase64'] as String), [
              0,
              1,
              255,
            ]);
          } else {
            expect(selected, isNot(contains('contentBase64')));
          }
        }
      },
    );
  }

  test('directory selection retains the authorized folder backend', () async {
    final messenger =
        TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
    const channel = MethodChannel('operit/folder_access');
    messenger.setMockMethodCallHandler(channel, (call) async {
      expect(call.method, 'pickWorkspaceDirectory');
      return {
        'backend': 'android_documents',
        'root': 'content://documents/tree/opaque',
        'namespace': '/mnt/android/documents',
        'name': 'Documents',
      };
    });
    addTearDown(() => messenger.setMockMethodCallHandler(channel, null));
    OperitFolderAccessAndroid.registerWith();
    final result =
        jsonDecode(
              await ComposeDslFilePickerService.open(
                jsonEncode({
                  'picker': 'directory',
                  'allowMultiple': true,
                  'mimeTypes': [],
                }),
                storage: storage,
              ),
            )
            as Map;
    final selected = (result['files'] as List).single as Map;
    expect(selected['path'], '/mnt/android/documents/selected-1');
    expect(selected['uri'], selected['path']);
    expect(storage.directories.single.backend, 'android_documents');
    expect(storage.directories.single.root, 'content://documents/tree/opaque');
  });

  test('iOS Photos returns bytes through both real Dart adapters', () async {
    final native = _IOSNativePicker();
    addTearDown(native.dispose);
    FileSelectorIOS.registerWith();
    final directory = await Directory.systemTemp.createTemp('compose-picker-');
    final file = await File(
      '${directory.path}/avatar.png',
    ).writeAsBytes([0, 1, 255]);
    addTearDown(() async {
      await file.delete();
      await directory.delete();
    });
    native.reply = [file.path];
    final result =
        jsonDecode(
              await ComposeDslFilePickerService.open(
                jsonEncode(avatar),
                storage: storage,
              ),
            )
            as Map;
    final selected = (result['files'] as List).single as Map;
    expect(base64Decode(selected['contentBase64'] as String), [0, 1, 255]);
    expect(selected['uri'], storage.allocated.single);
    expect(storage.files[selected['path']], [0, 1, 255]);
    expect(native.photoCalls, 1);
    expect(native.documentCalls, 0);
  });

  test(
    'iOS document selection publishes JSON without requesting inline content',
    () async {
      final native = _IOSNativePicker()..documentsAllowed = true;
      addTearDown(native.dispose);
      FileSelectorIOS.registerWith();
      final directory = await Directory.systemTemp.createTemp(
        'compose-document-',
      );
      final file = await File(
        '${directory.path}/Reborn2.4.2.json',
      ).writeAsString('{"entries":{}}');
      addTearDown(() async {
        await file.delete();
        await directory.delete();
      });
      native.reply = [
        [file.path],
      ];
      final result =
          jsonDecode(
                await ComposeDslFilePickerService.open(
                  jsonEncode({
                    'picker': 'document',
                    'allowMultiple': false,
                    'mimeTypes': [],
                  }),
                  storage: storage,
                ),
              )
              as Map;
      final selected = (result['files'] as List).single as Map;
      expect(selected['path'], storage.allocated.single);
      expect(selected['uri'], selected['path']);
      expect(selected, isNot(contains('contentBase64')));
      expect(utf8.decode(storage.files[selected['path']]!), '{"entries":{}}');
      expect(native.documentCalls, 1);
      expect(native.photoCalls, 0);
    },
  );

  test('iOS Photos cancellation completes with no Files picker', () async {
    final native = _IOSNativePicker();
    addTearDown(native.dispose);
    FileSelectorIOS.registerWith();
    final result = jsonDecode(
      await ComposeDslFilePickerService.open(
        jsonEncode(avatar),
        storage: storage,
      ),
    );
    expect(result, {'cancelled': true, 'files': []});
    expect(native.photoCalls, 1);
    expect(native.documentCalls, 0);
  });

  test('iOS Photos errors remain errors through the owner service', () async {
    final native = _IOSNativePicker()
      ..reply = ['PICK_FAILED', 'Native photo selection failed', null];
    addTearDown(native.dispose);
    FileSelectorIOS.registerWith();
    await expectLater(
      ComposeDslFilePickerService.open(jsonEncode(avatar), storage: storage),
      throwsA(isA<PlatformException>()),
    );
    expect(native.photoCalls, 1);
    expect(native.documentCalls, 0);
  });

  for (final path in [
    'blob:https://app.example/selected',
    'content://media/selected',
    'file://media/selected',
  ]) {
    test(
      'selected source locator stays behind the VFS boundary: $path',
      () async {
        FileSelectorPlatform.instance = _ContractSelector(path: path);
        final result =
            jsonDecode(
                  await ComposeDslFilePickerService.open(
                    jsonEncode(avatar),
                    storage: storage,
                  ),
                )
                as Map;
        final selected = (result['files'] as List).single as Map;
        expect(selected['uri'], storage.allocated.single);
        expect(selected['path'], selected['uri']);
        expect(storage.files[selected['path']], [0, 1, 255]);
      },
    );
  }
}

/// Records the complete filter descriptor without choosing a platform in application code.
class _ContractSelector extends FileSelectorPlatform {
  /// Selects one metadata and byte source for the contract cases.
  _ContractSelector({this.path = '/selected/avatar.png'});
  final String path;
  int calls = 0;
  bool multiple = false;
  List<XTypeGroup> filters = [];

  /// Records selection using the same XFile abstraction as native and web hosts.
  @override
  Future<XFile?> openFile({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    calls++;
    filters = acceptedTypeGroups!;
    return _ContractFile(path);
  }

  /// Records the multi-file flag while retaining the same filter contract.
  @override
  Future<List<XFile>> openFiles({
    List<XTypeGroup>? acceptedTypeGroups,
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    multiple = true;
    return [(await openFile(acceptedTypeGroups: acceptedTypeGroups))!];
  }
}

/// Preserves provider display metadata independently of the test runner's path syntax.
class _ContractFile extends XFile {
  /// Provides readable bytes and an opaque source locator to the picker.
  _ContractFile(String path)
    : super.fromData(
        Uint8List.fromList([0, 1, 255]),
        path: path,
        name: 'avatar.png',
      );

  /// Reports the provider name instead of interpreting a foreign URI as a local filename.
  @override
  String get name => 'avatar.png';
}

/// Supplies a host-owned directory URI without changing the file-selection path.
class _ContractFolders extends OperitFolderAccessPlatform {
  /// Returns the selected directory reference unchanged.
  @override
  Future<String?> pickDirectory({String? initialDirectory}) async =>
      '/selected/folder';

  /// Returns the selected directory references unchanged.
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async => [
    '/selected/folder',
  ];
}

/// Replaces only native channels; FileSelectorIOS and ImagePickerIOS run production code.
class _IOSNativePicker {
  /// Installs responses at the Pigeon channels used by the iOS plugins.
  _IOSNativePicker() {
    messenger.setMockDecodedMessageHandler<Object?>(photoChannel, (
      message,
    ) async {
      photoCalls++;
      final args = message! as List;
      expect(
        (args[0] as photos.SourceSpecification).type,
        photos.SourceType.gallery,
      );
      expect(args[3], false);
      return reply;
    });
    messenger.setMockDecodedMessageHandler<Object?>(documentChannel, (
      message,
    ) async {
      documentCalls++;
      if (!documentsAllowed) {
        throw StateError('Avatar selection must use iOS Photos');
      }
      return reply;
    });
  }
  static const photoChannel = BasicMessageChannel<Object?>(
    'dev.flutter.pigeon.image_picker_ios.ImagePickerApi.pickImage',
    photos.ImagePickerApi.pigeonChannelCodec,
  );
  static const documentChannel = BasicMessageChannel<Object?>(
    'dev.flutter.pigeon.file_selector_ios.FileSelectorApi.openFile',
    documents.FileSelectorApi.pigeonChannelCodec,
  );
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  List<Object?> reply = [null];
  int photoCalls = 0;
  int documentCalls = 0;
  bool documentsAllowed = false;

  /// Releases the native channel handlers after each adapter check.
  void dispose() {
    messenger.setMockDecodedMessageHandler<Object?>(photoChannel, null);
    messenger.setMockDecodedMessageHandler<Object?>(documentChannel, null);
  }
}
