// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../common/components/OperitDialog.dart';

enum _MessagePartType { text, xml }

class _ParsedMessagePart {
  const _ParsedMessagePart({
    required this.type,
    required this.content,
    this.tag,
    this.attributes,
  });

  final _MessagePartType type;
  final String content;
  final String? tag;
  final String? attributes;

  _ParsedMessagePart copyWith({String? content}) {
    return _ParsedMessagePart(
      type: type,
      content: content ?? this.content,
      tag: tag,
      attributes: attributes,
    );
  }
}

class MessageEditorDialog extends StatefulWidget {
  const MessageEditorDialog({
    super.key,
    required this.initialText,
    required this.showResendButton,
    required this.onSave,
    required this.onResend,
  });

  final String initialText;
  final bool showResendButton;
  final Future<void> Function(String content) onSave;
  final Future<void> Function(String content) onResend;

  @override
  State<MessageEditorDialog> createState() => _MessageEditorDialogState();
}

class _MessageEditorDialogState extends State<MessageEditorDialog> {
  late String _content;
  late List<_ParsedMessagePart> _parts;
  bool _rawEditMode = false;
  bool _submitting = false;

  @override
  void initState() {
    super.initState();
    _content = widget.initialText;
    _parts = _parseMessageContentForEditor(_content);
  }

  void _syncContentFromParts() {
    _content = _recomposeMessageFromParts(_parts);
  }

  Future<void> _submit(Future<void> Function(String content) action) async {
    if (_submitting) {
      return;
    }
    setState(() {
      _submitting = true;
    });
    final content = _rawEditMode
        ? _content
        : _recomposeMessageFromParts(_parts);
    Navigator.of(context).pop();
    await action(content);
  }

  void _setRawMode(bool value) {
    setState(() {
      if (value) {
        _syncContentFromParts();
      } else {
        _parts = _parseMessageContentForEditor(_content);
      }
      _rawEditMode = value;
    });
  }

  @override
  Widget build(BuildContext context) {
    return OperitDialogScaffold(
      title: widget.showResendButton ? '编辑消息' : '修改记忆',
      maxWidth: 520,
      contentPadding: EdgeInsets.zero,
      titleActions: <Widget>[
        TextButton(
          onPressed: _submitting ? null : () => _setRawMode(!_rawEditMode),
          child: Text(_rawEditMode ? '可视' : '纯文本'),
        ),
      ],
      showCloseButton: true,
      closeButtonEnabled: !_submitting,
      onClose: () => Navigator.of(context).pop(),
      actions: <Widget>[
        TextButton(
          onPressed: _submitting ? null : () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        if (widget.showResendButton) ...<Widget>[
          OutlinedButton(
            onPressed: _submitting ? null : () => _submit(widget.onSave),
            child: const Text('保存'),
          ),
          FilledButton.icon(
            onPressed: _submitting ? null : () => _submit(widget.onResend),
            icon: const Icon(Icons.send, size: 14),
            label: const Text('保存并重发'),
          ),
        ] else
          FilledButton(
            onPressed: _submitting ? null : () => _submit(widget.onSave),
            child: const Text('更新记忆'),
          ),
      ],
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxHeight: 450),
        child: _rawEditMode
            ? _RawEditor(
                content: _content,
                enabled: !_submitting,
                onChanged: (value) {
                  _content = value;
                },
              )
            : _VisualEditor(
                parts: _parts,
                enabled: !_submitting,
                onChanged: (parts) {
                  setState(() {
                    _parts = parts;
                    _syncContentFromParts();
                  });
                },
              ),
      ),
    );
  }
}

class _RawEditor extends StatefulWidget {
  const _RawEditor({
    required this.content,
    required this.enabled,
    required this.onChanged,
  });

  final String content;
  final bool enabled;
  final ValueChanged<String> onChanged;

  @override
  State<_RawEditor> createState() => _RawEditorState();
}

class _RawEditorState extends State<_RawEditor> {
  late final TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.content);
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      child: TextField(
        controller: _controller,
        enabled: widget.enabled,
        minLines: 6,
        maxLines: 16,
        style: Theme.of(
          context,
        ).textTheme.bodyMedium?.copyWith(fontFamily: 'monospace'),
        decoration: InputDecoration(
          labelText: '纯文本内容',
          alignLabelWithHint: true,
          filled: true,
          fillColor: Theme.of(
            context,
          ).colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
          border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
        ),
        onChanged: widget.onChanged,
      ),
    );
  }
}

class _VisualEditor extends StatelessWidget {
  const _VisualEditor({
    required this.parts,
    required this.enabled,
    required this.onChanged,
  });

