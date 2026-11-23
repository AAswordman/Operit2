// ignore_for_file: file_names
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../core/update/AppUpdateController.dart';
import '../../../l10n/generated/app_localizations.dart';
import '../../../core/update/AppUpdateModels.dart';
import '../../common/components/OperitDialog.dart';

bool _dialogOpen = false;

/// One dialog per application, even if the sidebar switches layout mid-download.
Future<void> showAppUpdateDialog(
  BuildContext context, {
  AppUpdateController? controller,
  bool automatic = false,
}) async {
  final selectedController = controller ?? AppUpdateController.instance;
  if (_dialogOpen || (automatic && !selectedController.hasUnpromptedUpdate)) {
    return;
  }
  selectedController.markUpdatePrompted();
  _dialogOpen = true;
  try {
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => AppUpdateDialog(
        controller: selectedController,
        requireConfirmation: automatic,
      ),
    );
  } finally {
    _dialogOpen = false;
  }
}

class AppUpdateDialog extends StatefulWidget {
  const AppUpdateDialog({
    super.key,
    required this.controller,
    this.requireConfirmation = false,
  });
  final AppUpdateController controller;
  final bool requireConfirmation;

  @override
  State<AppUpdateDialog> createState() => _AppUpdateDialogState();
}

class _AppUpdateDialogState extends State<AppUpdateDialog> {
  late bool _awaitingConfirmation;

