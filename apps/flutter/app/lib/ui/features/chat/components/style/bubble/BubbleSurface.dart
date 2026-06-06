// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:liquid_glass_widgets/widgets/shared/glass_effect.dart';

import '../../../../../../data/preferences/UserPreferencesManager.dart';

class BubbleImageStyle {
  const BubbleImageStyle({
    required this.imagePath,
    this.cropLeftRatio = 0,
    this.cropTopRatio = 0,
    this.cropRightRatio = 0,
    this.cropBottomRatio = 0,
    this.repeatXStartRatio = 0.35,
    this.repeatXEndRatio = 0.65,
    this.repeatYStartRatio = 0.35,
    this.repeatYEndRatio = 0.65,
    this.imageScale = 1,
    this.renderMode =
        UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
    this.showSliceGuides = false,
  });

  final String imagePath;
  final double cropLeftRatio;
  final double cropTopRatio;
  final double cropRightRatio;
  final double cropBottomRatio;
  final double repeatXStartRatio;
  final double repeatXEndRatio;
  final double repeatYStartRatio;
  final double repeatYEndRatio;
  final double imageScale;
  final String renderMode;
  final bool showSliceGuides;
}

class BubbleSurface extends StatelessWidget {
  const BubbleSurface({
    super.key,
    required this.color,
    required this.borderRadius,
    required this.child,
    this.imageStyle,
    this.transparentSurface = false,
  });

  final Color color;
  final BorderRadius borderRadius;
  final Widget child;
  final BubbleImageStyle? imageStyle;
  final bool transparentSurface;

  @override
  Widget build(BuildContext context) {
    final useImage = imageStyle != null && !transparentSurface;
    final surfaceColor = transparentSurface ? Colors.transparent : color;
    final decoration = BoxDecoration(
      color: surfaceColor,
      borderRadius: borderRadius,
    );
    final content = Stack(
      children: <Widget>[
        Positioned.fill(child: DecoratedBox(decoration: decoration)),
        if (useImage)
          Positioned.fill(
            child: BubbleImageBackgroundSurface(imageStyle: imageStyle!),
          ),
        child,
      ],
    );
    if (transparentSurface) {
      final transparentGlassSettings = _bubbleTransparentGlassSettings(color);
      return ClipRRect(
        borderRadius: borderRadius,
        child: Stack(
          children: <Widget>[
            Positioned.fill(
              child: BackdropFilter(
                filter: ui.ImageFilter.blur(
                  sigmaX: transparentGlassSettings.blur,
                  sigmaY: transparentGlassSettings.blur,
                ),
                child: const ColoredBox(color: Colors.transparent),
              ),
            ),
            Positioned.fill(
              child: _BubbleTransparentGlassSurface(
                glassRadius: _dominantCornerRadius(borderRadius),
                settings: transparentGlassSettings,
                child: const SizedBox.expand(),
              ),
            ),
            child,
          ],
        ),
      );
    }
    return ClipRRect(borderRadius: borderRadius, child: content);
  }
}

class _BubbleTransparentGlassSurface extends StatelessWidget {
  const _BubbleTransparentGlassSurface({
    required this.glassRadius,
    required this.settings,
    required this.child,
  });

  final double glassRadius;
  final LiquidGlassSettings settings;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    return GlassEffect(
      quality: GlassQuality.standard,
      shape: LiquidRoundedSuperellipse(borderRadius: glassRadius),
      settings: settings,
      interactionIntensity: 0.85,
      ambientRim: 0.11,
      baseAlphaMultiplier: 0.75,
      edgeAlphaMultiplier: 0.75,
      rimThickness: 1.48,
      rimSmoothing: 8,
      child: child,
    );
  }
}

