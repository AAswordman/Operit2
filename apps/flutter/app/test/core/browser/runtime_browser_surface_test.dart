import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSurface.dart';
import 'package:operit2/core/host/browser/RuntimeBrowserSurfaceRegistry.dart';
import 'package:operit2/ui/common/components/RetainedPage.dart';

void main() {
  testWidgets(
    'retaining a workspace releases presentation but keeps the owner mounted',
    (tester) async {
      final registry = RuntimeBrowserSurfaceRegistry();
      var active = false;
      var mounts = 0;
      var disposals = 0;
      var taps = 0;
      final ownerKey = GlobalKey();
      late StateSetter rebuild;
      await tester.pumpWidget(
        MaterialApp(
          home: StatefulBuilder(
            builder: (context, setState) {
              rebuild = setState;
              return Stack(
                children: [
                  Positioned(
                    left: 430,
                    top: 40,
                    width: 320,
                    height: 240,
                    child: RetainedPage(
                      active: active,
                      child: RuntimeBrowserSurfaceTarget(
                        sessionId: 'browser',
                        registry: registry,
                      ),
                    ),
                  ),
                  RuntimeBrowserSurface(
                    sessionId: 'browser',
                    registry: registry,
                    child: _OwnerProbe(
                      key: ownerKey,
                      onMount: () => mounts++,
                      onTap: () => taps++,
                      onDispose: () => disposals++,
                    ),
                  ),
                ],
              );
            },
          ),
        ),
      );
      await tester.pumpAndSettle();
      final original = ownerKey.currentState;
      expect(mounts, 1);
      expect(registry.presentationFor('browser'), isNull);
      expect(tester.getSize(find.byKey(ownerKey)), const Size(800, 600));

      rebuild(() => active = true);
      await tester.pumpAndSettle();
      expect(registry.presentationFor('browser')?.size, const Size(320, 240));
      expect(tester.getTopLeft(find.byKey(ownerKey)), const Offset(430, 40));
      expect(ownerKey.currentState, same(original));
      await tester.tapAt(const Offset(600, 100));
      expect(taps, 1);

      rebuild(() => active = false);
      await tester.pumpAndSettle();
      expect(registry.presentationFor('browser'), isNull);
      expect(ownerKey.currentState, same(original));
      expect(tester.getSize(find.byKey(ownerKey)), const Size(320, 240));
      expect(disposals, 0);
      await tester.tapAt(const Offset(600, 100));
      expect(taps, 1);

      rebuild(() => active = true);
      await tester.pumpAndSettle();
      expect(ownerKey.currentState, same(original));
      expect(mounts, 1);
      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pump();
      expect(disposals, 1);
    },
  );

  testWidgets('releasing a newer anchor restores the remaining presentation', (
    tester,
  ) async {
    final registry = RuntimeBrowserSurfaceRegistry();
    final first = RuntimeBrowserSurfacePresentation(
      owner: Object(),
      link: LayerLink(),
      size: const Size(200, 100),
    );
    final second = RuntimeBrowserSurfacePresentation(
      owner: Object(),
      link: LayerLink(),
      size: const Size(300, 200),
    );
    registry.attach('browser', first);
    registry.attach('browser', second);
    expect(registry.presentationFor('browser'), same(second));
    registry.detach('browser', Object());
    expect(registry.presentationFor('browser'), same(second));
    registry.detach('browser', second.owner);
    expect(registry.presentationFor('browser'), same(first));
    registry.detach('browser', first.owner);
    expect(registry.presentationFor('browser'), isNull);
    await tester.pump();
    registry.dispose();
  });

  testWidgets('an ancestor retained page releases nested browser anchors', (
    tester,
  ) async {
    final registry = RuntimeBrowserSurfaceRegistry();
    var open = true;
    late StateSetter rebuild;
    await tester.pumpWidget(
      MaterialApp(
        home: StatefulBuilder(
          builder: (context, setState) {
            rebuild = setState;
            return RetainedPage(
              active: open,
              child: RetainedPage(
                active: true,
                child: RuntimeBrowserSurfaceTarget(
                  sessionId: 'nested',
                  registry: registry,
                ),
              ),
            );
          },
        ),
      ),
    );
    await tester.pumpAndSettle();
    expect(registry.presentationFor('nested'), isNotNull);
    rebuild(() => open = false);
    await tester.pumpAndSettle();
    expect(registry.presentationFor('nested'), isNull);
    rebuild(() => open = true);
    await tester.pumpAndSettle();
    expect(registry.presentationFor('nested'), isNotNull);
    await tester.pumpWidget(const SizedBox.shrink());
    await tester.pump();
  });
}

class _OwnerProbe extends StatefulWidget {
  const _OwnerProbe({
    super.key,
    required this.onMount,
    required this.onDispose,
    this.onTap,
  });
  final VoidCallback? onTap;
  final VoidCallback onMount;
  final VoidCallback onDispose;
  @override
  State<_OwnerProbe> createState() => _OwnerProbeState();
}

class _OwnerProbeState extends State<_OwnerProbe> {
  @override
  void initState() {
    super.initState();
    widget.onMount();
  }

  @override
  void dispose() {
    widget.onDispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) => GestureDetector(
    onTap: widget.onTap,
    child: const ColoredBox(color: Colors.blue),
  );
}
