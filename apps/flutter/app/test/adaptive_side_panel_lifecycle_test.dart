import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';

void main() {
  for (final layout in SidePanelLayout.values) {
    for (final open in <bool>[false, true]) {
      testWidgets(
        'retains chat and panel while resizing $layout (open: $open)',
        (tester) async {
          tester.view.physicalSize = const Size(1600, 1000);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          final width = ValueNotifier<double>(1440);
          final scroll = ScrollController();
          final input = TextEditingController(text: 'unsent draft');
          final panelInput = TextEditingController(
            text: 'unsaved workspace edit',
          );
          const panelKey = ValueKey('workspace-editor');
          final counts = _LifecycleCounts();
          addTearDown(width.dispose);
          addTearDown(scroll.dispose);
          addTearDown(input.dispose);
          addTearDown(panelInput.dispose);
          final content = _ChatProbe(
            counts: counts,
            scroll: scroll,
            input: input,
          );
          await tester.pumpWidget(
            MaterialApp(
              home: Scaffold(
                body: Align(
                  alignment: Alignment.centerLeft,
                  child: ValueListenableBuilder<double>(
                    valueListenable: width,
                    child: content,
                    builder: (context, value, child) => SizedBox(
                      width: value,
                      child: AdaptiveSidePanel(
                        open: open,
                        layout: layout,
                        onOpenChanged: (_) {},
                        overlayTopInset: 64,
                        panel: TextField(key: panelKey, controller: panelInput),
                        animate: false,
                        child: child!,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          );
          await tester.pumpAndSettle();
          scroll.jumpTo(480);
          await tester.pumpAndSettle();
          final contentState = tester.state(find.byType(_ChatProbe));
          final panelState = open ? tester.state(find.byKey(panelKey)) : null;
          final scrollPosition = scroll.position;
          final baselineBuilds = counts.built;
          if (open) expect(panelState, isNotNull);
          for (var cycle = 0; cycle < 3; cycle++) {
            // Includes window and sidebar width changes across old boundaries.
            for (final value in <double>[
              1440,
              1024,
              1023,
              900,
              768,
              600,
              599,
              402,
              599,
              600,
              768,
              1440,
            ]) {
              width.value = value;
              await tester.pump();
              expect(tester.state(find.byType(_ChatProbe)), same(contentState));
              if (open) {
                expect(tester.state(find.byKey(panelKey)), same(panelState));
              }
              expect(panelInput.text, 'unsaved workspace edit');
              expect(scroll.position, same(scrollPosition));
              expect(scroll.offset, 480);
              expect(input.text, 'unsent draft');
              expect(counts.initialized, 1);
              expect(counts.disposed, 0);
              expect(counts.built, baselineBuilds);
            }
          }
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
          expect(counts.disposed, 1);
        },
      );
    }
  }

  testWidgets('opening the overlay preserves both content and panel state', (
    tester,
  ) async {
    final open = ValueNotifier<bool>(false);
    final counts = _LifecycleCounts();
    final scroll = ScrollController();
    final input = TextEditingController();
    addTearDown(open.dispose);
    addTearDown(scroll.dispose);
    addTearDown(input.dispose);
    final panelKey = GlobalKey();
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: SizedBox(
            width: 500,
            child: ValueListenableBuilder<bool>(
              valueListenable: open,
              child: _ChatProbe(counts: counts, scroll: scroll, input: input),
              builder: (context, value, child) => AdaptiveSidePanel(
                open: value,
                layout: SidePanelLayout.overlay,
                onOpenChanged: (value) => open.value = value,
                panel: TextField(key: panelKey),
                child: child!,
              ),
            ),
          ),
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(panelKey.currentState, isNull);
    open.value = true;
    await tester.pumpAndSettle();
    final panelState = panelKey.currentState;
    expect(panelState, isNotNull);
    for (var cycle = 0; cycle < 3; cycle++) {
      open.value = true;
      await tester.pumpAndSettle();
      expect(panelKey.currentState, same(panelState));
      open.value = false;
      await tester.pumpAndSettle();
      expect(open.value, isFalse);
      expect(panelKey.currentState, same(panelState));
      expect(counts.initialized, 1);
      expect(counts.disposed, 0);
    }
    expect(tester.takeException(), isNull);
    await tester.pumpWidget(const SizedBox.shrink());
  });
}

class _LifecycleCounts {
  int initialized = 0;
  int disposed = 0;
  int built = 0;
}

class _ChatProbe extends StatefulWidget {
  const _ChatProbe({
    required this.counts,
    required this.scroll,
    required this.input,
  });

  final _LifecycleCounts counts;
  final ScrollController scroll;
  final TextEditingController input;

  @override
  State<_ChatProbe> createState() => _ChatProbeState();
}

class _ChatProbeState extends State<_ChatProbe> {
  @override
  void initState() {
    super.initState();
    widget.counts.initialized++;
  }

  @override
  void dispose() {
    widget.counts.disposed++;
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    widget.counts.built++;
    return Column(
      children: <Widget>[
        Expanded(
          child: ListView.builder(
            controller: widget.scroll,
            itemExtent: 48,
            itemCount: 100,
            itemBuilder: (context, index) => Text('Message $index'),
          ),
        ),
        TextField(controller: widget.input),
      ],
    );
  }
}
