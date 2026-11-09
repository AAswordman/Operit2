// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../model/ExtensionCatalogController.dart';
import 'ExtensionAvatar.dart';
import 'ExtensionNavigation.dart';
import 'StoreCards.dart';
import 'StoreVisuals.dart';

/// Shared official extension row for the market shelf and complete local list.
class StoreOfficialExtensionRow extends StatelessWidget {
  const StoreOfficialExtensionRow({
    super.key,
    required this.candidate,
    required this.controller,
  });

  final core_proxy.ExtensionCatalogCandidate candidate;
  final ExtensionCatalogController controller;

  Future<void> _install(BuildContext context) async {
    try {
      await controller.installCandidate(
        candidate,
        candidate.requiredScope ?? defaultExtensionInstallScope,
      );
    } catch (error, stackTrace) {
      debugPrint('Failed to install official extension: $error\n$stackTrace');
      if (!context.mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(content: Text('$error'), behavior: SnackBarBehavior.floating),
      );
    }
  }

  @override
  Widget build(BuildContext context) {
    final kind = StoreKind.of(candidate.kind);
    final installed =
        controller.findCandidate(candidate.kind, candidate.id) == null &&
        controller.findEntry(candidate.kind, candidate.id) != null;
    void open() => openExtensionDetail(kind: candidate.kind, id: candidate.id);
    return StoreAppRow(
      icon: ExtensionAvatar(
        seed: '${candidate.kind}:${candidate.id}',
        label: candidate.displayName,
        kind: kind,
        size: 44,
      ),
      title: candidate.displayName,
      subtitle: candidate.description.trim().isEmpty
          ? kind.label
          : '${kind.label} · ${candidate.description}',
      action: StoreGetButton(
        label: '获取',
        busy: controller.isBusy(
          ExtensionCatalogController.candidateKey(candidate),
        ),
        done: installed,
        onPressed: installed ? open : () => _install(context),
      ),
      onTap: open,
    );
  }
}
