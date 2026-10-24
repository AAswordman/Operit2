import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/settings/components/SettingsCategoryList.dart';
import 'package:operit2/ui/features/settings/components/SettingsDetailView.dart';
import 'package:operit2/ui/features/settings/models/SettingsModels.dart';
import 'package:operit2/ui/features/settings/screens/SettingsScreen.dart';
import 'package:operit2/ui/main/TopBarController.dart';
import 'package:operit2/ui/theme/OperitGlassSurface.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

/// Verifies settings cards blur the actual background throughout navigation.
void main() {
  testWidgets('settings detail stays frosted during its entrance animation', (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1100, 700);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final captureKey = GlobalKey();
    final topBar = TopBarController();
    addTearDown(topBar.dispose);
    await tester.pumpWidget(
      OperitTheme(
        initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
          ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
          'transparentSurfaceEnabled': true,
        }),
        initialThemeIsReady: false,
        unconfiguredChildEnabled: true,
        hostInteractionHostsEnabled: false,
        child: TopBarScope(
          controller: topBar,
          child: RepaintBoundary(
            key: captureKey,
            child: Stack(
              fit: StackFit.expand,
              children: const [
                CustomPaint(painter: _StripePainter()),
                SettingsScreen(initialCategory: SettingsCategory.about),
              ],
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    final categoryKeys = _filterKeys(tester, find.byType(SettingsCategoryList));
    expect(categoryKeys, isNotEmpty);
    expect(categoryKeys, everyElement(isNotNull));
    expect(categoryKeys.toSet(), hasLength(1));
    final outgoingKeys = _filterKeys(
      tester,
      find.byKey(const ValueKey<SettingsCategory>(SettingsCategory.about)),
    );
    expect(outgoingKeys, isNotEmpty);
    expect(outgoingKeys, everyElement(isNotNull));
    expect(outgoingKeys.toSet(), hasLength(1));

    tester
        .widget<SettingsCategoryList>(find.byType(SettingsCategoryList))
        .onCategorySelected(SettingsCategory.globalBehavior);
    await tester.pump();
    final detail = find.byKey(
      const ValueKey<SettingsCategory>(SettingsCategory.globalBehavior),
    );
    expect(
      tester.widget<SettingsDetailView>(detail).category,
      SettingsCategory.globalBehavior,
    );
    final incomingKeys = _filterKeys(tester, detail);
    expect(incomingKeys, isNotEmpty);
    expect(incomingKeys, everyElement(isNotNull));
    expect(incomingKeys.toSet().intersection(outgoingKeys.toSet()), isEmpty);
    expect(
      _filterKeys(tester, find.byType(SettingsCategoryList)),
      categoryKeys,
    );
    final initialX = tester.getTopLeft(detail).dx;
    for (final elapsed in <int>[16, 64, 80]) {
      await tester.pump(Duration(milliseconds: elapsed));
      final card = find
          .descendant(of: detail, matching: find.byType(OperitGlassSurface))
          .first;
      final rect = tester.getRect(card);
      // The left padding has no text and the outgoing page has no card here.
      final position = Offset(rect.left + 8, rect.center.dy);
      await _expectBlurredPixel(tester, captureKey, position);
    }
    expect(tester.getTopLeft(detail).dx, lessThan(initialX));
    await tester.pumpAndSettle();
    final card = find
        .descendant(of: detail, matching: find.byType(OperitGlassSurface))
        .first;
    final rect = tester.getRect(card);
    await _expectBlurredPixel(
      tester,
      captureKey,
      Offset(rect.left + 8, rect.center.dy),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  for (final category in <SettingsCategory?>[null, SettingsCategory.about]) {
    testWidgets('phone settings $category keep frost during touch scrolling', (
      tester,
    ) async {
      final captureKey = GlobalKey();
      final topBar = TopBarController();
      addTearDown(topBar.dispose);
      await _pumpPhoneSettings(tester, captureKey, topBar, category);
      expect(find.byType(ThemeGlassStretchIndicator), findsOneWidget);
      final scrollable = tester.state<ScrollableState>(find.byType(Scrollable));
      expect(scrollable.position.maxScrollExtent, greaterThan(0));
      await _expectVisiblePhoneFrost(tester, captureKey);

      // Pull beyond the leading edge and inspect frames while the finger is down.
      var gesture = await tester.startGesture(const Offset(200, 180));
      for (var frame = 0; frame < 3; frame++) {
        await gesture.moveBy(const Offset(0, 55));
        await tester.pump(const Duration(milliseconds: 16));
        await _expectVisiblePhoneFrost(tester, captureKey);
      }
      final transform = tester.widget<Transform>(
        find
            .descendant(
              of: find.byType(ThemeGlassStretchIndicator),
              matching: find.byType(Transform),
            )
            .first,
      );
      expect(transform.transform.entry(1, 1), greaterThan(1));
      expect(transform.filterQuality, isNull);
      await gesture.up();
      await tester.pumpAndSettle();

      // Normal scrolling must move the list without isolating its backdrop.
      gesture = await tester.startGesture(const Offset(200, 350));
      for (var frame = 0; frame < 3; frame++) {
        await gesture.moveBy(const Offset(0, -45));
        await tester.pump(const Duration(milliseconds: 16));
        await _expectVisiblePhoneFrost(tester, captureKey);
      }
      expect(scrollable.position.pixels, greaterThan(0));
      await gesture.up();
      for (var frame = 0; frame < 4; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        await _expectVisiblePhoneFrost(tester, captureKey);
      }
      await tester.pumpAndSettle();

      // Repeat at the trailing edge and through the release animation.
      scrollable.position.jumpTo(scrollable.position.maxScrollExtent);
      await tester.pump();
      gesture = await tester.startGesture(const Offset(200, 300));
      await gesture.moveBy(const Offset(0, -100));
      await tester.pump(const Duration(milliseconds: 16));
      await _expectVisiblePhoneFrost(tester, captureKey);
      await gesture.up();
      for (var frame = 0; frame < 4; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        await _expectVisiblePhoneFrost(tester, captureKey);
      }
      await tester.pumpAndSettle();
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    });
  }
}

/// Mounts the real compact settings layout with touch overscroll enabled.
Future<void> _pumpPhoneSettings(
  WidgetTester tester,
  GlobalKey captureKey,
  TopBarController topBar,
  SettingsCategory? category,
) async {
  tester.view.physicalSize = const Size(400, 500);
  tester.view.devicePixelRatio = 1;
  addTearDown(tester.view.resetPhysicalSize);
  addTearDown(tester.view.resetDevicePixelRatio);
  await tester.pumpWidget(
    OperitTheme(
      initialThemePreferenceSnapshot: ThemePreferenceSnapshot.fromJson({
        ...UserPreferencesManager.defaultThemePreferenceSnapshot.toJson(),
        'transparentSurfaceEnabled': true,
      }),
      initialThemeIsReady: false,
      unconfiguredChildEnabled: true,
      hostInteractionHostsEnabled: false,
      child: Builder(
        builder: (context) => Theme(
          data: Theme.of(context).copyWith(platform: TargetPlatform.android),
          child: TopBarScope(
            controller: topBar,
            child: RepaintBoundary(
              key: captureKey,
              child: Stack(
                fit: StackFit.expand,
                children: [
                  const CustomPaint(painter: _StripePainter()),
                  SettingsScreen(initialCategory: category),
                ],
              ),
            ),
          ),
        ),
      ),
    ),
  );
  await tester.pumpAndSettle();
}

/// Samples visible card padding while content moves through the phone viewport.
Future<void> _expectVisiblePhoneFrost(
  WidgetTester tester,
  GlobalKey captureKey,
) async {
  final viewport = tester.getRect(find.byKey(captureKey)).deflate(24);
  for (final element in find.byType(OperitGlassSurface).evaluate()) {
    final surface = tester.getRect(find.byWidget(element.widget));
    final visible = surface.deflate(14).intersect(viewport);
    if (visible.width > 16 && visible.height > 4) {
      await _expectBlurredPixel(
        tester,
        captureKey,
        Offset(surface.left + 7, visible.center.dy),
        sampleWidth: 5,
      );
      return;
    }
  }
  fail('Expected a visible frosted settings card while scrolling.');
}

/// Reads the identities that Flutter passes to the engine for shared blurs.
List<BackdropKey?> _filterKeys(WidgetTester tester, Finder subtree) {
  return tester
      .widgetList<BackdropFilter>(
        find.descendant(of: subtree, matching: find.byType(BackdropFilter)),
      )
      .map((filter) => filter.backdropGroupKey)
      .toList();
}

/// Checks raster output so a filter reading an empty opacity layer fails.
Future<void> _expectBlurredPixel(
  WidgetTester tester,
  GlobalKey captureKey,
  Offset position, {
  int sampleWidth = 1,
}) async {
  final boundary = tester.renderObject<RenderRepaintBoundary>(
    find.byKey(captureKey),
  );
  final localPosition = boundary.globalToLocal(position);
  final red = await tester.runAsync(() async {
    final image = await boundary.toImage();
    try {
      final data = (await image.toByteData(
        format: ui.ImageByteFormat.rawRgba,
      ))!;
      final offset =
          (localPosition.dy.toInt() * image.width + localPosition.dx.toInt()) *
          4;
      return List<int>.generate(
        sampleWidth,
        (index) => data.getUint8(offset + index * 4),
      );
    } finally {
      image.dispose();
    }
  });
  expect(red, everyElement(inInclusiveRange(60, 195)));
}

class _StripePainter extends CustomPainter {
  /// Creates a high-contrast background that exposes missing backdrop blur.
  const _StripePainter();

  /// Draws stripes that blend to gray when sampled through a frosted card.
  @override
  void paint(Canvas canvas, Size size) {
    for (var x = 0.0; x < size.width; x += 4) {
      canvas.drawRect(
        Rect.fromLTWH(x, 0, 4, size.height),
        Paint()..color = (x / 4).toInt().isEven ? Colors.black : Colors.white,
      );
    }
  }

  /// Keeps the static background identical between animation frames.
  @override
  bool shouldRepaint(covariant _StripePainter oldDelegate) => false;
}
