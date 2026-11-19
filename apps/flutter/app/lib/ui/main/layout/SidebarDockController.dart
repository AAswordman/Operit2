// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../navigation/AppNavigationModels.dart';
import 'SidebarDockPreferences.dart';

/// Identifies the sidebar zone that owns one plugin view entry.
enum SidebarDockLocation { primary, secondary }

/// Carries one plugin entry through a sidebar drag operation.
@immutable
class SidebarDockDragPayload {
  /// Creates a drag payload for one registered plugin navigation entry.
  const SidebarDockDragPayload({required this.entryId});

  final String entryId;
}

/// Describes one plugin view that can render inside the secondary sidebar.
@immutable
class SidebarDockedPluginView {
  /// Creates the resolved plugin entry and route pair for a secondary view.
  const SidebarDockedPluginView({required this.entry, required this.route});

  final NavigationEntrySpec entry;
  final RouteSpec route;
}

/// Owns plugin view placement between the primary and secondary sidebars.
class SidebarDockController extends ChangeNotifier {
  /// Creates an empty dock controller before the plugin catalog is available.
  SidebarDockController({
    SidebarDockPreferences preferences = const SidebarDockPreferences(),
  }) : _preferences = preferences;

  static const String workspaceViewId = 'operit.workspace';

  final List<String> _primaryEntryIds = <String>[];
  final List<String> _secondaryEntryIds = <String>[];
  final Map<String, NavigationEntrySpec> _entriesById =
      <String, NavigationEntrySpec>{};
  final Map<String, SidebarDockedPluginView> _dockableViewsById =
      <String, SidebarDockedPluginView>{};
  String _selectedSecondaryViewId = workspaceViewId;
  final SidebarDockPreferences _preferences;
  SidebarDockLayout? _storedLayout;
  bool _preferencesLoaded = false;
  StreamSubscription<SidebarDockLayout?>? _preferencesSubscription;
  Future<void>? _saveFuture;
  bool _saveAgain = false;

  /// Returns the plugin entries currently shown in the primary sidebar.
  List<NavigationEntrySpec> get primaryEntries =>
      List<NavigationEntrySpec>.unmodifiable(_resolveEntries(_primaryEntryIds));

  /// Returns the plugin views currently shown in the secondary sidebar.
  List<SidebarDockedPluginView> get secondaryViews =>
      List<SidebarDockedPluginView>.unmodifiable(
        _resolveDockableViews(_secondaryEntryIds),
      );

  /// Returns the selected right-side view identifier.
  String get selectedSecondaryViewId => _selectedSecondaryViewId;

  /// Loads and observes committed layouts before applying subsequent catalog updates.
  Future<void> loadPreferences() async {
    await _preferencesSubscription?.cancel();
    final loaded = Completer<void>();
    _preferencesSubscription = _preferences.watch().listen(
      (layout) {
        _storedLayout = layout;
        _preferencesLoaded = true;
        if (_applyStoredLayout()) {
          notifyListeners();
        }
        if (!loaded.isCompleted) {
          loaded.complete();
        }
      },
      onError: (Object error, StackTrace stackTrace) {
        if (!loaded.isCompleted) {
          loaded.completeError(error, stackTrace);
          return;
        }
        FlutterError.reportError(
          FlutterErrorDetails(
            exception: error,
            stack: stackTrace,
            library: 'sidebar dock persistence',
            context: ErrorDescription('while observing sidebar dock layout'),
          ),
        );
      },
    );
    await loaded.future;
  }

  /// Detaches the preference watch before releasing the sidebar state owner.
  @override
  void dispose() {
    unawaited(_preferencesSubscription?.cancel());
    _preferencesSubscription = null;
    super.dispose();
  }

