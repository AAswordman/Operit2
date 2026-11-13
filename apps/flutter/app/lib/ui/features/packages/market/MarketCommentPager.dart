// ignore_for_file: file_names

import 'dart:math' as math;

import '../../../../core/proxy/generated/CoreProxyModels.g.dart';

/// Presents the market's oldest-first static pages as small newest-first pages.
/// Only the server pages covering the visible range are fetched and cached.
class MarketCommentPager {
  MarketCommentPager(this.loadPage);

  static const pageSize = 10;
  final Future<MarketCommentPage> Function(int page) loadPage;
  final Map<int, MarketCommentPage> _cache = {};
  MarketCommentPage? _metadata;
  final Map<String, MarketComment> _byId = {};

  Future<MarketCommentSlice> load(int page, {bool refresh = false}) async {
    if (refresh) {
      _cache.clear();
      _byId.clear();
      _metadata = null;
    }
    final metadata = _metadata ??= await _fetch(1);
    final total = metadata.total;
    final pageCount = math.max(1, (total / pageSize).ceil());
    final current = page.clamp(1, pageCount);
    if (total == 0) {
      return MarketCommentSlice(page: 1, total: 0, items: const []);
    }
    if (metadata.pageSize <= 0) {
      throw StateError('评论分页大小无效');
    }
    final newest = total - 1 - (current - 1) * pageSize;
    final oldest = math.max(0, newest - pageSize + 1);
    final items = <MarketComment>[];
    for (var index = newest; index >= oldest; index--) {
      final serverPage = await _fetch(index ~/ metadata.pageSize + 1);
      final offset = index % metadata.pageSize;
      if (offset >= serverPage.items.length) {
        throw StateError('评论列表已更新，请重试');
      }
      items.add(serverPage.items[offset]);
    }
    return MarketCommentSlice(page: current, total: total, items: items);
  }

  MarketComment? cachedComment(String? id) => _byId[id];

  /// Fetch older context only when the reader opens a reply's conversation.
  Future<MarketComment?> _findComment(String id) async {
    if (_byId.containsKey(id)) return _byId[id];
    final metadata = _metadata ??= await _fetch(1);
    if (metadata.pageSize <= 0) return null;
    final pages = (metadata.total / metadata.pageSize).ceil();
    for (var page = pages; page >= 1; page--) {
      await _fetch(page);
      if (_byId.containsKey(id)) return _byId[id];
    }
    return null;
  }

  /// Walk direct parents across server pages, then read the exchange in order.
  Future<List<MarketComment>> conversation(MarketComment comment) async {
    final chain = <MarketComment>[comment];
    final seen = <String>{comment.id};
    var parentId = comment.parentId;
    while (parentId != null && parentId.isNotEmpty && seen.add(parentId)) {
      final parent = await _findComment(parentId);
      if (parent == null) break;
      chain.add(parent);
      parentId = parent.parentId;
    }
    return chain.reversed.toList(growable: false);
  }

  Future<MarketCommentPage> _fetch(int page) async {
    final cached = _cache[page];
    if (cached != null) return cached;
    final result = await loadPage(page);
    _cache[page] = result;
    for (final comment in result.items) {
      _byId[comment.id] = comment;
    }
    return result;
  }
}

class MarketCommentSlice {
  const MarketCommentSlice({
    required this.page,
    required this.total,
    required this.items,
  });

  final int page;
  final int total;
  final List<MarketComment> items;
  int get pageCount =>
      math.max(1, (total / MarketCommentPager.pageSize).ceil());
}
