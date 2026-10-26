// ignore_for_file: file_names
import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';
import 'package:file_selector_platform_interface/file_selector_platform_interface.dart';
import 'package:flutter/material.dart';
import '../../core/bridge/PlatformCoreProxy.dart';
import '../../core/host/FileSaveService.dart';

typedef DataRescueRequest =
    Future<Object?> Function(Map<String, Object?> request);

/// Matches explicit maintenance launch contracts before any business startup work.
bool isDataRescueLaunch({
  required List<String> arguments,
  required String route,
  required Uri uri,
  bool hostRequested = false,
}) {
  return hostRequested ||
      arguments.any((argument) => argument == '--data-rescue') ||
      route == '/data-rescue' ||
      uri.queryParameters['mode'] == 'data-rescue';
}

/// Runs a fixed, standalone Flutter interface with no application services or plugin UI.
class DataRescueApp extends StatelessWidget {
  /// Creates the standalone maintenance app with an injectable offline request boundary.
  const DataRescueApp({
    super.key,
    this.request,
    this.canStopNormalProcess = false,
  });
  final DataRescueRequest? request;
  final bool canStopNormalProcess;

  /// Builds only framework widgets and a fixed theme independent of user preferences.
  @override
  Widget build(BuildContext context) => MaterialApp(
    title: 'Operit2 数据救援 / Data Rescue',
    debugShowCheckedModeBanner: false,
    theme: ThemeData(
      colorScheme: ColorScheme.fromSeed(seedColor: const Color(0xff245f9e)),
      useMaterial3: true,
    ),
    home: _DataRescueScreen(
      request: request ?? platformCoreProxy.dataRescue,
      canStopNormalProcess: canStopNormalProcess,
    ),
  );
}

class _DataRescueScreen extends StatefulWidget {
  /// Receives the independent Host request function without acquiring any business dependency.
  const _DataRescueScreen({
    required this.request,
    required this.canStopNormalProcess,
  });
  final DataRescueRequest request;
  final bool canStopNormalProcess;

  /// Creates the maintenance state owned by this standalone launch.
  @override
  State<_DataRescueScreen> createState() => _DataRescueScreenState();
}

class _DataRescueScreenState extends State<_DataRescueScreen> {
  Map<String, Object?>? _startup;
  Map<String, Object?>? _manifest;
  String? _archiveId;
  String? _error;
  String _status = '正在打开独立存储维护会话…';
  bool _busy = true;
  bool _configured = false;
  bool _closed = false;
  double? _progress;
  String? _progressDetail;

  /// Acquires exclusive storage access without initializing Core or reading a database.
  @override
  void initState() {
    super.initState();
    unawaited(_open());
  }

  /// Releases the maintenance session when its owning screen leaves the widget tree.
  @override
  void dispose() {
    unawaited(_releaseMaintenance());
    super.dispose();
  }

  /// Closes Host access after accepted work finishes and reports any resource-release failure.
  Future<void> _releaseMaintenance() async {
    if (_closed) return;
    _closed = true;
    try {
      await widget.request({'operation': 'close'});
    } catch (error, stackTrace) {
      FlutterError.reportError(
        FlutterErrorDetails(
          exception: error,
          stack: stackTrace,
          library: 'data rescue',
          context: ErrorDescription('while closing the maintenance session'),
        ),
      );
    }
  }

  /// Validates a structured response at the independent maintenance boundary.
  Future<Map<String, Object?>> _map(Map<String, Object?> request) async =>
      Map<String, Object?>.from((await widget.request(request))! as Map);

