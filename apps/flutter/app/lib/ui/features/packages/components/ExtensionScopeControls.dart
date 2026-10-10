// ignore_for_file: file_names
import 'package:flutter/material.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import 'PackageGrid.dart';

/// Resolves the display label of an explicit extension location.
String extensionScopeLabel(String scope) => switch (scope) {
  'space' => '设备空间',
  'device' => '仅本设备',
  'builtin' => '内置',
  _ => throw StateError('未知扩展位置：$scope'),
};

/// Asks where a new extension should live and explains shared configuration.
Future<String?> chooseExtensionScope(BuildContext context) =>
    showDialog<String>(
      context: context,
      builder: (context) => SimpleDialog(
        title: const Text('选择存放位置'),
        children: [
          SimpleDialogOption(
            onPressed: () => Navigator.pop(context, 'device'),
            child: const ListTile(
              leading: Icon(Icons.devices),
              title: Text('仅本设备'),
              subtitle: Text('内容和配置仅保存在本设备，不同步到其他设备。'),
            ),
          ),
          SimpleDialogOption(
            onPressed: () => Navigator.pop(context, 'space'),
            child: const ListTile(
              leading: Icon(Icons.cloud_outlined),
              title: Text('设备空间'),
              subtitle: Text('内容、配置和启用状态同步到空间中的其他设备。配置中的密钥也会共享。'),
            ),
          ),
        ],
      ),
    );

/// Confirms an ownership transfer and applies it through the runtime facade.
Future<bool> changeExtensionScope({
  required BuildContext context,
  required GeneratedCoreProxyClients clients,
  required String kind,
  required String id,
  required String currentScope,
}) async {
  final target = switch (currentScope) {
    'device' => 'space',
    'space' => 'device',
    _ => throw StateError('此扩展不能切换位置'),
  };
  final confirmed = await showDialog<bool>(
    context: context,
    builder: (context) => AlertDialog(
      title: Text('移至${extensionScopeLabel(target)}？'),
      content: Text(
        target == 'space'
            ? '“$id”的内容、配置和启用状态将同步给设备空间中的其他设备。配置中的密钥也会共享。'
            : '“$id”将只保留在本设备，空间中的其他设备会移除该扩展及其配置。',
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context, false),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: () => Navigator.pop(context, true),
          child: const Text('移动'),
        ),
      ],
    ),
  );
  if (confirmed != true) return false;
  await clients.application.setExtensionScope(
    kind: kind,
    id: id,
    scope: target,
  );
  return true;
}

/// Shows a location switch for portable extensions and a fixed label for built-ins.
class ExtensionScopeAction extends StatelessWidget {
  /// Creates a location action without inferring scope from filenames.
  const ExtensionScopeAction({
    super.key,
    required this.scope,
    required this.onMove,
    this.localMcp = false,
  });
  final String scope;
  final VoidCallback onMove;
  final bool localMcp;

  /// Builds the portable move button or the immutable location label.
  @override
  Widget build(BuildContext context) {
    if (scope == 'builtin' || localMcp) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: Text(
          localMcp ? '仅本设备 · 本地部署' : '内置',
          style: Theme.of(context).textTheme.labelSmall,
        ),
      );
    }
    return IconButton(
      tooltip:
          '移至${extensionScopeLabel(scope == 'space' ? 'device' : 'space')}',
      onPressed: onMove,
      icon: const Icon(Icons.drive_file_move_outline),
    );
  }
}

/// Separates installed extensions into independently rendered location sections.
class ScopedExtensionSliver<T> extends StatelessWidget {
  /// Creates scope-grouped lazy rows using the existing item renderer.
  const ScopedExtensionSliver({
    super.key,
    required this.items,
    required this.scopes,
    required this.identity,
    required this.itemBuilder,
  });
  final List<T> items;
  final Map<String, String> scopes;
  final String Function(T) identity;
  final Widget Function(BuildContext, T) itemBuilder;

  /// Builds one header and lazy list for each nonempty explicit scope.
  @override
  Widget build(BuildContext context) {
    final grouped = <String, List<T>>{'space': [], 'device': [], 'builtin': []};
    for (final item in items) {
      final scope = scopes[identity(item)];
      if (!grouped.containsKey(scope)) {
        throw StateError('扩展缺少有效的位置：${identity(item)}');
      }
      grouped[scope]!.add(item);
    }
    return SliverMainAxisGroup(
      slivers: [
        for (final entry in grouped.entries)
          if (entry.value.isNotEmpty) ...[
            SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.fromLTRB(4, 12, 4, 8),
                child: Text(
                  extensionScopeLabel(entry.key),
                  style: Theme.of(context).textTheme.titleMedium?.copyWith(
                    fontWeight: FontWeight.w700,
                  ),
                ),
              ),
            ),
            PackageSliverList(
              itemCount: entry.value.length,
              itemBuilder: (context, index) =>
                  itemBuilder(context, entry.value[index]),
            ),
          ],
      ],
    );
  }
}