  final List<_ParsedMessagePart> parts;
  final bool enabled;
  final ValueChanged<List<_ParsedMessagePart>> onChanged;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ListView(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      shrinkWrap: true,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.fromLTRB(4, 4, 4, 8),
          child: Text(
            '内容片段',
            style: Theme.of(
              context,
            ).textTheme.labelMedium?.copyWith(color: colorScheme.primary),
          ),
        ),
        for (var index = 0; index < parts.length; index++)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: parts[index].type == _MessagePartType.text
                ? _TextPartEditor(
                    part: parts[index],
                    enabled: enabled,
                    onChanged: (content) {
                      final next = List<_ParsedMessagePart>.of(parts);
                      next[index] = parts[index].copyWith(content: content);
                      onChanged(next);
                    },
                    onDelete: () => _removeAt(index),
                  )
                : _XmlTagItem(
                    part: parts[index],
                    enabled: enabled,
                    onEdit: () => _editXml(context, index, parts[index]),
                    onDelete: () => _removeAt(index),
                  ),
          ),
        Row(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            OutlinedButton.icon(
              onPressed: enabled
                  ? () => onChanged(<_ParsedMessagePart>[
                      ...parts,
                      const _ParsedMessagePart(
                        type: _MessagePartType.text,
                        content: '',
                      ),
                    ])
                  : null,
              icon: const Icon(Icons.add, size: 16),
              label: const Text('添加文本'),
            ),
            const SizedBox(width: 8),
            OutlinedButton.icon(
              onPressed: enabled
                  ? () => _editXml(
                      context,
                      null,
                      const _ParsedMessagePart(
                        type: _MessagePartType.xml,
                        content: '',
                        tag: '',
                        attributes: '',
                      ),
                    )
                  : null,
              icon: const Icon(Icons.tag_outlined, size: 16),
              label: const Text('添加标签'),
            ),
          ],
        ),
      ],
    );
  }

  void _removeAt(int index) {
    final next = List<_ParsedMessagePart>.of(parts)..removeAt(index);
    onChanged(next);
  }

  Future<void> _editXml(
    BuildContext context,
    int? index,
    _ParsedMessagePart part,
  ) async {
    final updated = await showDialog<_ParsedMessagePart>(
      context: context,
      builder: (context) => _TagEditorDialog(part: part),
    );
    if (updated == null) {
      return;
    }
    final next = List<_ParsedMessagePart>.of(parts);
    if (index == null) {
      next.add(updated);
    } else {
      next[index] = updated;
    }
    onChanged(next);
  }
}

class _TextPartEditor extends StatefulWidget {
  const _TextPartEditor({
    required this.part,
    required this.enabled,
    required this.onChanged,
    required this.onDelete,
  });

  final _ParsedMessagePart part;
  final bool enabled;
  final ValueChanged<String> onChanged;
  final VoidCallback onDelete;

  @override
  State<_TextPartEditor> createState() => _TextPartEditorState();
}

class _TextPartEditorState extends State<_TextPartEditor> {
  late final TextEditingController _controller;

  @override
  void initState() {
    super.initState();
    _controller = TextEditingController(text: widget.part.content);
  }

  @override
  void didUpdateWidget(covariant _TextPartEditor oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.part.content != widget.part.content &&
        _controller.text != widget.part.content) {
      _controller.text = widget.part.content;
    }
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return TextField(
      controller: _controller,
      enabled: widget.enabled,
      minLines: 1,
      maxLines: 12,
      style: theme.textTheme.bodyMedium,
      decoration: InputDecoration(
        labelText: '文本',
        hintText: '输入文本内容',
        filled: true,
        fillColor: colorScheme.surfaceContainerHighest.withValues(alpha: 0.3),
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(12)),
        suffixIconConstraints: const BoxConstraints.tightFor(
          width: 40,
          height: 40,
        ),
        suffixIcon: IconButton(
          onPressed: widget.enabled ? widget.onDelete : null,
          icon: const Icon(Icons.delete, size: 16),
          tooltip: '删除',
          visualDensity: VisualDensity.compact,
          padding: EdgeInsets.zero,
          constraints: const BoxConstraints.tightFor(width: 32, height: 32),
        ),
      ),
      onChanged: widget.onChanged,
    );
  }
}

class _XmlTagItem extends StatefulWidget {
  const _XmlTagItem({
    required this.part,
    required this.enabled,
    required this.onEdit,
    required this.onDelete,
  });

  final _ParsedMessagePart part;
  final bool enabled;
  final VoidCallback onEdit;
  final VoidCallback onDelete;

  @override
  State<_XmlTagItem> createState() => _XmlTagItemState();
}

