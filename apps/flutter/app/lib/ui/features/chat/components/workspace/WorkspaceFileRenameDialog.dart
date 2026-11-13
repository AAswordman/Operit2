// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../l10n/generated/app_localizations.dart';
import '../../viewmodel/WorkspaceFileModels.dart';

class WorkspaceFileRenameDialog extends StatefulWidget {
  const WorkspaceFileRenameDialog({super.key, required this.entry});

  final WorkspaceFileEntry entry;

  @override
  State<WorkspaceFileRenameDialog> createState() =>
      _WorkspaceFileRenameDialogState();
}

class _WorkspaceFileRenameDialogState extends State<WorkspaceFileRenameDialog> {
  late final _controller = TextEditingController(text: widget.entry.name);
  bool _invalid = false;

  @override
  void initState() {
    super.initState();
    final dot = widget.entry.isDirectory
        ? -1
        : widget.entry.name.lastIndexOf('.');
    _controller.selection = TextSelection(
      baseOffset: 0,
      extentOffset: dot > 0 ? dot : widget.entry.name.length,
    );
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() {
    final name = _controller.text.trim();
    if (name.isEmpty ||
        name == '.' ||
        name == '..' ||
        name.contains(RegExp(r'[/\\\x00-\x1f]'))) {
      setState(() => _invalid = true);
      return;
    }
    Navigator.pop(context, name);
  }

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return AlertDialog(
      title: Text(l10n.workspaceFileRename),
      content: TextField(
        controller: _controller,
        autofocus: true,
        decoration: InputDecoration(
          labelText: l10n.workspaceFileName,
          errorText: _invalid ? l10n.workspaceFileInvalidName : null,
        ),
        onSubmitted: (_) => _submit(),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l10n.cancel),
        ),
        FilledButton(onPressed: _submit, child: Text(l10n.workspaceFileRename)),
      ],
    );
  }
}
