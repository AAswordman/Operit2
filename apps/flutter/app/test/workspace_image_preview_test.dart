import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceFilePreviewContent.dart';
import 'package:operit2/ui/features/chat/components/workspace/WorkspaceTabModels.dart';
import 'package:operit2/ui/features/chat/components/workspace/file_preview/WorkspaceImagePreview.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

final _png = base64Decode(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAAAXNSR0IArs4c6QAAAARnQU1BAACxjwv8YQUAAAAJcEhZcwAADsMAAA7DAcdvqGQAAAANSURBVBhXY2Bg+P8fAAMCAf/Jsq3uAAAAAElFTkSuQmCC',
);
final _svg = Uint8List.fromList(
  utf8.encode(
    '<svg xmlns="http://www.w3.org/2000/svg" width="160" height="100" '
    'viewBox="0 0 160 100"><rect width="160" height="100" fill="red"/></svg>',
  ),
);

/// Provides localization and a bounded workspace preview surface.
Widget _app(Widget child) => OperitTheme(
  initialThemePreferenceSnapshot:
      UserPreferencesManager.defaultThemePreferenceSnapshot,
  initialThemeIsReady: false,
  unconfiguredChildEnabled: true,
  hostInteractionHostsEnabled: false,
  child: MaterialApp(
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(body: child),
  ),
);

/// Reads the rendered transform rather than the gesture configuration.
Matrix4 _matrix(WidgetTester tester) => tester
    .widget<Transform>(
      find
          .descendant(
            of: find.byType(InteractiveViewer),
            matching: find.byType(Transform),
          )
          .first,
    )
    .transform
    .clone();

/// Sends wheel events at an off-center point to exercise focal point retention.
Future<void> _scroll(
  WidgetTester tester,
  double delta, {
  PointerDeviceKind kind = PointerDeviceKind.mouse,
}) async {
  tester.binding.handlePointerEvent(
    PointerScrollEvent(
      kind: kind,
      position:
          tester.getCenter(find.byType(InteractiveViewer)) +
          const Offset(70, 30),
      scrollDelta: Offset(0, delta),
    ),
  );
  await tester.pump();
}

/// Exercises the file router while allowing parent rebuilds and new callbacks.
Widget _content(
  Future<Uint8List> Function(String) read, {
  String path = 'picture.png',
  double width = 640,
}) => _app(
  SizedBox(
    width: width,
    child: WorkspaceFilePreviewContent(
      tab: WorkspaceTab(
        kind: WorkspaceTabKind.filePreview,
        title: path,
        icon: Icons.image,
        filePath: path,
        previewKind: WorkspaceFilePreviewKind.image,
      ),
      onReadWorkspaceFileBytes: read,
      onWriteWorkspaceFileBytes: (_, _) async {},
      onOpenWorkspaceFile: (_) async {},
      onOpenBrowser: ({url, localFilePath, workspaceHtmlPath}) {},
      splitMarkdownContent: (_) async => [],
    ),
  ),
);

/// Decodes actual raster frames before checking retention across rebuilds.
Future<void> _settleImage(WidgetTester tester) async {
  await tester.pumpAndSettle();
  for (final element in find.byType(Image).evaluate()) {
    await tester.runAsync(
      () => precacheImage((element.widget as Image).image, element),
    );
  }
  await tester.pumpAndSettle();
}