LiquidGlassSettings _bubbleTransparentGlassSettings(Color color) {
  return LiquidGlassSettings(
    glassColor: color.withValues(alpha: 0.018),
    thickness: 80,
    blur: 5,
    chromaticAberration: 0.18,
    lightIntensity: 0.42,
    ambientStrength: 0.3,
    refractiveIndex: 1.16,
    saturation: 1.04,
    glowIntensity: 0.17,
    specularSharpness: GlassSpecularSharpness.soft,
    standardOpacityMultiplier: 0.12,
  );
}

double _dominantCornerRadius(BorderRadius borderRadius) {
  return <double>[
    borderRadius.topLeft.x,
    borderRadius.topLeft.y,
    borderRadius.topRight.x,
    borderRadius.topRight.y,
    borderRadius.bottomRight.x,
    borderRadius.bottomRight.y,
    borderRadius.bottomLeft.x,
    borderRadius.bottomLeft.y,
  ].reduce(math.max);
}

class BubbleImageBackgroundSurface extends StatefulWidget {
  const BubbleImageBackgroundSurface({super.key, required this.imageStyle});

  final BubbleImageStyle imageStyle;

  @override
  State<BubbleImageBackgroundSurface> createState() =>
      _BubbleImageBackgroundSurfaceState();
}

class _BubbleImageBackgroundSurfaceState
    extends State<BubbleImageBackgroundSurface> {
  ui.Image? _image;
  int _loadToken = 0;

  @override
  void initState() {
    super.initState();
    unawaited(_loadImage());
  }

  @override
  void didUpdateWidget(covariant BubbleImageBackgroundSurface oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.imageStyle.imagePath != widget.imageStyle.imagePath) {
      unawaited(_loadImage());
    }
  }

  Future<void> _loadImage() async {
    final token = ++_loadToken;
    final bytes = await File(widget.imageStyle.imagePath).readAsBytes();
    final codec = await ui.instantiateImageCodec(bytes);
    final frame = await codec.getNextFrame();
    if (!mounted || token != _loadToken) {
      frame.image.dispose();
      return;
    }
    final previous = _image;
    setState(() {
      _image = frame.image;
    });
    previous?.dispose();
  }

  @override
  void dispose() {
    _loadToken++;
    _image?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final image = _image;
    if (image == null) {
      return const SizedBox.expand();
    }
    return CustomPaint(
      painter: _BubbleImagePainter(image: image, style: widget.imageStyle),
    );
  }
}

class _BubbleSliceLayout {
  const _BubbleSliceLayout({
    required this.srcX,
    required this.srcY,
    required this.srcWidth,
    required this.srcHeight,
    required this.leftCapWidth,
    required this.centerWidth,
    required this.rightCapWidth,
    required this.topCapHeight,
    required this.centerHeight,
    required this.bottomCapHeight,
  });

  final int srcX;
  final int srcY;
  final int srcWidth;
  final int srcHeight;
  final int leftCapWidth;
  final int centerWidth;
  final int rightCapWidth;
  final int topCapHeight;
  final int centerHeight;
  final int bottomCapHeight;
}

class _BubbleDstSliceLayout {
  const _BubbleDstSliceLayout({
    required this.leftDstWidth,
    required this.rightDstWidth,
    required this.topDstHeight,
    required this.bottomDstHeight,
    required this.centerDstStartX,
    required this.centerDstEndX,
    required this.centerDstStartY,
    required this.centerDstEndY,
    required this.scaleX,
    required this.scaleY,
  });

  final int leftDstWidth;
  final int rightDstWidth;
  final int topDstHeight;
  final int bottomDstHeight;
  final int centerDstStartX;
  final int centerDstEndX;
  final int centerDstStartY;
  final int centerDstEndY;
  final double scaleX;
  final double scaleY;
}

class _BubbleImagePainter extends CustomPainter {
  const _BubbleImagePainter({required this.image, required this.style});

  final ui.Image image;
  final BubbleImageStyle style;

