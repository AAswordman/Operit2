import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart' as core;
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/model/ApiKeyPoolEditor.dart';

/// Builds an imported entry with usage history that editing must preserve.
core.ApiKeyInfo _entry(String id) => core.ApiKeyInfo(
  id: id,
  key: 'fixture-$id',
  name: id,
  isEnabled: true,
  availabilityStatus: core.ApiKeyAvailabilityStatus.untested,
  usageCount: 7,
  lastUsed: 8,
  errorCount: 2,
);

/// Exercises draft edits, eligibility validation, and explicit credential tests.
void main() {
  testWidgets(
    'empty imported pool can explicitly add its single key on mobile',
    (tester) async {
      tester.view.physicalSize = const Size(390, 1000);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final key = GlobalKey<_HarnessState>();
      await tester.pumpWidget(_Harness(key: key, entries: const []));
      final state = key.currentState!;
      expect(state.form.currentState!.validate(), isFalse);
      await tester.pump();
      expect(find.textContaining('Enable at least one'), findsOneWidget);
      await tester.tap(find.text('Add current single key'));
      await tester.pump();
      expect(state.entries.single.key, 'existing-single');
      expect(state.form.currentState!.validate(), isTrue);
      await tester.tap(find.byKey(const ValueKey('api-key-pool-toggle')));
      await tester.pump();
      expect(state.enabled, isFalse);
      expect(state.entries, hasLength(1));
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'batch paste deduplicates keys and rename preserves imported metadata',
    (tester) async {
      final key = GlobalKey<_HarnessState>();
      await tester.pumpWidget(
        _Harness(key: key, entries: [_entry('original')]),
      );
      await tester.tap(find.text('Add key'));
      await tester.pumpAndSettle();
      await tester.enterText(
        find.byKey(const ValueKey('pool-key-input')),
        'fixture-original\nnew-a\nnew-b\nnew-a\n\n',
      );
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      final state = key.currentState!;
      expect(state.entries.map((entry) => entry.key), [
        'fixture-original',
        'new-a',
        'new-b',
      ]);
      expect(state.entries.map((entry) => entry.id).toSet(), hasLength(3));
      await tester.tap(find.byTooltip('Edit key').first);
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextFormField).first, 'Renamed');
      await tester.tap(find.text('Save'));
      await tester.pumpAndSettle();
      expect(state.entries.first.id, 'original');
      expect(state.entries.first.name, 'Renamed');
      expect(state.entries.first.usageCount, 7);
      expect(state.entries.first.lastUsed, 8);
      expect(state.entries.first.errorCount, 2);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'immediate sequential probes keep every status and clear marks repairs eligibility',
    (tester) async {
      final key = GlobalKey<_HarnessState>();
      final tested = <String>[];
      await tester.pumpWidget(
        _Harness(
          key: key,
          entries: [_entry('a'), _entry('b')],
          testEntry: (entry) async {
            tested.add(entry.id);
            return core.ApiKeyInfo.fromJson({
              ...entry.toJson(),
              'availabilityStatus': 'UNAVAILABLE',
            });
          },
        ),
      );
      await tester.tap(find.text('Test untested keys'));
      await tester.pumpAndSettle();
      expect(tested, ['a', 'b']);
      final state = key.currentState!;
      expect(
        state.entries.every(
          (entry) =>
              entry.availabilityStatus ==
              core.ApiKeyAvailabilityStatus.unavailable,
        ),
        isTrue,
      );
      expect(state.form.currentState!.validate(), isFalse);
      await tester.tap(find.text('Reset key health'));
      await tester.pump();
      expect(state.form.currentState!.validate(), isTrue);
      expect(
        state.entries.every(
          (entry) =>
              entry.availabilityStatus ==
              core.ApiKeyAvailabilityStatus.untested,
        ),
        isTrue,
      );
    },
  );
}

class _Harness extends StatefulWidget {
  /// Creates an editable provider draft for the pool widget.
  const _Harness({super.key, required this.entries, this.testEntry});
  final List<core.ApiKeyInfo> entries;
  final Future<core.ApiKeyInfo> Function(core.ApiKeyInfo)? testEntry;

  /// Creates mutable test draft state.
  @override
  State<_Harness> createState() => _HarnessState();
}

class _HarnessState extends State<_Harness> {
  final form = GlobalKey<FormState>();
  late List<core.ApiKeyInfo> entries;
  bool enabled = true;

  /// Copies initial imported entries into the editable draft.
  @override
  void initState() {
    super.initState();
    entries = List.of(widget.entries);
  }

  /// Hosts the production pool widget with localization and form validation.
  @override
  Widget build(BuildContext context) => MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: SingleChildScrollView(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Form(
            key: form,
            child: ApiKeyPoolEditor(
              enabled: enabled,
              entries: entries,
              singleKey: 'existing-single',
              onChanged: (value) => setState(() => entries = value),
              onEnabledChanged: (value) => setState(() => enabled = value),
              onTestEntry: widget.testEntry,
            ),
          ),
        ),
      ),
    ),
  );
}
