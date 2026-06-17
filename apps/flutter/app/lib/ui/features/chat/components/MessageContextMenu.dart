// ignore_for_file: file_names

import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../common/interactions/MessagePressShield.dart';
import '../../../../util/ChatMarkupRegex.dart';
import '../viewmodel/ChatViewModel.dart';

typedef MessageIndexAction = Future<void> Function(int index);
typedef MessageIndexBoolAction = Future<bool> Function(int index);
typedef MessageSelectionAction =
    void Function(int index, ChatUiMessage message);
typedef MessageTimestampAction = Future<void> Function(int timestamp);
typedef MessageVariantAction =
    Future<void> Function(int timestamp, int variantIndex);
typedef MessageFavoriteAction =
    Future<void> Function(int timestamp, bool isFavorite);

class MessageContextMenu extends StatefulWidget {
  const MessageContextMenu({
    super.key,
    required this.index,
    required this.message,
    required this.onToggleFavoriteMessage,
    required this.child,
    this.onDeleteMessage,
    this.onDeleteMessagesFrom,
    this.onDeleteMessageVariant,
    this.onRollbackToMessage,
    this.onSelectMessageToEdit,
    this.onRegenerateMessage,
    this.onInsertSummary,
    this.onCreateBranch,
    this.onReplyToMessage,
    this.onToggleMultiSelectMode,
    this.onRefresh,
  });

  final int index;
  final ChatUiMessage message;
  final MessageFavoriteAction onToggleFavoriteMessage;
  final MessageIndexAction? onDeleteMessage;
  final MessageIndexBoolAction? onDeleteMessagesFrom;
  final MessageVariantAction? onDeleteMessageVariant;
  final ValueChanged<int>? onRollbackToMessage;
  final MessageSelectionAction? onSelectMessageToEdit;
  final MessageIndexAction? onRegenerateMessage;
  final ValueChanged<ChatUiMessage>? onInsertSummary;
  final MessageTimestampAction? onCreateBranch;
  final ValueChanged<ChatUiMessage>? onReplyToMessage;
  final ValueChanged<int>? onToggleMultiSelectMode;
  final Future<void> Function()? onRefresh;
  final Widget child;

  @override
  State<MessageContextMenu> createState() => _MessageContextMenuState();
}

class _MessageContextMenuState extends State<MessageContextMenu> {
  static const Duration _longPressDuration = Duration(milliseconds: 500);
  static const double _longPressMoveTolerance = 12;

  Offset? _menuPosition;
  Offset? _longPressStartPosition;
  int? _longPressPointer;
  Timer? _longPressTimer;
  bool _isPressing = false;
  final MessagePressShieldController _pressShieldController =
      MessagePressShieldController();

  bool get _isActionable {
    return widget.message.sender == 'user' || widget.message.sender == 'ai';
  }

