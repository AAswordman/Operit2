// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../market/ArtifactMarketSupport.dart';

class ArtifactVersionAssetDetail {
  const ArtifactVersionAssetDetail({
    required this.versionId,
    required this.version,
    required this.formatVer,
    required this.minAppVer,
    required this.maxAppVer,
    required this.publishedAt,
    required this.assetUrl,
    required this.assetKind,
  });

  final String versionId;
  final String version;
  final String formatVer;
  final String minAppVer;
  final String? maxAppVer;
  final String? publishedAt;
  final String assetUrl;
  final String assetKind;
}

class ArtifactVersionListDialog extends StatelessWidget {
  const ArtifactVersionListDialog({
    super.key,
    required this.entry,
  });

  final core_proxy.MarketEntrySummary entry;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    final versions = _versionAssets(entry);
    final latestVersionId = entry.latestVersion?.id ?? '';

    return AlertDialog(
      title: Row(
        children: <Widget>[
          Expanded(
            child: Text(
              entry.title,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
            ),
          ),
          IconButton(
            icon: const Icon(Icons.close),
            onPressed: () => Navigator.of(context).pop(),
          ),
        ],
      ),
      titlePadding: const EdgeInsets.fromLTRB(20, 16, 8, 0),
      contentPadding: const EdgeInsets.fromLTRB(0, 0, 0, 0),
      content: SizedBox(
        width: double.maxFinite,
        height: 520,
        child: versions.isEmpty
            ? Center(
                child: Text(
                  '暂无可用版本',
                  style: textTheme.bodyMedium,
                ),
              )
            : ListView.separated(
                padding: const EdgeInsets.only(top: 8),
                itemCount: versions.length,
                separatorBuilder: (context, index) =>
                    const Divider(height: 1, indent: 72),
                itemBuilder: (context, index) {
                  final version = versions[index];
                  final isLatest = version.versionId == latestVersionId ||
                      (latestVersionId.isEmpty && index == 0);

                  return ListTile(
                    contentPadding:
                        const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
                    leading: Icon(
                      isLatest ? Icons.check_circle : Icons.circle_outlined,
                      color: isLatest ? colorScheme.primary : colorScheme.outline,
                    ),
                    title: Row(
                      children: <Widget>[
                        Text(
                          version.version.isNotEmpty ? version.version : 'v?',
                          style: textTheme.titleSmall,
                        ),
                        if (isLatest) ...[
                          const SizedBox(width: 8),
                          Container(
                            padding: const EdgeInsets.symmetric(
                                horizontal: 6, vertical: 2),
                            decoration: BoxDecoration(
                              color: colorScheme.primaryContainer,
                              borderRadius: BorderRadius.circular(999),
                            ),
                            child: Text(
                              '最新',
                              style: textTheme.labelSmall?.copyWith(
                                color: colorScheme.onPrimaryContainer,
                                fontWeight: FontWeight.w600,
                              ),
                            ),
                          ),
                        ],
                      ],
                    ),
                    subtitle: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        if (version.publishedAt != null)
                          Text(
                            formatMarketDate(version.publishedAt!),
                            style: textTheme.labelSmall?.copyWith(
                              color: colorScheme.outline,
                            ),
                          ),
                        if (version.minAppVer.isNotEmpty ||
                            (version.maxAppVer?.isNotEmpty ?? false))
                          Text(
                            _formatVersionRange(
                                version.minAppVer, version.maxAppVer),
                            style: textTheme.labelSmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                        if (version.assetKind.trim().isNotEmpty)
                          Text(
                            version.assetKind,
                            style: textTheme.labelSmall?.copyWith(
                              color: colorScheme.outline,
                            ),
                          ),
                      ],
                    ),
                    onTap: () {
                      Navigator.of(context).pop(version);
                    },
                  );
                },
              ),
      ),
    );
  }

  List<ArtifactVersionAssetDetail> _versionAssets(core_proxy.MarketEntrySummary entry) {
    final assetsByVersionId = <String, core_proxy.MarketEntryAsset>{
      for (final asset in entry.assets) asset.versionId: asset,
    };
    final versions = entry.versions
        .map((version) {
          final asset = assetsByVersionId[version.id];
          return ArtifactVersionAssetDetail(
            versionId: version.id,
            version: version.version,
            formatVer: version.formatVer,
            minAppVer: version.minAppVer,
            maxAppVer: version.maxAppVer,
            publishedAt: version.publishedAt,
            assetUrl: asset?.url ?? '',
            assetKind: asset?.kind ?? '',
          );
        })
        .toList(growable: false);
    return versions.reversed.toList(growable: false);
  }

  String _formatVersionRange(String min, String? max) {
    final minStr = min.trim();
    final maxStr = (max ?? '').trim();
    if (minStr.isNotEmpty && maxStr.isNotEmpty) {
      return '$minStr - $maxStr';
    }
    if (maxStr.isNotEmpty) {
      return '≤ $maxStr';
    }
    return '$minStr+';
  }
}

/// Show a linear version list dialog and return the selected version detail.
Future<ArtifactVersionAssetDetail?> showArtifactVersionListDialog(
  BuildContext context, {
  required core_proxy.MarketEntrySummary entry,
}) {
  return showDialog<ArtifactVersionAssetDetail>(
    context: context,
    builder: (context) => ArtifactVersionListDialog(entry: entry),
  );
}