  @override
  void paint(Canvas canvas, Size size) {
    final layout = _buildSliceLayout(image, style);
    if (style.renderMode ==
        UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH) {
      _drawStretchedNinePatchBubble(canvas, size, layout);
    } else {
      _drawRepeatedCenterBubble(canvas, size, layout);
    }
    if (style.showSliceGuides) {
      _drawNinePatchSliceGuides(canvas, size, layout);
    }
  }

  @override
  bool shouldRepaint(covariant _BubbleImagePainter oldDelegate) {
    return oldDelegate.image != image || oldDelegate.style != style;
  }

  _BubbleSliceLayout _buildSliceLayout(
    ui.Image bitmap,
    BubbleImageStyle config,
  ) {
    final width = math.max(1, bitmap.width);
    final height = math.max(1, bitmap.height);

    final cropLeft = config.cropLeftRatio.clamp(0.0, 0.45);
    final cropTop = config.cropTopRatio.clamp(0.0, 0.45);
    final cropRight = config.cropRightRatio.clamp(0.0, 0.45);
    final cropBottom = config.cropBottomRatio.clamp(0.0, 0.45);

    final srcLeft = (width * cropLeft).round().clamp(0, width - 1);
    final srcTop = (height * cropTop).round().clamp(0, height - 1);
    final srcRight = (width * (1 - cropRight)).round().clamp(
      srcLeft + 1,
      width,
    );
    final srcBottom = (height * (1 - cropBottom)).round().clamp(
      srcTop + 1,
      height,
    );

    final croppedWidth = math.max(1, srcRight - srcLeft);
    final croppedHeight = math.max(1, srcBottom - srcTop);

    final repeatXStart = config.repeatXStartRatio.clamp(0.05, 0.9);
    final repeatXEnd = config.repeatXEndRatio.clamp(repeatXStart + 0.01, 0.95);
    final repeatYStart = config.repeatYStartRatio.clamp(0.05, 0.9);
    final repeatYEnd = config.repeatYEndRatio.clamp(repeatYStart + 0.01, 0.95);
    final repeatStartXPx = croppedWidth < 3
        ? 0
        : (croppedWidth * repeatXStart).round().clamp(1, croppedWidth - 2);
    final repeatEndXPx = croppedWidth < 3
        ? croppedWidth
        : (croppedWidth * repeatXEnd).round().clamp(
            repeatStartXPx + 1,
            croppedWidth - 1,
          );
    final repeatStartYPx = croppedHeight < 3
        ? 0
        : (croppedHeight * repeatYStart).round().clamp(1, croppedHeight - 2);
    final repeatEndYPx = croppedHeight < 3
        ? croppedHeight
        : (croppedHeight * repeatYEnd).round().clamp(
            repeatStartYPx + 1,
            croppedHeight - 1,
          );

    return _BubbleSliceLayout(
      srcX: srcLeft,
      srcY: srcTop,
      srcWidth: croppedWidth,
      srcHeight: croppedHeight,
      leftCapWidth: repeatStartXPx,
      centerWidth: math.max(1, repeatEndXPx - repeatStartXPx),
      rightCapWidth: math.max(0, croppedWidth - repeatEndXPx),
      topCapHeight: repeatStartYPx,
      centerHeight: math.max(1, repeatEndYPx - repeatStartYPx),
      bottomCapHeight: math.max(0, croppedHeight - repeatEndYPx),
    );
  }

