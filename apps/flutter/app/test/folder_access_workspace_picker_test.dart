import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_folder_access/operit_folder_access.dart';

void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  const channel = MethodChannel('operit/folder_access');
  final messenger =
      TestDefaultBinaryMessengerBinding.instance.defaultBinaryMessenger;
  final originalFolderAccess = OperitFolderAccessPlatform.instance;
  final originalFileSelector = FileSelectorPlatform.instance;

  tearDown(() {
    OperitFolderAccessPlatform.instance = originalFolderAccess;
    FileSelectorPlatform.instance = originalFileSelector;
    messenger.setMockMethodCallHandler(channel, null);
  });

  test(
    'Android workspace selection retains the opaque authorized tree URI',
    () async {
      final nativePicker = _NativePathPicker();
      FileSelectorPlatform.instance = nativePicker;
      MethodCall? request;
      messenger.setMockMethodCallHandler(channel, (call) async {
        request = call;
        return <String, String>{
          'backend': 'android_documents',
          'root': 'content://com.termux.documents/tree/opaque%2Fproject',
          'namespace': '/virtual/android/documents',
          'name': 'Project',
        };
      });
      OperitFolderAccessAndroid.registerWith();
      expect(
        (await OperitFolderAccess.pickWorkspaceDirectory(
          initialDirectory: 'content://provider/tree/initial',
        ))!.root,
        'content://com.termux.documents/tree/opaque%2Fproject',
      );
      expect(request!.method, 'pickWorkspaceDirectory');
      expect(request!.arguments, {
        'initialDirectory': 'content://provider/tree/initial',
      });
      expect(nativePicker.calls, 0);
    },
  );

  test(
    'mount descriptors preserve backend, display name, and opaque roots',
    () {
      final source = FolderMountSource.fromMap(<String, Object?>{
        'backend': 'android_documents',
        'root': 'content://provider/tree/id%2Fopaque',
        'namespace': '/virtual/android/documents',
        'name': '项目 name',
      });
      expect(source.name, '项目 name');
      expect(
        source.encode(),
        contains('"root":"content://provider/tree/id%2Fopaque"'),
      );
      expect(
        source.encode(),
        contains('"namespace":"/virtual/android/documents"'),
      );
      final native = FolderMountSource.native(r'C:\My Projects\');
      expect(native.name, 'My Projects');
      expect(native.namespace, '/virtual/local/folders');
    },
  );

  test('legacy folder descriptors encode only the canonical namespace', () {
    final source = FolderMountSource.fromMap(<String, Object?>{
      'backend': 'native',
      'root': '/mnt/physical-root',
      'namespace': '/mnt/local/folders',
      'name': 'Legacy',
    });
    expect(source.namespace, '/virtual/local/folders');
    expect(source.root, '/mnt/physical-root');
    expect(source.encode(), contains('"namespace":"/virtual/local/folders"'));
  });

  test('Android storage roots still use the native-path picker', () async {
    final picker = _NativePathPicker();
    FileSelectorPlatform.instance = picker;
    messenger.setMockMethodCallHandler(
      channel,
      (_) async => throw StateError(
        'Workspace picker must not be used for storage roots',
      ),
    );
    OperitFolderAccessAndroid.registerWith();
    expect(
      await OperitFolderAccess.pickDirectory(),
      '/storage/emulated/0/Documents',
    );
    expect(picker.calls, 1);
  });

  test('cancelled workspace selection is null rather than a mount', () async {
    messenger.setMockMethodCallHandler(channel, (_) async => null);
    OperitFolderAccessAndroid.registerWith();
    expect(await OperitFolderAccess.pickWorkspaceDirectory(), isNull);
  });

  test(
    'grant failure is propagated instead of masquerading as cancellation',
    () async {
      messenger.setMockMethodCallHandler(
        channel,
        (_) async => throw PlatformException(
          code: 'TREE_GRANT_FAILED',
          message: 'Access denied',
        ),
      );
      OperitFolderAccessAndroid.registerWith();
      await expectLater(
        OperitFolderAccess.pickWorkspaceDirectory(),
        throwsA(
          isA<PlatformException>().having(
            (e) => e.code,
            'code',
            'TREE_GRANT_FAILED',
          ),
        ),
      );
    },
  );

  test(
    'other platforms retain their own folder authorization implementation',
    () async {
      OperitFolderAccessPlatform.instance = _OtherPlatform();
      expect(
        (await OperitFolderAccess.pickWorkspaceDirectory(
          initialDirectory: '/Volumes',
        ))!.root,
        '/Volumes/Project',
      );
    },
  );

  test(
    'macOS workspace selection retains security-scoped native picking',
    () async {
      messenger.setMockMethodCallHandler(channel, (call) async {
        expect(call.method, 'pickDirectory');
        return '/Users/test/Project';
      });
      OperitFolderAccessMacOS.registerWith();
      expect(
        (await OperitFolderAccess.pickWorkspaceDirectory())!.root,
        '/Users/test/Project',
      );
    },
  );
}

class _NativePathPicker extends FileSelectorPlatform {
  int calls = 0;
  @override
  Future<String?> getDirectoryPath({
    String? initialDirectory,
    String? confirmButtonText,
  }) async {
    calls++;
    return '/storage/emulated/0/Documents';
  }
}

class _OtherPlatform extends OperitFolderAccessPlatform {
  @override
  Future<String?> pickDirectory({String? initialDirectory}) async =>
      '$initialDirectory/Project';
  @override
  Future<List<String>> pickDirectories({String? initialDirectory}) async =>
      const [];
}
