import 'dart:async';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/bridge/OperitRuntimeBridge.dart';
import 'package:operit2/core/link/CoreLinkCodec.dart';
import 'package:operit2/core/link/CoreLinkProtocol.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/data/preferences/UserPreferencesManager.dart';
import 'package:operit2/ui/features/packages/market/MarketFilters.dart';
import 'package:operit2/ui/features/packages/screens/StoreBrowseScreen.dart';
import 'package:operit2/ui/features/packages/screens/MarketPane.dart';
import 'package:operit2/ui/features/packages/model/ExtensionCatalogController.dart';
import 'package:operit2/ui/theme/OperitTheme.dart';

final _now = DateTime.utc(2026, 11, 14, 12);
const _categories = [
  MarketCategoryInfo(id: 'search_research', name: '搜索与研究', description: null),
  MarketCategoryInfo(id: 'dev_code', name: '开发工具', description: null),
  MarketCategoryInfo(id: 'automation_workflow', name: '自动化', description: null),
  MarketCategoryInfo(id: 'docs_knowledge', name: '文档与知识', description: null),
  MarketCategoryInfo(id: 'media_content', name: '媒体与内容', description: null),
  MarketCategoryInfo(
    id: 'chat_communication',
    name: '聊天与通信',
    description: null,
  ),
  MarketCategoryInfo(id: 'integration_api', name: '服务与接口', description: null),
  MarketCategoryInfo(id: 'system_data', name: '系统与数据', description: null),
  MarketCategoryInfo(
    id: 'business_productivity',
    name: '商务与效率',
    description: null,
  ),
  MarketCategoryInfo(
    id: 'life_entertainment',
    name: '生活与娱乐',
    description: null,
  ),
];

MarketEntrySummary _entry(
  String id, {
  int days = 1,
  int likes = 25,
  String categoryId = 'dev_code',
  DateTime? now,
}) {
  final date = now ?? _now;
  return MarketEntrySummary.fromJson({
    'id': id,
    'type': 'skill',
    'title': id,
    'description': '让日常工作更轻松，发现社区里的实用扩展。',
    'author': {'id': '1', 'login': 'operit-community'},
    'createdAt': date.subtract(Duration(days: days)).toIso8601String(),
    'updatedAt': date.toIso8601String(),
    'categoryId': categoryId,
    'downloads': 2460,
    'reactions': [
      {'reaction': '+1', 'total': likes},
    ],
  });
}