  _BubbleDstSliceLayout _computeDstSliceLayout(
    _BubbleSliceLayout layout,
    int dstWidth,
    int dstHeight,
    double imageScale,
  ) {
    final uniformScale = imageScale.clamp(0.2, 3.0);
    var leftDstWidth = math.max(
      0,
      (layout.leftCapWidth * uniformScale).round(),
    );
    var rightDstWidth = math.max(
      0,
      (layout.rightCapWidth * uniformScale).round(),
    );
    var topDstHeight = math.max(
      0,
      (layout.topCapHeight * uniformScale).round(),
    );
    var bottomDstHeight = math.max(
      0,
      (layout.bottomCapHeight * uniformScale).round(),
    );

    if (leftDstWidth + rightDstWidth >= dstWidth) {
      final target = math.max(0, dstWidth - 1);
      final total = math.max(1, leftDstWidth + rightDstWidth);
      final ratio = target / total;
      leftDstWidth = math.max(0, (leftDstWidth * ratio).round());
      rightDstWidth = math.max(0, (rightDstWidth * ratio).round());
      final overflow = leftDstWidth + rightDstWidth - target;
      if (overflow > 0) {
        rightDstWidth = math.max(0, rightDstWidth - overflow);
      }
    }

    if (topDstHeight + bottomDstHeight >= dstHeight) {
      final target = math.max(0, dstHeight - 1);
      final total = math.max(1, topDstHeight + bottomDstHeight);
      final ratio = target / total;
      topDstHeight = math.max(0, (topDstHeight * ratio).round());
      bottomDstHeight = math.max(0, (bottomDstHeight * ratio).round());
      final overflow = topDstHeight + bottomDstHeight - target;
      if (overflow > 0) {
        bottomDstHeight = math.max(0, bottomDstHeight - overflow);
      }
    }

    final centerDstStartX = leftDstWidth;
    final centerDstEndX = math.max(centerDstStartX, dstWidth - rightDstWidth);
    final centerDstStartY = topDstHeight;
    final centerDstEndY = math.max(
      centerDstStartY,
      dstHeight - bottomDstHeight,
    );

    return _BubbleDstSliceLayout(
      leftDstWidth: leftDstWidth,
      rightDstWidth: rightDstWidth,
      topDstHeight: topDstHeight,
      bottomDstHeight: bottomDstHeight,
      centerDstStartX: centerDstStartX,
      centerDstEndX: centerDstEndX,
      centerDstStartY: centerDstStartY,
      centerDstEndY: centerDstEndY,
      scaleX: uniformScale,
      scaleY: uniformScale,
    );
  }

  void _drawRepeatedCenterBubble(
    Canvas canvas,
    Size size,
    _BubbleSliceLayout layout,
  ) {
    final dstWidth = math.max(1, size.width.round());
    final dstHeight = math.max(1, size.height.round());
    if (layout.centerWidth <= 0 ||
        layout.centerHeight <= 0 ||
        layout.srcWidth <= 0 ||
        layout.srcHeight <= 0) {
      _drawSlice(
        canvas,
        layout.srcX,
        layout.srcY,
        layout.srcWidth,
        layout.srcHeight,
        0,
        0,
        dstWidth,
        dstHeight,
      );
      return;
    }

    final dstLayout = _computeDstSliceLayout(
      layout,
      dstWidth,
      dstHeight,
      style.imageScale,
    );
    final srcLeftX = layout.srcX;
    final srcCenterX = layout.srcX + layout.leftCapWidth;
    final srcRightX = layout.srcX + layout.srcWidth - layout.rightCapWidth;
    final srcTopY = layout.srcY;
    final srcCenterY = layout.srcY + layout.topCapHeight;
    final srcBottomY = layout.srcY + layout.srcHeight - layout.bottomCapHeight;

    _drawCorners(
      canvas,
      layout,
      dstLayout,
      dstWidth,
      dstHeight,
      srcLeftX,
      srcRightX,
      srcTopY,
      srcBottomY,
    );
    _drawTiledHorizontally(
      canvas,
      srcCenterX,
      srcTopY,
      layout.centerWidth,
      layout.topCapHeight,
      dstLayout.centerDstStartX,
      0,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.topDstHeight,
      dstLayout.scaleX,
    );
    _drawTiledHorizontally(
      canvas,
      srcCenterX,
      srcBottomY,
      layout.centerWidth,
      layout.bottomCapHeight,
      dstLayout.centerDstStartX,
      dstHeight - dstLayout.bottomDstHeight,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.bottomDstHeight,
      dstLayout.scaleX,
    );
    _drawTiledVertically(
      canvas,
      srcLeftX,
      srcCenterY,
      layout.leftCapWidth,
      layout.centerHeight,
      0,
      dstLayout.centerDstStartY,
      dstLayout.leftDstWidth,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
      dstLayout.scaleY,
    );
    _drawTiledVertically(
      canvas,
      srcRightX,
      srcCenterY,
      layout.rightCapWidth,
      layout.centerHeight,
      dstWidth - dstLayout.rightDstWidth,
      dstLayout.centerDstStartY,
      dstLayout.rightDstWidth,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
      dstLayout.scaleY,
    );
    _drawTiled2D(
      canvas,
      srcCenterX,
      srcCenterY,
      layout.centerWidth,
      layout.centerHeight,
      dstLayout.centerDstStartX,
      dstLayout.centerDstStartY,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
      dstLayout.scaleX,
      dstLayout.scaleY,
    );
  }

