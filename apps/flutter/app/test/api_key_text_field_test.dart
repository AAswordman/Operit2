import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/settings/components/ApiKeyTextField.dart';

/// Hosts the field with English localization so tooltips stay stable in tests.
class _Harness extends StatefulWidget {
  const _Harness({super.key, required this.controller, this.fieldKey});

  final TextEditingController controller;
  final Key? fieldKey;

  @override
  State<_Harness> createState() => _HarnessState();
}

class _HarnessState extends State<_Harness> {
  Key? fieldKey;

  @override
  void initState() {
    super.initState();
    fieldKey = widget.fieldKey;
  }

  /// Swaps the field key the way a provider type change rebuilds the dialog.
  void useFieldKey(Key value) => setState(() => fieldKey = value);

  @override
  Widget build(BuildContext context) => MaterialApp(
    locale: const Locale('en'),
    localizationsDelegates: AppLocalizations.localizationsDelegates,
    supportedLocales: AppLocalizations.supportedLocales,
    home: Scaffold(
      body: ApiKeyTextField(
        key: fieldKey,
        controller: widget.controller,
        labelText: 'API Key',
      ),
    ),
  );
}

EditableText _editable(WidgetTester tester) =>
    tester.widget<EditableText>(find.byType(EditableText));

void main() {
  testWidgets('starts masked with the reveal affordance', (tester) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    expect(_editable(tester).obscureText, isTrue);
    expect(find.byIcon(Icons.visibility_outlined), findsOneWidget);
    expect(find.byIcon(Icons.visibility_off_outlined), findsNothing);
  });

  testWidgets('eye button toggles both ways without touching the text', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    await tester.enterText(find.byType(EditableText), 'sk-replaced');
    await tester.tap(find.byIcon(Icons.visibility_outlined));
    await tester.pump();
    expect(_editable(tester).obscureText, isFalse);
    expect(controller.text, 'sk-replaced');
    await tester.tap(find.byIcon(Icons.visibility_off_outlined));
    await tester.pump();
    expect(_editable(tester).obscureText, isTrue);
    expect(controller.text, 'sk-replaced');
  });

  testWidgets('tooltip follows the display state', (tester) async {
    final controller = TextEditingController();
    addTearDown(controller.dispose);
    await tester.pumpWidget(_Harness(controller: controller));
    expect(find.byTooltip('Show API key'), findsOneWidget);
    expect(find.byTooltip('Hide API key'), findsNothing);
    await tester.tap(find.byTooltip('Show API key'));
    await tester.pump();
    expect(find.byTooltip('Hide API key'), findsOneWidget);
    expect(find.byTooltip('Show API key'), findsNothing);
  });

  testWidgets('rebuilding with a new ValueKey resets to masked', (
    tester,
  ) async {
    final controller = TextEditingController(text: 'sk-test');
    addTearDown(controller.dispose);
    final harnessKey = GlobalKey<_HarnessState>();
    await tester.pumpWidget(
      _Harness(
        key: harnessKey,
        controller: controller,
        fieldKey: const ValueKey<String?>('DEEPSEEK'),
      ),
    );
    await tester.tap(find.byIcon(Icons.visibility_outlined));
    await tester.pump();
    expect(_editable(tester).obscureText, isFalse);
    harnessKey.currentState!.useFieldKey(const ValueKey<String?>('OPENAI'));
    await tester.pump();
    expect(_editable(tester).obscureText, isTrue);
    expect(find.byIcon(Icons.visibility_outlined), findsOneWidget);
    expect(controller.text, 'sk-test');
  });
}