  /// Reconciles dock placement with the current plugin navigation catalog.
  void synchronize({
    required List<NavigationEntrySpec> pluginEntries,
    required Map<String, RouteSpec> routesById,
  }) {
    final entriesById = <String, NavigationEntrySpec>{
      for (final entry in pluginEntries) entry.entryId: entry,
    };
    final dockableViewsById = <String, SidebarDockedPluginView>{};
    for (final entry in pluginEntries) {
      final route = routesById[entry.routeId];
      if (entry.action == null &&
          route != null &&
          route.runtime == RouteRuntime.toolPkgComposeDsl &&
          route.ownerPackageName != null &&
          route.toolPkgUiModuleId != null) {
        dockableViewsById[entry.entryId] = SidebarDockedPluginView(
          entry: entry,
          route: route,
        );
      }
    }

    final knownEntryIds = entriesById.keys.toSet();
    final primaryBefore = List<String>.of(_primaryEntryIds);
    final secondaryBefore = List<String>.of(_secondaryEntryIds);
    _primaryEntryIds.removeWhere((entryId) => !knownEntryIds.contains(entryId));
    _secondaryEntryIds.removeWhere(
      (entryId) => !dockableViewsById.containsKey(entryId),
    );
    for (final entry in pluginEntries) {
      if (!_primaryEntryIds.contains(entry.entryId) &&
          !_secondaryEntryIds.contains(entry.entryId)) {
        _primaryEntryIds.add(entry.entryId);
      }
    }

    _entriesById
      ..clear()
      ..addAll(entriesById);
    _dockableViewsById
      ..clear()
      ..addAll(dockableViewsById);
    if (_preferencesLoaded) {
      _applyStoredLayout();
    }
    if (_selectedSecondaryViewId != workspaceViewId &&
        !_secondaryEntryIds.contains(_selectedSecondaryViewId)) {
      _selectedSecondaryViewId = workspaceViewId;
    }
    if (!listEquals(primaryBefore, _primaryEntryIds) ||
        !listEquals(secondaryBefore, _secondaryEntryIds)) {
      notifyListeners();
    }
  }

  /// Reports whether a plugin entry may move into the requested sidebar zone.
  bool canMove(String entryId, SidebarDockLocation location) {
    if (!_entriesById.containsKey(entryId)) {
      return false;
    }
    return switch (location) {
      SidebarDockLocation.primary => true,
      SidebarDockLocation.secondary => _dockableViewsById.containsKey(entryId),
    };
  }

  /// Moves one plugin entry into the requested zone at the supplied insertion index.
  void move(
    String entryId, {
    required SidebarDockLocation location,
    required int insertionIndex,
  }) {
    if (!canMove(entryId, location)) {
      throw StateError('Plugin entry cannot move to $location: $entryId');
    }
    final source = _sourceFor(entryId);
    final target = switch (location) {
      SidebarDockLocation.primary => _primaryEntryIds,
      SidebarDockLocation.secondary => _secondaryEntryIds,
    };
    final sourceIndex = source.indexOf(entryId);
    var targetIndex = insertionIndex;
    if (identical(source, target) && sourceIndex < targetIndex) {
      targetIndex -= 1;
    }
    source.removeAt(sourceIndex);
    target.insert(targetIndex.clamp(0, target.length), entryId);
    if (location == SidebarDockLocation.secondary) {
      _selectedSecondaryViewId = entryId;
    } else if (_selectedSecondaryViewId == entryId) {
      _selectedSecondaryViewId = workspaceViewId;
    }
    notifyListeners();
    _scheduleSave();
  }

  /// Selects the workspace or one currently docked plugin view on the right.
  void selectSecondaryView(String viewId) {
    if (viewId != workspaceViewId && !_secondaryEntryIds.contains(viewId)) {
      throw StateError('Unknown secondary sidebar view: $viewId');
    }
    if (_selectedSecondaryViewId == viewId) {
      return;
    }
    _selectedSecondaryViewId = viewId;
    notifyListeners();
    _scheduleSave();
  }