  void _drawStretchedNinePatchBubble(
    Canvas canvas,
    Size size,
    _BubbleSliceLayout layout,
  ) {
    final dstWidth = math.max(1, size.width.round());
    final dstHeight = math.max(1, size.height.round());
    final dstLayout = _computeDstSliceLayout(
      layout,
      dstWidth,
      dstHeight,
      style.imageScale,
    );
    final srcLeftX = layout.srcX;
    final srcCenterX = layout.srcX + layout.leftCapWidth;
    final srcRightX = layout.srcX + layout.srcWidth - layout.rightCapWidth;
    final srcTopY = layout.srcY;
    final srcCenterY = layout.srcY + layout.topCapHeight;
    final srcBottomY = layout.srcY + layout.srcHeight - layout.bottomCapHeight;

    _drawCorners(
      canvas,
      layout,
      dstLayout,
      dstWidth,
      dstHeight,
      srcLeftX,
      srcRightX,
      srcTopY,
      srcBottomY,
    );
    _drawSlice(
      canvas,
      srcCenterX,
      srcTopY,
      layout.centerWidth,
      layout.topCapHeight,
      dstLayout.centerDstStartX,
      0,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.topDstHeight,
    );
    _drawSlice(
      canvas,
      srcCenterX,
      srcBottomY,
      layout.centerWidth,
      layout.bottomCapHeight,
      dstLayout.centerDstStartX,
      dstHeight - dstLayout.bottomDstHeight,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.bottomDstHeight,
    );
    _drawSlice(
      canvas,
      srcLeftX,
      srcCenterY,
      layout.leftCapWidth,
      layout.centerHeight,
      0,
      dstLayout.centerDstStartY,
      dstLayout.leftDstWidth,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
    );
    _drawSlice(
      canvas,
      srcRightX,
      srcCenterY,
      layout.rightCapWidth,
      layout.centerHeight,
      dstWidth - dstLayout.rightDstWidth,
      dstLayout.centerDstStartY,
      dstLayout.rightDstWidth,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
    );
    _drawSlice(
      canvas,
      srcCenterX,
      srcCenterY,
      layout.centerWidth,
      layout.centerHeight,
      dstLayout.centerDstStartX,
      dstLayout.centerDstStartY,
      dstLayout.centerDstEndX - dstLayout.centerDstStartX,
      dstLayout.centerDstEndY - dstLayout.centerDstStartY,
    );
  }