  /// Start after a visible dialog frame, so a tap always gets immediate feedback.
  @override
  void initState() {
    super.initState();
    _awaitingConfirmation = widget.requireConfirmation;
    if (_awaitingConfirmation) return;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && widget.controller.phase == AppUpdatePhase.available) {
        unawaited(widget.controller.start());
      }
    });
  }

  @override
  Widget build(BuildContext context) => ListenableBuilder(
    listenable: widget.controller,
    builder: (context, _) {
      final l10n = AppLocalizations.of(context)!;
      final controller = widget.controller;
      final release = controller.release;
      if (release == null) return const SizedBox.shrink();
      if (_awaitingConfirmation) return _buildUpdatePrompt(context, release);
      final colors = Theme.of(context).colorScheme;
      final phase = controller.phase;
      if (phase == AppUpdatePhase.unavailable) {
        return PopScope(
          canPop: false,
          child: OperitDialogScaffold(
            title: l10n.appUpdateUnavailableTitle,
            icon: const Icon(Icons.info_outline),
            maxWidth: 480,
            actions: [
              FilledButton(
                onPressed: () {
                  controller.acknowledgeUnavailable();
                  Navigator.of(context).pop();
                },
                child: Text(l10n.ok),
              ),
            ],
            child: Text(
              l10n.appUpdateUnavailableHelp(release.version, controller.plan?.baseVersion ?? release.currentVersion),
            ),
          ),
        );
      }
      final needsBase = phase == AppUpdatePhase.awaitingBase;
      final validatingBase = phase == AppUpdatePhase.validatingBase;
      final planning = phase == AppUpdatePhase.planning;
      final selectingSource = phase == AppUpdatePhase.selectingSource;
      final downloading = phase == AppUpdatePhase.downloading;
      final reconstructing = phase == AppUpdatePhase.reconstructing;
      final ready = phase == AppUpdatePhase.ready;
      final preparing = phase == AppUpdatePhase.preparing;
      final installing = phase == AppUpdatePhase.installing;
      final failed = phase == AppUpdatePhase.failed;
      final awaiting = phase == AppUpdatePhase.awaitingInstall;
      final permission = phase == AppUpdatePhase.awaitingPermission;
      final status = switch (phase) {
        AppUpdatePhase.planning => l10n.appUpdatePlanning,
        AppUpdatePhase.selectingSource => l10n.appUpdateSelectingSource,
        AppUpdatePhase.awaitingBase => l10n.appUpdateAwaitingBase,
        AppUpdatePhase.validatingBase => l10n.appUpdateValidatingBase,
        AppUpdatePhase.downloading =>
          controller.plan?.mode == AppUpdateMode.incremental
              ? l10n.appUpdateDownloadingPatch
              : l10n.appUpdateDownloadingFull,
        AppUpdatePhase.reconstructing =>
          switch (controller.preparation?.phase) {
            AppUpdatePreparationPhase.verifyingBase => l10n.appUpdateVerifyingBase,
            AppUpdatePreparationPhase.verifyingPatch => l10n.appUpdateVerifyingPatch,
            AppUpdatePreparationPhase.expandingBase => l10n.appUpdateExpandingBase,
            AppUpdatePreparationPhase.verifyingExpandedBase => l10n.appUpdateVerifyingExpandedBase,
            AppUpdatePreparationPhase.applyingPatch => l10n.appUpdateApplyingPatch,
            AppUpdatePreparationPhase.restoringPackage => l10n.appUpdateRestoringPackage,
            AppUpdatePreparationPhase.verifyingPackage => l10n.appUpdateVerifyingPackage,
            AppUpdatePreparationPhase.savingPackage => l10n.appUpdateSavingPackage,
            null => l10n.appUpdatePreparingFiles,
          },
        AppUpdatePhase.ready => l10n.appUpdateReady,
        AppUpdatePhase.preparing => l10n.appUpdatePreparingInstall,
        AppUpdatePhase.installing => l10n.appUpdateInstalling,
        AppUpdatePhase.awaitingInstall =>
          controller.restartRequired ? l10n.appUpdateRestartRequired : l10n.appUpdateInstallSubmitted,
        AppUpdatePhase.awaitingPermission => l10n.appUpdatePermissionRequired,
        AppUpdatePhase.failed =>
          controller.packageReceipt == null ? l10n.appUpdateUpdateIncomplete : l10n.appUpdateInstallIncomplete,
        _ => l10n.appUpdatePreparingDownload,
      };
      return PopScope(
        canPop: !preparing && !installing && !validatingBase,
        child: OperitDialogScaffold(
          title: l10n.appUpdateTitle,
          icon: Icon(
            failed
                ? Icons.error_outline_rounded
                : Icons.system_update_alt_rounded,
          ),
          maxWidth: 480,
          actions: [
            if (needsBase) ...[
              TextButton(
                onPressed: () => unawaited(controller.withoutBase()),
                child: Text(l10n.appUpdateNoOriginalInstaller),
              ),
              FilledButton(
                onPressed: () => unawaited(controller.selectBase()),
                child: Text(l10n.appUpdateChooseInstaller),
              ),
            ],
            if (!needsBase && !validatingBase && !preparing && !installing)
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(
                  selectingSource || downloading || reconstructing
                      ? l10n.appUpdateBackground
                      : l10n.close,
                ),
              ),
            if (selectingSource || downloading || reconstructing)
              TextButton(
                onPressed: controller.cancelDownload,
                child: Text(l10n.appUpdateCancel),
              ),
            if (failed)
              TextButton(
                onPressed: () => unawaited(_openReleasePage()),
                child: Text(l10n.appUpdateOpenRelease),
              ),
            if (permission)
              FilledButton(
                onPressed: () => unawaited(controller.confirmInstall()),
                child: Text(l10n.appUpdateContinueInstall),
              ),
            if (failed || phase == AppUpdatePhase.available)
              FilledButton(
                onPressed: () => unawaited(
                  controller.packageReceipt == null
                      ? controller.start()
                      : controller.confirmInstall(),
                ),
                child: Text(
                  controller.packageReceipt == null ? l10n.appUpdateRetry : l10n.appUpdateReinstall,
                ),
              ),
            if (awaiting)
              FilledButton.icon(
                onPressed: () => unawaited(
                  controller.restartRequired
                      ? controller.restartToInstall()
                      : controller.confirmInstall(),
                ),
                icon: Icon(
                  controller.restartRequired
                      ? Icons.restart_alt
                      : Icons.open_in_new,
                ),
                label: Text(controller.restartRequired ? l10n.appUpdateRestartInstall : l10n.appUpdateReopenInstaller),
              ),
            if (ready)
              FilledButton.icon(
                onPressed: () => unawaited(controller.confirmInstall()),
                icon: const Icon(Icons.system_update_alt),
                label: Text(l10n.appUpdateRestartUpdate),
              ),
          ],
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                l10n.appUpdateAvailableVersion(release.version),
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 6),
              Text(
                l10n.appUpdateInstalledVersion(release.currentVersion),
                style: TextStyle(color: colors.onSurfaceVariant),
              ),
              const SizedBox(height: 24),
              if (needsBase || validatingBase) ...[
                Text(
                  l10n.appUpdateOriginalVersionPrompt(controller.plan!.baseVersion.toString()),
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 10),
                Text(
                  l10n.appUpdateSelectOriginalInstaller,
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
                if (controller.plan!.incrementalBytes != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    l10n.appUpdatePatchSize(formatUpdateBytes(controller.plan!.incrementalBytes!)),
                  ),
                ],
                const SizedBox(height: 18),
              ],
              if (!needsBase)
                Row(
                  children: [
                    Icon(
                      failed
                          ? Icons.error_outline
                          : awaiting || ready
                          ? Icons.check_circle_outline
                          : Icons.downloading_rounded,
                      size: 20,
                      color: failed ? colors.error : colors.primary,
                    ),
                    const SizedBox(width: 8),
                    Expanded(
                      child: Text(
                        status,
                        style: const TextStyle(fontWeight: FontWeight.w600),
                      ),
                    ),
                  ],
                ),
              if (planning ||
                  selectingSource ||
                  validatingBase ||
                  downloading ||
                  reconstructing ||
                  preparing ||
                  installing) ...[
                const SizedBox(height: 14),
                LinearProgressIndicator(
                  value: downloading
                      ? controller.progress
                      : reconstructing
                      ? controller.preparation?.fraction
                      : null,
                  minHeight: 6,
                  borderRadius: BorderRadius.circular(8),
                ),
              ],
              if (reconstructing) ...[
                const SizedBox(height: 10),
                Wrap(
                  spacing: 16,
                  runSpacing: 6,
                  children: [
                    if (controller.preparation?.fraction
                        case final double fraction)
                      Text(
                        l10n.appUpdateStepProgress((fraction * 100).floor()),
                        style: const TextStyle(
                          fontFeatures: [FontFeature.tabularFigures()],
                        ),
                      ),
                    Text(
                      l10n.appUpdateElapsed(formatUpdateDuration(controller.preparationElapsed)),
                      style: TextStyle(
                        color: colors.onSurfaceVariant,
                        fontSize: 12,
                      ),
                    ),
                  ],
                ),
                const SizedBox(height: 10),
                Text(
                  l10n.appUpdateProcessingHelp,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ],
              if (downloading) ...[
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        controller.totalBytes == null
                            ? l10n.appUpdateDownloadSizePending(formatUpdateBytes(controller.receivedBytes))
                            : '${formatUpdateBytes(controller.receivedBytes)} / ${formatUpdateBytes(controller.totalBytes!)}',
                        style: TextStyle(
                          color: colors.onSurfaceVariant,
                          fontSize: 12,
                        ),
                      ),
                    ),
                    if (controller.progress != null)
                      Text(
                        '${(controller.progress! * 100).floor()}%',
                        style: const TextStyle(
                          fontFeatures: [FontFeature.tabularFigures()],
                        ),
                      ),
                  ],
                ),
                const SizedBox(height: 12),
                Text(
                  l10n.appUpdateDownloadHelp,
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ],
              if (permission) ...[
                const SizedBox(height: 10),
                Text(
                  controller.installMessage ?? l10n.appUpdatePermissionHelp,
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
              ],
              if (ready) ...[
                const SizedBox(height: 10),
                Text(
                  l10n.appUpdateReadyHelp,
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
              ],
              if (awaiting) ...[
                const SizedBox(height: 10),
                Text(
                  controller.restartRequired
                      ? l10n.appUpdateRestartHelp
                      : l10n.appUpdateSystemInstallHelp,
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
              ],
              if (controller.error != null) ...[
                const SizedBox(height: 10),
                SelectableText(
                  controller.error!,
                  style: TextStyle(color: colors.error),
                ),
                if (controller.packageReceipt != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    l10n.appUpdateInstallerKept,
                    style: TextStyle(color: colors.onSurfaceVariant),
                  ),
                ],
              ],
              if (release.releaseNotes.trim().isNotEmpty) ...[
                const SizedBox(height: 18),
                ExpansionTile(
                  tilePadding: EdgeInsets.zero,
                  title: Text(l10n.appUpdateReleaseNotes),
                  children: [
                    Align(
                      alignment: Alignment.centerLeft,
                      child: SelectableText(release.releaseNotes),
                    ),
                  ],
                ),
              ],
            ],
          ),
        ),
      );
    },
  );

  Widget _buildUpdatePrompt(BuildContext context, AppUpdateRelease release) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final ready = widget.controller.phase == AppUpdatePhase.ready;
    final notes = release.releaseNotes.trim();
    return OperitDialogScaffold(
      title: l10n.appUpdateAvailableTitle,
      icon: Container(
        padding: const EdgeInsets.all(10),
        decoration: BoxDecoration(
          color: colors.primaryContainer,
          borderRadius: BorderRadius.circular(14),
        ),
        child: Icon(
          Icons.system_update_alt_rounded,
          color: colors.onPrimaryContainer,
        ),
      ),
      maxWidth: 480,
      actions: [
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton.icon(
          onPressed: () {
            setState(() => _awaitingConfirmation = false);
            final controller = widget.controller;
            if (controller.phase == AppUpdatePhase.ready) {
              unawaited(controller.confirmInstall());
            } else if (controller.phase == AppUpdatePhase.available ||
                controller.phase == AppUpdatePhase.failed) {
              unawaited(controller.start());
            }
          },
          icon: Icon(
            ready ? Icons.restart_alt : Icons.system_update_alt_rounded,
          ),
          label: Text(ready ? l10n.appUpdateRestartUpdate : l10n.appUpdateNow),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            l10n.appUpdateAvailable,
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            ready ? l10n.appUpdatePromptReady : l10n.appUpdatePromptAvailable,
            style: theme.textTheme.bodyMedium?.copyWith(
              color: colors.onSurfaceVariant,
            ),
          ),
          const SizedBox(height: 20),
          Container(
            width: double.infinity,
            padding: const EdgeInsets.all(16),
            decoration: BoxDecoration(
              color: colors.surfaceContainerHighest,
              borderRadius: BorderRadius.circular(14),
            ),
            child: Wrap(
              spacing: 16,
              runSpacing: 12,
              crossAxisAlignment: WrapCrossAlignment.center,
              children: [
                _versionLabel(context, l10n.appUpdateCurrentVersion, release.currentVersion),
                Icon(
                  Icons.arrow_forward_rounded,
                  color: colors.onSurfaceVariant,
                  size: 20,
                ),
                _versionLabel(context, l10n.appUpdateLatestVersion, release.version, latest: true),
              ],
            ),
          ),
          if (notes.isNotEmpty) ...[
            const SizedBox(height: 20),
            Text(
              l10n.appUpdateReleaseNotes,
              style: theme.textTheme.labelLarge?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 10),
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 180),
              child: SingleChildScrollView(
                primary: false,
                child: SelectableText(notes, style: theme.textTheme.bodyMedium),
              ),
            ),
          ],
          const SizedBox(height: 20),
          Text(
            l10n.appUpdatePromptLater,
            style: theme.textTheme.bodySmall?.copyWith(
              color: colors.onSurfaceVariant,
            ),
          ),
        ],
      ),
    );
  }

  Widget _versionLabel(
    BuildContext context,
    String label,
    String version, {
    bool latest = false,
  }) {
    final theme = Theme.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          label,
          style: theme.textTheme.bodySmall?.copyWith(
            color: theme.colorScheme.onSurfaceVariant,
          ),
        ),
        const SizedBox(height: 4),
        Text(
          version,
          style: theme.textTheme.titleMedium?.copyWith(
            fontWeight: FontWeight.w700,
            color: latest ? theme.colorScheme.primary : null,
          ),
        ),
      ],
    );
  }

  Future<void> _openReleasePage() async {
    final l10n = AppLocalizations.of(context)!;
    try {
      final opened = await launchUrl(
        widget.controller.release!.releasePageUrl,
        mode: LaunchMode.externalApplication,
      );
      if (!opened) throw StateError(l10n.appUpdateOpenReleaseFailed);
    } catch (error) {
      if (mounted) {
        ScaffoldMessenger.of(
          context,
        ).showSnackBar(SnackBar(content: Text('$error')));
      }
    }
  }
}

String formatUpdateBytes(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  if (bytes < 1024 * 1024 * 1024) {
    return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
  }
  return '${(bytes / (1024 * 1024 * 1024)).toStringAsFixed(1)} GB';
}

/// Formats measured local processing time without predicting remaining work.
String formatUpdateDuration(Duration duration) {
  final minutes = duration.inMinutes.toString().padLeft(2, '0');
  final seconds = (duration.inSeconds % 60).toString().padLeft(2, '0');
  return '$minutes:$seconds';
}
