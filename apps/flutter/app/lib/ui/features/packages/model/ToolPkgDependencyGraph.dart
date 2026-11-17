// ignore_for_file: file_names

import 'package:pub_semver/pub_semver.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import 'ExtensionCatalogController.dart';

enum ToolPkgDependencyState {
  satisfied('已满足'),
  missing('未安装'),
  incompatible('版本不满足'),
  disabled('未启用'),
  blocked('依赖未满足'),
  cycle('循环依赖');

  const ToolPkgDependencyState(this.label);
  final String label;
}

class ToolPkgDependencySource {
  const ToolPkgDependencySource(this.parentId, this.requirement, this.optional);

  final String parentId;
  final core_proxy.ToolPkgManifestRequirement requirement;
  final bool optional;
}

class ToolPkgDependencyNode {
  ToolPkgDependencyNode(this.id, this.entry);

  final String id;
  final core_proxy.ExtensionCatalogEntry? entry;
  final List<ToolPkgDependencySource> sources = [];
  bool required = false;
  ToolPkgDependencyState state = ToolPkgDependencyState.missing;
}

/// Expands installed manifests. A node is required only if a path from the root
/// contains no optional edges. Revisit that node when a required path is found.
class ToolPkgDependencyGraph {
  ToolPkgDependencyGraph({
    required core_proxy.ToolPkgContainerRuntime root,
    required Iterable<core_proxy.ToolPkgContainerRuntime> runtimes,
    required Iterable<core_proxy.ExtensionCatalogEntry> installed,
  }) {
    final manifests = {
      for (final runtime in runtimes) runtime.packageName: runtime,
    };
    final entries = {
      for (final entry in installed)
        if (entry.kind == 'toolpkg') entry.id: entry,
    };
    final pending = <(String, bool)>[(root.packageName, true)];
    final visited = <(String, bool)>{};
    for (var index = 0; index < pending.length; index++) {
      final (parentId, requiredPath) = pending[index];
      if (!visited.add((parentId, requiredPath))) continue;
      final parent = manifests[parentId];
      if (parent == null) continue;
      for (final (requirements, optional) in [
        (parent.requires, false),
        (parent.optionalRequires, true),
      ]) {
        for (final requirement in requirements) {
          final id = requirement.id.trim();
          // A cycle may point at the detail page's own plugin. Keep traversing
          // safely, without listing the plugin as its own prerequisite.
          if (id == root.packageName) continue;
          final node = _nodes.putIfAbsent(
            id,
            () => ToolPkgDependencyNode(id, entries[id]),
          );
          if (!node.sources.any(
            (source) =>
                source.parentId == parentId && source.optional == optional,
          )) {
            node.sources.add(
              ToolPkgDependencySource(parentId, requirement, optional),
            );
          }
          final required = requiredPath && !optional;
          node.required |= required;
          pending.add((id, required));
        }
      }
    }

    // Check the mandatory subtree independently of optional branches. An
    // enabled plugin with a missing prerequisite is not reported as satisfied.
    final memo = <String, ToolPkgDependencyState>{};
    ToolPkgDependencyState evaluate(String id, Set<String> path) {
      if (path.contains(id)) return ToolPkgDependencyState.cycle;
      final cached = memo[id];
      if (cached != null) return cached;
      final entry = entries[id];
      if (entry == null) return ToolPkgDependencyState.missing;
      if (!ExtensionCatalogController.isEnabled(entry)) {
        return ToolPkgDependencyState.disabled;
      }
      final nextPath = {...path, id};
      for (final requirement
          in manifests[id]?.requires ??
              <core_proxy.ToolPkgManifestRequirement>[]) {
        final childId = requirement.id.trim();
        final child = entries[childId];
        if (child != null && !_matches(child.version, requirement)) {
          return memo[id] = ToolPkgDependencyState.blocked;
        }
        final state = evaluate(childId, nextPath);
        if (state != ToolPkgDependencyState.satisfied) {
          return memo[id] = state == ToolPkgDependencyState.cycle
              ? ToolPkgDependencyState.cycle
              : ToolPkgDependencyState.blocked;
        }
      }
      if (entry.dependencyIssueCount > 0) {
        return memo[id] = ToolPkgDependencyState.blocked;
      }
      return memo[id] = ToolPkgDependencyState.satisfied;
    }

    for (final node in nodes) {
      final entry = node.entry;
      node.state = entry == null
          ? ToolPkgDependencyState.missing
          : node.sources.any(
              (source) => !_matches(entry.version, source.requirement),
            )
          ? ToolPkgDependencyState.incompatible
          : evaluate(node.id, {});
    }
  }

  final Map<String, ToolPkgDependencyNode> _nodes = {};
  Iterable<ToolPkgDependencyNode> get nodes => _nodes.values;

  static bool _matches(
    String version,
    core_proxy.ToolPkgManifestRequirement requirement,
  ) {
    if (requirement.minVersion == null && requirement.maxVersion == null) {
      return true;
    }
    Version parse(String value) => Version.parse(
      value.trim().replaceFirst(RegExp(r'^v+'), '').split('+').first,
    );
    try {
      final current = parse(version);
      return (requirement.minVersion == null ||
              current >= parse(requirement.minVersion!)) &&
          (requirement.maxVersion == null ||
              current <= parse(requirement.maxVersion!));
    } on FormatException {
      return false;
    }
  }
}
