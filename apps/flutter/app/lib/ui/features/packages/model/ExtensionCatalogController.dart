// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../common/contributions/ChatSidebarTabHost.dart';
import '../../../main/navigation/ToolPkgCatalogChangeBus.dart';
import '../market/MarketInstallStateStore.dart';
import '../utils/MCPCommandRunner.dart';

/// Location for quick installs; the detail page lets users pick another one.
const String defaultExtensionInstallScope = 'device';

/// Owns the local extension catalog and every mutation the extension UI performs.
///
/// One instance serves each Core connection, so the list, device and detail
/// routes read the same state. Pages [attach] while they are visible.
class ExtensionCatalogController extends ChangeNotifier {
  ExtensionCatalogController._(this.clients);

  static final Expando<ExtensionCatalogController> _controllers =
      Expando<ExtensionCatalogController>();

  static ExtensionCatalogController of(GeneratedCoreProxyClients clients) =>
      _controllers[clients.bridge] ??= ExtensionCatalogController._(clients);

  /// How long an added or changed extension stays marked as recent.
  static const Duration recentWindow = Duration(seconds: 6);

  final GeneratedCoreProxyClients clients;

  core_proxy.ExtensionCatalogSnapshot? _snapshot;
  String? _error;
  bool _loading = false;
  bool _checkingUpdates = false;
  DateTime? _lastUpdateCheck;
  Future<void>? _load;
  bool _loadPending = false;
  final Set<String> _busy = <String>{};
  final Map<String, bool> _pendingEnabled = <String, bool>{};
  final Set<String> _marketUpdateEntryIds = <String>{};
  int _attachments = 0;
  StreamSubscription<void>? _busSubscription;
  StreamSubscription<int>? _revisionSubscription;
  final Map<String, DateTime> _recentChanges = <String, DateTime>{};
  final Set<String> _quietKeys = <String>{};
  Timer? _recentExpiry;

  core_proxy.ExtensionCatalogSnapshot? get snapshot => _snapshot;
  String? get error => _error;
  bool get loading => _loading;
  bool get checkingUpdates => _checkingUpdates;
  DateTime? get lastUpdateCheck => _lastUpdateCheck;

  static String entryKey(core_proxy.ExtensionCatalogEntry entry) =>
      '${entry.kind}:${entry.id}';

  static String candidateKey(core_proxy.ExtensionCatalogCandidate candidate) =>
      'bundled:${candidate.id}';

  static bool isEnabled(core_proxy.ExtensionCatalogEntry entry) =>
      entry.activationState == 'enabled' ||
      entry.activationState == 'aiVisible';

  /// Whether the extension reports a runtime or dependency problem.
  static bool hasProblem(core_proxy.ExtensionCatalogEntry entry) =>
      entry.runtimeState == 'error' ||
      entry.runtimeState.startsWith('error:') ||
      entry.dependencyIssueCount > 0;

  /// Maps a catalog kind to the ownership kind used by extension scopes.
  static String scopeKindOf(String kind) => switch (kind) {
    'toolpkg' || 'script' => 'package',
    'skill' => 'skill',
    'mcp' => 'mcp',
    'a2a' => 'a2a',
    _ => throw StateError('Unsupported extension kind: $kind'),
  };

  /// Starts following catalog changes while a page that shows them is visible.
  void attach() {
    _attachments += 1;
    if (_attachments == 1) {
      _busSubscription = ToolPkgCatalogChangeBus.listen(() {
        unawaited(refresh());
      });
      // The refresh below reads the current revision; follow only later ones.
      _revisionSubscription = clients.application
          .extensionCatalogRevisionFlow()
          .skip(1)
          .listen((_) => unawaited(refresh()));
    }
    unawaited(refresh());
  }

  /// Stops following changes once no visible page shows the catalog.
  void detach() {
    if (_attachments == 0) {
      throw StateError('ExtensionCatalogController.detach without attach');
    }
    _attachments -= 1;
    if (_attachments == 0) {
      _busSubscription?.cancel();
      _busSubscription = null;
      _revisionSubscription?.cancel();
      _revisionSubscription = null;
    }
  }

  bool isBusy(String key) => _busy.contains(key);

