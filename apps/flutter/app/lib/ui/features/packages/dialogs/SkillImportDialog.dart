// ignore_for_file: file_names

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';

class SkillImportResult {
  const SkillImportResult({required this.message});

  final String message;
}

class SkillImportDialog extends StatefulWidget {
  const SkillImportDialog({super.key, required this.clients});

  final GeneratedCoreProxyClients clients;

  @override
  State<SkillImportDialog> createState() => _SkillImportDialogState();
}

enum _SkillImportMode { github, zip, direct }

class _SkillImportDialogState extends State<SkillImportDialog> {
  final _formKey = GlobalKey<FormState>();
  final _repoUrlController = TextEditingController();
  final _skillIdController = TextEditingController();
  final _descriptionController = TextEditingController();
  final _contentController = TextEditingController();

  _SkillImportMode _mode = _SkillImportMode.github;
  XFile? _zipFile;
  List<XFile> _attachments = <XFile>[];
  bool _busy = false;

  @override
  void dispose() {
    _repoUrlController.dispose();
    _skillIdController.dispose();
    _descriptionController.dispose();
    _contentController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      icon: const Icon(Icons.build_outlined),
      title: const Text('添加技能'),
      content: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 560),
        child: SingleChildScrollView(
          child: Form(
            key: _formKey,
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                SegmentedButton<_SkillImportMode>(
                  segments: const <ButtonSegment<_SkillImportMode>>[
                    ButtonSegment<_SkillImportMode>(
                      value: _SkillImportMode.github,
                      icon: Icon(Icons.code),
                      label: Text('GitHub'),
                    ),
                    ButtonSegment<_SkillImportMode>(
                      value: _SkillImportMode.zip,
                      icon: Icon(Icons.archive_outlined),
                      label: Text('ZIP'),
                    ),
                    ButtonSegment<_SkillImportMode>(
                      value: _SkillImportMode.direct,
                      icon: Icon(Icons.edit_note),
                      label: Text('手写'),
                    ),
                  ],
                  selected: <_SkillImportMode>{_mode},
                  onSelectionChanged: _busy
                      ? null
                      : (value) {
                          setState(() {
                            _mode = value.single;
                          });
                        },
                ),
                const SizedBox(height: 16),
                AnimatedSwitcher(
                  duration: const Duration(milliseconds: 160),
                  child: switch (_mode) {
                    _SkillImportMode.github => TextFormField(
                      key: const ValueKey<_SkillImportMode>(
                        _SkillImportMode.github,
                      ),
                      controller: _repoUrlController,
                      enabled: !_busy,
                      decoration: const InputDecoration(
                        labelText: '仓库链接',
                        hintText: 'https://github.com/username/repo',
                        prefixIcon: Icon(Icons.link),
                      ),
                      validator: _required,
                    ),
                    _SkillImportMode.zip => _ZipPickerRow(
                      key: const ValueKey<_SkillImportMode>(
                        _SkillImportMode.zip,
                      ),
                      file: _zipFile,
                      enabled: !_busy,
                      onPick: _pickZip,
                    ),
                    _SkillImportMode.direct => Column(
                      key: const ValueKey<_SkillImportMode>(
                        _SkillImportMode.direct,
                      ),
                      mainAxisSize: MainAxisSize.min,
                      children: <Widget>[
                        TextFormField(
                          controller: _skillIdController,
                          enabled: !_busy,
                          decoration: const InputDecoration(
                            labelText: '技能 ID',
                            hintText: 'my-skill',
                            prefixIcon: Icon(Icons.tag),
                          ),
                          validator: _required,
                        ),
                        const SizedBox(height: 12),
                        TextFormField(
                          controller: _descriptionController,
                          enabled: !_busy,
                          decoration: const InputDecoration(
                            labelText: '描述',
                            prefixIcon: Icon(Icons.notes),
                          ),
                        ),
                        const SizedBox(height: 12),
                        TextFormField(
                          controller: _contentController,
                          enabled: !_busy,
                          minLines: 7,
                          maxLines: 12,
                          decoration: const InputDecoration(
                            labelText: '内容',
                            hintText: '写下这个技能的使用说明',
                            alignLabelWithHint: true,
                          ),
                          validator: _required,
                        ),
                        const SizedBox(height: 12),
                        Row(
                          children: <Widget>[
                            Expanded(
                              child: Text(
                                '附件 ${_attachments.length}',
                                style: Theme.of(context).textTheme.bodySmall
                                    ?.copyWith(
                                      color: colorScheme.onSurfaceVariant,
                                    ),
                              ),
                            ),
                            TextButton.icon(
                              onPressed: _busy ? null : _pickAttachments,
                              icon: const Icon(Icons.attach_file, size: 18),
                              label: const Text('添加附件'),
                            ),
                          ],
                        ),
                        if (_attachments.isNotEmpty)
                          ConstrainedBox(
                            constraints: const BoxConstraints(maxHeight: 140),
                            child: ListView.builder(
                              shrinkWrap: true,
                              itemCount: _attachments.length,
                              itemBuilder: (context, index) {
                                final file = _attachments[index];
                                return ListTile(
                                  dense: true,
                                  contentPadding: EdgeInsets.zero,
                                  title: Text(
                                    file.name,
                                    maxLines: 1,
                                    overflow: TextOverflow.ellipsis,
                                  ),
                                  trailing: IconButton(
                                    onPressed: _busy
                                        ? null
                                        : () {
                                            setState(() {
                                              _attachments = <XFile>[
                                                ..._attachments.take(index),
                                                ..._attachments.skip(index + 1),
                                              ];
                                            });
                                          },
                                    icon: const Icon(Icons.close, size: 18),
                                  ),
                                );
                              },
                            ),
                          ),
                      ],
                    ),
                  },
                ),
                if (_busy) ...<Widget>[
                  const SizedBox(height: 16),
                  LinearProgressIndicator(
                    minHeight: 2,
                    color: colorScheme.primary,
                  ),
                ],
              ],
            ),
          ),
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: _busy ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _busy ? null : _import,
          child: const Text('导入'),
        ),
      ],
    );
  }

  Future<void> _pickZip() async {
    final file = await openFile(
      acceptedTypeGroups: const <XTypeGroup>[
        XTypeGroup(label: 'Zip', extensions: <String>['zip']),
      ],
    );
    if (file == null) {
      return;
    }
    setState(() {
      _zipFile = file;
    });
  }

  Future<void> _pickAttachments() async {
    final files = await openFiles();
    setState(() {
      _attachments = <XFile>[..._attachments, ...files];
    });
  }

  Future<void> _import() async {
    if (!_formKey.currentState!.validate()) {
      return;
    }
    if (_mode == _SkillImportMode.zip && _zipFile == null) {
      ScaffoldMessenger.of(context).showSnackBar(
        const SnackBar(
          content: Text('请选择 ZIP 文件'),
          behavior: SnackBarBehavior.floating,
        ),
      );
      return;
    }
    setState(() {
      _busy = true;
    });
    try {
      final result = await switch (_mode) {
        _SkillImportMode.github =>
          widget.clients.permissionsSkillRuntimeSkillRepository.importSkillFromGitHubRepo(
            repoUrl: _repoUrlController.text.trim(),
          ),
        _SkillImportMode.zip =>
          widget.clients.permissionsSkillRuntimeSkillRepository.importSkillFromZip(
            zipFile: _zipFile!.path,
          ),
        _SkillImportMode.direct =>
          widget.clients.permissionsSkillRuntimeSkillRepository.importSkillFromDirectInput(
            skillId: _skillIdController.text.trim(),
            description: _descriptionController.text.trim(),
            content: _contentController.text.trim(),
            attachmentPaths: _attachments
                .map((file) => file.path)
                .toList(growable: false),
          ),
      };
      if (!mounted) {
        return;
      }
      Navigator.of(context).pop(SkillImportResult(message: result));
    } catch (error, stackTrace) {
      debugPrint('Failed to import skill: $error\n$stackTrace');
      if (!mounted) {
        return;
      }
      setState(() {
        _busy = false;
      });
      ScaffoldMessenger.of(context).showSnackBar(
        SnackBar(
          content: Text(error.toString()),
          behavior: SnackBarBehavior.floating,
        ),
      );
    }
  }

  String? _required(String? value) {
    return value == null || value.trim().isEmpty ? '必填' : null;
  }
}

class _ZipPickerRow extends StatelessWidget {
  const _ZipPickerRow({
    super.key,
    required this.file,
    required this.enabled,
    required this.onPick,
  });

  final XFile? file;
  final bool enabled;
  final VoidCallback onPick;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return OutlinedButton.icon(
      onPressed: enabled ? onPick : null,
      icon: const Icon(Icons.folder_zip_outlined),
      label: Align(
        alignment: Alignment.centerLeft,
        child: Text(
          file?.name ?? '选择 ZIP 文件',
          maxLines: 1,
          overflow: TextOverflow.ellipsis,
          style: TextStyle(
            color: file == null ? colorScheme.onSurfaceVariant : null,
          ),
        ),
      ),
    );
  }
}
