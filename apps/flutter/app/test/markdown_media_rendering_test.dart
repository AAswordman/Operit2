import 'dart:async';
import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/ui/common/markdown/MarkdownFileScope.dart';
import 'package:operit2/ui/common/markdown/MarkdownImageRenderer.dart';
import 'package:operit2/ui/common/markdown/MarkdownVideoRenderer.dart';
import 'package:operit2/ui/features/chat/components/attachments/ChatMessageImageAttachments.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:video_player/video_player.dart';
import 'package:video_player_platform_interface/video_player_platform_interface.dart';

final _heic = File(
  '../../../core/crates/runtime/application/tests/fixtures/heif-checker-64.heic',
).readAsBytesSync();
final _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
);

/// Exercises file routing and the platform player contract used by Markdown.
void main() {
  test('HEIF and HEIC are images in attachment and workspace presentation', () {
    for (final extension in ['heic', 'HEIF']) {
      expect(
        AttachmentData(
          id: 'photo',
          filename: 'photo.$extension',
          type: 'application/octet-stream',
        ).isImage,
        isTrue,
      );
      expect(
        workspacePreviewKindForPath('photo.$extension'),
        WorkspaceFilePreviewKind.image,
      );
    }
  });

  testWidgets(
    'Markdown VFS HEIC images decode through the owning runtime and cache pixels',
    (tester) async {
      final bridge = _MediaBridge();
      final clients = GeneratedCoreProxyClients(bridge);
      Future<void> render(double width) => tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: SizedBox(
              width: width,
              child: MarkdownFileScope(
                basePath: '/operitapp/outputs/session',
                nodeId: null,
                clients: clients,
                child: const MarkdownImageRenderer(
                  imageMarkdown: '![HEIC](./photo.heic)',
                  textColor: Colors.black,
                ),
              ),
            ),
          ),
        ),
      );
      await render(500);
      await tester.pumpAndSettle();
      await tester.runAsync(
        () => precacheImage(
          tester.widget<Image>(find.byType(Image)).image,
          tester.element(find.byType(Image)),
        ),
      );
      await tester.pumpAndSettle();
      expect(bridge.paths, ['/operitapp/outputs/session/photo.heic']);
      expect(bridge.decodes, 1);
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
      await render(300);
      await tester.pumpAndSettle();
      expect(bridge.decodes, 1);
      expect(find.byIcon(Icons.broken_image_outlined), findsNothing);
      expect(tester.takeException(), isNull);
    },
  );

  final originalPlatform = VideoPlayerPlatform.instance;
  late _VideoPlatform player;
  setUp(() {
    player = _VideoPlatform();
    VideoPlayerPlatform.instance = player;
  });
  tearDown(() => VideoPlayerPlatform.instance = originalPlatform);

  for (final source in [
    'clip.mp4',
    'https://example.com/clip.MP4?download=1',
    '<https://example.com/clip.mp4>',
  ]) {
    testWidgets('image syntax displays a playable video: $source', (
      tester,
    ) async {
      final bridge = _MediaBridge();
      await tester.pumpWidget(
        MaterialApp(
          home: Scaffold(
            body: MarkdownFileScope(
              basePath: '/operitapp/outputs/session',
              nodeId: null,
              clients: GeneratedCoreProxyClients(bridge),
              child: MarkdownImageRenderer(
                imageMarkdown: '![视频]($source)',
                textColor: Colors.black,
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      expect(find.byType(MarkdownVideoRenderer), findsOneWidget);
      expect(find.byType(VideoPlayer), findsOneWidget);
      if (source == 'clip.mp4') {
        expect(bridge.paths, ['/operitapp/outputs/session/clip.mp4']);
        final data = Uri.parse(player.sources.single.uri!).data!;
        expect(data.mimeType, 'video/mp4');
        expect(data.contentAsBytes(), _heic);
      } else {
        expect(bridge.paths, isEmpty);
        expect(
          player.sources.single.uri,
          extractMarkdownImageUrl('![]($source)'),
        );
      }
      await tester.tap(find.byIcon(Icons.play_arrow));
      await tester.pump();
      expect(find.byIcon(Icons.pause), findsOneWidget);
      await tester.tap(find.byIcon(Icons.pause));
      await tester.pump();
      expect(find.byIcon(Icons.play_arrow), findsOneWidget);
      await tester.pumpWidget(const SizedBox());
      await tester.pumpAndSettle();
      await tester.runAsync(
        () => player.released.future.timeout(const Duration(seconds: 2)),
      );
      expect(player.disposed, [1]);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('disposing Markdown while a file is loading creates no player', (
    tester,
  ) async {
    final bridge = _MediaBridge()..pendingRead = Completer<void>();
    await tester.pumpWidget(
      MaterialApp(
        home: MarkdownFileScope(
          basePath: '/operitapp/outputs',
          nodeId: null,
          clients: GeneratedCoreProxyClients(bridge),
          child: const MarkdownImageRenderer(
            imageMarkdown: '![](clip.mp4)',
            textColor: Colors.black,
          ),
        ),
      ),
    );
    await tester.pump();
    await tester.pumpWidget(const SizedBox());
    bridge.pendingRead!.complete();
    await tester.pumpAndSettle();
    expect(player.sources, isEmpty);
    expect(tester.takeException(), isNull);
  });
}

/// Serves VFS bytes and the PNG result returned by the runtime decoder.
class _MediaBridge extends OperitRuntimeBridge {
  final paths = <String>[];
  var decodes = 0;
  Completer<void>? pendingRead;

  /// Verifies actual proxy method names and payloads from the production widgets.
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    expect(request.methodName, 'executeTool');
    final tool = (request.args as Map)['tool'] as Map;
    expect(tool['name'], 'read_file_binary');
    final path =
        ((tool['parameters'] as List).single as Map)['value'] as String;
    paths.add(path);
    await pendingRead?.future;
    return encodeCoreLink([
      0,
      {
        'toolName': 'read_file_binary',
        'success': true,
        'error': null,
        'result': {
          '__type': 'BinaryFileContentData',
          'path': path,
          'contentBase64': base64Encode(_heic),
          'size': _heic.length,
        },
      },
    ]);
  }

  /// Serves HEIF display bytes over the existing runtime image stream contract.
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) async* {
    expect(request.propertyName, 'imageBytes');
    final source = Uri.parse((request.args as Map)['url'] as String).data!;
    expect(source.mimeType, 'image/heic');
    expect(source.contentAsBytes(), _heic);
    decodes++;
    yield CoreEvent.raw(
      requestId: request.requestId,
      target: request.target,
      propertyName: request.propertyName,
      kind: 'Changed',
      valueBytes: encodeCoreLink({
        'bytes': _png,
        'displayUrl': null,
        'error': null,
      }),
      decodeValue: (bytes) => decodeCoreLink<Object?>(bytes),
    );
  }

  /// Rejects push operations outside this fixture's contract.
  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();

  /// Rejects snapshots outside this fixture's contract.
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
}

/// Emulates player initialization and records source and disposal ownership.
class _VideoPlatform extends VideoPlayerPlatform {
  final sources = <DataSource>[];
  final disposed = <int>[];
  final released = Completer<void>();

  /// Initializes the fake player backend.
  @override
  Future<void> init() async {}

  /// Records each source handed to the platform player.
  @override
  Future<int?> createWithOptions(VideoCreationOptions options) async {
    sources.add(options.dataSource);
    return sources.length;
  }

  /// Emits dimensions so the real video controller can finish initialization.
  @override
  Stream<VideoEvent> videoEventsFor(int playerId) => Stream.value(
    VideoEvent(
      eventType: VideoEventType.initialized,
      duration: const Duration(seconds: 2),
      size: const Size(320, 180),
    ),
  );

  /// Records released players.
  @override
  Future<void> dispose(int playerId) async {
    disposed.add(playerId);
    if (!released.isCompleted) released.complete();
  }

  /// Accepts looping configuration from the controller.
  @override
  Future<void> setLooping(int playerId, bool looping) async {}

  /// Accepts playback commands from the widget.
  @override
  Future<void> play(int playerId) async {}

  /// Accepts pause commands from the widget.
  @override
  Future<void> pause(int playerId) async {}

  /// Accepts initial volume configuration.
  @override
  Future<void> setVolume(int playerId, double volume) async {}

  /// Accepts playback speed configuration.
  @override
  Future<void> setPlaybackSpeed(int playerId, double speed) async {}

  /// Returns a stable test position during playback.
  @override
  Future<Duration> getPosition(int playerId) async => Duration.zero;

  /// Renders a platform surface without requiring a native texture.
  @override
  Widget buildViewWithOptions(VideoViewOptions options) => const SizedBox();
}