  /// Displays measured stage counters without inventing an overall percentage.
  void _showProgress(Map<String, Object?> progress) {
    const phases = <String, String>{
      'scan': '正在统计数据',
      'export': '正在读取并打包数据',
      'finalizeExport': '正在完成备份归档',
      'readManifest': '正在读取备份文件清单',
      'verifyArchive': '正在校验备份完整性',
      'prepareRestore': '正在准备恢复位置',
      'restore': '正在解压并写入恢复数据',
      'verifyRestored': '正在复核恢复数据',
      'publish': '正在应用恢复数据',
      'retain': '正在保留原有数据',
      'save': '正在保存备份文件',
      'readSource': '正在读取所选备份',
      'decodeRecord': '正在恢复数据库二进制记录',
      'snapshotBytes': '正在读取浏览器数据',
      'snapshotRuntime': '正在整理运行数据',
      'snapshotDatabase': '正在读取数据库记录',
      'validateBrowser': '正在检查浏览器恢复数据',
      'publishDatabase': '正在恢复数据库记录',
      'publishSettings': '正在恢复应用设置',
    };
    final phase = progress['phase']! as String;
    final label = phases[phase];
    if (label == null) {
      throw StateError('Unknown rescue progress phase: $phase');
    }
    final completed = (progress['completed']! as num).toInt();
    final total = (progress['total'] as num?)?.toInt();
    if (completed < 0 || (total != null && (total < completed || total < 0))) {
      throw StateError('Invalid rescue progress counter');
    }
    final unit = switch (progress['unit']) {
      'bytes' => '字节',
      'files' => '个文件',
      'items' => '项',
      _ => throw StateError('Unknown rescue progress unit'),
    };
    if (!mounted) return;
    setState(() {
      _status = label;
      _progress = total == null
          ? null
          : total == 0
          ? 1
          : completed / total;
      _progressDetail = total == null
          ? '已处理 $completed $unit，总量尚未知…'
          : '$completed / $total $unit（${((_progress!) * 100).toStringAsFixed(1)}%）';
    });
  }

  /// Polls a Host-owned task while the platform executes its storage work independently.
  Future<Object?> _task(Map<String, Object?> request) async {
    final started = await _map({'operation': 'start', 'request': request});
    while (true) {
      final response = await _map({
        'operation': 'poll',
        'jobId': started['jobId'],
      });
      final progress = response['progress'];
      if (progress != null) {
        _showProgress(Map<String, Object?>.from(progress as Map));
      }
      if (response['done'] == true) return response['value'];
      await Future<void>.delayed(const Duration(milliseconds: 80));
    }
  }

  /// Requires an object result for operations that return an archive or storage description.
  Future<Map<String, Object?>> _taskMap(Map<String, Object?> request) async =>
      Map<String, Object?>.from((await _task(request))! as Map);

  /// Prepares Host-owned data locations without exposing storage paths to the user.
  Future<void> _open() async {
    await _run('正在打开数据救援，请先关闭其他 Operit 窗口和后台进程…', () async {
      final startup = await _map({'operation': 'open'});
      _startup = startup;
      final description = await _taskMap({'operation': 'configureAutomatic'});
      _startup = {...startup, ...description};
      _configured = true;
      _status = '已准备就绪，可以导出全部数据或选择备份恢复。';
    });
  }

  /// Requests the platform-owned normal process stop action before retrying the exclusive lease.
  Future<void> _stopNormalProcess() async {
    await _run('正在请求停止正常运行进程…', () async {
      await widget.request({'operation': 'stopNormalProcess'});
      await _open();
    });
  }

  /// Runs a user-requested operation and displays its concrete diagnostic on failure.
  Future<void> _run(String status, Future<void> Function() action) async {
    if (!mounted) return;
    setState(() {
      _busy = true;
      _error = null;
      _status = status;
      _progress = null;
      _progressDetail = null;
    });
    try {
      await action();
    } catch (error) {
      var diagnostic = '$error';
      if (_configured && !_closed) {
        try {
          final description = await _map({'operation': 'describe'});
          _startup = {
            ...?_startup,
            'commitPending': description['commitPending'] == true,
          };
        } catch (descriptionError) {
          diagnostic = '$diagnostic\n无法读取维护状态：$descriptionError';
        }
      }
      if (mounted) {
        setState(() {
          _status = '操作未完成，请查看错误信息。';
          _error = diagnostic;
        });
      }
    } finally {
      if (mounted) {
        setState(() {
          _busy = false;
          _progress = null;
          _progressDetail = null;
        });
      }
    }
  }

