import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/l10n/generated/app_localizations.dart';
import 'package:operit2/ui/features/chat/components/ChatMessageLocatorPanel.dart';

void main() {
  testWidgets(
    'searches persisted history and combines the query with favorites',
    (tester) async {
      final queries = <String>[];
      final selected = <int>[];
      final saved = <String>[];
      await _pumpPanel(
        tester,
        onSelected: selected.add,
        onToggleFavorite: (timestamp, favorite) async =>
            saved.add('$timestamp:$favorite'),
        loadEntries: (query) async {
          queries.add(query);
          return query.isEmpty
              ? [_entry(1), _entry(2, favorite: true)]
              : [
                  _entry(
                    99,
                    favorite: true,
                    content: 'Historical needle result',
                  ),
                ];
        },
      );
      await tester.tap(find.widgetWithText(ChoiceChip, '收藏'));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('message-locator-entry-1')),
        findsNothing,
      );
      expect(
        find.byKey(const ValueKey('message-locator-entry-2')),
        findsOneWidget,
      );
      await tester.enterText(find.byType(TextField), 'needle');
      await tester.pump(const Duration(milliseconds: 250));
      await tester.pumpAndSettle();
      expect(queries, ['', 'needle']);
      expect(
        find.byKey(const ValueKey('message-locator-entry-99')),
        findsOneWidget,
      );
      await tester.tap(
        find.byKey(const ValueKey('message-locator-favorite-99')),
      );
      await tester.pumpAndSettle();
      expect(saved, ['99:false']);
      expect(selected, isEmpty);
      expect(
        find.byKey(const ValueKey('message-locator-entry-99')),
        findsNothing,
      );
      await tester.tap(find.widgetWithText(ChoiceChip, '全部'));
      await tester.pumpAndSettle();
      await tester.tap(find.byKey(const ValueKey('message-locator-entry-99')));
      expect(selected, [99]);
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets(
    'rejects stale searches during debounce and after newer results arrive',
    (tester) async {
      final requests = <String, Completer<List<ChatMessageLocatorPreview>>>{};
      await _pumpPanel(
        tester,
        loadEntries: (query) {
          if (query.isEmpty) return Future.value([_entry(1)]);
          return (requests[query] =
                  Completer<List<ChatMessageLocatorPreview>>())
              .future;
        },
      );
      await tester.enterText(find.byType(TextField), 'old');
      await tester.pump(const Duration(milliseconds: 230));
      await tester.enterText(find.byType(TextField), 'new');
      requests['old']!.complete([_entry(2)]);
      await tester.pump();
      expect(
        find.byKey(const ValueKey('message-locator-entry-2')),
        findsNothing,
      );
      await tester.pump(const Duration(milliseconds: 230));
      requests['new']!.complete([_entry(3)]);
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('message-locator-entry-3')),
        findsOneWidget,
      );
      expect(
        find.byKey(const ValueKey('message-locator-entry-2')),
        findsNothing,
      );
    },
  );

  testWidgets(
    'favorite updates survive subsequent search results without jumping',
    (tester) async {
      final save = Completer<void>();
      final selected = <int>[];
      var saves = 0;
      await _pumpPanel(
        tester,
        loadEntries: (_) async => [_entry(1)],
        onSelected: selected.add,
        onToggleFavorite: (_, _) {
          saves++;
          return save.future;
        },
      );
      final star = find.byKey(const ValueKey('message-locator-favorite-1'));
      await tester.tap(star);
      await tester.pump();
      await tester.tap(star);
      expect(saves, 1);
      save.complete();
      await tester.pumpAndSettle();
      await tester.enterText(find.byType(TextField), 'message');
      await tester.pump(const Duration(milliseconds: 250));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(ChoiceChip, '收藏'));
      await tester.pumpAndSettle();
      expect(
        find.byKey(const ValueKey('message-locator-entry-1')),
        findsOneWidget,
      );
      expect(selected, isEmpty);
    },
  );

  testWidgets(
    'load failures can retry and favorite failures keep the original state',
    (tester) async {
      var fail = true;
      await _pumpPanel(
        tester,
        loadEntries: (_) async {
          if (fail) throw StateError('offline');
          return [_entry(1, favorite: true)];
        },
        onToggleFavorite: (_, _) async => throw StateError('offline'),
      );
      expect(find.text('暂时无法加载消息，请重试'), findsOneWidget);
      fail = false;
      await tester.tap(find.text('重试'));
      await tester.pumpAndSettle();
      await tester.tap(find.widgetWithText(ChoiceChip, '收藏'));
      await tester.pumpAndSettle();
      await tester.tap(
        find.byKey(const ValueKey('message-locator-favorite-1')),
      );
      await tester.pumpAndSettle();
      expect(find.text('收藏更新失败，请重试'), findsOneWidget);
      expect(
        find.byKey(const ValueKey('message-locator-entry-1')),
        findsOneWidget,
      );
      expect(tester.takeException(), isNull);
    },
  );

  testWidgets('empty favorites and empty search have distinct messages', (
    tester,
  ) async {
    await _pumpPanel(tester, loadEntries: (_) async => []);
    await tester.tap(find.widgetWithText(ChoiceChip, '收藏'));
    await tester.pumpAndSettle();
    expect(find.text('还没有收藏的消息'), findsOneWidget);
    await tester.enterText(find.byType(TextField), 'missing');
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
    expect(find.text('没有匹配的消息'), findsOneWidget);
    await tester.tap(find.byTooltip('清空搜索'));
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
    expect(find.text('还没有收藏的消息'), findsOneWidget);
  });

  testWidgets('disposing during a search or save ignores late completions', (
    tester,
  ) async {
    final load = Completer<List<ChatMessageLocatorPreview>>();
    await _pumpPanel(tester, loadEntries: (_) => load.future, settle: false);
    await tester.pumpWidget(const SizedBox());
    load.complete([_entry(1)]);
    await tester.pump();
    expect(tester.takeException(), isNull);
  });

  testWidgets('narrow panels fit large text and short keyboard viewports', (
    tester,
  ) async {
    for (final height in [560.0, 220.0]) {
      await _pumpPanel(
        tester,
        width: 320,
        height: height,
        textScale: 2,
        loadEntries: (_) async => [_entry(1)],
      );
      expect(tester.takeException(), isNull);
    }
  });

  testWidgets('shows saved identities and highlights query matches', (
    tester,
  ) async {
    await _pumpPanel(
      tester,
      loadEntries: (_) async => [_entry(1, content: 'before NEEDLE after')],
    );
    await tester.enterText(find.byType(TextField), 'needle');
    await tester.pump(const Duration(milliseconds: 250));
    await tester.pumpAndSettle();
    expect(find.text('Saved assistant'), findsOneWidget);
    final text = tester.widget<Text>(
      find.byWidgetPredicate(
        (widget) =>
            widget is Text &&
            widget.textSpan?.toPlainText() == 'before NEEDLE after',
      ),
    );
    final spans = (text.textSpan! as TextSpan).children!.whereType<TextSpan>();
    expect(
      spans.singleWhere((span) => span.text == 'NEEDLE').style!.backgroundColor,
      isNotNull,
    );
  });
}