  /// Activation state a running toggle is moving to, so switches can settle early.
  bool? pendingEnabled(String key) => _pendingEnabled[key];

  /// Whether a recent catalog read added or visibly changed this extension.
  bool isRecentlyChanged(String key) => _recentChanges.containsKey(key);

  bool hasUpdate(core_proxy.ExtensionCatalogEntry entry) {
    final marketEntryId = entry.marketEntryId;
    return entry.updateState == 'available' ||
        (marketEntryId != null &&
            _marketUpdateEntryIds.contains(marketEntryId));
  }

  List<core_proxy.ExtensionCatalogEntry> updatableEntries() {
    final snapshot = _snapshot;
    if (snapshot == null) {
      return const <core_proxy.ExtensionCatalogEntry>[];
    }
    return snapshot.installed.where(hasUpdate).toList(growable: false);
  }

  core_proxy.ExtensionCatalogEntry? findEntry(String kind, String id) {
    final snapshot = _snapshot;
    if (snapshot == null) {
      return null;
    }
    for (final entry in snapshot.installed) {
      if (entry.kind == kind && entry.id == id) {
        return entry;
      }
    }
    return null;
  }

  core_proxy.ExtensionCatalogCandidate? findCandidate(String kind, String id) {
    final snapshot = _snapshot;
    if (snapshot == null) {
      return null;
    }
    for (final candidate in snapshot.bundledAvailable) {
      if (candidate.kind == kind && candidate.id == id) {
        return candidate;
      }
    }
    return null;
  }

  /// Coalesces overlapping requests and always ends with the latest catalog.
  ///
  /// Page activity changes call this while the framework is building, so the
  /// read starts on a later event-loop turn instead of notifying synchronously.
  Future<void> refresh() {
    _loadPending = true;
    return _load ??= Future<void>(_drainLoads);
  }

  Future<void> _drainLoads() async {
    _loading = true;
    notifyListeners();
    try {
      while (_loadPending) {
        _loadPending = false;
        try {
          final snapshot = await clients.application
              .getExtensionCatalogSnapshot();
          _recordChanges(_snapshot, snapshot);
          _snapshot = snapshot;
          _error = null;
        } catch (error, stackTrace) {
          debugPrint('Failed to load extension catalog: $error\n$stackTrace');
          _error = error.toString();
        }
        notifyListeners();
      }
    } finally {
      _load = null;
      _loading = false;
      notifyListeners();
    }
  }

  /// Marks extensions that a later read added or changed, except the ones the
  /// user just toggled here.
  void _recordChanges(
    core_proxy.ExtensionCatalogSnapshot? previous,
    core_proxy.ExtensionCatalogSnapshot next,
  ) {
    if (previous == null) {
      return;
    }
    final before = <String, core_proxy.ExtensionCatalogEntry>{
      for (final entry in previous.installed) entryKey(entry): entry,
    };
    final now = DateTime.now();
    for (final entry in next.installed) {
      final key = entryKey(entry);
      if (_quietKeys.contains(key)) {
        continue;
      }
      final old = before[key];
      if (old == null || _visiblyChanged(old, entry)) {
        _recentChanges[key] = now;
      }
    }
    _scheduleRecentExpiry();
  }

  static bool _visiblyChanged(
    core_proxy.ExtensionCatalogEntry before,
    core_proxy.ExtensionCatalogEntry after,
  ) {
    return before.activationState != after.activationState ||
        before.runtimeState != after.runtimeState ||
        before.scope != after.scope ||
        before.version != after.version ||
        before.updateState != after.updateState ||
        before.dependencyIssueCount != after.dependencyIssueCount;
  }

  void _scheduleRecentExpiry() {
    _recentExpiry?.cancel();
    if (_recentChanges.isEmpty) {
      return;
    }
    final oldest = _recentChanges.values.reduce(
      (a, b) => a.isBefore(b) ? a : b,
    );
    final remaining = recentWindow - DateTime.now().difference(oldest);
    _recentExpiry = Timer(remaining.isNegative ? Duration.zero : remaining, () {
      final cutoff = DateTime.now().subtract(recentWindow);
      _recentChanges.removeWhere((_, at) => !at.isAfter(cutoff));
      notifyListeners();
      _scheduleRecentExpiry();
    });
  }

