import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/ui/features/chat/components/part/ChatActivityDisclosure.dart';
import 'package:operit2/ui/features/chat/components/part/ToolDisplayComponents.dart';

const _directoryDisplay = ToolDisplayDeclaration(
  labels: {'en': 'List'},
  icon: 'folder',
  targetParameter: 'path',
  resultRenderer: ToolResultRenderer.directory,
);

/// Hosts tool details at a constrained chat width without a native runtime.
Widget _host(Widget child, {double width = 600, double textScale = 1}) {
  return MaterialApp(
    home: Scaffold(
      body: MediaQuery(
        data: MediaQueryData(textScaler: TextScaler.linear(textScale)),
        child: Align(
          alignment: Alignment.topLeft,
          child: SizedBox(width: width, child: child),
        ),
      ),
    ),
  );
}

/// Builds a declared directory call with independently readable raw output.
ToolCallRow _directory({
  int count = 3,
  String result = 'Raw directory output',
}) {
  return ToolCallRow(
    isStreaming: false,
    presentation: ToolPresentation.fromCall(
      'arbitrary:directory',
      '<param name="path">/app/workspaces/11111</param>',
      result: result,
      isSuccess: true,
      display: _directoryDisplay,
      resultData: {
        'path': '/app/workspaces/11111',
        'entries': [
          for (var index = 0; index < count; index++)
            {
              'name': index == 0 ? '.operit' : 'document_$index.txt',
              'isDirectory': index == 0,
              'size': index == 0 ? 0 : 42,
            },
        ],
      },
    ),
  );
}

