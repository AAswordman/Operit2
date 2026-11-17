import 'dart:ffi';
import 'dart:io';

import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:fvp/src/fvp_platform_interface.dart';
import 'package:fvp/src/video_player_mdk.dart';
import 'package:operit2/ui/theme/ThemeVideoAssetLoader.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:video_player_platform_interface/video_player_platform_interface.dart';

/// Exercises the actual decoder with Windows FVP/MDK DLLs on the loader path.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();
  final enabled = Platform.environment['OPERIT_FVP_NATIVE_TEST'] == '1';
  late VideoPlayerPlatform previousPlayer;
  late FvpPlatform previousTextures;
  late MdkVideoPlayerPlatform platform;
  late _Textures textures;

  setUpAll(() {
    if (!enabled) return;
    // flutter_tester does not register the Windows plugin that enables posts
    // from MDK worker threads to Dart. Invoke that plugin lifecycle explicitly.
    DynamicLibrary.open(
      'fvp.dll',
    ).lookupFunction<Void Function(), void Function()>('MdkCallbacksStartup')();
  });
  tearDownAll(() {
    if (!enabled) return;
    DynamicLibrary.open(
      'fvp.dll',
    ).lookupFunction<Void Function(), void Function()>(
      'MdkCallbacksShutdown',
    )();
  });

  setUp(() {
    previousPlayer = VideoPlayerPlatform.instance;
    previousTextures = FvpPlatform.instance;
    textures = _Textures();
    FvpPlatform.instance = textures;
    platform = MdkVideoPlayerPlatform();
    VideoPlayerPlatform.instance = platform;
  });
  tearDown(() {
    VideoPlayerPlatform.instance = previousPlayer;
    FvpPlatform.instance = previousTextures;
  });

  test(
    'native decoder opens the same data URL used by video backgrounds',
    () async {
      final request = ThemeVideoAssetLoader.fromBytes(
        bytes: await File('test/fixtures/background_video.mp4').readAsBytes(),
        reference: 'background_video.mp4',
      );
      try {
        final controller = await request.initialize();
        expect(controller.value.isInitialized, isTrue);
        expect(controller.value.size.width, 32);
        expect(controller.value.size.height, 32);
        expect(controller.value.volume, 0);
        expect(controller.value.isLooping, isTrue);
      } finally {
        await request.dispose(throwOnFailure: true);
      }
      expect(textures.active, isEmpty);
    },
    skip: !enabled,
  );

  test('native invalid media fails locally and releases the player', () async {
    final request = ThemeVideoAssetLoader.fromBytes(
      bytes: Uint8List.fromList([1, 2, 3]),
      reference: 'invalid.mp4',
    );
    try {
      await expectLater(
        request.initialize(),
        throwsA(
          isA<ThemeAssetFailure>().having(
            (failure) => (failure.cause as PlatformException).code,
            'native error',
            'media open error',
          ),
        ),
      );
    } finally {
      await request.dispose();
    }
    expect(textures.active, isEmpty);
  }, skip: !enabled);

  test(
    'failed native creations retain separate IDs until subscription and cleanup',
    () async {
      final data = DataSource(
        sourceType: DataSourceType.network,
        uri: Uri.dataFromBytes([1, 2, 3], mimeType: 'video/mp4').toString(),
      );
      final ids = <int>[];
      try {
        ids.add((await platform.create(data))!);
        ids.add((await platform.create(data))!);
        expect(ids[0], isNot(ids[1]));
        // A delayed controller subscription must not emit an unhandled Zone error.
        await Future<void>.delayed(Duration.zero);
        for (final id in ids) {
          await expectLater(
            platform.videoEventsFor(id),
            emitsError(
              isA<PlatformException>().having(
                (error) => error.code,
                'code',
                'media open error',
              ),
            ),
          );
        }
      } finally {
        for (final id in ids) {
          await platform.dispose(id);
        }
      }
    },
    skip: !enabled,
  );
}

/// Replaces only texture allocation; media loading and decoding use real MDK.
class _Textures extends FvpPlatform {
  final active = <int>{};
  int _next = 1;

  /// Records a texture allocation after MDK has obtained valid video geometry.
  @override
  Future<int> createTexture(
    int playerHandle,
    int width,
    int height,
    bool tunnel,
  ) async {
    final id = _next++;
    active.add(id);
    return id;
  }

  /// Records native player teardown without requesting a GPU surface.
  @override
  Future<void> releaseTexture(int playerHandle, int textureId) async {
    active.remove(textureId);
  }

  /// Avoids system audio configuration in decoder checks.
  @override
  Future<void> setMixWithOthers(bool mixWithOthers) async {}
}
