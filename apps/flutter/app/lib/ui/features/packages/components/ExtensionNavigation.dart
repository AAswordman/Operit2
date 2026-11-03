// ignore_for_file: file_names

import 'dart:collection';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../main/navigation/AppNavigationModels.dart';
import '../../../main/screens/OperitScreens.dart';
import '../../../main/screens/ScreenRouteRegistry.dart';
import '../market/MarketStatsSupport.dart';

/// Entries the user just tapped, so their detail route renders immediately.
final LinkedHashMap<String, core_proxy.MarketEntrySummary> _openedMarketEntries =
    LinkedHashMap<String, core_proxy.MarketEntrySummary>();

/// The summary a list handed to [openMarketEntryDetail] for [entryId], if any.
core_proxy.MarketEntrySummary? openedMarketEntry(String entryId) =>
    _openedMarketEntries[entryId];

/// Opens a shell route, keeping the sidebar and the shared back stack.
void openExtensionScreen(OperitScreen screen) {
  final entry = ScreenRouteRegistry.toEntry(screen: screen);
  AppRouterGateway.navigate(
    routeId: entry.routeId,
    args: entry.args,
    source: entry.source,
  );
}

void openExtensionDetail({required String kind, required String id}) {
  openExtensionScreen(ExtensionDetailScreenRoute(kind: kind, id: id));
}

void openMarketEntryDetail(core_proxy.MarketEntrySummary entry) {
  _openedMarketEntries
    ..remove(entry.id)
    ..[entry.id] = entry;
  while (_openedMarketEntries.length > 32) {
    _openedMarketEntries.remove(_openedMarketEntries.keys.first);
  }
  openExtensionScreen(MarketEntryDetailScreenRoute(entryId: entry.id));
}

void openMarketBrowse({
  required String heading,
  String? type,
  String? categoryId,
  MarketSortOption sort = MarketSortOption.updated,
}) {
  openExtensionScreen(
    MarketBrowseScreenRoute(
      heading: heading,
      type: type,
      categoryId: categoryId,
      sort: sort,
    ),
  );
}

void openMarketCreator() {
  openExtensionScreen(const MarketCreatorScreenRoute());
}
