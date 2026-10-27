import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/markdown/MarkdownTextReveal.dart';
import 'package:operit2/ui/common/markdown/StreamMarkdownRenderer.dart';

class _CountingParagraph extends RenderParagraph {
  _CountingParagraph(super.text) : super(textDirection: TextDirection.ltr);

  int layouts = 0;
  int semanticsUpdates = 0;

  @override
  void performLayout() {
    layouts++;
    super.performLayout();
  }

  @override
  void describeSemanticsConfiguration(SemanticsConfiguration config) {
    semanticsUpdates++;
    super.describeSemanticsConfiguration(config);
  }
}

class _Paragraph extends LeafRenderObjectWidget {
  const _Paragraph(this.span);
  final TextSpan span;

  @override
  _CountingParagraph createRenderObject(BuildContext context) =>
      _CountingParagraph(span);
}

Widget _host({
  required Animation<double> animation,
  required String text,
  Widget? paragraph,
  TextDirection direction = TextDirection.ltr,
}) => MaterialApp(
  home: Directionality(
    textDirection: direction,
    child: Center(
      child: SizedBox(
        width: 220,
        child: MarkdownTextReveal(
          animation: animation,
          text: text,
          revealOffset: 0,
          nextRevealOffset: null,
          showCursor: true,
          paragraph: paragraph ?? Text(text),
          cursor: const StreamingCursor(blink: false),
        ),
      ),
    ),
  ),
);

void main() {
  testWidgets(
    'reveal frames move the cursor without relayout or semantics churn',
    (tester) async {
      final semantics = tester.ensureSemantics();
        final controller = AnimationController(
        vsync: tester,
        duration: const Duration(milliseconds: 200),
        upperBound: 16,
      );
      addTearDown(controller.dispose);
      const text = 'Hello streaming!';
      await tester.pumpWidget(
        _host(
          animation: controller,
          text: text,
          paragraph: const _Paragraph(TextSpan(text: text)),
        ),
      );
      final paragraph = tester.renderObject<_CountingParagraph>(
        find.byType(_Paragraph),
      );
      final layouts = paragraph.layouts;
      final semanticUpdates = paragraph.semanticsUpdates;
      final initialCursor = tester.getTopLeft(find.byType(StreamingCursor));
      controller.forward();
      await tester.pump();
      for (var i = 0; i < 6; i++) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      expect(
        tester.getTopLeft(find.byType(StreamingCursor)).dx,
        greaterThan(initialCursor.dx),
      );
      expect(paragraph.layouts, layouts);
      expect(paragraph.semanticsUpdates, semanticUpdates);
      expect(tester.getSemantics(find.byType(_Paragraph)).label, text);
      await tester.pump(const Duration(milliseconds: 220));
      await tester.pump();
      expect(tester.binding.hasScheduledFrame, isFalse);
      expect(tester.binding.transientCallbackCount, 0);
    semantics.dispose();
    },
  );

  testWidgets('keeps selection, inline widgets and link hit testing intact', (
    tester,
  ) async {
    var taps = 0;
    final recognizer = TapGestureRecognizer()..onTap = () => taps++;
    addTearDown(recognizer.dispose);
    final controller = AnimationController(vsync: tester, upperBound: 20);
    addTearDown(controller.dispose);
    final span = TextSpan(
      style: const TextStyle(color: Colors.black, fontSize: 16),
      children: [
        TextSpan(text: 'link', recognizer: recognizer),
        const WidgetSpan(
          child: RepaintBoundary(child: SizedBox(width: 18, height: 18)),
        ),
        const TextSpan(text: '👨‍👩‍👧‍👦e\u0301 end'),
      ],
    );
    final text = span.toPlainText(includeSemanticsLabels: false);
    await tester.pumpWidget(
      _host(
        animation: controller,
        text: text,
        paragraph: SelectionArea(child: Text.rich(span)),
      ),
    );
    final paragraph = tester.renderObject<RenderParagraph>(
      find.byType(RichText).first,
    );
    final linkPosition =
        tester.getTopLeft(find.byType(RichText).first) + const Offset(5, 8);
    await tester.tapAt(linkPosition);
    expect(taps, 0);
    controller.value = 4.5;
    await tester.pump();
    await tester.tapAt(linkPosition);
    expect(taps, 1);
    expect(paragraph.text.toPlainText(), text);
    for (final value in [5.5, 6.5, 7.5, 20.0]) {
      controller.value = value;
      await tester.pump();
      expect(tester.takeException(), isNull);
    }
    expect(find.byType(SelectionArea), findsOneWidget);
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets('preserves the paragraph width and RTL alignment', (
    tester,
  ) async {
    final controller = AnimationController(vsync: tester, upperBound: 20)
      ..value = 20;
    addTearDown(controller.dispose);
    await tester.pumpWidget(
      _host(animation: controller, text: 'مرحبا', direction: TextDirection.rtl),
    );
    final paragraph = tester.renderObject<RenderParagraph>(
      find.byType(RichText),
    );
    expect(paragraph.size.width, 220);
    final cursor = tester.getTopLeft(find.byType(StreamingCursor));
    expect(cursor.dx, greaterThan(tester.getTopLeft(find.byType(RichText)).dx));
    expect(tester.takeException(), isNull);
  });
}