Future<void> _pumpPanel(
  WidgetTester tester, {
  required Future<List<ChatMessageLocatorPreview>> Function(String) loadEntries,
  Future<void> Function(int, bool)? onToggleFavorite,
  ValueChanged<int>? onSelected,
  bool settle = true,
  double width = 380,
  double height = 560,
  double textScale = 1,
}) async {
  await tester.pumpWidget(
    MaterialApp(
      locale: const Locale('zh'),
      localizationsDelegates: AppLocalizations.localizationsDelegates,
      supportedLocales: AppLocalizations.supportedLocales,
      builder: (context, child) => MediaQuery(
        data: MediaQuery.of(
          context,
        ).copyWith(textScaler: TextScaler.linear(textScale)),
        child: child!,
      ),
      home: Scaffold(
        body: Center(
          child: SizedBox(
            width: width,
            height: height,
            child: ChatMessageLocatorPanel(
              loadEntries: loadEntries,
              onToggleFavorite: onToggleFavorite ?? (_, _) async {},
              onSelected: onSelected ?? (_) {},
              onClose: () {},
            ),
          ),
        ),
      ),
    ),
  );
  if (settle) await tester.pumpAndSettle();
}

ChatMessageLocatorPreview _entry(
  int timestamp, {
  bool favorite = false,
  String? content,
}) => ChatMessageLocatorPreview(
  messageIndex: timestamp,
  timestamp: timestamp,
  sender: 'ai',
  displayName: 'Saved assistant',
  previewContent: content ?? 'message $timestamp',
  contentLength: content?.length ?? 9,
  displayMode: 'NORMAL',
  isFavorite: favorite,
);
