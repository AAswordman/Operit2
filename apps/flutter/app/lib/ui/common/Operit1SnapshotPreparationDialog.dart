// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/material.dart';

import '../../core/logging/ClientLogger.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart';
import '../../core/snapshot/SnapshotImportUploader.dart';

class PreparedOperit1Snapshot {
  const PreparedOperit1Snapshot({
    required this.session,
    required this.preview,
    required this.fileName,
  });

  final SnapshotImportSession session;
  final Operit1SnapshotPreview preview;
  final String fileName;
}

/// Both import entry points prepare a snapshot visibly before asking to import.
Future<PreparedOperit1Snapshot?> prepareOperit1Snapshot(
  BuildContext context,
  GeneratedCoreProxyClients clients,
) {
  return showDialog<PreparedOperit1Snapshot>(
    context: context,
    barrierDismissible: false,
    builder: (_) => _SnapshotPreparationDialog(clients: clients),
  );
}

enum _PreparationPhase { opening, reading, inspecting }

class _SnapshotPreparationDialog extends StatefulWidget {
  const _SnapshotPreparationDialog({required this.clients});
  final GeneratedCoreProxyClients clients;

  @override
  State<_SnapshotPreparationDialog> createState() =>
      _SnapshotPreparationDialogState();
}

class _SnapshotPreparationDialogState extends State<_SnapshotPreparationDialog> {
  final Stopwatch _elapsed = Stopwatch()..start();
  Timer? _timer;
  _PreparationPhase _phase = _PreparationPhase.opening;
  String? _fileName;
  String? _error;
  int _bytesRead = 0;
  int _totalBytes = 0;

  @override
  void initState() {
    super.initState();
    _timer = Timer.periodic(const Duration(seconds: 1), (_) {
      if (mounted) setState(() {});
    });
    // Paint the waiting dialog before handing control to the file provider.
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) unawaited(_prepare());
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    _elapsed.stop();
    super.dispose();
  }

  Future<void> _prepare() async {
    SnapshotImportSession? session;
    var handedOff = false;
    try {
      final file = await SnapshotImportFile.pick();
      if (file == null) {
        if (mounted) Navigator.of(context).pop();
        return;
      }
      if (!mounted) {
        await file.close();
        return;
      }
      setState(() {
        _phase = _PreparationPhase.reading;
        _fileName = file.name;
        _totalBytes = file.byteLength;
        _elapsed.reset();
      });
      ClientLogger.i(
        'snapshot selected name=${file.name} bytes=${file.byteLength}',
        tag: 'Operit1SnapshotImport',
      );
      session = await SnapshotImportUploader(widget.clients).stage(
        file,
        onBytesRead: (bytes) {
          if (mounted) setState(() => _bytesRead = bytes);
        },
      );
      if (!mounted) return;
      setState(() {
        _phase = _PreparationPhase.inspecting;
        _elapsed.reset();
      });
      ClientLogger.i(
        'snapshot upload completed bytes=${session.byteLength}; inspection started',
        tag: 'Operit1SnapshotImport',
      );
      final preview = await session.completeOperit1();
      ClientLogger.i(
        'snapshot inspection completed format=${preview.formatVersion} '
        'chats=${preview.chatCount} messages=${preview.messageCount}',
        tag: 'Operit1SnapshotImport',
      );
      if (!mounted) return;
      Navigator.of(context).pop(PreparedOperit1Snapshot(
        session: session,
        preview: preview,
        fileName: file.name,
      ));
      handedOff = true;
    } catch (error, stackTrace) {
      ClientLogger.e(
        'snapshot selection, upload, or inspection failed',
        tag: 'Operit1SnapshotImport',
        error: error,
        stackTrace: stackTrace,
      );
      _timer?.cancel();
      _elapsed.stop();
      if (mounted) setState(() => _error = '$error');
    } finally {
      if (!handedOff) await _discardPreparedSnapshot(session);
    }
  }

  String _duration(int seconds) => seconds < 60
      ? '$seconds 秒'
      : '${seconds ~/ 60} 分 ${seconds % 60} 秒';

  String _size(int bytes) => bytes < 1024 * 1024
      ? '${(bytes / 1024).toStringAsFixed(1)} KB'
      : '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';

  @override
  Widget build(BuildContext context) {
    final seconds = _elapsed.elapsed.inSeconds;
    final reading = _phase == _PreparationPhase.reading;
    final progress = reading && _totalBytes > 0
        ? (_bytesRead / _totalBytes).clamp(0.0, 1.0).toDouble()
        : null;
    final canEstimate = reading && seconds >= 2 &&
        _bytesRead > 0 && _bytesRead < _totalBytes;
    final remaining = canEstimate
        ? ((_totalBytes - _bytesRead) *
            _elapsed.elapsedMilliseconds / _bytesRead / 1000).ceil()
        : null;
    final label = switch (_phase) {
      _PreparationPhase.opening => '正在选择或打开快照文件',
      _PreparationPhase.reading => progress == 1.0
          ? '文件读取完成，正在确认'
          : '正在准备快照文件',
      _PreparationPhase.inspecting => '正在检查备份内容',
    };
    return PopScope(
      canPop: _error != null,
      child: AlertDialog(
        title: Text(_error == null ? '准备导入' : '快照准备失败'),
        content: SingleChildScrollView(
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              if (_fileName != null) ...<Widget>[
                Text(_fileName!, maxLines: 2, overflow: TextOverflow.ellipsis),
                const SizedBox(height: 12),
              ],
              if (_error != null)
                SelectableText(_error!)
              else ...<Widget>[
                Text(label),
                const SizedBox(height: 12),
                LinearProgressIndicator(value: progress),
                const SizedBox(height: 12),
                if (reading)
                  Text('已读取 ${_size(_bytesRead)} / ${_size(_totalBytes)}'
                      '${progress == null ? '' : '（${(progress * 100).floor()}%）'}'),
                Text('此阶段已等待 ${_duration(seconds)}'),
                if (remaining != null)
                  Text('按当前速度，读取预计还需 ${_duration(remaining)}'),
                const SizedBox(height: 8),
                Text(_phase == _PreparationPhase.inspecting
                    ? '文件读取完成。正在检查聊天和资源，完成后显示预览；检查时间取决于备份内容。'
                    : '选中文件后会自动读取，随后检查备份并显示预览。此时尚未导入数据。'),
              ],
            ],
          ),
        ),
        actions: _error == null ? null : <Widget>[
          TextButton(
            onPressed: () => Navigator.of(context).pop(),
            child: const Text('关闭'),
          ),
        ],
      ),
    );
  }
}

Future<void> _discardPreparedSnapshot(SnapshotImportSession? session) async {
  if (session == null) return;
  try {
    await session.discard();
  } catch (error, stackTrace) {
    ClientLogger.e(
      'snapshot preparation cleanup failed',
      tag: 'Operit1SnapshotImport',
      error: error,
      stackTrace: stackTrace,
    );
  }
}