  /// Saves a streamed, portable archive using the registered system file picker.
  Future<void> _export() async {
    await _run('正在打包全部数据，请保持设备供电…', () async {
      Map<String, Object?>? archive;
      final name =
          'operit-data-rescue-${DateTime.now().toUtc().toIso8601String().replaceAll(':', '-')}.zip';
      try {
        final saved = await FileSaveService.saveGeneratedFileWithProgress(
          onChooseDestination: () {
            if (!mounted) return;
            setState(() {
              _status = '请选择备份的保存位置…';
              _progress = null;
              _progressDetail = null;
            });
          },
          generate: () async {
            archive = await _taskMap({'operation': 'export'});
            return XFile(
              archive!['fileReference']! as String,
              name: name,
              mimeType: 'application/zip',
              length: archive!['byteLength']! as int,
            );
          },
          write: (file, destination) async {
            await _task({
              'operation': 'saveArchive',
              'archiveId': archive!['archiveId'],
              'destination': destination,
            });
          },
          onProgress: (completed, total) => _showProgress({
            'phase': 'save',
            'completed': completed,
            'total': total,
            'unit': 'bytes',
          }),
          name: name,
          acceptedTypeGroups: const [
            XTypeGroup(
              label: 'Operit data rescue',
              extensions: ['zip'],
              mimeTypes: ['application/zip'],
            ),
          ],
        );
        _status = saved == null ? '已取消保存。' : '备份已保存：$saved\n请将备份保存到设备之外。';
      } finally {
        final id = archive?['archiveId'];
        if (id != null) {
          await widget.request({
            'operation': 'discardArchive',
            'archiveId': id,
          });
        }
      }
    });
  }

  /// Uploads bounded file chunks and verifies every payload before enabling import.
  Future<void> _inspect() async {
    await _run('请选择备份文件…', () async {
      final picker = FileSelectorPlatform.instance;
      final file = await picker.openFileWithProgress(
        onProgress: (completed, total) => _showProgress({
          'phase': 'readSource',
          'completed': completed,
          'total': total,
          'unit': 'bytes',
        }),
        acceptedTypeGroups: const [
          XTypeGroup(
            label: 'Operit data rescue',
            extensions: ['zip'],
            mimeTypes: ['application/zip'],
          ),
        ],
      );
      if (file == null) {
        _status = '已取消选择。';
        return;
      }
      try {
        await _discardArchive();
        final length = await file.length();
        final upload = await _map({
          'operation': 'beginUpload',
          'byteLength': length,
        });
        _archiveId = upload['archiveId']! as String;
        var sent = 0;
        for (var start = 0; start < length; start += 1024 * 1024) {
          final end = (start + 1024 * 1024).clamp(0, length);
          var received = 0;
          await for (final chunk in file.openRead(start, end)) {
            if (received + chunk.length > end - start) {
              throw StateError(
                'Selected backup exceeded its requested read range',
              );
            }
            for (var offset = 0; offset < chunk.length; offset += 65536) {
              final limit = (offset + 65536).clamp(0, chunk.length);
              final bytes = Uint8List.sublistView(chunk, offset, limit);
              await widget.request({
                'operation': 'uploadChunk',
                'archiveId': _archiveId,
                'bytes': base64Encode(bytes),
              });
              received += bytes.length;
              sent += bytes.length;
              if (mounted) {
                setState(() {
                  _status = '正在读取并暂存备份';
                  _progress = sent / length;
                  _progressDetail =
                      '$sent / $length 字节（${(_progress! * 100).toStringAsFixed(1)}%）';
                });
              }
            }
          }
          if (received != end - start) {
            throw StateError('Selected backup was truncated while reading');
          }
        }
        await widget.request({
          'operation': 'sealUpload',
          'archiveId': _archiveId,
        });
      } finally {
        await picker.releaseFile(file);
      }
      if (mounted) {
        setState(() {
          _status = '正在校验文件清单及全部文件完整性…';
          _progress = null;
        });
      }
      _manifest = await _taskMap({
        'operation': 'inspect',
        'archiveId': _archiveId,
      });
      _status = '完整性校验通过。确认导入前，当前数据不会被替换。';
    });
  }

