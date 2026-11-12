import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/appearance/BubbleImageAdjustEditor.dart';

/// Checks source visibility, compact controls, and the rendered nine-slice grid.
void main() {
  testWidgets('phone keeps the original pinned while changing parameter tabs', (
    tester,
  ) async {
    final sourceImage = _sourceImage();
    addTearDown(sourceImage.dispose);
    tester.view.devicePixelRatio = 1;
    tester.view.physicalSize = const Size(360, 720);
    addTearDown(tester.view.resetDevicePixelRatio);
    addTearDown(tester.view.resetPhysicalSize);
    BubbleImageStyle? draft;
    await tester.pumpWidget(
      _editor(sourceImage, onChanged: (value) => draft = value),
    );
    await tester.pumpAndSettle();
    final source = find.byType(BubbleSliceSourceGuide);
    final sourceBounds = tester.getRect(source);
    expect(sourceBounds.height, greaterThan(280));
    final sliders = find.byType(Slider);
    expect(sliders, findsNWidgets(4));
    expect(
      tester.getTopLeft(sliders.at(0)).dy,
      tester.getTopLeft(sliders.at(1)).dy,
    );
    await tester.tap(sliders.first);
    await tester.pumpAndSettle();
    expect(draft!.cropLeftRatio, greaterThan(0));
    expect(tester.getRect(source), sourceBounds);

    await tester.tap(find.text('九宫格'));
    await tester.pumpAndSettle();
    expect(tester.getRect(source), sourceBounds);
    expect(find.byType(Slider), findsNWidgets(4));
    final viewer = tester.widget<InteractiveViewer>(
      find.byType(InteractiveViewer),
    );
    viewer.transformationController!.value = Matrix4.diagonal3Values(2, 2, 1);
    await tester.pump();
    await tester.tap(find.text('预览'));
    await tester.pumpAndSettle();
    expect(find.text('preview result'), findsOneWidget);
    expect(
      tester
          .widget<InteractiveViewer>(find.byType(InteractiveViewer))
          .transformationController!
          .value
          .getMaxScaleOnAxis(),
      2,
    );
    await tester.tap(find.byTooltip('完整显示原图'));
    await tester.pump();
    expect(viewer.transformationController!.value.isIdentity(), isTrue);
    expect(tester.takeException(), isNull);
  });

  for (final size in [const Size(320, 568), const Size(840, 400)]) {
    testWidgets('editor fits $size with enlarged text', (tester) async {
      final sourceImage = _sourceImage();
      addTearDown(sourceImage.dispose);
      tester.view.devicePixelRatio = 1;
      tester.view.physicalSize = size;
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(tester.view.resetPhysicalSize);
      await tester.pumpWidget(_editor(sourceImage, textScale: 1.6));
      await tester.pumpAndSettle();
      await tester.tap(find.text('九宫格'));
      await tester.pumpAndSettle();
      expect(find.byType(BubbleSliceSourceGuide), findsOneWidget);
      expect(
        tester.getSize(find.byType(BubbleSliceSourceGuide)).height,
        greaterThan(100),
      );
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('source grid crosses all nine regions and retains cropped pixels', (
    tester,
  ) async {
    final sourceImage = _sourceImage();
    addTearDown(sourceImage.dispose);
    const captureKey = ValueKey('source-capture');
    await tester.pumpWidget(
      MaterialApp(
        home: ThemeAssetScope(
          resolver: _ImageResolver(sourceImage),
          child: Center(
            child: RepaintBoundary(
              key: captureKey,
              child: const SizedBox(
                width: 300,
                height: 300,
                child: BubbleSliceSourceGuide(
                  imageStyle: BubbleImageStyle(
                    imagePath: 'source.png',
                    cropLeftRatio: 0.1,
                    cropTopRatio: 0.1,
                    cropRightRatio: 0.1,
                    cropBottomRatio: 0.1,
                    repeatXStartRatio: 0.25,
                    repeatXEndRatio: 0.75,
                    repeatYStartRatio: 0.25,
                    repeatYEndRatio: 0.75,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final boundary = tester.renderObject<RenderRepaintBoundary>(
      find.byKey(captureKey),
    );
    final pixels = (await tester.runAsync(() async {
      final capture = await boundary.toImage(pixelRatio: 1);
      final data = await capture.toByteData(format: ui.ImageByteFormat.rawRgba);
      capture.dispose();
      return data;
    }))!;
    // Both vertical cuts extend above and below the center cell.
    for (final point in [
      const Offset(90, 50),
      const Offset(210, 250),
      const Offset(50, 90),
      const Offset(250, 210),
    ]) {
      final offset = (point.dy.toInt() * 300 + point.dx.toInt()) * 4;
      expect(
        pixels.getUint8(offset + 2),
        greaterThan(pixels.getUint8(offset) + 40),
      );
    }
    // Cropped-out pixels remain present but dimmed; retained pixels stay white.
    expect(pixels.getUint8((15 * 300 + 15) * 4), inInclusiveRange(110, 125));
    expect(pixels.getUint8((50 * 300 + 50) * 4), 255);
    expect(tester.takeException(), isNull);
  });
}

/// Builds a localized editor without runtime or filesystem dependencies.
Widget _editor(
  ui.Image image, {
  ValueChanged<BubbleImageStyle>? onChanged,
  double textScale = 1,
}) {
  return MaterialApp(
    locale: const Locale('zh'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    builder: (context, child) => MediaQuery(
      data: MediaQuery.of(
        context,
      ).copyWith(textScaler: TextScaler.linear(textScale)),
      child: child!,
    ),
    home: Scaffold(
      appBar: AppBar(title: const Text('调整用户气泡图片')),
      body: ThemeAssetScope(
        resolver: _ImageResolver(image),
        child: BubbleImageAdjustEditor(
          initialStyle: const BubbleImageStyle(imagePath: 'source.png'),
          onChanged: onChanged ?? (_) {},
          previewBuilder: (context, style) => const Text('preview result'),
        ),
      ),
    ),
  );
}

/// Creates a uniform image so guide colors and crop shading can be sampled.
ui.Image _sourceImage() {
  final recorder = ui.PictureRecorder();
  Canvas(recorder).drawColor(Colors.white, BlendMode.src);
  final picture = recorder.endRecording();
  final image = picture.toImageSync(100, 100);
  picture.dispose();
  return image;
}

class _ImageResolver implements ThemeDecodedImageResolver {
  /// Shares a prepared image with each rendered source view.
  const _ImageResolver(this.image);

  final ui.Image image;

  /// Clones a prepared image for independent widget ownership.
  @override
  ui.Image cloneImage(String reference) => image.clone();

  /// Supplies the prepared image synchronously during widget tests.
  @override
  ui.Image? tryCloneImage(String reference) => image.clone();

  /// Rejects byte reads because this fixture provides already decoded pixels.
  @override
  Future<Uint8List> readBytes(String reference) =>
      throw UnsupportedError('decoded fixture');
}
