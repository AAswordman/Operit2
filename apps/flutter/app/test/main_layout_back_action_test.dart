import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/common/components/AdaptiveSidePanel.dart';
import 'package:operit2/ui/main/MainLayoutController.dart';

/// Exercises surface dismissal through Flutter's system back dispatch.
void main() {
  for (final width in <double>[402, 900]) {
    testWidgets('back closes the overlay before exit at width $width', (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 900);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final key = GlobalKey<_BackFixtureState>();
      await tester.pumpWidget(_BackFixture(key: key));
      await tester.pumpAndSettle();
      final state = key.currentState!;
      final editor = tester.state(find.byKey(const ValueKey('editor')));
      await tester.enterText(find.byType(TextField), 'unsaved workspace edit');

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(state.open, isFalse);
      expect(state.exitRequests, 0);
      expect(state.layout.hasBackAction, isFalse);

      state.setOpen(true);
      await tester.pumpAndSettle();
      expect(tester.state(find.byKey(const ValueKey('editor'))), same(editor));
      expect(find.text('unsaved workspace edit'), findsOneWidget);
      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(state.open, isFalse);
      expect(state.exitRequests, 0);

      await tester.binding.handlePopRoute();
      await tester.pumpAndSettle();
      expect(state.exitRequests, 1);
      expect(tester.takeException(), isNull);
    });
  }

  testWidgets('dialog and drawer close before the workspace overlay', (
    tester,
  ) async {
    final key = GlobalKey<_BackFixtureState>();
    await tester.pumpWidget(_BackFixture(key: key));
    await tester.pumpAndSettle();
    final state = key.currentState!;
    final dialog = showDialog<void>(
      context: tester.element(find.byType(TextField)),
      builder: (_) => const AlertDialog(content: Text('Workspace dialog')),
    );
    await tester.pumpAndSettle();

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    await dialog;
    expect(find.byType(AlertDialog), findsNothing);
    expect(state.open, isTrue);
    expect(state.exitRequests, 0);

    state.drawerOpen = true;
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(state.drawerOpen, isFalse);
    expect(state.open, isTrue);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(state.open, isFalse);
    expect(state.exitRequests, 0);
  });

  testWidgets('retained inactive pages release their back action', (
    tester,
  ) async {
    final key = GlobalKey<_BackFixtureState>();
    await tester.pumpWidget(_BackFixture(key: key));
    await tester.pumpAndSettle();
    final state = key.currentState!;
    state.setCurrent(false);
    await tester.pumpAndSettle();
    expect(state.layout.hasBackAction, isFalse);

    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(state.open, isTrue);
    expect(state.exitRequests, 1);
    state.setCurrent(true);
    await tester.pumpAndSettle();
    expect(state.layout.hasBackAction, isTrue);
    await tester.binding.handlePopRoute();
    await tester.pumpAndSettle();
    expect(state.open, isFalse);
    expect(state.exitRequests, 1);
  });

  testWidgets('outgoing surface disposal preserves the next page action', (
    tester,
  ) async {
    final key = GlobalKey<_BackFixtureState>();
    await tester.pumpWidget(_BackFixture(key: key));
    await tester.pumpAndSettle();
    final state = key.currentState!;
    final layout = state.layout;
    final nextOwner = Object();
    var nextDismissed = false;
    layout.clear();
    layout.setBackAction(() => nextDismissed = true, owner: nextOwner);

    await tester.pumpWidget(const SizedBox());
    expect(layout.handleBack(), isTrue);
    expect(nextDismissed, isTrue);
    layout.clear();
    expect(layout.handleBack(), isFalse);
  });

  testWidgets('changing layout attachment preserves the surface back action', (
    tester,
  ) async {
    final layout = MainLayoutController();
    addTearDown(layout.dispose);
    final owner = Object();
    var dismissed = false;
    layout.setAttachment((_, child) => child, owner: owner);
    layout.setBackAction(() => dismissed = true, owner: owner);
    layout.clearAttachment(owner: owner);
    expect(layout.attachmentBuilder, isNull);
    expect(layout.handleBack(), isTrue);
    expect(dismissed, isTrue);
    layout.clear();
    await tester.pump();
  });
}

class _BackFixture extends StatefulWidget {
  /// Creates a shell fixture using the production layout and dismiss owner.
  const _BackFixture({super.key});

  /// Creates the controlled workspace visibility and exit observation state.
  @override
  State<_BackFixture> createState() => _BackFixtureState();
}

class _BackFixtureState extends State<_BackFixture> {
  final MainLayoutController layout = MainLayoutController();
  bool open = true;
  bool current = true;
  bool drawerOpen = false;
  int exitRequests = 0;

  /// Applies workspace visibility through its owning page.
  void setOpen(bool value) => setState(() => open = value);

  /// Simulates retaining the chat page while another route becomes current.
  void setCurrent(bool value) => setState(() => current = value);

  /// Keeps the layout alive until assertions about outgoing disposal finish.
  @override
  void initState() {
    super.initState();
    addTearDown(layout.dispose);
  }

  /// Builds the Android-style root pop boundary around the overlay surface.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: MainLayoutScope(
        controller: layout,
        child: PopScope<Object?>(
          canPop: false,
          onPopInvokedWithResult: (didPop, result) {
            if (didPop) return;
            if (drawerOpen) {
              drawerOpen = false;
              return;
            }
            if (layout.handleBack()) return;
            exitRequests++;
          },
          child: Scaffold(
            body: MainScreenActivityScope(
              isCurrentScreen: current,
              child: MainLayoutBackAction(
                enabled: open,
                onBack: () => setOpen(false),
                child: AdaptiveSidePanel(
                  open: open,
                  onOpenChanged: setOpen,
                  layout: SidePanelLayout.overlay,
                  animate: false,
                  panel: const TextField(key: ValueKey('editor')),
                  child: const Text('Chat'),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
