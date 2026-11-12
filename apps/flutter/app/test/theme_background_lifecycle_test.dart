import 'dart:typed_data';

import 'package:flutter/material.dart';
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