/// Verifies input behavior and file request lifetimes at the preview boundary.
void main() {
  for (final file in {'picture.png': _png, 'picture.svg': _svg}.entries) {
    testWidgets('${file.key} supports mouse, trackpad, and touch zoom', (
      tester,
    ) async {
      await tester.pumpWidget(
        _app(WorkspaceImagePreview(bytes: file.value, fileName: file.key)),
      );
      await _settleImage(tester);

      await _scroll(tester, 100);
      expect(_matrix(tester).getMaxScaleOnAxis(), lessThan(1));
      await _scroll(tester, -200);
      expect(_matrix(tester).getMaxScaleOnAxis(), greaterThan(1));
      final mouseScale = _matrix(tester).getMaxScaleOnAxis();
      await _scroll(tester, -80, kind: PointerDeviceKind.trackpad);
      expect(_matrix(tester).getMaxScaleOnAxis(), greaterThan(mouseScale));

      final center = tester.getCenter(find.byType(InteractiveViewer));
      final trackpad = await tester.createGesture(
        kind: PointerDeviceKind.trackpad,
      );
      await trackpad.panZoomStart(center);
      await trackpad.panZoomUpdate(center, scale: 1.1);
      final startScale = _matrix(tester).getMaxScaleOnAxis();
      await trackpad.panZoomUpdate(center, scale: 1.5);
      await tester.pump();
      expect(_matrix(tester).getMaxScaleOnAxis(), greaterThan(startScale));
      await trackpad.panZoomEnd();

      final beforePinch = _matrix(tester).getMaxScaleOnAxis();
      final first = await tester.startGesture(
        center - const Offset(40, 0),
        pointer: 1,
      );
      final second = await tester.startGesture(
        center + const Offset(40, 0),
        pointer: 2,
      );
      await first.moveTo(center - const Offset(60, 0));
      await second.moveTo(center + const Offset(60, 0));
      await tester.pump();
      expect(_matrix(tester).getMaxScaleOnAxis(), greaterThan(beforePinch));
      await first.up();
      await second.up();
      await tester.pumpAndSettle();

      final beforeDrag = _matrix(tester).getTranslation().x;
      await tester.drag(find.byType(InteractiveViewer), const Offset(90, 30));
      await tester.pumpAndSettle();
      expect(_matrix(tester).getTranslation().x, greaterThan(beforeDrag));

      await _scroll(tester, -10000);
      expect(_matrix(tester).getMaxScaleOnAxis(), closeTo(6, 0.001));
      await _scroll(tester, 10000);
      expect(_matrix(tester).getMaxScaleOnAxis(), closeTo(0.2, 0.001));
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('rebuilds retain pending reads, decoded images, and transforms', (
    tester,
  ) async {
    var reads = 0;
    final pending = Completer<Uint8List>();
    await tester.pumpWidget(
      _content((_) {
        reads++;
        return pending.future;
      }),
    );
    await tester.pumpWidget(
      _content((_) {
        reads++;
        return pending.future;
      }, width: 500),
    );
    expect(reads, 1);
    pending.complete(_png);
    await _settleImage(tester);
    await _scroll(tester, -100);
    await tester.drag(find.byType(InteractiveViewer), const Offset(40, 20));
    await tester.pumpAndSettle();
    final transform = _matrix(tester);
    final element = tester.element(find.byType(InteractiveViewer));
    final frame = tester.widget<RawImage>(find.byType(RawImage)).image;
    expect(frame, isNotNull);
    for (final width in [400.0, 640.0, 450.0]) {
      await tester.pumpWidget(
        _content((_) async {
          reads++;
          return Uint8List.fromList(_png);
        }, width: width),
      );
      expect(find.byType(CircularProgressIndicator), findsNothing);
      await tester.pumpAndSettle();
      expect(reads, 1);
      expect(tester.element(find.byType(InteractiveViewer)), same(element));
      expect(tester.widget<RawImage>(find.byType(RawImage)).image, same(frame));
      expect(_matrix(tester), transform);
    }
    expect(tester.takeException(), isNull);
  });

  testWidgets('a new file ignores stale results and starts at fit scale', (
    tester,
  ) async {
    final pending = <String, Completer<Uint8List>>{};
    final reads = <String>[];
    Future<Uint8List> read(String path) {
      reads.add(path);
      return (pending[path] = Completer<Uint8List>()).future;
    }

    await tester.pumpWidget(_content(read));
    await tester.pumpWidget(_content(read, path: 'second.png'));
    pending['picture.png']!.complete(_png);
    await tester.pump();
    expect(find.byType(WorkspaceImagePreview), findsNothing);
    pending['second.png']!.complete(Uint8List.fromList(_png));
    await _settleImage(tester);
    await _scroll(tester, -100);
    expect(_matrix(tester).getMaxScaleOnAxis(), greaterThan(1));
    await tester.pumpWidget(_content(read, path: 'third.png'));
    pending['third.png']!.complete(Uint8List.fromList(_png));
    await _settleImage(tester);
    expect(_matrix(tester), Matrix4.identity());
    expect(reads, ['picture.png', 'second.png', 'third.png']);
    expect(tester.takeException(), isNull);
  });
}
