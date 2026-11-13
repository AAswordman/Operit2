import 'dart:math' as math;

import 'package:flutter_test/flutter_test.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';
import 'package:operit2/ui/features/packages/market/MarketCommentPager.dart';

void main() {
  test(
    'newest first across static page boundaries, without fetching all pages',
    () async {
      final calls = <int>[];
      final pager = MarketCommentPager((page) async {
        calls.add(page);
        return _page(page, 103);
      });
      final first = await pager.load(1);
      expect(first.total, 103);
      expect(first.pageCount, 11);
      expect(first.items.map((c) => c.id), [
        for (var i = 102; i >= 93; i--) '$i',
      ]);
      expect(calls, [1, 3, 2]);
      final second = await pager.load(2);
      expect(second.items.first.id, '92');
      expect(second.items.last.id, '83');
      expect(calls, [1, 3, 2]);
      final last = await pager.load(11);
      expect(last.items.map((c) => c.id), ['2', '1', '0']);
    },
  );

  test(
    'refresh after deleting last page clamps to the remaining page',
    () async {
      var total = 11;
      final pager = MarketCommentPager((page) async => _page(page, total));
      expect((await pager.load(2)).items.single.id, '0');
      total = 10;
      final result = await pager.load(2, refresh: true);
      expect(result.page, 1);
      expect(result.pageCount, 1);
      expect(result.items.first.id, '9');
      total = 0;
      expect((await pager.load(1, refresh: true)).items, isEmpty);
    },
  );

  test('reply context crosses server pages and keeps ancestor order', () async {
    final calls = <int>[];
    final pager = MarketCommentPager((page) async {
      calls.add(page);
      return _page(page, 160, parents: {'159': '80', '80': '30'});
    });
    final latest = await pager.load(1);
    expect(calls, [1, 4]);
    expect(pager.cachedComment('80'), isNull);
    final conversation = await pager.conversation(latest.items.first);
    expect(conversation.map((c) => c.id), ['30', '80', '159']);
    expect(calls, [1, 4, 3, 2]);
    expect(pager.cachedComment('80')?.parentId, '30');
    await pager.conversation(latest.items.first);
    expect(calls, [1, 4, 3, 2]);
    expect((await pager.load(2)).items.first.id, '149');
  });

  test(
    'deleted parents and cyclic reply data preserve the visible reply',
    () async {
      final missing = MarketCommentPager(
        (page) async => _page(page, 3, parents: {'2': 'deleted'}),
      );
      final latest = await missing.load(1);
      expect(
        (await missing.conversation(latest.items.first)).map((c) => c.id),
        ['2'],
      );
      final cycle = MarketCommentPager(
        (page) async => _page(page, 3, parents: {'2': '1', '1': '2'}),
      );
      final cyclic = await cycle.load(1);
      expect((await cycle.conversation(cyclic.items.first)).map((c) => c.id), [
        '1',
        '2',
      ]);
    },
  );

  test('failed newest page can be retried', () async {
    var fail = true;
    final pager = MarketCommentPager((page) async {
      if (page == 2 && fail) throw StateError('offline');
      return _page(page, 60);
    });
    await expectLater(pager.load(1), throwsStateError);
    fail = false;
    expect((await pager.load(1)).items.first.id, '59');
  });
}

MarketCommentPage _page(
  int page,
  int total, {
  Map<String, String> parents = const {},
}) => MarketCommentPage.fromJson({
  'page': page,
  'pageSize': 50,
  'total': total,
  'items': [
    for (var i = (page - 1) * 50; i < math.min(page * 50, total); i++)
      {
        'id': '$i',
        'parentId': parents['$i'],
        'body': 'Comment $i',
        'createdAt': '2026-11-13T08:00:00Z',
        'author': <String, Object?>{},
      },
  ],
});