  Future<void> setEnabled(
    core_proxy.ExtensionCatalogEntry entry,
    bool enabled,
  ) async {
    final key = entryKey(entry);
    if (_busy.contains(key)) {
      return;
    }
    _pendingEnabled[key] = enabled;
    _quietKeys.add(key);
    try {
      await _toggle(entry, key, enabled);
    } finally {
      _pendingEnabled.remove(key);
      _quietKeys.remove(key);
      notifyListeners();
    }
  }

  Future<void> _toggle(
    core_proxy.ExtensionCatalogEntry entry,
    String key,
    bool enabled,
  ) {
    return _mutate(key, () async {
      final packageManager = clients.application.packageManager();
      switch (entry.kind) {
        case 'toolpkg':
          if (enabled) {
            await packageManager.enableToolPkgContainer(
              containerPackageName: entry.id,
            );
          } else {
            await packageManager.disableToolPkgContainer(
              containerPackageName: entry.id,
            );
            await _resetOwnedSidebarTab(entry);
          }
        case 'script':
          if (enabled) {
            await packageManager.enablePackage(packageName: entry.id);
          } else {
            await packageManager.disablePackage(packageName: entry.id);
          }
        case 'skill':
          await clients.application.skillRepository().setSkillVisibleToAi(
            skillName: entry.id,
            visible: enabled,
          );
        case 'a2a':
          await clients.application.setA2AEnabled(
            id: entry.id,
            enabled: enabled,
          );
        case 'mcp':
          await clients.permissionsMcpRuntimeMcpLocalServer.setServerEnabled(
            serverId: entry.id,
            enabled: enabled,
          );
          await applyMcpServerLifecycle(
            clients: clients,
            serverId: entry.id,
            enabled: enabled,
          );
        default:
          throw StateError('Unsupported extension kind: ${entry.kind}');
      }
    }, reloadPluginCode: entry.kind == 'toolpkg' || entry.kind == 'script');
  }

  /// Deletes an extension; for customized built-ins this restores the bundled copy.
  Future<void> remove(core_proxy.ExtensionCatalogEntry entry) {
    return _mutate(entryKey(entry), () async {
      switch (entry.kind) {
        case 'toolpkg' || 'script':
          await clients.application.packageManager().deletePackage(
            packageName: entry.id,
          );
        case 'skill':
          final deleted = await clients.application
              .skillRepository()
              .deleteSkill(skillName: entry.id);
          if (!deleted) {
            throw StateError('删除技能失败：${entry.id}');
          }
        case 'a2a':
          await clients.application.deleteA2A(id: entry.id);
        case 'mcp':
          await killMcpServer(clients: clients, serverId: entry.id);
          await clients.permissionsMcpRuntimeMcpLocalServer.removeMcpServer(
            serverId: entry.id,
          );
        default:
          throw StateError('Unsupported extension kind: ${entry.kind}');
      }
    });
  }

  /// Installs an offline official extension into [scope].
  Future<void> installCandidate(
    core_proxy.ExtensionCatalogCandidate candidate,
    String scope,
  ) {
    return _mutate(candidateKey(candidate), () async {
      final packageManager = clients.application.packageManager();
      switch (candidate.kind) {
        case 'skill':
          await clients.application
              .skillRepository()
              .importBundledExternalSkill(skillName: candidate.id);
          await clients.application.setExtensionScope(
            kind: 'skill',
            id: candidate.id,
            scope: scope,
          );
        case 'toolpkg':
          await packageManager.importBundledExternalToolPkgContainer(
            containerPackageName: candidate.id,
          );
          if (candidate.requiredScope == null) {
            await clients.application.setExtensionScope(
              kind: 'package',
              id: candidate.id,
              scope: scope,
            );
          }
        case 'script':
          await packageManager.importBundledExternalPackage(
            packageName: candidate.id,
          );
          if (candidate.requiredScope == null) {
            await clients.application.setExtensionScope(
              kind: 'package',
              id: candidate.id,
              scope: scope,
            );
          }
        default:
          throw StateError('Unsupported extension kind: ${candidate.kind}');
      }
    });
  }