  /// Applies stored ordering to the currently registered plugin catalog.
  bool _applyStoredLayout() {
    final layout = _storedLayout;
    if (layout == null) {
      final changed =
          !listEquals(_primaryEntryIds, _entriesById.keys.toList()) ||
          _secondaryEntryIds.isNotEmpty ||
          _selectedSecondaryViewId != workspaceViewId;
      _primaryEntryIds
        ..clear()
        ..addAll(_entriesById.keys);
      _secondaryEntryIds.clear();
      _selectedSecondaryViewId = workspaceViewId;
      return changed;
    }
    final primaryBefore = List<String>.of(_primaryEntryIds);
    final secondaryBefore = List<String>.of(_secondaryEntryIds);
    final selectedBefore = _selectedSecondaryViewId;
    final knownIds = _entriesById.keys.toSet();
    final dockableIds = _dockableViewsById.keys.toSet();
    _primaryEntryIds
      ..clear()
      ..addAll(
        layout.primaryEntryIds.where((entryId) => knownIds.contains(entryId)),
      );
    _secondaryEntryIds
      ..clear()
      ..addAll(
        layout.secondaryEntryIds.where(
          (entryId) => dockableIds.contains(entryId),
        ),
      );
    for (final entryId in _entriesById.keys) {
      if (!_primaryEntryIds.contains(entryId) &&
          !_secondaryEntryIds.contains(entryId)) {
        _primaryEntryIds.add(entryId);
      }
    }
    _selectedSecondaryViewId = layout.selectedSecondaryViewId;
    if (_selectedSecondaryViewId != workspaceViewId &&
        !_secondaryEntryIds.contains(_selectedSecondaryViewId)) {
      _selectedSecondaryViewId = workspaceViewId;
    }
    return !listEquals(primaryBefore, _primaryEntryIds) ||
        !listEquals(secondaryBefore, _secondaryEntryIds) ||
        selectedBefore != _selectedSecondaryViewId;
  }

  /// Queues one serialized layout save without blocking drag interactions.
  void _scheduleSave() {
    if (!_preferencesLoaded) {
      return;
    }
    if (_saveFuture != null) {
      _saveAgain = true;
      return;
    }
    _saveFuture = _saveLayout();
  }

  /// Saves the latest layout and releases the serialized save lock.
  Future<void> _saveLayout() async {
    try {
      await _preferences.save(
        SidebarDockLayout(
          primaryEntryIds: List<String>.unmodifiable(_primaryEntryIds),
          secondaryEntryIds: List<String>.unmodifiable(_secondaryEntryIds),
          selectedSecondaryViewId: _selectedSecondaryViewId,
        ),
      );
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'sidebar dock persistence',
          context: ErrorDescription('while saving sidebar dock layout'),
        ),
      );
    } finally {
      _saveFuture = null;
      if (_saveAgain) {
        _saveAgain = false;
        _scheduleSave();
      }
    }
  }

  /// Resolves ordered primary entries from the authoritative catalog snapshot.
  List<NavigationEntrySpec> _resolveEntries(List<String> entryIds) {
    return <NavigationEntrySpec>[
      for (final entryId in entryIds)
        _entriesById[entryId] ??
            (throw StateError('Unknown sidebar plugin entry: $entryId')),
    ];
  }

  /// Resolves ordered secondary views from the authoritative catalog snapshot.
  List<SidebarDockedPluginView> _resolveDockableViews(List<String> entryIds) {
    return <SidebarDockedPluginView>[
      for (final entryId in entryIds)
        _dockableViewsById[entryId] ??
            (throw StateError('Unknown secondary sidebar view: $entryId')),
    ];
  }

  /// Returns the current zone list that contains an entry being moved.
  List<String> _sourceFor(String entryId) {
    if (_primaryEntryIds.contains(entryId)) {
      return _primaryEntryIds;
    }
    if (_secondaryEntryIds.contains(entryId)) {
      return _secondaryEntryIds;
    }
    throw StateError('Unplaced sidebar plugin entry: $entryId');
  }
}

/// Publishes the application-wide plugin sidebar dock controller.
class SidebarDockScope extends InheritedNotifier<SidebarDockController> {
  /// Creates a scope that exposes the dock controller to both sidebar zones.
  const SidebarDockScope({
    super.key,
    required SidebarDockController controller,
    required super.child,
  }) : super(notifier: controller);

  /// Reads the dock controller installed for the current application layout.
  static SidebarDockController of(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<SidebarDockScope>();
    if (scope == null) {
      throw StateError('SidebarDockScope is not installed');
    }
    final controller = scope.notifier;
    if (controller == null) {
      throw StateError('SidebarDockController is not installed');
    }
    return controller;
  }

  /// Reads the dock controller when the current subtree participates in docking.
  static SidebarDockController? maybeOf(BuildContext context) {
    return context
        .dependOnInheritedWidgetOfExactType<SidebarDockScope>()
        ?.notifier;
  }
}