  void _drawCorners(
    Canvas canvas,
    _BubbleSliceLayout layout,
    _BubbleDstSliceLayout dstLayout,
    int dstWidth,
    int dstHeight,
    int srcLeftX,
    int srcRightX,
    int srcTopY,
    int srcBottomY,
  ) {
    _drawSlice(
      canvas,
      srcLeftX,
      srcTopY,
      layout.leftCapWidth,
      layout.topCapHeight,
      0,
      0,
      dstLayout.leftDstWidth,
      dstLayout.topDstHeight,
    );
    _drawSlice(
      canvas,
      srcRightX,
      srcTopY,
      layout.rightCapWidth,
      layout.topCapHeight,
      dstWidth - dstLayout.rightDstWidth,
      0,
      dstLayout.rightDstWidth,
      dstLayout.topDstHeight,
    );
    _drawSlice(
      canvas,
      srcLeftX,
      srcBottomY,
      layout.leftCapWidth,
      layout.bottomCapHeight,
      0,
      dstHeight - dstLayout.bottomDstHeight,
      dstLayout.leftDstWidth,
      dstLayout.bottomDstHeight,
    );
    _drawSlice(
      canvas,
      srcRightX,
      srcBottomY,
      layout.rightCapWidth,
      layout.bottomCapHeight,
      dstWidth - dstLayout.rightDstWidth,
      dstHeight - dstLayout.bottomDstHeight,
      dstLayout.rightDstWidth,
      dstLayout.bottomDstHeight,
    );
  }

  void _drawSlice(
    Canvas canvas,
    int srcX,
    int srcY,
    int srcW,
    int srcH,
    int dstX,
    int dstY,
    int dstW,
    int dstH,
  ) {
    if (srcW <= 0 || srcH <= 0 || dstW <= 0 || dstH <= 0) {
      return;
    }
    final safeSrcX = srcX.clamp(0, image.width - 1);
    final safeSrcY = srcY.clamp(0, image.height - 1);
    final safeSrcW = math.min(srcW, image.width - safeSrcX);
    final safeSrcH = math.min(srcH, image.height - safeSrcY);
    if (safeSrcW <= 0 || safeSrcH <= 0) {
      return;
    }
    final safeDstW = safeSrcW == srcW
        ? dstW
        : math.max(1, (dstW * (safeSrcW / srcW)).round());
    final safeDstH = safeSrcH == srcH
        ? dstH
        : math.max(1, (dstH * (safeSrcH / srcH)).round());
    canvas.drawImageRect(
      image,
      Rect.fromLTWH(
        safeSrcX.toDouble(),
        safeSrcY.toDouble(),
        safeSrcW.toDouble(),
        safeSrcH.toDouble(),
      ),
      Rect.fromLTWH(
        dstX.toDouble(),
        dstY.toDouble(),
        safeDstW.toDouble(),
        safeDstH.toDouble(),
      ),
      Paint(),
    );
  }

  void _drawNinePatchSliceGuides(
    Canvas canvas,
    Size size,
    _BubbleSliceLayout layout,
  ) {
    final dstWidth = math.max(1, size.width.round());
    final dstHeight = math.max(1, size.height.round());
    final dstLayout = _computeDstSliceLayout(
      layout,
      dstWidth,
      dstHeight,
      style.imageScale,
    );
    final paint = Paint()
      ..color = Colors.white.withValues(alpha: 0.58)
      ..strokeWidth = 1.2
      ..style = PaintingStyle.stroke;
    final shadowPaint = Paint()
      ..color = Colors.black.withValues(alpha: 0.34)
      ..strokeWidth = 2.2
      ..style = PaintingStyle.stroke;

    void drawGuideLine(Offset start, Offset end) {
      canvas.drawLine(start, end, shadowPaint);
      canvas.drawLine(start, end, paint);
    }

    final leftX = dstLayout.centerDstStartX
        .toDouble()
        .clamp(0, size.width)
        .toDouble();
    final rightX = dstLayout.centerDstEndX
        .toDouble()
        .clamp(0, size.width)
        .toDouble();
    final topY = dstLayout.centerDstStartY
        .toDouble()
        .clamp(0, size.height)
        .toDouble();
    final bottomY = dstLayout.centerDstEndY
        .toDouble()
        .clamp(0, size.height)
        .toDouble();

    drawGuideLine(Offset(leftX, 0), Offset(leftX, size.height));
    drawGuideLine(Offset(rightX, 0), Offset(rightX, size.height));
    drawGuideLine(Offset(0, topY), Offset(size.width, topY));
    drawGuideLine(Offset(0, bottomY), Offset(size.width, bottomY));
  }

