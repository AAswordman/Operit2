import 'dart:convert';

import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/ComposeDslFileSaveService.dart';

/// Verifies the Compose export owner against the registered file-selector boundary.
void main() {
  final original = FileSelectorPlatform.instance;
  const content = '{"name":"角色🌍","setting":"第一行\\n第二行"}\n';
  final request = jsonEncode(<String, Object?>{
    'routeInstanceId': 'character-route',
    'executionContextKey': 'character-owner',
    'options': <String, Object?>{
      'suggestedName': '角色-operit.json',
      'content': content,
      'mimeType': 'application/json',
      'extensions': <String>['json'],
    },
  });

  /// Restores the application file-selector registration after each test.
  tearDown(() => FileSelectorPlatform.instance = original);

  /// Preserves exact UTF-8 bytes, file name, MIME type, and destination selected by the host.
  test(
    'Compose export saves exact UTF-8 through the registered selector',
    () async {
      final platform = _ExportPlatform('/chosen/角色.json');
      FileSelectorPlatform.instance = platform;
      final result = jsonDecode(await ComposeDslFileSaveService.save(request));
      expect(result, <String, Object?>{
        'cancelled': false,
        'path': '/chosen/角色.json',
      });
      expect(platform.bytes, utf8.encode(content));
      expect(platform.mimeType, 'application/json');
      expect(platform.options!.suggestedName, '角色-operit.json');
      expect(platform.groups!.single.extensions, <String>['json']);
      expect(platform.groups!.single.mimeTypes, <String>['application/json']);
      expect(platform.calls, 1);
    },
  );

  /// Reports a cancelled selector without declaring a successful export.
  test('Compose export cancellation remains explicit', () async {
    final platform = _ExportPlatform(null);
    FileSelectorPlatform.instance = platform;
    final result = jsonDecode(await ComposeDslFileSaveService.save(request));
    expect(result, <String, Object?>{'cancelled': true, 'path': null});
    expect(platform.calls, 1);
  });

  /// Propagates the original host error without a second save attempt.
  test('Compose export preserves host save errors', () async {
    final error = StateError('Host save denied');
    final platform = _ExportPlatform('/chosen/角色.json', error: error);
    FileSelectorPlatform.instance = platform;
    await expectLater(
      ComposeDslFileSaveService.save(request),
      throwsA(same(error)),
    );
    expect(platform.calls, 1);
  });
}

/// Captures only the external file-selector boundary used by the production save service.
class _ExportPlatform extends FileSelectorPlatform {
  /// Supplies an explicit destination, cancellation, or host write error.
  _ExportPlatform(this.destination, {this.error});

  final String? destination;
  final Object? error;
  int calls = 0;
  List<int>? bytes;
  String? mimeType;
  SaveDialogOptions? options;
  List<XTypeGroup>? groups;

  /// Observes the exact payload and responds only with the configured host outcome.
  @override
  Future<FileSaveLocation?> saveFile({
    required XFile file,
    List<XTypeGroup>? acceptedTypeGroups,
    SaveDialogOptions options = const SaveDialogOptions(),
  }) async {
    calls++;
    this.options = options;
    groups = acceptedTypeGroups;
    mimeType = file.mimeType;
    bytes = await file.readAsBytes();
    if (error != null) throw error!;
    return destination == null ? null : FileSaveLocation(destination!);
  }
}