  @override
  Widget build(BuildContext context) {
    return MessagePressShield(
      controller: _pressShieldController,
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: _isActionable ? _handlePointerDown : null,
        onPointerMove: _isActionable ? _handlePointerMove : null,
        onPointerUp: _isActionable
            ? (event) => _cancelLongPress(pointer: event.pointer)
            : null,
        onPointerCancel: _isActionable
            ? (event) => _cancelLongPress(pointer: event.pointer)
            : null,
        child: GestureDetector(
          behavior: HitTestBehavior.translucent,
          onSecondaryTapDown: _isActionable
              ? (details) {
                  _menuPosition = details.globalPosition;
                }
              : null,
          onSecondaryTap: _isActionable ? _showContextMenu : null,
          child: _PressFeedback(isPressing: _isPressing, child: widget.child),
        ),
      ),
    );
  }

  void _handlePointerDown(PointerDownEvent event) {
    if (event.buttons != kPrimaryButton) {
      return;
    }
    _longPressPointer = event.pointer;
    _longPressStartPosition = event.position;
    _menuPosition = event.position;
    scheduleMicrotask(() {
      if (!mounted || _longPressPointer != event.pointer) {
        return;
      }
      if (_pressShieldController.isPointerShielded(event.pointer)) {
        _longPressPointer = null;
        _longPressStartPosition = null;
        return;
      }
      _setPressing(true);
      _longPressTimer?.cancel();
      _longPressTimer = Timer(_longPressDuration, () {
        _longPressTimer = null;
        if (!mounted) {
          return;
        }
        _showContextMenu();
      });
    });
  }

  void _handlePointerMove(PointerMoveEvent event) {
    if (_longPressPointer != event.pointer) {
      return;
    }
    final startPosition = _longPressStartPosition;
    if (startPosition == null) {
      return;
    }
    if ((event.position - startPosition).distance > _longPressMoveTolerance) {
      _cancelLongPress(pointer: event.pointer);
    }
  }

  void _cancelLongPress({int? pointer}) {
    if (pointer != null && _longPressPointer != pointer) {
      return;
    }
    _longPressTimer?.cancel();
    _longPressTimer = null;
    _longPressStartPosition = null;
    _longPressPointer = null;
    _setPressing(false);
  }

  void _setPressing(bool value) {
    if (_isPressing == value || !mounted) {
      return;
    }
    setState(() {
      _isPressing = value;
    });
  }

  Future<void> _showContextMenu() async {
    _cancelLongPress();
    final position = _menuPosition;
    if (position == null) {
      return;
    }
    final overlay = Overlay.of(context).context.findRenderObject() as RenderBox;
    final rect = RelativeRect.fromRect(
      position & const Size(1, 1),
      Offset.zero & overlay.size,
    );
    final action = await showMenu<_MessageMenuAction>(
      context: context,
      position: rect,
      constraints: const BoxConstraints(minWidth: 180, maxWidth: 220),
      popUpAnimationStyle: AnimationStyle.noAnimation,
      items: _menuItems(context),
    );
    if (!mounted || action == null) {
      return;
    }
    await _handleAction(action);
  }

  @override
  void dispose() {
    _cancelLongPress();
    super.dispose();
  }

  List<PopupMenuEntry<_MessageMenuAction>> _menuItems(BuildContext context) {
    final message = widget.message;
    final items = <PopupMenuEntry<_MessageMenuAction>>[
      _menuItem(
        value: _MessageMenuAction.copy,
        icon: Icons.content_copy,
        label: '复制消息',
      ),
    ];
    if (message.sender == 'user') {
      items.addAll(<PopupMenuEntry<_MessageMenuAction>>[
        _menuItem(
          value: _MessageMenuAction.editAndResend,
          icon: Icons.edit,
          label: '编辑并重发',
        ),
        _menuItem(
          value: _MessageMenuAction.rollback,
          icon: Icons.delete_sweep,
          label: '回滚到此处',
        ),
      ]);
    }
    if (message.sender == 'ai') {
      items.addAll(<PopupMenuEntry<_MessageMenuAction>>[
        _menuItem(
          value: _MessageMenuAction.regenerate,
          icon: Icons.refresh,
          label: '重新生成',
        ),
        _menuItem(
          value: _MessageMenuAction.modifyMemory,
          icon: Icons.auto_fix_high,
          label: '修改记忆',
        ),
      ]);
      if (message.variantCount > 1) {
        items.add(
          _menuItem(
            value: _MessageMenuAction.deleteVariant,
            icon: Icons.delete,
            label: '删除当前变体',
          ),
        );
      }
    }
    items.addAll(<PopupMenuEntry<_MessageMenuAction>>[
      _menuItem(
        value: _MessageMenuAction.delete,
        icon: Icons.delete,
        label: '删除',
      ),
    ]);
    if (message.sender == 'ai') {
      items.add(
        _menuItem(
          value: _MessageMenuAction.reply,
          icon: Icons.reply,
          label: '回复',
        ),
      );
    }
    items.addAll(<PopupMenuEntry<_MessageMenuAction>>[
      _menuItem(
        value: _MessageMenuAction.insertSummary,
        icon: Icons.summarize,
        label: '插入总结',
      ),
      _menuItem(
        value: _MessageMenuAction.createBranch,
        icon: Icons.account_tree,
        label: '创建分支',
      ),
      _menuItem(value: _MessageMenuAction.info, icon: Icons.info, label: '信息'),
      _menuItem(
        value: _MessageMenuAction.multiSelect,
        icon: Icons.check_circle,
        label: '多选',
      ),
    ]);
    return items;
  }

  PopupMenuItem<_MessageMenuAction> _menuItem({
    required _MessageMenuAction value,
    required IconData icon,
    required String label,
  }) {
    return PopupMenuItem<_MessageMenuAction>(
      value: value,
      height: 36,
      child: Row(
        children: <Widget>[
          Icon(icon, size: 16),
          const SizedBox(width: 12),
          Text(label, style: Theme.of(context).textTheme.bodyMedium),
        ],
      ),
    );
  }

  Future<void> _handleAction(_MessageMenuAction action) async {
    switch (action) {
      case _MessageMenuAction.copy:
        await Clipboard.setData(
          ClipboardData(text: cleanMessageContent(widget.message.content)),
        );
        break;
      case _MessageMenuAction.editAndResend:
        widget.onSelectMessageToEdit?.call(widget.index, widget.message);
        break;
      case _MessageMenuAction.modifyMemory:
        widget.onSelectMessageToEdit?.call(widget.index, widget.message);
        break;
      case _MessageMenuAction.rollback:
        widget.onRollbackToMessage?.call(widget.index);
        break;
      case _MessageMenuAction.regenerate:
        await widget.onRegenerateMessage?.call(widget.index);
        await widget.onRefresh?.call();
        break;
      case _MessageMenuAction.deleteVariant:
        await widget.onDeleteMessageVariant?.call(
          widget.message.timestamp,
          widget.message.selectedVariantIndex,
        );
        await widget.onRefresh?.call();
        break;
      case _MessageMenuAction.delete:
        await _confirmDelete();
        break;
      case _MessageMenuAction.reply:
        widget.onReplyToMessage?.call(widget.message);
        break;
      case _MessageMenuAction.insertSummary:
        widget.onInsertSummary?.call(widget.message);
        break;
      case _MessageMenuAction.createBranch:
        await widget.onCreateBranch?.call(widget.message.timestamp);
        await widget.onRefresh?.call();
        break;
      case _MessageMenuAction.info:
        await _showInfoDialog();
        break;
      case _MessageMenuAction.multiSelect:
        widget.onToggleMultiSelectMode?.call(widget.index);
        break;
    }
  }

  Future<void> _confirmDelete() async {
    final confirmed = await _confirm('确认删除', '确定删除这条消息？');
    if (!confirmed) {
      return;
    }
    await widget.onDeleteMessage?.call(widget.index);
    await widget.onRefresh?.call();
  }

  Future<bool> _confirm(String title, String message) async {
    final result = await showDialog<bool>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: Text(title),
          content: Text(message),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(false),
              child: const Text('取消'),
            ),
            TextButton(
              onPressed: () => Navigator.of(context).pop(true),
              child: const Text('删除'),
            ),
          ],
        );
      },
    );
    return result == true;
  }

  Future<void> _showInfoDialog() {
    final message = widget.message;
    return showDialog<void>(
      context: context,
      builder: (context) {
        return AlertDialog(
          title: const Text('消息信息'),
          content: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text('发送者: ${message.sender}'),
              Text('时间戳: ${message.timestamp}'),
              if (message.roleName.isNotEmpty) Text('角色: ${message.roleName}'),
              if (message.modelName.isNotEmpty)
                Text('模型: ${message.modelName}'),
              if (message.provider.isNotEmpty) Text('提供商: ${message.provider}'),
              Text('输入 token: ${message.inputTokens}'),
              Text('缓存输入 token: ${message.cachedInputTokens}'),
              Text('输出 token: ${message.outputTokens}'),
              Text('等待耗时: ${message.waitDurationMs}ms'),
              Text('输出耗时: ${message.outputDurationMs}ms'),
            ],
          ),
          actions: <Widget>[
            TextButton(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('确定'),
            ),
          ],
        );
      },
    );
  }
}