  /// Requires an explicit destructive-action confirmation before publishing a verified archive.
  Future<void> _restore() async {
    final accepted = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text('确认导入全部数据？'),
        content: const Text(
          '将切换到备份中的全部身份、配置、聊天、附件和工作区。原有数据会保留在维护目录中。导入后请退出维护界面，再启动软件。',
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text('取消'),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text('确认导入'),
          ),
        ],
      ),
    );
    if (accepted != true) return;
    await _run('正在导入，请勿关闭程序或断电…', () async {
      final result = await _taskMap({
        'operation': 'restore',
        'archiveId': _archiveId,
      });
      await _discardArchive();
      _manifest = null;
      _status = '导入完成。请关闭维护会话后重新启动软件。\n${jsonEncode(result)}';
    });
  }

  /// Explicitly completes an interrupted Host publication while normal startup stays blocked.
  Future<void> _resume() async {
    await _run('正在完成中断的导入…', () async {
      final result = await _task({'operation': 'resumeRestore'});
      _startup = {...?_startup, 'commitPending': false};
      _status = '中断的导入已完成。请关闭维护会话后重新启动软件。\n${jsonEncode(result)}';
    });
  }

  /// Releases a staged archive owned by this interface before replacing its selection.
  Future<void> _discardArchive() async {
    final id = _archiveId;
    if (id == null) return;
    await widget.request({'operation': 'discardArchive', 'archiveId': id});
    _archiveId = null;
    _manifest = null;
  }

  /// Closes maintenance access without entering normal application initialization.
  Future<void> _close() async {
    await _run('正在关闭维护会话…', () async {
      await _discardArchive();
      await widget.request({'operation': 'close'});
      _closed = true;
      _status = '维护会话已关闭。现在可以关闭此窗口并正常启动软件。';
    });
  }

  /// Builds an accessible, scrollable maintenance screen independent of custom user themes.
  @override
  Widget build(BuildContext context) {
    final ready = _configured && !_busy && !_closed;
    final pending = _startup?['commitPending'] == true;
    final files = _manifest?['files'] as List<Object?>?;
    return PopScope(
      canPop: !_busy,
      child: Scaffold(
        appBar: AppBar(
          title: const Text('Operit2 数据救援 / Data Rescue'),
          automaticallyImplyLeading: false,
        ),
        body: Center(
          child: ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 800),
            child: ListView(
              padding: const EdgeInsets.all(24),
              children: [
                const Text(
                  '备份与恢复',
                  style: TextStyle(fontSize: 26, fontWeight: FontWeight.bold),
                ),
                const SizedBox(height: 12),
                const Text('用于在应用无法正常启动时导出或恢复数据。请先关闭其他 Operit 窗口和后台进程。'),
                const SizedBox(height: 16),
                const Text(
                  '备份包含全部身份的数据、工作区、启动配置及必要密钥，配置以明文导出。请妥善保管备份，并保存到其他设备。',
                ),
                const SizedBox(height: 20),
                Wrap(
                  spacing: 12,
                  runSpacing: 12,
                  children: [
                    FilledButton.icon(
                      onPressed: ready && !pending ? _export : null,
                      icon: const Icon(Icons.save_alt),
                      label: const Text('导出全部数据'),
                    ),
                    OutlinedButton.icon(
                      onPressed: ready && !pending ? _inspect : null,
                      icon: const Icon(Icons.fact_check_outlined),
                      label: const Text('从备份恢复'),
                    ),
                    if (files != null)
                      FilledButton.icon(
                        onPressed: ready ? _restore : null,
                        icon: const Icon(Icons.restore),
                        label: Text('导入已校验的 ${files.length} 个文件'),
                      ),
                    if (pending)
                      FilledButton(
                        onPressed: ready ? _resume : null,
                        child: const Text('完成中断的导入'),
                      ),
                  ],
                ),
                const SizedBox(height: 20),
                if (_busy) ...[
                  LinearProgressIndicator(
                    value: _progress,
                    semanticsLabel: '当前阶段进度',
                  ),
                  const SizedBox(height: 12),
                ],
                SelectableText(_status),
                if (_busy && _progressDetail != null) ...[
                  const SizedBox(height: 8),
                  Text(_progressDetail!, semanticsLabel: '当前阶段已完成量与总量'),
                ],
                if (_error != null) ...[
                  const SizedBox(height: 16),
                  SelectableText(
                    _error!,
                    style: TextStyle(
                      color: Theme.of(context).colorScheme.error,
                    ),
                  ),
                ],
                const SizedBox(height: 20),
                if (!_configured && !_closed && !_busy)
                  Wrap(
                    spacing: 12,
                    runSpacing: 12,
                    children: [
                      OutlinedButton(
                        onPressed: _open,
                        child: const Text('重新打开维护会话'),
                      ),
                      if (widget.canStopNormalProcess)
                        OutlinedButton(
                          onPressed: _stopNormalProcess,
                          child: const Text('停止正常运行进程后重试'),
                        ),
                    ],
                  ),
                if (!_closed && _startup != null)
                  TextButton(
                    onPressed: _busy ? null : _close,
                    child: const Text('关闭维护会话'),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
