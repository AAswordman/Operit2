// ignore_for_file: file_names
import 'dart:async';

import 'package:flutter/material.dart';
import 'package:url_launcher/url_launcher.dart';

import '../../../core/update/AppUpdateController.dart';
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
            title: '暂时无法完成更新',
            icon: const Icon(Icons.info_outline),
            maxWidth: 480,
            actions: [
              FilledButton(
                onPressed: () {
                  controller.acknowledgeUnavailable();
                  Navigator.of(context).pop();
                },
                child: const Text('知道了'),
              ),
            ],
            child: Text(
              '当前无法取得更新所需的安装包，且没有可用的完整包。请自行下载 ${release.version} 的安装包进行安装，或找到上一次安装（版本 ${controller.plan?.baseVersion ?? release.currentVersion}）的原始安装包后再试。',
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
        AppUpdatePhase.planning => '正在检查更新并自动选择下载源…',
        AppUpdatePhase.selectingSource => '正在自动选择下载源…',
        AppUpdatePhase.awaitingBase => '需要上一次安装的原始安装包',
        AppUpdatePhase.validatingBase => '正在校验所选安装包…',
        AppUpdatePhase.downloading =>
          controller.plan?.mode == AppUpdateMode.incremental
              ? '正在下载增量补丁…'
              : '正在下载完整安装包…',
        AppUpdatePhase.reconstructing => '正在合并并校验安装包…',
        AppUpdatePhase.ready => '更新已下载并校验完成，等待你确认安装',
        AppUpdatePhase.preparing => '正在准备安装，请稍候…',
        AppUpdatePhase.installing => '正在启动更新安装，应用即将重启…',
        AppUpdatePhase.awaitingInstall =>
          controller.restartRequired ? '下载完成，重启即可安装新版本' : '安装请求已提交，请完成系统确认',
        AppUpdatePhase.awaitingPermission => '需要允许安装应用',
        AppUpdatePhase.failed =>
          controller.packageReceipt == null ? '更新未完成' : '安装未完成',
        _ => '准备下载更新…',
      };
      return PopScope(
        canPop: !preparing && !installing && !validatingBase,
        child: OperitDialogScaffold(
          title: '更新 Operit',
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
                child: const Text('我没安装包了'),
              ),
              FilledButton(
                onPressed: () => unawaited(controller.selectBase()),
                child: const Text('选择'),
              ),
            ],
            if (!needsBase && !validatingBase && !preparing && !installing)
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(
                  selectingSource || downloading || reconstructing
                      ? '后台更新'
                      : '关闭',
                ),
              ),
            if (selectingSource || downloading || reconstructing)
              TextButton(
                onPressed: controller.cancelDownload,
                child: const Text('取消更新'),
              ),
            if (failed)
              TextButton(
                onPressed: () => unawaited(_openReleasePage()),
                child: const Text('前往发布页'),
              ),
            if (permission)
              FilledButton(
                onPressed: () => unawaited(controller.confirmInstall()),
                child: const Text('继续安装'),
              ),
            if (failed || phase == AppUpdatePhase.available)
              FilledButton(
                onPressed: () => unawaited(
                  controller.packageReceipt == null
                      ? controller.start()
                      : controller.confirmInstall(),
                ),
                child: Text(
                  controller.packageReceipt == null ? '重试更新' : '重新安装',
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
                label: Text(controller.restartRequired ? '重启并安装' : '重新打开安装器'),
              ),
            if (ready)
              FilledButton.icon(
                onPressed: () => unawaited(controller.confirmInstall()),
                icon: const Icon(Icons.system_update_alt),
                label: const Text('重启并更新'),
              ),
          ],
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                '发现新版本 ${release.version}',
                style: Theme.of(
                  context,
                ).textTheme.titleMedium?.copyWith(fontWeight: FontWeight.w700),
              ),
              const SizedBox(height: 6),
              Text(
                '当前版本 ${release.currentVersion}',
                style: TextStyle(color: colors.onSurfaceVariant),
              ),
              const SizedBox(height: 24),
              if (needsBase || validatingBase) ...[
                Text(
                  '你还有上一次安装（版本 ${controller.plan!.baseVersion}）的安装包吗？',
                  style: Theme.of(context).textTheme.titleMedium,
                ),
                const SizedBox(height: 10),
                Text(
                  '请选择当时安装此应用的原始安装包。校验通过后，将下载增量补丁并合并为新版本安装包。',
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
                if (controller.plan!.incrementalBytes != null) ...[
                  const SizedBox(height: 8),
                  Text(
                    '增量下载约 ${formatUpdateBytes(controller.plan!.incrementalBytes!)}',
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
                  value: downloading ? controller.progress : null,
                  minHeight: 6,
                  borderRadius: BorderRadius.circular(8),
                ),
              ],
              if (downloading) ...[
                const SizedBox(height: 10),
                Row(
                  children: [
                    Expanded(
                      child: Text(
                        controller.totalBytes == null
                            ? '${formatUpdateBytes(controller.receivedBytes)} · 正在获取文件大小'
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
                  '下载完成后会校验并保留安装包，等待你确认更新。你可以关闭此窗口，稍后点击 Operit 标志查看进度。',
                  style: Theme.of(context).textTheme.bodySmall?.copyWith(
                    color: colors.onSurfaceVariant,
                  ),
                ),
              ],
              if (permission) ...[
                const SizedBox(height: 10),
                Text(
                  controller.installMessage ?? '请完成系统授权，再点击继续安装。安装包已保留。',
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
              ],
              if (ready) ...[
                const SizedBox(height: 10),
                Text(
                  '点击“重启并更新”后才会开始安装。系统可能要求安装授权。',
                  style: TextStyle(color: colors.onSurfaceVariant),
                ),
              ],
              if (awaiting) ...[
                const SizedBox(height: 10),
                Text(
                  controller.restartRequired
                      ? '点击“重启并安装”后，应用会退出、替换为新版本并重新打开。'
                      : '请在系统安装窗口中完成确认。安装完成后重新打开 Operit；若已关闭安装窗口，可以再次打开。',
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
                    '安装包已保留，重试无需重新下载。',
                    style: TextStyle(color: colors.onSurfaceVariant),
                  ),
                ],
              ],
              if (release.releaseNotes.trim().isNotEmpty) ...[
                const SizedBox(height: 18),
                ExpansionTile(
                  tilePadding: EdgeInsets.zero,
                  title: const Text('本次更新内容'),
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
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final ready = widget.controller.phase == AppUpdatePhase.ready;
    final notes = release.releaseNotes.trim();
    return OperitDialogScaffold(
      title: '发现新版本',
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
          child: const Text('取消'),
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
          label: Text(ready ? '重启并更新' : '立即更新'),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Text(
            'Operit 有新版本可用了',
            style: theme.textTheme.titleMedium?.copyWith(
              fontWeight: FontWeight.w700,
            ),
          ),
          const SizedBox(height: 8),
          Text(
            ready ? '更新已准备就绪，确认后将重启应用并安装。' : '更新到最新版本，体验最新功能与改进。',
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
                _versionLabel(context, '当前版本', release.currentVersion),
                Icon(
                  Icons.arrow_forward_rounded,
                  color: colors.onSurfaceVariant,
                  size: 20,
                ),
                _versionLabel(context, '最新版本', release.version, latest: true),
              ],
            ),
          ),
          if (notes.isNotEmpty) ...[
            const SizedBox(height: 20),
            Text(
              '本次更新内容',
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
            '也可以稍后点击侧栏的 Operit 标志继续更新。',
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
    try {
      final opened = await launchUrl(
        widget.controller.release!.releasePageUrl,
        mode: LaunchMode.externalApplication,
      );
      if (!opened) throw StateError('无法打开发布页');
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
