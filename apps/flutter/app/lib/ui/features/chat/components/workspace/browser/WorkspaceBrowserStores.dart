// ignore_for_file: file_names

import 'bookmarks/WorkspaceBrowserBookmarkStore.dart';
import 'downloads/WorkspaceBrowserDownloadStore.dart';
import 'history/WorkspaceBrowserHistoryStore.dart';
import '../../../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'userscripts/WorkspaceUserscriptRuntime.dart';
import 'userscripts/WorkspaceUserscriptStore.dart';

class WorkspaceBrowserStores {
  /// Binds browser stores to the same Core as their owning browser or viewer.
  WorkspaceBrowserStores({
    bool observeDownloads = false,
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients {
    final runtimeStorage = _clients.repositoryRuntimeStorageRepository;
    userscripts = WorkspaceUserscriptStore(
      runtimeStorage: runtimeStorage,
      storagePath: runtimeStorage.webSessionUserscriptsStatePath,
    );
    history = WorkspaceBrowserHistoryStore(
      runtimeStorage: runtimeStorage,
      storagePath: runtimeStorage.webSessionBrowserHistoryPath,
    );
    bookmarks = WorkspaceBrowserBookmarkStore(
      runtimeStorage: runtimeStorage,
      storagePath: runtimeStorage.webSessionBrowserBookmarksPath,
    );
    downloads = WorkspaceBrowserDownloadStore(
      runtimeStorage: runtimeStorage,
      storagePath: runtimeStorage.webSessionBrowserDownloadsPath,
      downloadDirectoryPath:
          runtimeStorage.webSessionBrowserDownloadFilesDirPath,
      observer: observeDownloads,
    );
    userscriptRuntime = WorkspaceUserscriptRuntime(store: userscripts);
  }

  final GeneratedCoreProxyClients _clients;
  late final WorkspaceBrowserHistoryStore history;
  late final WorkspaceBrowserBookmarkStore bookmarks;
  late final WorkspaceBrowserDownloadStore downloads;
  late final WorkspaceUserscriptStore userscripts;
  late final WorkspaceUserscriptRuntime userscriptRuntime;

  Future<void> load() async {
    await Future.wait(<Future<void>>[
      history.load(),
      bookmarks.load(),
      downloads.load(),
      userscripts.load(),
    ]);
  }
}