void main() {
  test('rankings default to a month and updated defaults to all time', () {
    for (final sort in [MarketSortOption.likes, MarketSortOption.downloads]) {
      expect(MarketFilters.forSort(sort).timeRange, MarketTimeRange.month);
    }
    expect(
      MarketFilters.forSort(MarketSortOption.updated).timeRange,
      MarketTimeRange.all,
    );
  });

  test('rankings filter first publication, updates filter updated time', () {
    final filters = MarketFilters.forSort(MarketSortOption.likes);
    expect(filters.matches(_entry('old', days: 365), _now), isFalse);
    expect(filters.matches(_entry('new', days: 20), _now), isTrue);
    expect(filters.matches(_entry('future', days: -1), _now), isFalse);
    expect(
      filters.matches(
        MarketEntrySummary.fromJson({'updatedAt': _now.toIso8601String()}),
        _now,
      ),
      isFalse,
    );
    expect(
      const MarketFilters(
        timeRange: MarketTimeRange.week,
      ).matches(_entry('old updated now', days: 365), _now),
      isTrue,
    );
    expect(
      const MarketFilters(
        sort: MarketSortOption.likes,
        timeRange: MarketTimeRange.week,
      ).matches(_entry('8 days', days: 8), _now),
      isFalse,
    );
    expect(filters.matches(_entry('boundary', days: 30), _now), isTrue);
    expect(filters.matches(_entry('outside', days: 31), _now), isFalse);
    expect(
      filters.matches(
        MarketEntrySummary.fromJson({
          ..._entry('draft created long ago', days: 365).toJson(),
          'publishedAt': _now
              .subtract(const Duration(days: 2))
              .toIso8601String(),
        }),
        _now,
      ),
      isTrue,
    );
  });

  test(
    'period filtering scans beyond empty pages and uses combined endpoints',
    () async {
      final bridge = _FilterBridge()
        ..pages = {
          1: [_entry('old leader', days: 365)],
          2: [_entry('new later-page entry')],
          3: [_entry('last page', days: 3)],
        };
      final entries = await loadMarketPeriodEntries(
        GeneratedCoreProxyClients(bridge).providersMarketStatsApiService,
        filters: const MarketFilters(
          sort: MarketSortOption.likes,
          timeRange: MarketTimeRange.month,
          categoryId: 'dev_code',
        ),
        type: 'skill',
        now: _now,
      );
      expect(entries.map((entry) => entry.id), [
        'new later-page entry',
        'last page',
      ]);
      expect(
        bridge.calls.map((call) => call.$1),
        everyElement('get_type_category_page'),
      );
      expect(bridge.calls.map((call) => call.$2['page']), [1, 2, 3]);
      expect(
        bridge.calls.map((call) => call.$2['sort']),
        everyElement('likes'),
      );
      expect(
        bridge.calls.map((call) => call.$2['category_id']),
        everyElement('dev_code'),
      );
    },
  );

  testWidgets('apply, dismiss, reset and category selection use a draft', (
    tester,
  ) async {
    final bridge = _FilterBridge();
    final clients = GeneratedCoreProxyClients(bridge);
    var filters = MarketFilters.forSort(MarketSortOption.likes);
    await tester.pumpWidget(
      MaterialApp(
        home: Scaffold(
          body: StatefulBuilder(
            builder: (context, setState) => MarketSortMenu(
              clients: clients,
              filters: filters,
              onChanged: (value) => setState(() => filters = value),
            ),
          ),
        ),
      ),
    );
    await tester.tap(find.byIcon(Icons.sort_rounded));
    await tester.pumpAndSettle();
    expect(
      tester
          .widget<ChoiceChip>(find.widgetWithText(ChoiceChip, '最近一个月'))
          .selected,
      isTrue,
    );
    await tester.tap(find.text('最近一周'));
    await tester.tap(find.text('开发工具'));
    expect(filters.categoryId, isNull);
    await tester.tap(find.text('应用筛选'));
    await tester.pumpAndSettle();
    expect(filters.timeRange, MarketTimeRange.week);
    expect(filters.categoryId, 'dev_code');
    await tester.tap(find.byIcon(Icons.sort_rounded));
    await tester.pumpAndSettle();
    await tester.tap(find.text('全部时间'));
    await tester.tap(find.byTooltip('关闭'));
    await tester.pumpAndSettle();
    expect(filters.timeRange, MarketTimeRange.week);
    await tester.tap(find.byIcon(Icons.sort_rounded));
    await tester.pumpAndSettle();
    await tester.tap(find.text('重置'));
    await tester.tap(find.text('应用筛选'));
    await tester.pumpAndSettle();
    expect(filters, MarketFilters.forSort(MarketSortOption.likes));
    expect(tester.takeException(), isNull);
  });

  testWidgets(
    'changing filters while an old request is pending keeps the new result',
    (tester) async {
      final pending = Completer<Map<String, Object?>>();
      final bridge = _FilterBridge()..pending = pending;
      final clients = GeneratedCoreProxyClients(bridge);
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: StoreMarketListing(
              clients: clients,
              sort: MarketSortOption.likes,
            ),
          ),
        ),
      );
      await tester.pump();
      bridge.pending = null;
      await tester.pumpWidget(
        OperitTheme(
          initialThemePreferenceSnapshot:
              UserPreferencesManager.defaultThemePreferenceSnapshot,
          initialThemeIsReady: false,
          unconfiguredChildEnabled: true,
          hostInteractionHostsEnabled: false,
          child: Scaffold(
            body: StoreMarketListing(
              clients: clients,
              sort: MarketSortOption.updated,
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      pending.complete({
        'page': 1,
        'pageSize': 10,
        'total': 1,
        'items': [_entry('stale result', now: DateTime.now()).toJson()],
      });
      await tester.pumpAndSettle();
      expect(find.text('stale result'), findsNothing);
      expect(find.text('智能搜索助手'), findsOneWidget);
      expect(tester.takeException(), isNull);
    },
  );

  for (final width in [390.0, 1000.0]) {
    testWidgets('filter panel fits and renders at $width', (tester) async {
      tester.view.physicalSize = Size(width, 940);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.reset);
      const output = String.fromEnvironment('MARKET_SCREENSHOT_DIR');
      if (output.isNotEmpty) {
        await tester.runAsync(() async {
          for (final font in {
            'MarketChinese': '/System/Library/Fonts/Hiragino Sans GB.ttc',
            'MaterialIcons':
                '/Users/ylvira/fvm/default/bin/cache/artifacts/material_fonts/MaterialIcons-Regular.otf',
          }.entries) {
            await (FontLoader(font.key)..addFont(
                  Future.value(
                    ByteData.sublistView(await File(font.value).readAsBytes()),
                  ),
                ))
                .load();
          }
        });
      }
      final clients = GeneratedCoreProxyClients(_FilterBridge());
      final boundary = GlobalKey();
      await tester.pumpWidget(
        RepaintBoundary(
          key: boundary,
          child: OperitTheme(
            initialThemePreferenceSnapshot:
                UserPreferencesManager.defaultThemePreferenceSnapshot,
            initialThemeIsReady: false,
            unconfiguredChildEnabled: true,
            hostInteractionHostsEnabled: false,
            child: Builder(
              builder: (context) => Theme(
                data: Theme.of(context).copyWith(
                  primaryTextTheme: Theme.of(context).primaryTextTheme.apply(
                    fontFamily: output.isEmpty ? null : 'MarketChinese',
                  ),
                  textTheme: Theme.of(context).textTheme.apply(
                    fontFamily: output.isEmpty ? null : 'MarketChinese',
                  ),
                ),
                child: Scaffold(
                  body: MarketPane(
                    controller: ExtensionCatalogController.of(clients),
                    query: '',
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pumpAndSettle();
      // Open the original type tab and sorting control, without a preview
      // app bar or extra filter entry on the home page.
      expect(find.byIcon(Icons.sort_rounded), findsNothing);
      expect(find.byIcon(Icons.tune_rounded), findsNothing);
      await tester.ensureVisible(find.text('技能').first);
      await tester.pumpAndSettle();
      await tester.tap(find.text('技能').first);
      await tester.pumpAndSettle();
      if (output.isNotEmpty) {
        await _capture(
          tester,
          boundary,
          output,
          'original-sort-entry-${width.toInt()}',
        );
      }
      await tester.tap(find.byIcon(Icons.sort_rounded));
      await tester.pumpAndSettle();
      await tester.tap(find.text('最受欢迎'));
      await tester.pumpAndSettle();
      expect(find.text('排序方式'), findsOneWidget);
      expect(find.text('时间范围'), findsOneWidget);
      expect(find.text('内容分类'), findsOneWidget);
      expect(find.text('应用筛选').hitTestable(), findsOneWidget);
      if (output.isNotEmpty) {
        await _capture(
          tester,
          boundary,
          output,
          'market-filters-${width.toInt()}',
        );
      }
      await tester.ensureVisible(find.text('开发工具'));
      await tester.tap(find.text('开发工具'));
      await tester.tap(find.text('最近一周'));
      await tester.tap(find.text('应用筛选'));
      await tester.pumpAndSettle();
      final listing = tester.widget<StoreMarketListing>(
        find.byType(StoreMarketListing),
      );
      expect(listing.sort, MarketSortOption.likes);
      expect(listing.timeRange, MarketTimeRange.week);
      expect(listing.categoryId, 'dev_code');
      expect(find.byIcon(Icons.sort_rounded), findsOneWidget);
      expect(find.byIcon(Icons.tune_rounded), findsNothing);
      expect(tester.takeException(), isNull);
    });
  }
}

Future<void> _capture(
  WidgetTester tester,
  GlobalKey boundary,
  String directory,
  String name,
) async {
  await tester.runAsync(() async {
    final image =
        await (boundary.currentContext!.findRenderObject()!
                as RenderRepaintBoundary)
            .toImage(pixelRatio: 2);
    final data = await image.toByteData(format: ui.ImageByteFormat.png);
    await Directory(directory).create(recursive: true);
    await File('$directory/$name.png').writeAsBytes(data!.buffer.asUint8List());
    image.dispose();
  });
}

class _FilterBridge extends OperitRuntimeBridge {
  final List<(String, Map)> calls = [];
  Map<int, List<MarketEntrySummary>>? pages;
  Completer<Map<String, Object?>>? pending;
  @override
  Future<Uint8List> callBytes(CoreCallRequest request) async {
    final args = request.args as Map;
    final Object result;
    switch (request.methodName) {
      case 'isLoggedIn':
        result = false;
      case 'getPublishablePackageSources':
        result = <Object?>[];
      case 'get_manifest':
        result = {
          'categories': _categories.map((item) => item.toJson()).toList(),
        };
      case 'getInstalledMarketVersions':
        result = <String, String>{};
      case 'get_list_page':
      case 'get_type_page':
      case 'get_category_page':
      case 'get_type_category_page':
        calls.add((request.methodName, Map.of(args)));
        if (pending != null) {
          result = await pending!.future;
        } else {
          final page = args['page'] as int;
          final entries =
              pages?[page] ??
              [
                _entry('智能搜索助手', now: DateTime.now(), likes: 328),
                _entry('代码审查搭档', now: DateTime.now(), likes: 216),
                _entry('自动化工作流', now: DateTime.now(), likes: 182),
                _entry('知识库检索', now: DateTime.now(), likes: 146),
                _entry('灵感收集箱', now: DateTime.now(), likes: 108),
              ];
          result = {
            'page': page,
            'pageSize': entries.length,
            'total': pages == null ? entries.length : pages!.length,
            'items': entries.map((entry) => entry.toJson()).toList(),
          };
        }
      default:
        throw StateError('Unexpected market method ${request.methodName}');
    }
    return encodeCoreLink(<Object?>[0, result]);
  }

  @override
  Future<CorePushSink> push(CorePushRequest request) =>
      throw UnimplementedError();
  @override
  Future<CoreEvent> watchSnapshot(CoreWatchRequest request) =>
      throw UnimplementedError();
  @override
  Stream<CoreEvent> watchStream(CoreWatchRequest request) =>
      const Stream.empty();
}