class _PressFeedback extends StatelessWidget {
  const _PressFeedback({required this.isPressing, required this.child});

  final bool isPressing;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AnimatedScale(
      scale: isPressing ? 0.99 : 1,
      duration: const Duration(milliseconds: 80),
      curve: Curves.easeOut,
      child: Stack(
        children: <Widget>[
          child,
          Positioned.fill(
            child: IgnorePointer(
              child: AnimatedOpacity(
                opacity: isPressing ? 1 : 0,
                duration: const Duration(milliseconds: 80),
                curve: Curves.easeOut,
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: colorScheme.onSurface.withValues(alpha: 0.06),
                  ),
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

enum _MessageMenuAction {
  copy,
  editAndResend,
  modifyMemory,
  rollback,
  regenerate,
  deleteVariant,
  delete,
  reply,
  insertSummary,
  createBranch,
  info,
  multiSelect,
}

String cleanMessageContent(String content) {
  return content
      .replaceAll(ChatMarkupRegex.memoryTag, '')
      .replaceAll(ChatMarkupRegex.workspaceAttachmentTag, '')
      .replaceAll(ChatMarkupRegex.attachmentTag, '')
      .replaceAll(ChatMarkupRegex.attachmentSelfClosingTag, '')
      .replaceAll(
        RegExp(r'<status\b[\s\S]*?</status>', caseSensitive: false),
        '',
      )
      .replaceAll(RegExp(r'<status\b[\s\S]*?/>', caseSensitive: false), '')
      .replaceAll(RegExp(r'<think\b[\s\S]*?</think>', caseSensitive: false), '')
      .replaceAll(
        RegExp(r'<thinking\b[\s\S]*?</thinking>', caseSensitive: false),
        '',
      )
      .replaceAll(
        RegExp(r'<search\b[\s\S]*?</search>', caseSensitive: false),
        '',
      )
      .replaceAll(
        RegExp(
          r'<tool(?:_(?!result(?:_|$))[A-Za-z0-9_]+)?\b[\s\S]*?</tool(?:_(?!result(?:_|$))[A-Za-z0-9_]+)?>',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'<tool(?:_(?!result(?:_|$))[A-Za-z0-9_]+)?\b[\s\S]*?/>',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'<tool_result(?:_[A-Za-z0-9_]+)?\b[\s\S]*?</tool_result(?:_[A-Za-z0-9_]+)?>',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(
          r'<tool_result(?:_[A-Za-z0-9_]+)?\b[\s\S]*?/>',
          caseSensitive: false,
        ),
        '',
      )
      .replaceAll(
        RegExp(r'<emotion\b[\s\S]*?</emotion>', caseSensitive: false),
        '',
      )
      .trim();
}