  void _drawTiledHorizontally(
    Canvas canvas,
    int srcX,
    int srcY,
    int srcW,
    int srcH,
    int dstX,
    int dstY,
    int dstW,
    int dstH,
    double scaleX,
  ) {
    if (srcW <= 0 || srcH <= 0 || dstW <= 0 || dstH <= 0) {
      return;
    }
    final baseTileDstWidth = math.max(1, (srcW * scaleX).round());
    var currentX = dstX;
    final dstEnd = dstX + dstW;
    while (currentX < dstEnd) {
      final remaining = dstEnd - currentX;
      final tileDstWidth = math.min(baseTileDstWidth, remaining);
      final tileSrcWidth = tileDstWidth == baseTileDstWidth
          ? srcW
          : math.max(1, (srcW * (tileDstWidth / baseTileDstWidth)).round());
      _drawSlice(
        canvas,
        srcX,
        srcY,
        tileSrcWidth,
        srcH,
        currentX,
        dstY,
        tileDstWidth,
        dstH,
      );
      currentX += tileDstWidth;
    }
  }

  void _drawTiledVertically(
    Canvas canvas,
    int srcX,
    int srcY,
    int srcW,
    int srcH,
    int dstX,
    int dstY,
    int dstW,
    int dstH,
    double scaleY,
  ) {
    if (srcW <= 0 || srcH <= 0 || dstW <= 0 || dstH <= 0) {
      return;
    }
    final baseTileDstHeight = math.max(1, (srcH * scaleY).round());
    var currentY = dstY;
    final dstEnd = dstY + dstH;
    while (currentY < dstEnd) {
      final remaining = dstEnd - currentY;
      final tileDstHeight = math.min(baseTileDstHeight, remaining);
      final tileSrcHeight = tileDstHeight == baseTileDstHeight
          ? srcH
          : math.max(1, (srcH * (tileDstHeight / baseTileDstHeight)).round());
      _drawSlice(
        canvas,
        srcX,
        srcY,
        srcW,
        tileSrcHeight,
        dstX,
        currentY,
        dstW,
        tileDstHeight,
      );
      currentY += tileDstHeight;
    }
  }

  void _drawTiled2D(
    Canvas canvas,
    int srcX,
    int srcY,
    int srcW,
    int srcH,
    int dstX,
    int dstY,
    int dstW,
    int dstH,
    double scaleX,
    double scaleY,
  ) {
    if (srcW <= 0 || srcH <= 0 || dstW <= 0 || dstH <= 0) {
      return;
    }
    final baseTileDstWidth = math.max(1, (srcW * scaleX).round());
    final baseTileDstHeight = math.max(1, (srcH * scaleY).round());
    var currentY = dstY;
    final dstEndY = dstY + dstH;
    while (currentY < dstEndY) {
      final remainingY = dstEndY - currentY;
      final tileDstHeight = math.min(baseTileDstHeight, remainingY);
      final tileSrcHeight = tileDstHeight == baseTileDstHeight
          ? srcH
          : math.max(1, (srcH * (tileDstHeight / baseTileDstHeight)).round());
      var currentX = dstX;
      final dstEndX = dstX + dstW;
      while (currentX < dstEndX) {
        final remainingX = dstEndX - currentX;
        final tileDstWidth = math.min(baseTileDstWidth, remainingX);
        final tileSrcWidth = tileDstWidth == baseTileDstWidth
            ? srcW
            : math.max(1, (srcW * (tileDstWidth / baseTileDstWidth)).round());
        _drawSlice(
          canvas,
          srcX,
          srcY,
          tileSrcWidth,
          tileSrcHeight,
          currentX,
          currentY,
          tileDstWidth,
          tileDstHeight,
        );
        currentX += tileDstWidth;
      }
      currentY += tileDstHeight;
    }
  }
}