/// Guards tool expansion density, payload access, and narrow-screen layout.
void main() {
  testWidgets('tool body animates in and remains mounted while folding', (
    tester,
  ) async {
    await tester.pumpWidget(_host(_directory()));
    final row = find.byType(ToolCallRow);
    final closedHeight = tester.getSize(row).height;

    await tester.tap(find.text('List'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 80));
    final openingHeight = tester.getSize(row).height;
    await tester.pumpAndSettle();
    final openHeight = tester.getSize(row).height;
    expect(openingHeight, greaterThan(closedHeight));
    expect(openingHeight, lessThan(openHeight));

    final switcher = tester.widget<AnimatedSwitcher>(
      find.descendant(of: row, matching: find.byType(AnimatedSwitcher)).first,
    );
    expect(switcher.duration, ChatActivityDisclosure.duration);
    expect(switcher.reverseDuration, ChatActivityDisclosure.duration);
    expect(switcher.switchInCurve, ChatActivityDisclosure.curve);
    expect(switcher.switchOutCurve, ChatActivityDisclosure.curve);

    await tester.tap(find.text('List'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 80));
    expect(find.text('document_1.txt'), findsOneWidget);
    expect(tester.getSize(row).height, greaterThan(closedHeight));
    expect(tester.getSize(row).height, lessThan(openHeight));
    await tester.pumpAndSettle();
    expect(find.text('document_1.txt'), findsNothing);
    expect(tester.getSize(row).height, closedHeight);
  });

  testWidgets('payload uses the same motion and handles rapid reversal', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const ToolInlineDetailSurface(
          title: 'Parameters',
          text: 'payload',
          initiallyCollapsed: true,
        ),
      ),
    );
    final surface = find.byType(ToolInlineDetailSurface);
    final closedHeight = tester.getSize(surface).height;
    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    final openHeight = tester.getSize(surface).height;
    await tester.tap(find.text('Parameters'));
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 80));
    expect(find.text('payload'), findsOneWidget);
    expect(tester.getSize(surface).height, greaterThan(closedHeight));
    expect(tester.getSize(surface).height, lessThan(openHeight));
    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    expect(find.text('payload'), findsOneWidget);
    expect(tester.getSize(surface).height, openHeight);
    expect(tester.takeException(), isNull);
  });

  testWidgets('directory expansion stays compact without duplicate paths', (
    tester,
  ) async {
    await tester.pumpWidget(_host(_directory()));
    await tester.tap(find.text('List'));
    await tester.pumpAndSettle();

    expect(find.text('/app/workspaces/11111'), findsOneWidget);
    expect(find.text('path: /app/workspaces/11111'), findsNothing);
    expect(find.text('Raw directory output'), findsNothing);
    expect(find.byType(ListTile), findsNothing);
    expect(find.byType(ExpansionTile), findsNothing);
    expect(find.text('.operit'), findsOneWidget);
    expect(find.text('0 B'), findsNothing);
    expect(tester.getSize(find.byType(ToolCallRow)).height, lessThan(200));

    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    expect(find.text('path: /app/workspaces/11111'), findsOneWidget);
    await tester.tap(find.text('Result'));
    await tester.pumpAndSettle();
    expect(find.text('Raw directory output'), findsOneWidget);
    expect(find.text('Result'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('structured-only results keep JSON collapsed', (tester) async {
    await tester.pumpWidget(_host(_directory(result: '')));
    await tester.tap(find.text('List'));
    await tester.pumpAndSettle();

    expect(find.text('document_1.txt'), findsOneWidget);
    expect(find.byType(SelectableText), findsNWidgets(3));
    await tester.tap(find.text('Result'));
    await tester.pumpAndSettle();
    final payloads = tester.widgetList<SelectableText>(
      find.byType(SelectableText),
    );
    expect(payloads.any((text) => text.data!.startsWith('{')), isTrue);
  });

  testWidgets('an empty directory has an explicit compact result', (
    tester,
  ) async {
    await tester.pumpWidget(_host(_directory(count: 0)));
    await tester.tap(find.text('List'));
    await tester.pumpAndSettle();

    expect(find.text('This folder is empty'), findsOneWidget);
    expect(find.text('/app/workspaces/11111'), findsOneWidget);
  });

  testWidgets('long directory listings scroll within a bounded result', (
    tester,
  ) async {
    await tester.pumpWidget(_host(_directory(count: 100)));
    await tester.tap(find.text('List'));
    await tester.pumpAndSettle();

    expect(tester.getSize(find.byType(DirectoryListingDisplay)).height, 248);
    final scroll = find.descendant(
      of: find.byType(DirectoryListingDisplay),
      matching: find.byType(SingleChildScrollView),
    );
    await tester.drag(scroll, const Offset(0, -4000));
    await tester.pumpAndSettle();
    expect(find.text('document_99.txt').hitTestable(), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets('directory details fit narrow screens and larger chat text', (
    tester,
  ) async {
    await tester.pumpWidget(_host(_directory(), width: 280, textScale: 1.6));
    await tester.tap(find.text('List'));
    await tester.pumpAndSettle();

    expect(find.text('document_1.txt'), findsOneWidget);
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'long plain output stays readable, scrollable and fully copyable',
    (tester) async {
      final output = List.generate(
        100,
        (index) => 'output line $index',
      ).join('\n');
      String? copied;
      tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
        SystemChannels.platform,
        (call) async {
          if (call.method == 'Clipboard.setData') {
            copied = (call.arguments as Map)['text'] as String;
          }
          return null;
        },
      );
      addTearDown(() {
        tester.binding.defaultBinaryMessenger.setMockMethodCallHandler(
          SystemChannels.platform,
          null,
        );
      });
      await tester.pumpWidget(
        _host(
          ToolInlineDetailSurface(title: 'Result', text: output),
          width: 280,
        ),
      );

      expect(
        tester.getSize(find.byType(ToolInlineDetailSurface)).height,
        lessThan(300),
      );
      expect(
        tester.widget<SelectableText>(find.byType(SelectableText)).data,
        output,
      );
      await tester.drag(
        find.byType(SingleChildScrollView),
        const Offset(0, -4000),
      );
      await tester.pumpAndSettle();
      await tester.tap(find.byIcon(Icons.copy));
      expect(copied, output);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('streaming updates retain the user disclosure choice', (
    tester,
  ) async {
    await tester.pumpWidget(
      _host(
        const ToolInlineDetailSurface(
          title: 'Parameters',
          text: 'first',
          initiallyCollapsed: true,
        ),
      ),
    );
    await tester.tap(find.text('Parameters'));
    await tester.pumpWidget(
      _host(
        const ToolInlineDetailSurface(
          title: 'Parameters',
          text: 'updated',
          initiallyCollapsed: true,
        ),
      ),
    );
    expect(find.text('updated'), findsOneWidget);
    await tester.tap(find.text('Parameters'));
    await tester.pumpAndSettle();
    expect(find.text('updated'), findsNothing);
  });
}