class _XmlTagItemState extends State<_XmlTagItem> {
  bool _expanded = false;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Card(
      color: colorScheme.primary.withValues(alpha: 0.08),
      elevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      child: Column(
        children: <Widget>[
          InkWell(
            borderRadius: BorderRadius.circular(12),
            onTap: () => setState(() => _expanded = !_expanded),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                children: <Widget>[
                  Icon(Icons.code, size: 18, color: colorScheme.primary),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Column(
                      crossAxisAlignment: CrossAxisAlignment.start,
                      children: <Widget>[
                        Text(
                          widget.part.tag?.isNotEmpty == true
                              ? widget.part.tag!
                              : 'XML',
                          style: theme.textTheme.bodyLarge?.copyWith(
                            color: colorScheme.primary,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                        if (widget.part.attributes?.trim().isNotEmpty == true)
                          Text(
                            widget.part.attributes!,
                            maxLines: 1,
                            overflow: TextOverflow.ellipsis,
                            style: theme.textTheme.labelSmall?.copyWith(
                              color: colorScheme.onSurfaceVariant,
                            ),
                          ),
                      ],
                    ),
                  ),
                  IconButton(
                    onPressed: widget.enabled ? widget.onEdit : null,
                    icon: const Icon(Icons.edit, size: 16),
                    tooltip: '编辑',
                  ),
                  IconButton(
                    onPressed: widget.enabled ? widget.onDelete : null,
                    icon: const Icon(Icons.delete, size: 16),
                    tooltip: '删除',
                  ),
                  Icon(
                    _expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
          if (_expanded)
            Container(
              width: double.infinity,
              color: colorScheme.surface.withValues(alpha: 0.5),
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Text(
                widget.part.content,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurface.withValues(alpha: 0.9),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

class _TagEditorDialog extends StatefulWidget {
  const _TagEditorDialog({required this.part});

  final _ParsedMessagePart part;

  @override
  State<_TagEditorDialog> createState() => _TagEditorDialogState();
}

class _TagEditorDialogState extends State<_TagEditorDialog> {
  late final TextEditingController _tagController;
  late final TextEditingController _attributesController;
  late final TextEditingController _contentController;

  @override
  void initState() {
    super.initState();
    _tagController = TextEditingController(text: widget.part.tag ?? '');
    _attributesController = TextEditingController(
      text: widget.part.attributes ?? '',
    );
    _contentController = TextEditingController(text: widget.part.content);
  }

  @override
  void dispose() {
    _tagController.dispose();
    _attributesController.dispose();
    _contentController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return OperitDialogScaffold(
      title: '编辑标签',
      icon: const Icon(Icons.tag_outlined, size: 20),
      maxWidth: 520,
      actions: <Widget>[
        OutlinedButton(
          onPressed: () => Navigator.of(context).pop(),
          child: const Text('取消'),
        ),
        FilledButton(
          onPressed: _tagController.text.trim().isEmpty
              ? null
              : () {
                  Navigator.of(context).pop(
                    _ParsedMessagePart(
                      type: _MessagePartType.xml,
                      content: _contentController.text,
                      tag: _tagController.text,
                      attributes: _attributesController.text,
                    ),
                  );
                },
          child: const Text('保存'),
        ),
      ],
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          TextField(
            controller: _tagController,
            decoration: const InputDecoration(
              labelText: '标签名',
              hintText: '例如 memory',
            ),
            onChanged: (_) => setState(() {}),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _attributesController,
            decoration: const InputDecoration(
              labelText: '属性（可选）',
              hintText: '例如 type="note"',
            ),
          ),
          const SizedBox(height: 12),
          TextField(
            controller: _contentController,
            minLines: 4,
            maxLines: 6,
            decoration: const InputDecoration(
              labelText: '内容',
              alignLabelWithHint: true,
            ),
          ),
        ],
      ),
    );
  }
}

List<_ParsedMessagePart> _parseMessageContentForEditor(String content) {
  final parts = <_ParsedMessagePart>[];
  final regex = RegExp(
    r'<([a-zA-Z0-9_-]+)([^>]*)>([\s\S]*?)</\1>',
    multiLine: true,
  );
  var lastIndex = 0;
  for (final match in regex.allMatches(content)) {
    if (match.start > lastIndex) {
      final textPart = content.substring(lastIndex, match.start);
      if (textPart.trim().isNotEmpty) {
        parts.add(
          _ParsedMessagePart(type: _MessagePartType.text, content: textPart),
        );
      }
    }
    parts.add(
      _ParsedMessagePart(
        type: _MessagePartType.xml,
        content: match.group(3) ?? '',
        tag: match.group(1),
        attributes: match.group(2),
      ),
    );
    lastIndex = match.end;
  }
  if (lastIndex < content.length) {
    final trailingText = content.substring(lastIndex);
    if (trailingText.trim().isNotEmpty) {
      parts.add(
        _ParsedMessagePart(type: _MessagePartType.text, content: trailingText),
      );
    }
  }
  return parts;
}

String _recomposeMessageFromParts(List<_ParsedMessagePart> parts) {
  return parts.map((part) {
    if (part.type == _MessagePartType.text) {
      return part.content;
    }
    return '<${part.tag}${part.attributes ?? ''}>${part.content}</${part.tag}>';
  }).join();
}
