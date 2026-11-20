// ignore_for_file: file_names

part of 'LocalModelSettingsPanel.dart';

/// Presents the two device import formats supported by the owning runtime.
class _LocalModelFileImportDialog extends StatefulWidget {
  /// Opens a device file import session with the available embedded engines.
  const _LocalModelFileImportDialog({
    required this.clients,
    required this.engines,
  });

  final GeneratedCoreProxyClients clients;
  final List<core_proxy.LocalEngineKind> engines;

  /// Owns the selected inputs until the dialog closes.
  @override
  State<_LocalModelFileImportDialog> createState() =>
      _LocalModelFileImportDialogState();
}

class _LocalModelFileImportDialogState
    extends State<_LocalModelFileImportDialog> {
  late core_proxy.LocalEngineKind _engine = widget.engines.first;
  final _name = TextEditingController();
  List<SelectedFileInput> _files = const [];
  bool _picking = false;
  bool _importing = false;
  int _copiedBytes = 0;
  String? _error;

  /// Returns the required storage for the current selection.
  int get _totalBytes =>
      _files.fold(0, (size, file) => size + file.byteLength!);

  /// Returns the picker instructions for the current engine.
  String _formatHint(AppLocalizations l10n) =>
      _engine == core_proxy.LocalEngineKind.llamaCpp
      ? l10n.localModelsImportGgufHint
      : l10n.localModelsImportMnnHint;

  /// Clears files when the selected format changes.
  void _selectEngine(core_proxy.LocalEngineKind engine) {
    if (engine == _engine) return;
    final previous = _files;
    _name.clear();
    setState(() {
      _engine = engine;
      _files = const [];
      _error = null;
    });
    unawaited(_closeFiles(previous));
  }

  /// Opens the platform's bounded input picker and retains a valid selection.
  Future<void> _pickFiles() async {
    final l10n = AppLocalizations.of(context)!;
    var selected = <SelectedFileInput>[];
    setState(() {
      _picking = true;
      _error = null;
    });
    try {
      selected = await SelectedFileInput.pick();
      if (!mounted || selected.isEmpty) return;
      final gguf = _engine == core_proxy.LocalEngineKind.llamaCpp;
      final matchesFormat = gguf
          ? selected.length == 1 &&
                selected.single.name.toLowerCase().endsWith('.gguf')
          : selected.any((file) => file.name == 'config.json');
      if (!matchesFormat) {
        setState(
          () => _error = gguf
              ? l10n.localModelsImportInvalidGguf
              : l10n.localModelsImportMissingConfig,
        );
        return;
      }
      if (selected.any(
        (file) => file.byteLength == null || file.byteLength! <= 0,
      )) {
        setState(() => _error = l10n.localModelsImportFileSizeRequired);
        return;
      }
      if (gguf) {
        final filename = selected.single.name;
        _name.text = filename.substring(0, filename.length - '.gguf'.length);
      }
      final previous = _files;
      setState(() => _files = selected);
      selected = previous;
    } catch (error) {
      if (mounted) {
        setState(() => _error = l10n.localModelsOperationFailed('$error'));
      }
    } finally {
      if (mounted) setState(() => _picking = false);
      await _closeFiles(selected);
    }
  }

  /// Streams the selected files in declared order and reports transfer progress.
  Stream<Uint8List> _readFiles(
    List<SelectedFileInput> files,
    int totalBytes,
  ) async* {
    var copiedBytes = 0;
    for (final file in files) {
      await for (final chunk in file.chunks()) {
        yield chunk;
        copiedBytes += chunk.length;
        if (mounted &&
            copiedBytes * 100 ~/ totalBytes !=
                _copiedBytes * 100 ~/ totalBytes) {
          setState(() => _copiedBytes = copiedBytes);
        }
      }
    }
    yield Uint8List(0);
  }

  /// Publishes the selection as an installed model through the Core import service.
  Future<void> _importFiles() async {
    final l10n = AppLocalizations.of(context)!;
    final files = _files;
    final totalBytes = _totalBytes;
    FocusScope.of(context).unfocus();
    setState(() {
      _importing = true;
      _copiedBytes = 0;
      _error = null;
    });
    try {
      await widget.clients.servicesLocalModelService.importModelFromFiles(
        engine: _engine,
        displayName: _name.text.trim(),
        files: [
          for (final file in files)
            core_proxy.LocalModelImportFile(
              name: file.name,
              byteLength: file.byteLength!,
            ),
        ],
        bytes: _readFiles(files, totalBytes),
      );
      if (mounted) Navigator.of(context).pop(true);
    } catch (error) {
      if (mounted) {
        setState(() {
          _files = const [];
          _importing = false;
          _error = l10n.localModelsOperationFailed('$error');
        });
      }
      await _closeFiles(files);
    }
  }

  /// Releases every picker-owned stream or document handle.
  Future<void> _closeFiles(List<SelectedFileInput> files) async {
    for (final file in files) {
      try {
        await file.close();
      } catch (error) {
        debugPrint('Could not release model input: $error');
      }
    }
  }

  /// Releases selected files when the dialog leaves the widget tree.
  @override
  void dispose() {
    unawaited(_closeFiles(_files));
    _name.dispose();
    super.dispose();
  }

  /// Lays out format, file selection, and transfer status in a scrollable dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final locked = _picking || _importing;
    final hasFiles = _files.isNotEmpty;
    return PopScope(
      canPop: !locked,
      child: OperitDialogScaffold(
        title: l10n.localModelsImportFromDevice,
        icon: const Icon(Icons.file_upload_outlined),
        expandContent: false,
        maxWidth: 460,
        actions: [
          TextButton(
            onPressed: locked ? null : () => Navigator.of(context).pop(),
            child: Text(l10n.cancel),
          ),
          FilledButton(
            onPressed: !locked && hasFiles && _name.text.trim().isNotEmpty
                ? _importFiles
                : null,
            child: Text(l10n.localModelsHubImport),
          ),
        ],
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          mainAxisSize: MainAxisSize.min,
          children: [
            Wrap(
              spacing: 8,
              runSpacing: 6,
              children: [
                for (final engine in widget.engines)
                  ChoiceChip(
                    label: Text(
                      engine == core_proxy.LocalEngineKind.llamaCpp
                          ? 'GGUF · llama.cpp'
                          : _localEngineTitle(engine),
                    ),
                    selected: engine == _engine,
                    onSelected: locked ? null : (_) => _selectEngine(engine),
                  ),
              ],
            ),
            const SizedBox(height: 10),
            Text(
              _formatHint(l10n),
              style: theme.textTheme.bodyMedium?.copyWith(
                color: colors.onSurfaceVariant,
              ),
            ),
            const SizedBox(height: 16),
            _buildFileTile(context, locked: locked),
            if (hasFiles) ...[
              const SizedBox(height: 16),
              TextField(
                controller: _name,
                enabled: !locked,
                onChanged: (_) => setState(() {}),
                textInputAction: TextInputAction.done,
                decoration: InputDecoration(
                  labelText: l10n.localModelsImportName,
                  border: OutlineInputBorder(
                    borderRadius: BorderRadius.circular(12),
                  ),
                ),
              ),
            ],
            const SizedBox(height: 14),
            if (_importing)
              _buildProgress(context)
            else
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  Icon(
                    Icons.info_outline,
                    size: 16,
                    color: colors.onSurfaceVariant,
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      l10n.localModelsImportCopyNotice,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colors.onSurfaceVariant,
                      ),
                    ),
                  ),
                ],
              ),
            if (_error case final error?) ...[
              const SizedBox(height: 12),
              Semantics(
                liveRegion: true,
                child: SelectableText(
                  error,
                  style: TextStyle(color: colors.error),
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }

  /// Shows the picker action and summarizes the files once selected.
  Widget _buildFileTile(BuildContext context, {required bool locked}) {
    final l10n = AppLocalizations.of(context)!;
    final theme = Theme.of(context);
    final colors = theme.colorScheme;
    final hasFiles = _files.isNotEmpty;
    final label = switch (_files.length) {
      0 => l10n.selectFile,
      1 => _files.single.name,
      final count => l10n.localModelsImportFileCount(count),
    };
    return Material(
      color: colors.surfaceContainerHigh,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.circular(14),
        side: BorderSide(color: colors.outlineVariant),
      ),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        onTap: locked ? null : _pickFiles,
        child: Padding(
          padding: const EdgeInsets.all(12),
          child: Row(
            children: [
              Container(
                width: 36,
                height: 36,
                decoration: BoxDecoration(
                  color: colors.primaryContainer,
                  borderRadius: BorderRadius.circular(10),
                ),
                child: Icon(
                  hasFiles
                      ? Icons.description_outlined
                      : Icons.folder_open_outlined,
                  size: 19,
                  color: colors.onPrimaryContainer,
                ),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      label,
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.titleSmall,
                    ),
                    if (hasFiles)
                      Text(
                        _formatBytes(_totalBytes),
                        style: theme.textTheme.bodySmall?.copyWith(
                          color: colors.onSurfaceVariant,
                        ),
                      ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              if (_picking)
                const SizedBox.square(
                  dimension: 18,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              else if (!locked)
                Tooltip(
                  message: hasFiles
                      ? l10n.localModelsImportChangeFiles
                      : l10n.selectFile,
                  child: Icon(
                    hasFiles
                        ? Icons.drive_folder_upload_outlined
                        : Icons.chevron_right,
                    size: 20,
                    color: colors.onSurfaceVariant,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  /// Distinguishes byte transfer from the final commit to the registry.
  Widget _buildProgress(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final style = Theme.of(context).textTheme.bodySmall;
    final progress = (_copiedBytes / _totalBytes).clamp(0.0, 1.0);
    final phase = progress < 1
        ? l10n.localModelsImportCopying
        : l10n.localModelsImportFinishing;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Semantics(liveRegion: true, child: Text(phase, style: style)),
        const SizedBox(height: 8),
        LinearProgressIndicator(value: progress, semanticsLabel: phase),
        const SizedBox(height: 6),
        Text(
          '${_formatBytes(_copiedBytes)} / ${_formatBytes(_totalBytes)}',
          style: style,
        ),
      ],
    );
  }
}