  /// Updates from the bundled copy or the marketplace entry the extension came from.
  Future<void> update(core_proxy.ExtensionCatalogEntry entry) {
    return _mutate(entryKey(entry), () async {
      if (entry.updateState == 'available' &&
          (entry.source == 'bundled' || entry.source == 'builtin')) {
        if (entry.kind == 'skill') {
          await clients.application
              .skillRepository()
              .importBundledExternalSkill(skillName: entry.id);
          return;
        }
        final packageManager = clients.application.packageManager();
        if (entry.kind == 'toolpkg') {
          await packageManager.importBundledExternalToolPkgContainer(
            containerPackageName: entry.id,
          );
        } else {
          await packageManager.importBundledExternalPackage(
            packageName: entry.id,
          );
        }
        return;
      }
      final marketEntryId = entry.marketEntryId;
      if (marketEntryId == null || marketEntryId.trim().isEmpty) {
        throw StateError('「${entry.displayName}」没有可用的更新来源。');
      }
      await clients.application.installMarketEntry(
        entryId: marketEntryId,
        versionId: null,
      );
      _marketUpdateEntryIds.remove(marketEntryId);
      unawaited(MarketInstallStateStore.of(clients).refresh());
    });
  }

  Future<void> updateAll() async {
    for (final entry in updatableEntries()) {
      await update(entry);
    }
  }

  Future<void> moveScope(core_proxy.ExtensionCatalogEntry entry, String scope) {
    return _mutate(entryKey(entry), () async {
      await clients.application.setExtensionScope(
        kind: scopeKindOf(entry.kind),
        id: entry.id,
        scope: scope,
      );
    });
  }

  /// Imports a package file chosen by the user into [scope].
  Future<void> importPackageFile(
    String path, {
    required String scope,
    required bool requireToolPkg,
  }) {
    return _mutate('import:$path', () async {
      final result = await clients.application
          .packageManager()
          .addPackageFileFromExternalStorageResult(filePath: path);
      if (requireToolPkg && !result.packageFormat.contains('toolpkg')) {
        throw StateError('所选文件不是 ToolPkg：$path');
      }
      await clients.application.setExtensionScope(
        kind: 'package',
        id: result.packageName,
        scope: scope,
      );
    });
  }

  /// Refreshes bundled updates and asks the marketplace for newer installed versions.
  Future<void> checkUpdates() async {
    if (_checkingUpdates) {
      return;
    }
    _checkingUpdates = true;
    notifyListeners();
    try {
      await refresh();
      final checks = await clients.application.checkExtensionCatalogUpdates();
      _marketUpdateEntryIds
        ..clear()
        ..addAll(
          checks
              .where((check) => check.updateAvailable)
              .map((check) => check.marketEntryId),
        );
      _lastUpdateCheck = DateTime.now();
    } finally {
      _checkingUpdates = false;
      notifyListeners();
    }
  }

  Future<void> deleteDiagnostic(
    core_proxy.ExtensionCatalogDiagnostic diagnostic,
  ) {
    return _mutate('diagnostic:${diagnostic.sourcePath}', () async {
      await clients.application.deleteExtensionCatalogDiagnostic(
        diagnostic: diagnostic,
      );
    });
  }

  /// Runs one mutation with a per-item busy flag and resyncs the catalog afterwards.
  Future<void> _mutate(
    String key,
    Future<void> Function() action, {
    bool reloadPluginCode = true,
  }) async {
    if (!_busy.add(key)) {
      return;
    }
    notifyListeners();
    try {
      await action();
      ToolPkgCatalogChangeBus.notifyCatalogChanged(
        reloadPluginCode: reloadPluginCode,
      );
    } finally {
      await refresh();
      _busy.remove(key);
      notifyListeners();
    }
  }

  Future<void> _resetOwnedSidebarTab(
    core_proxy.ExtensionCatalogEntry entry,
  ) async {
    final preferences = UserPreferencesManager(clients: clients);
    final selectedTab = await preferences.loadChatSidebarTab();
    final ownsSelectedTab = entry.chatSidebarEntryIds
        .map((entryId) => chatSidebarTabIdentity(entry.id, entryId))
        .contains(selectedTab);
    if (ownsSelectedTab) {
      await preferences.saveChatSidebarTab(
        UserPreferencesManager.CHAT_SIDEBAR_WORKSPACE_TAB,
      );
    }
  }
}
