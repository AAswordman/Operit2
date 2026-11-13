import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

/// Verifies that page navigation and effect edits retain loaded background media.
void main() {
  testWidgets(
    'cached backgrounds are present on the first frame after remount',
    (tester) async {
      final resolver = _CountingAssets();
      await tester.pumpWidget(_scene(resolver));
      await tester.runAsync(() async {
        await Future<void>.delayed(const Duration(milliseconds: 50));
      });
      await tester.pumpAndSettle();
      expect(resolver.reads, 1);
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpWidget(_scene(resolver));
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
      expect(resolver.reads, 1);

      final otherOwner = _CountingAssets();
      await tester.pumpWidget(_scene(otherOwner));
      await tester.runAsync(() async {
        await Future<void>.delayed(const Duration(milliseconds: 50));
      });
      await tester.pumpAndSettle();
      expect(otherOwner.reads, 1);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
      PaintingBinding.instance.imageCache.clear();
    },
  );

  for (final platform in TargetPlatform.values) {
    testWidgets(
      'shared background remains visible during $platform navigation',
      (tester) async {
        final boundary = GlobalKey();
        final navigator = GlobalKey<NavigatorState>();
        final resolver = _CountingAssets();
        final snapshot = ThemePreferenceSnapshot.fromJson({
          ...ThemePreferenceDefaults.snapshot.toJson(),
          'useBackgroundImage': true,
          'backgroundImageUri': 'background.svg',
        });
        await tester.pumpWidget(
          RepaintBoundary(
            key: boundary,
            child: MaterialApp(
              navigatorKey: navigator,
              theme: buildOperitThemeData(
                ColorScheme.fromSeed(seedColor: Colors.blue),
                snapshot,
                fontFamily: 'sans-serif',
                fontFamilyFallback: const [],
              ).copyWith(platform: platform),
              builder: (context, child) => ThemeBackground(
                themePreferenceSnapshot: snapshot,
                media: ResolvedThemeImage(
                  resolver: resolver,
                  reference: 'background.svg',
                  fit: BoxFit.cover,
                ),
                child: child!,
              ),
              home: const Scaffold(body: Text('Theme library')),
            ),
          ),
        );
        await tester.runAsync(() async {
          await Future<void>.delayed(const Duration(milliseconds: 50));
        });
        await tester.pumpAndSettle();
        final backgroundState = tester.state(find.byType(ResolvedThemeImage));
        await _expectBackground(tester, boundary, 'before navigation');
        navigator.currentState!.push<void>(
          MaterialPageRoute<void>(
            builder: (_) => const Scaffold(body: Text('Theme editor')),
          ),
        );
        await tester.pump();
        for (var frame = 0; frame < 32; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          await _expectBackground(tester, boundary, 'push frame $frame');
        }
        await tester.pumpAndSettle();
        final editorContext = tester.element(find.text('Theme editor'));
        expect(Theme.of(editorContext).colorScheme.surface.a, 1);
        if (platform == TargetPlatform.android) {
          final route = ModalRoute.of(editorContext)!;
          expect(
            await _backGesture(tester, 'startBackGesture', {
              'touchOffset': [5.0, 300.0],
              'progress': 0.0,
              'swipeEdge': 0,
            }),
            isTrue,
          );
          await tester.pump();
          expect(route.popGestureInProgress, isTrue);
          await _backGesture(tester, 'updateBackGestureProgress', {
            'touchOffset': [100.0, 300.0],
            'progress': 0.35,
            'swipeEdge': 0,
          });
          await tester.pumpAndSettle();
          await _expectBackground(tester, boundary, 'predictive back');
          await _backGesture(tester, 'cancelBackGesture');
          await tester.pumpAndSettle();
          expect(route.popGestureInProgress, isFalse);
          expect(find.text('Theme editor'), findsOneWidget);
        }
        navigator.currentState!.pop();
        await tester.pump();
        for (var frame = 0; frame < 32; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          await _expectBackground(tester, boundary, 'pop frame $frame');
        }
        await tester.pumpAndSettle();
        expect(
          tester.state(find.byType(ResolvedThemeImage)),
          same(backgroundState),
        );
        expect(resolver.reads, 1);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
        PaintingBinding.instance.imageCache.clear();
      },
    );
  }

  testWidgets('blur changes keep the same media state', (tester) async {
    /// Changes the actual shared background effect wrapper around the media.
    Widget scene(bool blur) => MaterialApp(
      home: ThemeBackgroundMedia(
        opacity: 1,
        blurEnabled: blur,
        blurRadius: 12,
        child: _MediaProbe(),
      ),
    );

    await tester.pumpWidget(scene(false));
    final state = tester.state(find.byType(_MediaProbe));
    await tester.pumpWidget(scene(true));
    expect(tester.state(find.byType(_MediaProbe)), same(state));
    await tester.pumpWidget(scene(false));
    expect(tester.state(find.byType(_MediaProbe)), same(state));
    expect(tester.takeException(), isNull);
  });
}

/// Mounts the same asset reference as a newly entered background preview.
Widget _scene(ThemeAssetResolver resolver) => MaterialApp(
  home: ResolvedThemeImage(
    resolver: resolver,
    reference: 'background.svg',
    fit: BoxFit.cover,
  ),
);

class _CountingAssets implements ThemeAssetResolver {
  int reads = 0;

  /// Counts storage access while returning a tiny valid static background.
  @override
  Future<Uint8List> readBytes(String reference) async {
    reads++;
    return Uint8List.fromList(
      '<svg xmlns="http://www.w3.org/2000/svg" width="8" height="8">'
              '<rect width="8" height="8" fill="#339966"/></svg>'
          .codeUnits,
    );
  }
}

class _MediaProbe extends StatefulWidget {
  /// Creates a media lifecycle probe independent of image cache behavior.
  const _MediaProbe();

  /// Creates the state whose identity must survive effect changes.
  @override
  State<_MediaProbe> createState() => _MediaProbeState();
}

class _MediaProbeState extends State<_MediaProbe> {
  /// Provides paintable media without a platform video controller.
  @override
  Widget build(BuildContext context) => const ColoredBox(color: Colors.green);
}

/// Samples an empty area below the controls while route transitions are active.
Future<void> _expectBackground(
  WidgetTester tester,
  GlobalKey boundary,
  String frame,
) async {
  final render =
      boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final pixels = (await tester.runAsync(() async {
    final image = await render.toImage(pixelRatio: 1);
    try {
      final data = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final offset = ((image.height - 20) * image.width + image.width ~/ 2) * 4;
      return data.buffer.asUint8List(data.offsetInBytes + offset, 4).toList();
    } finally {
      image.dispose();
    }
  }))!;
  // Native shadows and Zoom's scrim may dim the image. A solid transition
  // surface instead raises red/blue and washes out the background.
  expect(pixels[0], lessThanOrEqualTo(63), reason: frame);
  expect(pixels[2], lessThanOrEqualTo(114), reason: frame);
  expect(pixels[1] - pixels[0], greaterThan(30), reason: frame);
  expect(pixels[1] - pixels[2], greaterThan(15), reason: frame);
}

/// Exercises the same system channel used by Android's predictive back gesture.
Future<Object?> _backGesture(
  WidgetTester tester,
  String method, [
  Map<String, Object?>? arguments,
]) async {
  const codec = StandardMethodCodec();
  final reply = await tester.binding.defaultBinaryMessenger
      .handlePlatformMessage(
        'flutter/backgesture',
        codec.encodeMethodCall(MethodCall(method, arguments)),
        null,
      );
  return codec.decodeEnvelope(reply!);
}
