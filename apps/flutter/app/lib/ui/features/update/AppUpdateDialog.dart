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
}) async {
  if (_dialogOpen) return;
  _dialogOpen = true;
  try {
    await showDialog<void>(
      context: context,
      barrierDismissible: false,
      builder: (_) => AppUpdateDialog(
        controller: controller ?? AppUpdateController.instance,
      ),
    );
  } finally {
    _dialogOpen = false;
  }
}

class AppUpdateDialog extends StatefulWidget {
  const AppUpdateDialog({super.key, required this.controller});
  final AppUpdateController controller;

  @override
  State<AppUpdateDialog> createState() => _AppUpdateDialogState();
}

class _AppUpdateDialogState extends State<AppUpdateDialog> {
  /// Start after a visible dialog frame, so a tap always gets immediate feedback.
  @override
  void initState() {
    super.initState();
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
      final colors = Theme.of(context).colorScheme;
      final phase = controller.phase;
      final downloading = phase == AppUpdatePhase.downloading;
      final preparing = phase == AppUpdatePhase.preparing;
      final installing = phase == AppUpdatePhase.installing;
      final failed = phase == AppUpdatePhase.failed;
      final awaiting = phase == AppUpdatePhase.awaitingInstall;
      final permission = phase == AppUpdatePhase.awaitingPermission;
      final status = switch (phase) {
        AppUpdatePhase.downloading => '正在下载安装包…',
        AppUpdatePhase.preparing => '正在准备安装，请稍候…',
        AppUpdatePhase.installing => '正在启动更新安装，应用即将重启…',
        AppUpdatePhase.awaitingInstall =>
          controller.restartRequired ? '下载完成，重启即可安装新版本' : '安装请求已提交，请完成系统确认',
        AppUpdatePhase.awaitingPermission => '需要允许安装应用',
        AppUpdatePhase.failed =>
          controller.packageReceipt == null ? '下载失败' : '安装未完成',
        _ => '准备下载更新…',
      };
      return PopScope(
        canPop: !preparing && !installing,
        child: OperitDialogScaffold(
          title: '更新 Operit',
          icon: Icon(
            failed
                ? Icons.error_outline_rounded
                : Icons.system_update_alt_rounded,
          ),
          maxWidth: 480,
          actions: [
            if (!preparing && !installing)
              TextButton(
                onPressed: () => Navigator.of(context).pop(),
                child: Text(downloading ? '后台下载' : '关闭'),
              ),
            if (downloading)
              TextButton(
                onPressed: controller.cancelDownload,
                child: const Text('取消下载'),
              ),
            if (failed)
              TextButton(
                onPressed: () => unawaited(_openReleasePage()),
                child: const Text('前往发布页'),
              ),
            if (permission)
              FilledButton(
                onPressed: () => unawaited(controller.start()),
                child: const Text('继续安装'),
              ),
            if (failed || phase == AppUpdatePhase.available)
              FilledButton(
                onPressed: () => unawaited(controller.start()),
                child: Text(
                  controller.packageReceipt == null ? '重新下载' : '重新安装',
                ),
              ),
            if (awaiting)
              FilledButton.icon(
                onPressed: () => unawaited(
                  controller.restartRequired
                      ? controller.restartToInstall()
                      : controller.start(),
                ),
                icon: Icon(
                  controller.restartRequired
                      ? Icons.restart_alt
                      : Icons.open_in_new,
                ),
                label: Text(controller.restartRequired ? '重启并安装' : '重新打开安装器'),
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
              Row(
                children: [
                  Icon(
                    failed
                        ? Icons.error_outline
                        : awaiting
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
              if (downloading || preparing || installing) ...[
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
                  '下载完成后会自动准备安装。你可以关闭此窗口，稍后点击更新图标查看进度。',
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
