import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/part/CustomXmlRenderer.dart';

void main() {
  testWidgets('preserves literal partial fences and replaces closed fences', (
    tester,
  ) async {
    await tester.pumpWidget(_surface('before ```code'));
    expect(find.text('before ```code'), findsOneWidget);
    await tester.pumpWidget(_surface('before ```code``` after'));
    expect(find.text('before [代码] after'), findsOneWidget);
    await tester.pumpWidget(_surface('replacement body'));
    expect(find.text('replacement body'), findsOneWidget);
    await tester.pumpWidget(_surface('replacement body', streaming: false));
    expect(find.text('replacement body'), findsOneWidget);
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'catches up after streaming while the thinking body is expanded',
    (tester) async {
      await tester.pumpWidget(_surface('initial text'));
      await tester.tap(find.text('Thinking Process').first);
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('initial text'), findsNothing);
      await tester.pumpWidget(_surface('initial text\n# appended heading'));
      await tester.tap(find.text('Thinking Process').first);
      await tester.pump(const Duration(milliseconds: 300));
      expect(find.text('initial text appended heading'), findsOneWidget);
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('keeps the limited preview scrolling to the latest content', (
    tester,
  ) async {
    final source = 'planning more work ' * 100;
    await tester.pumpWidget(_surface(source));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    final preview = source.trim();
    expect(find.text(preview.substring(preview.length - 1000)), findsOneWidget);
    final controller = tester
        .widget<SingleChildScrollView>(find.byType(SingleChildScrollView))
        .controller!;
    expect(
      controller.offset,
      closeTo(controller.position.maxScrollExtent, 0.01),
    );
    await tester.pumpWidget(_surface('$source latest thoughts'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 200));
    expect(
      controller.offset,
      closeTo(controller.position.maxScrollExtent, 0.01),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
    expect(tester.takeException(), isNull);
  });
  testWidgets('waits for layout before scrolling the thinking preview', (
    tester,
  ) async {
    final source = 'planning more work ' * 100;
    await tester.pumpWidget(_surface(source, deferLayout: true));
    final controller = tester
        .widget<SingleChildScrollView>(
          find.byType(SingleChildScrollView, skipOffstage: false),
        )
        .controller!;
    expect(controller.hasClients, isTrue);
    expect(controller.position.hasContentDimensions, isFalse);
    expect(tester.takeException(), isNull);

    // No new text arrives when the previously deferred subtree is laid out.
    await tester.pumpWidget(_surface(source));
    await tester.pump(); // Retry after the scroll metrics become available.
    await tester.pump(); // Start the scroll animation's ticker.
    await tester.pump(const Duration(milliseconds: 200));
    expect(controller.position.maxScrollExtent, greaterThan(0));
    expect(
      controller.offset,
      closeTo(controller.position.maxScrollExtent, 0.01),
    );
    await tester.pumpWidget(const SizedBox.shrink());
    expect(tester.takeException(), isNull);
  });

  for (final (sourceLength, addition) in [
    (990, 'bcdefghijklm'),
    (1000, 'bcdefghijklm'),
    (1400, 'bcdefghijklm'),
    (1400, 'aaaaaaaaaaaa'),
  ]) {
    testWidgets('animates incoming text at $sourceLength units: $addition', (
      tester,
    ) async {
      final source = 'a' * sourceLength;
      await tester.pumpWidget(_surface(source));
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 200));
      final controller = tester
          .widget<SingleChildScrollView>(find.byType(SingleChildScrollView))
          .controller!;
      expect(
        controller.offset,
        closeTo(controller.position.maxScrollExtent, 0.01),
      );

      await tester.pumpWidget(_surface('$source$addition'));
      final start = controller.offset;
      final end = controller.position.maxScrollExtent;
      expect(
        start,
        lessThan(end - 1),
        reason: 'new text must enter through the animation',
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 64));
      expect(controller.offset, greaterThan(start));
      expect(controller.offset, lessThan(end));
      await tester.pump(const Duration(milliseconds: 160));
      expect(controller.offset, closeTo(end, 0.01));
      await tester.pumpWidget(const SizedBox.shrink());
      expect(tester.takeException(), isNull);
    });
  }
}

Widget _surface(
  String body, {
  bool streaming = true,
  bool deferLayout = false,
}) => MaterialApp(
  locale: const Locale('en'),
  localizationsDelegates: AppLocalizations.localizationsDelegates,
  supportedLocales: AppLocalizations.supportedLocales,
  home: Scaffold(
    body: _DeferredLayout(
      deferred: deferLayout,
      child: SizedBox(
        width: 360,
        child: CustomXmlRenderer(
          xmlContent: '<think>$body${streaming ? '' : '</think>'}',
          isStreaming: streaming,
          xmlStream: streaming ? const Stream<String>.empty() : null,
          textColor: Colors.black,
          splitMarkdownContent: (_) async => [],
        ),
      ),
    ),
  ),
);

// Models a built scrollable whose parent has not laid out its subtree yet.
class _DeferredLayout extends SingleChildRenderObjectWidget {
  const _DeferredLayout({required this.deferred, required super.child});

  final bool deferred;

  @override
  RenderObject createRenderObject(BuildContext context) =>
      _RenderDeferredLayout(deferred);

  @override
  void updateRenderObject(
    BuildContext context,
    _RenderDeferredLayout renderObject,
  ) {
    if (renderObject.deferred == deferred) return;
    renderObject.deferred = deferred;
    renderObject.markNeedsLayout();
  }
}

class _RenderDeferredLayout extends RenderProxyBox {
  _RenderDeferredLayout(this.deferred);

  bool deferred;

  @override
  void performLayout() {
    if (deferred) {
      size = constraints.smallest;
      return;
    }
    super.performLayout();
  }

  @override
  void visitChildrenForSemantics(RenderObjectVisitor visitor) {
    if (!deferred) super.visitChildrenForSemantics(visitor);
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    if (!deferred) super.paint(context, offset);
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) =>
      !deferred && super.hitTestChildren(result, position: position);
}
