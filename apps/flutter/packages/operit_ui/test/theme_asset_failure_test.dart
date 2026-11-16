import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit_ui/operit_ui.dart';

const _surface = Color(0xFF123456);
const _bubbleColor = Color(0xFFBADA55);
const _goodColor = Color(0xFF339966);
const _otherColor = Color(0xFF3355CC);

/// Exercises actual image decoding and the resource boundary, not native paths.
void main() {
  TestWidgetsFlutterBinding.ensureInitialized();

  tearDown(() {
    PaintingBinding.instance.imageCache.clear();
    PaintingBinding.instance.imageCache.clearLiveImages();
  });

  for (final reference in <String>[
    'missing.png',
    'empty.png',
    'broken.png',
    'broken.svg',
    'unreadable.png',
  ]) {
    testWidgets('$reference leaves the background surface visible', (
      tester,
    ) async {
      final boundary = GlobalKey();
      final diagnostics = ThemeAssetDiagnostics();
      final resolver = _MutableAssets(<String, Uint8List>{
        'empty.png': Uint8List(0),
        'broken.png': Uint8List.fromList(<int>[137, 80, 78, 71, 13, 10, 26, 10]),
        'broken.svg': Uint8List.fromList(utf8.encode('not an SVG document')),
      });
      final cause = StateError('Storage became unreadable');
      final stack = StackTrace.fromString('storage-read-origin');
      resolver.failures['unreadable.png'] = (cause, stack);
      addTearDown(diagnostics.dispose);

      await tester.pumpWidget(
        _imageScene(resolver, diagnostics, reference, boundary),
      );
      await _finishImageWork(tester);

      expect(tester.takeException(), isNull);
      expect(diagnostics.failures, hasLength(1));
      final failure = diagnostics.failures.single;
      expect(failure.reference, reference);
      expect(failure.purpose, ThemeAssetPurpose.background);
      expect(failure.retainedPrevious, isFalse);
      expect(failure.details, contains(reference));
      expect(failure.requestStackTrace.toString(), isNotEmpty);
      if (reference == 'unreadable.png') {
        expect(failure.stage, ThemeAssetStage.read);
        expect(failure.cause, same(cause));
        expect(failure.originalStackTrace.toString(), 'storage-read-origin');
      } else if (reference.startsWith('broken.')) {
        expect(failure.stage, ThemeAssetStage.decode);
      }
      await _expectPixel(tester, boundary, _surface);
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  testWidgets('background and bubble share one in-flight read', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{});
    final pending = Completer<Uint8List>();
    final boundary = GlobalKey();
    resolver.pending['shared.svg'] = pending;
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(_composition(
      resolver,
      diagnostics,
      background: 'shared.svg',
      bubble: 'shared.svg',
      bubbleBoundary: boundary,
    ));
    expect(resolver.reads['shared.svg'], 1);
    pending.complete(_svg(_goodColor));
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);
    expect(resolver.reads['shared.svg'], 1);
    expect(tester.widget<RawImage>(find.byType(RawImage)).image, isNotNull);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('a failed background keeps a valid bubble and the input draft', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{
      'bubble.svg': _svg(_goodColor),
    });
    final text = TextEditingController();
    addTearDown(diagnostics.dispose);
    addTearDown(text.dispose);
    await tester.pumpWidget(
      _composition(
        resolver,
        diagnostics,
        background: 'missing-background.svg',
        bubble: 'bubble.svg',
        bubbleBoundary: boundary,
        text: text,
      ),
    );
    await _finishImageWork(tester);
    await tester.enterText(find.byType(TextField), 'Unsaved message');
    final inputState = tester.state(find.byType(EditableText));

    expect(find.text('Visible message'), findsOneWidget);
    await _expectPixel(tester, boundary, _goodColor);
    expect(diagnostics.failures, hasLength(1));
    expect(diagnostics.failures.single.reference, 'missing-background.svg');

    resolver.bytes['missing-background.svg'] = _svg(_otherColor);
    diagnostics.retry('missing-background.svg');
    await _finishImageWork(tester);
    expect(diagnostics.failures, isEmpty);
    expect(tester.state(find.byType(EditableText)), same(inputState));
    expect(text.text, 'Unsaved message');
    expect(resolver.reads['bubble.svg'], 1);
    expect(resolver.reads['missing-background.svg'], 2);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  for (final transparent in <bool>[false, true]) {
    testWidgets(
      'failed bubble uses its fallback color, transparent=$transparent',
      (tester) async {
        final boundary = GlobalKey();
        final diagnostics = ThemeAssetDiagnostics();
        final resolver = _MutableAssets(<String, Uint8List>{
          'background.svg': _svg(_goodColor),
        });
        addTearDown(diagnostics.dispose);
        await tester.pumpWidget(
          _composition(
            resolver,
            diagnostics,
            background: 'background.svg',
            bubble: 'missing-bubble.svg',
            bubbleBoundary: boundary,
            transparent: transparent,
          ),
        );
        await _finishImageWork(tester);

        expect(find.text('Visible message'), findsOneWidget);
        await _expectPixel(tester, boundary, _bubbleColor);
        expect(diagnostics.failures, hasLength(1));
        final failure = diagnostics.failures.single;
        expect(failure.reference, 'missing-bubble.svg');
        expect(failure.purpose, ThemeAssetPurpose.bubble);
        expect(resolver.reads['background.svg'], 1);
        final background = tester.widget<RawImage>(
          find.descendant(
            of: find.byType(ResolvedThemeImage),
            matching: find.byType(RawImage),
          ),
        );
        expect(background.image, isNotNull);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }

  testWidgets('even a transparent bubble color has an opaque resource fallback', (tester) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{});
    final boundary = GlobalKey();
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(_bubbleScene(
      resolver,
      diagnostics,
      const BubbleImageStyle(imagePath: 'missing.svg'),
      boundary,
      color: _bubbleColor.withValues(alpha: 0),
    ));
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _bubbleColor);
    expect(find.text('Visible message'), findsOneWidget);
    expect(diagnostics.failures, hasLength(1));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('background retains its successful image while a replacement fails', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{
      'first.svg': _svg(_goodColor),
    });
    final replacement = Completer<Uint8List>();
    resolver.pending['replacement.svg'] = replacement;
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'first.svg', boundary),
    );
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);
    final imageState = tester.state(_backgroundImages());

    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'replacement.svg', boundary),
    );
    await _expectPixel(tester, boundary, _goodColor);
    replacement.completeError(StateError('Replacement vanished'));
    await _finishImageWork(tester);

    await _expectPixel(tester, boundary, _goodColor);
    expect(tester.state(_backgroundImages()), same(imageState));
    expect(diagnostics.failures.single.retainedPrevious, isTrue);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('a retained bubble keeps the crop belonging to the old image', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final png = await tester.runAsync(_splitPng);
    final resolver = _MutableAssets(<String, Uint8List>{'first.png': png!});
    addTearDown(diagnostics.dispose);
    Widget scene(BubbleImageStyle style) => _bubbleScene(
      resolver,
      diagnostics,
      style,
      boundary,
    );
    await tester.pumpWidget(
      scene(
        const BubbleImageStyle(
          imagePath: 'first.png',
          cropRightRatio: 0.45,
          renderMode: ThemePreferenceDefaults.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
        ),
      ),
    );
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _otherColor);

    await tester.pumpWidget(
      scene(
        const BubbleImageStyle(
          imagePath: 'missing.png',
          cropLeftRatio: 0.45,
          renderMode: ThemePreferenceDefaults.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
        ),
      ),
    );
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _otherColor);
    expect(diagnostics.failures.single.retainedPrevious, isTrue);
    expect(find.text('Visible message'), findsOneWidget);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('late background success and failure cannot replace the latest image', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{
      'current.svg': _svg(_goodColor),
    });
    final oldSuccess = Completer<Uint8List>();
    final oldFailure = Completer<Uint8List>();
    resolver.pending['old-success.svg'] = oldSuccess;
    resolver.pending['old-failure.svg'] = oldFailure;
    addTearDown(diagnostics.dispose);

    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'old-success.svg', boundary),
    );
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'old-failure.svg', boundary),
    );
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'current.svg', boundary),
    );
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);

    oldSuccess.complete(_svg(_otherColor));
    oldFailure.completeError(StateError('Obsolete read failed'));
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('unmounting ignores late image reads and their failures', (
    tester,
  ) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{});
    final pending = Completer<Uint8List>();
    resolver.pending['pending.svg'] = pending;
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'pending.svg', GlobalKey()),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    pending.completeError(StateError('Read finished after disposal'));
    await _finishImageWork(tester);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
  });

  testWidgets('obsolete bubble reads cannot overwrite the current decoration', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{
      'current.svg': _svg(_goodColor),
    });
    final oldSuccess = Completer<Uint8List>();
    final oldFailure = Completer<Uint8List>();
    resolver.pending['old-success.svg'] = oldSuccess;
    resolver.pending['old-failure.svg'] = oldFailure;
    addTearDown(diagnostics.dispose);
    Widget scene(String reference) => _bubbleScene(
      resolver,
      diagnostics,
      BubbleImageStyle(imagePath: reference),
      boundary,
    );
    await tester.pumpWidget(scene('old-success.svg'));
    await tester.pumpWidget(scene('old-failure.svg'));
    await tester.pumpWidget(scene('current.svg'));
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);
    oldSuccess.complete(_svg(_otherColor));
    oldFailure.completeError(StateError('Obsolete bubble failed'));
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _goodColor);
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('listeners share a failed read and its same-path retry', (
    tester,
  ) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{});
    final pending = Completer<Uint8List>();
    resolver.pending['shared.svg'] = pending;
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(
      ThemeAssetScope(
        resolver: resolver,
        diagnostics: diagnostics,
        child: MaterialApp(
          home: Row(
            children: <Widget>[
              for (var index = 0; index < 2; index++)
                SizedBox(
                  width: 120,
                  height: 80,
                  child: ResolvedThemeImage(
                    resolver: resolver,
                    reference: 'shared.svg',
                    fit: BoxFit.cover,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
    pending.completeError(StateError('Shared read failed'));
    await _finishImageWork(tester);
    expect(resolver.reads['shared.svg'], 1);
    expect(diagnostics.failures, hasLength(1));
    resolver.pending.remove('shared.svg');
    resolver.bytes['shared.svg'] = _svg(_goodColor);
    diagnostics.retry('shared.svg');
    await _finishImageWork(tester);
    expect(resolver.reads['shared.svg'], 2);
    expect(diagnostics.failures, isEmpty);
    expect(find.byType(RawImage), findsNWidgets(2));
    for (final image in tester.widgetList<RawImage>(find.byType(RawImage))) {
      expect(image.image, isNotNull);
    }
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('one successful bubble cannot hide another bubble needing retry', (
    tester,
  ) async {
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _TransientBubbleAssets();
    final firstBoundary = GlobalKey();
    final secondBoundary = GlobalKey();
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(
      ThemeAssetScope(
        resolver: resolver,
        diagnostics: diagnostics,
        child: MaterialApp(
          home: Row(
            children: <Widget>[
              for (final boundary in <GlobalKey>[
                firstBoundary,
                secondBoundary,
              ])
                RepaintBoundary(
                  key: boundary,
                  child: const SizedBox(
                    width: 180,
                    height: 80,
                    child: BubbleSurface(
                      color: _bubbleColor,
                      borderRadius: BorderRadius.zero,
                      imageStyle: BubbleImageStyle(imagePath: 'transient.svg'),
                      child: Align(
                        alignment: Alignment.bottomRight,
                        child: Text('Visible message', style: TextStyle(fontSize: 12)),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
    await _finishImageWork(tester);
    await _expectPixel(tester, firstBoundary, _bubbleColor);
    expect(diagnostics.failures, hasLength(1));
    expect(diagnostics.failures.single.reference, 'transient.svg');
    final readsBeforeRetry = resolver.reads['transient.svg']!;
    diagnostics.retry('transient.svg');
    await _finishImageWork(tester);
    await _expectPixel(tester, firstBoundary, _goodColor);
    await _expectPixel(tester, secondBoundary, _goodColor);
    expect(resolver.reads['transient.svg'], greaterThan(readsBeforeRetry));
    expect(diagnostics.failures, isEmpty);
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('the same bubble reference recovers without rereading a good background', (
    tester,
  ) async {
    final boundary = GlobalKey();
    final diagnostics = ThemeAssetDiagnostics();
    final resolver = _MutableAssets(<String, Uint8List>{
      'background.svg': _svg(_goodColor),
    });
    addTearDown(diagnostics.dispose);
    await tester.pumpWidget(
      _composition(
        resolver,
        diagnostics,
        background: 'background.svg',
        bubble: 'later.svg',
        bubbleBoundary: boundary,
      ),
    );
    await _finishImageWork(tester);
    expect(diagnostics.failures.single.reference, 'later.svg');
    final backgroundState = tester.state(_backgroundImages());

    resolver.bytes['later.svg'] = _svg(_otherColor);
    diagnostics.retry('later.svg');
    await _finishImageWork(tester);
    await _expectPixel(tester, boundary, _otherColor);
    expect(diagnostics.failures, isEmpty);
    expect(resolver.reads['background.svg'], 1);
    expect(resolver.reads['later.svg'], 2);
    expect(tester.state(_backgroundImages()), same(backgroundState));
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });

  testWidgets('prepared image clones are released on replacement and disposal', (
    tester,
  ) async {
    final source = await tester.runAsync(() async {
      return decodeThemeImageBytes('source.svg', _svg(_goodColor));
    });
    final resolver = _PreparedAssets(source!);
    final diagnostics = ThemeAssetDiagnostics();
    addTearDown(source.dispose);
    addTearDown(diagnostics.dispose);
    final boundary = GlobalKey();
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'first.svg', boundary),
    );
    final first = resolver.clones.single;
    await tester.pumpWidget(
      _imageScene(resolver, diagnostics, 'second.svg', boundary),
    );
    expect(resolver.clones, hasLength(2));
    _expectDisposed(first);
    final second = resolver.clones.last;
    await tester.pumpWidget(const SizedBox.shrink());
    _expectDisposed(second);
    expect(tester.takeException(), isNull);
  });

  test('diagnostics deduplicate failures and retry only failed references', () {
    final diagnostics = ThemeAssetDiagnostics();
    addTearDown(diagnostics.dispose);
    final failure = ThemeAssetFailure.capture(
      reference: 'missing.svg',
      purpose: ThemeAssetPurpose.bubble,
      stage: ThemeAssetStage.read,
      kind: ThemeAssetFailureKind.missing,
      cause: StateError('Missing bytes'),
      originalStackTrace: StackTrace.empty,
      requestStackTrace: StackTrace.fromString('request-origin'),
    );
    diagnostics.report(failure);
    diagnostics.report(failure);
    expect(diagnostics.failures, hasLength(1));
    diagnostics.retry('valid.svg');
    expect(diagnostics.retryEpoch('valid.svg'), 0);
    diagnostics.retry('missing.svg');
    expect(diagnostics.retryEpoch('missing.svg'), 1);
    expect(diagnostics.failures, isEmpty);
    diagnostics.report(failure);
    expect(diagnostics.failures, hasLength(1));
  });

  test('diagnostics retain a shared failure until every owner recovers', () {
    final diagnostics = ThemeAssetDiagnostics();
    addTearDown(diagnostics.dispose);
    final first = Object();
    final second = Object();
    final failure = ThemeAssetFailure.capture(
      reference: 'shared.svg',
      purpose: ThemeAssetPurpose.bubble,
      stage: ThemeAssetStage.read,
      kind: ThemeAssetFailureKind.missing,
      cause: StateError('Missing bytes'),
      originalStackTrace: StackTrace.empty,
      requestStackTrace: StackTrace.empty,
      retainedPrevious: true,
    );
    diagnostics.report(failure, owner: first);
    diagnostics.report(failure.withContext(retainedPrevious: false), owner: second);
    expect(diagnostics.failures.single.retainedPrevious, isFalse);
    diagnostics.resolve('shared.svg', purpose: ThemeAssetPurpose.bubble, owner: second);
    expect(diagnostics.failures.single.retainedPrevious, isTrue);
    diagnostics.release(first);
    expect(diagnostics.failures, isEmpty);
  });

  testWidgets('missing ThemeAssetScope remains a programming error', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MaterialApp(
        home: BubbleSurface(
          color: _bubbleColor,
          borderRadius: BorderRadius.zero,
          imageStyle: BubbleImageStyle(imagePath: 'missing.svg'),
          child: SizedBox(width: 120, height: 80),
        ),
      ),
    );
    expect(tester.takeException(), isA<StateError>());
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

class _MutableAssets implements ThemeAssetResolver {
  _MutableAssets(this.bytes);

  final Map<String, Uint8List> bytes;
  final Map<String, int> reads = <String, int>{};
  final Map<String, Completer<Uint8List>> pending =
      <String, Completer<Uint8List>>{};
  final Map<String, (Object, StackTrace)> failures =
      <String, (Object, StackTrace)>{};

  @override
  Future<Uint8List> readBytes(String reference) async {
    reads.update(reference, (count) => count + 1, ifAbsent: () => 1);
    final failure = failures[reference];
    if (failure != null) Error.throwWithStackTrace(failure.$1, failure.$2);
    final delayed = pending[reference];
    if (delayed != null) return delayed.future;
    final content = bytes[reference];
    if (content == null) throw StateError('Missing theme asset: $reference');
    return content;
  }
}

class _PreparedAssets implements ThemeDecodedImageResolver {
  _PreparedAssets(this.source);

  final ui.Image source;
  final List<ui.Image> clones = <ui.Image>[];

  @override
  Future<Uint8List> readBytes(String reference) =>
      throw StateError('Prepared images must not reread bytes');

  @override
  ui.Image cloneImage(String reference) => tryCloneImage(reference)!;

  @override
  ui.Image? tryCloneImage(String reference) {
    final clone = source.clone();
    clones.add(clone);
    return clone;
  }
}

/// A brief read failure affects one consumer while the bytes are already usable.
class _TransientBubbleAssets extends _MutableAssets {
  _TransientBubbleAssets() : super(<String, Uint8List>{});

  @override
  Future<Uint8List> readBytes(String reference) async {
    reads.update(reference, (count) => count + 1, ifAbsent: () => 1);
    if (reads[reference] == 1) {
      throw StateError('Transient theme read failure: $reference');
    }
    return _svg(_goodColor);
  }
}

Widget _imageScene(
  ThemeAssetResolver resolver,
  ThemeAssetDiagnostics diagnostics,
  String reference,
  GlobalKey boundary,
) => ThemeAssetScope(
  resolver: resolver,
  diagnostics: diagnostics,
  child: MaterialApp(
    home: Center(
      child: RepaintBoundary(
        key: boundary,
        child: SizedBox(
          width: 120,
          height: 80,
          child: ColoredBox(
            color: _surface,
            child: ResolvedThemeImage(
              resolver: resolver,
              reference: reference,
              fit: BoxFit.fill,
            ),
          ),
        ),
      ),
    ),
  ),
);

Widget _bubbleScene(
  ThemeAssetResolver resolver,
  ThemeAssetDiagnostics diagnostics,
  BubbleImageStyle style,
  GlobalKey boundary, {
  Color color = _bubbleColor,
}) => ThemeAssetScope(
  resolver: resolver,
  diagnostics: diagnostics,
  child: MaterialApp(
    home: Center(
      child: RepaintBoundary(
        key: boundary,
        child: SizedBox(
          width: 180,
          height: 80,
          child: BubbleSurface(
            color: color,
            borderRadius: BorderRadius.zero,
            imageStyle: style,
            child: const Align(
              alignment: Alignment.bottomRight,
              child: Text('Visible message', style: TextStyle(fontSize: 12)),
            ),
          ),
        ),
      ),
    ),
  ),
);

Widget _composition(
  ThemeAssetResolver resolver,
  ThemeAssetDiagnostics diagnostics, {
  required String background,
  required String bubble,
  required GlobalKey bubbleBoundary,
  bool transparent = false,
  TextEditingController? text,
}) {
  final snapshot = ThemePreferenceSnapshot.fromJson(<String, Object?>{
    ...ThemePreferenceDefaults.snapshot.toJson(),
    'useBackgroundImage': true,
    'backgroundImageUri': background,
  });
  return ThemeAssetScope(
    resolver: resolver,
    diagnostics: diagnostics,
    child: MaterialApp(
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue).copyWith(
          surface: _surface,
        ),
      ),
      builder: (context, child) => ThemeBackground(
        themePreferenceSnapshot: snapshot,
        media: ResolvedThemeImage(
          resolver: resolver,
          reference: background,
          fit: BoxFit.cover,
        ),
        child: child!,
      ),
      home: Scaffold(
        backgroundColor: Colors.transparent,
        body: Column(
          children: <Widget>[
            RepaintBoundary(
              key: bubbleBoundary,
              child: SizedBox(
                width: 240,
                height: 100,
                child: BubbleSurface(
                  color: _bubbleColor,
                  borderRadius: BorderRadius.zero,
                  imageStyle: BubbleImageStyle(imagePath: bubble),
                  transparentSurface: transparent,
                  child: const Align(
                    alignment: Alignment.bottomRight,
                    child: Text('Visible message', style: TextStyle(fontSize: 12)),
                  ),
                ),
              ),
            ),
            if (text != null) TextField(controller: text),
          ],
        ),
      ),
    ),
  );
}

Uint8List _svg(Color color) => Uint8List.fromList(
  utf8.encode(
    '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" '
    'viewBox="0 0 16 16"><rect width="16" height="16" '
    'fill="#${color.toARGB32().toRadixString(16).substring(2)}"/></svg>',
  ),
);

Future<Uint8List> _splitPng() async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawRect(const Rect.fromLTWH(0, 0, 16, 16), Paint()..color = _otherColor);
  canvas.drawRect(const Rect.fromLTWH(16, 0, 16, 16), Paint()..color = _goodColor);
  final picture = recorder.endRecording();
  try {
    final image = await picture.toImage(32, 16);
    try {
      final encoded = (await image.toByteData(format: ui.ImageByteFormat.png))!;
      return encoded.buffer.asUint8List(encoded.offsetInBytes, encoded.lengthInBytes);
    } finally {
      image.dispose();
    }
  } finally {
    picture.dispose();
  }
}

Future<void> _finishImageWork(WidgetTester tester) async {
  await tester.runAsync(() async {
    await Future<void>.delayed(const Duration(milliseconds: 50));
  });
  await tester.pumpAndSettle();
}

Future<void> _expectPixel(
  WidgetTester tester,
  GlobalKey boundary,
  Color expected,
) async {
  final render = boundary.currentContext!.findRenderObject()! as RenderRepaintBoundary;
  final actual = await tester.runAsync(() async {
    final image = await render.toImage(pixelRatio: 1);
    try {
      final data = (await image.toByteData(format: ui.ImageByteFormat.rawRgba))!;
      final offset = ((image.height ~/ 4) * image.width + image.width ~/ 2) * 4;
      return data.buffer.asUint8List(data.offsetInBytes + offset, 4).toList();
    } finally {
      image.dispose();
    }
  });
  final argb = expected.toARGB32();
  expect(actual, <int>[(argb >> 16) & 255, (argb >> 8) & 255, argb & 255, 255]);
}

void _expectDisposed(ui.Image image) {
  ui.Image? unexpectedClone;
  try {
    expect(() => unexpectedClone = image.clone(), throwsA(anything));
  } finally {
    unexpectedClone?.dispose();
  }
}

Finder _backgroundImages() => find.byWidgetPredicate(
  (widget) => widget is ResolvedThemeImage && widget.purpose == ThemeAssetPurpose.background,
);
