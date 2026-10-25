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

/// Exercises the shared Rust owner protocol and real iOS adapters at their native channels.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
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
                  await ComposeDslFilePickerService.open(jsonEncode(request)),
                )
                as Map<String, dynamic>;
        expect(result['cancelled'], false);
        final selected = (result['files'] as List).single as Map;
        if (request['picker'] == 'directory') {
          expect(selected['uri'], 'file://selected/folder');
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
        jsonDecode(await ComposeDslFilePickerService.open(jsonEncode(avatar)))
            as Map;
    final selected = (result['files'] as List).single as Map;
    expect(base64Decode(selected['contentBase64'] as String), [0, 1, 255]);
    expect(selected['uri'], file.uri.toString());
    expect(native.photoCalls, 1);
    expect(native.documentCalls, 0);
  });

  test('iOS Photos cancellation completes with no Files picker', () async {
    final native = _IOSNativePicker();
    addTearDown(native.dispose);
    FileSelectorIOS.registerWith();
    final result = jsonDecode(
      await ComposeDslFilePickerService.open(jsonEncode(avatar)),
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
      ComposeDslFilePickerService.open(jsonEncode(avatar)),
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
    test('selected URI survives serialization: $path', () async {
      FileSelectorPlatform.instance = _ContractSelector(path: path);
      final result =
          jsonDecode(await ComposeDslFilePickerService.open(jsonEncode(avatar)))
              as Map;
      expect((result['files'] as List).single['uri'], path);
    });
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
    return XFile.fromData(
      Uint8List.fromList([0, 1, 255]),
      path: path,
      name: 'avatar.png',
    );
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

/// Supplies a host-owned directory URI without changing the file-selection path.
class _ContractFolders extends OperitFolderAccessPlatform {
  /// Returns the selected directory reference unchanged.
  @override
  Future<String?> pickDirectory({String? initialDirectory}) async =>
      'file://selected/folder';

  /// Returns the selected directory references unchanged.
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async => [
    'file://selected/folder',
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
      throw StateError('Avatar selection must use iOS Photos');
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

  /// Releases the native channel handlers after each adapter check.
  void dispose() {
    messenger.setMockDecodedMessageHandler<Object?>(photoChannel, null);
    messenger.setMockDecodedMessageHandler<Object?>(documentChannel, null);
  }
}
