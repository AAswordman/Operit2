// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/components/PageActivityMixin.dart';
import '../components/ExtensionTopBar.dart';
import '../components/StoreOfficialExtensionRow.dart';
import '../components/StoreDetailKit.dart';
import '../components/StoreVisuals.dart';
import '../model/ExtensionCatalogController.dart';
import 'StoreBrowseScreen.dart';

/// All bundled extensions, available without requesting the online market.
class OfficialExtensionsScreen extends StatefulWidget {
  const OfficialExtensionsScreen({
    super.key,
    GeneratedCoreProxyClients? clients,
  }) : clients =
           clients ?? const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());

  final GeneratedCoreProxyClients clients;

  @override
  State<OfficialExtensionsScreen> createState() =>
      _OfficialExtensionsScreenState();
}

class _OfficialExtensionsScreenState extends State<OfficialExtensionsScreen>
    with PageActivityMixin<OfficialExtensionsScreen> {
  late final ExtensionCatalogController _controller =
      ExtensionCatalogController.of(widget.clients);

  // Retain candidates as they are installed so rows keep their place.
  final List<core_proxy.ExtensionCatalogCandidate> _candidates = [];

  @override
  void initState() {
    super.initState();
    _controller.addListener(_onChanged);
  }

  @override
  void onPageActivityChanged(bool active) {
    if (active) {
      _controller.attach();
    } else {
      _controller.detach();
    }
  }

  @override
  void dispose() {
    _controller.removeListener(_onChanged);
    if (isPageActive) _controller.detach();
    super.dispose();
  }

  void _onChanged() {
    if (mounted) setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    for (final candidate
        in _controller.snapshot?.bundledAvailable ??
            const <core_proxy.ExtensionCatalogCandidate>[]) {
      if (!_candidates.any(
        (item) => item.kind == candidate.kind && item.id == candidate.id,
      )) {
        _candidates.add(candidate);
      }
    }
    return ExtensionTopBar(
      child: Material(
        type: MaterialType.transparency,
        child: LayoutBuilder(
          builder: (context, constraints) {
            final padding = storeSliverPadding(
              constraints.maxWidth,
              gutter: constraints.maxWidth < 600 ? 12 : 24,
              top: 8,
              bottom: 40,
            );
            return CustomScrollView(
              slivers: <Widget>[
                SliverPadding(
                  padding: padding.copyWith(bottom: 0),
                  sliver: SliverToBoxAdapter(
                    child: StoreSectionHeader(
                      title: '官方扩展',
                      count: _candidates.length,
                      subtitle: '随 Operit2 一起提供，不联网也能安装',
                    ),
                  ),
                ),
                if (_controller.loading && _controller.snapshot == null)
                  SliverPadding(
                    padding: padding,
                    sliver: const SliverToBoxAdapter(
                      child: StoreListSkeleton(),
                    ),
                  )
                else if (_controller.error != null &&
                    _controller.snapshot == null)
                  SliverPadding(
                    padding: padding,
                    sliver: SliverToBoxAdapter(
                      child: StoreNoticeCard(
                        icon: Icons.error_outline_rounded,
                        tone: StoreNoticeTone.error,
                        title: '官方扩展暂时无法加载',
                        message: _controller.error,
                        action: TextButton(
                          onPressed: _controller.refresh,
                          child: const Text('重试'),
                        ),
                      ),
                    ),
                  )
                else if (_candidates.isEmpty)
                  const SliverFillRemaining(
                    hasScrollBody: false,
                    child: Center(child: Text('暂无可安装的官方扩展')),
                  )
                else
                  SliverPadding(
                    padding: padding,
                    sliver: SliverGrid(
                      gridDelegate: SliverGridDelegateWithMaxCrossAxisExtent(
                        maxCrossAxisExtent: 520,
                        mainAxisExtent: 88 * storeTextScale(context),
                        crossAxisSpacing: 16,
                        mainAxisSpacing: 4,
                      ),
                      delegate: SliverChildBuilderDelegate(
                        (context, index) => StoreOfficialExtensionRow(
                          candidate: _candidates[index],
                          controller: _controller,
                        ),
                        childCount: _candidates.length,
                      ),
                    ),
                  ),
              ],
            );
          },
        ),
      ),
    );
  }
}
