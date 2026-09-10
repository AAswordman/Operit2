// ignore_for_file: file_names

import 'dart:async';
import 'dart:convert';

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../common/interactions/MessagePressShield.dart';
import '../../../../util/ChatMarkupRegex.dart';
import '../viewmodel/ChatViewModel.dart';
import 'MessageCopyPreview.dart';
import '../../packages/screens/ToolPkgUiLauncherScreen.dart';

typedef MessageTimestampAction = Future<void> Function(int timestamp);
typedef MessageTimestampBoolAction = Future<bool> Function(int timestamp);
typedef MessageTimestampSelectionAction = void Function(int timestamp);
typedef MessageSelectionAction = void Function(ChatUiMessage message);
typedef MessageVariantAction =
    Future<void> Function(int timestamp, int variantIndex);
typedef MessageFavoriteAction =
    Future<void> Function(int timestamp, bool isFavorite);
typedef MessageVoiceAction = Future<void> Function(ChatUiMessage message);

class MessageContextMenu extends StatefulWidget {
  const MessageContextMenu({
    super.key,
    required this.message,
    required this.chatId,
    required this.messageIndex,
    required this.clients,
    required this.packageManager,
    required this.onToggleFavoriteMessage,
    this.splitMarkdownContent,
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
    this.onPlayVoice,
    this.onToggleMultiSelectMode,
    this.onRefresh,
  });

  final ChatUiMessage message;
  final String chatId;
  final int messageIndex;
  final GeneratedCoreProxyClients clients;
  final GeneratedApplicationPackageManagerCoreProxy packageManager;
  final MessageFavoriteAction onToggleFavoriteMessage;
  final MarkdownCopySplitter? splitMarkdownContent;
  final MessageTimestampAction? onDeleteMessage;
  final MessageTimestampBoolAction? onDeleteMessagesFrom;
  final MessageVariantAction? onDeleteMessageVariant;
  final MessageTimestampSelectionAction? onRollbackToMessage;
  final MessageSelectionAction? onSelectMessageToEdit;
  final MessageTimestampAction? onRegenerateMessage;
  final ValueChanged<ChatUiMessage>? onInsertSummary;
  final MessageTimestampAction? onCreateBranch;
  final ValueChanged<ChatUiMessage>? onReplyToMessage;
  final MessageVoiceAction? onPlayVoice;
  final MessageTimestampSelectionAction? onToggleMultiSelectMode;
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
          child: widget.child,
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
    final useEnglish = Localizations.localeOf(context).languageCode == 'en';
    final toolPkgItems = await widget.packageManager
        .getToolPkgChatMessageMenuItems(
          sender: widget.message.sender,
          useEnglish: useEnglish,
        );
    if (!mounted) {
      return;
    }
    final action = await showMenu<_MessageMenuSelection>(
      context: context,
      position: rect,
      constraints: const BoxConstraints(minWidth: 180, maxWidth: 220),
      popUpAnimationStyle: AnimationStyle.noAnimation,
      items: _menuItems(context, toolPkgItems),
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

  /// Builds built-in and ToolPkg context-menu entries for the selected message.
  List<PopupMenuEntry<_MessageMenuSelection>> _menuItems(
    BuildContext context,
    List<core_proxy.ToolPkgChatMessageMenuItem> toolPkgItems,
  ) {
    final message = widget.message;
    final items = <PopupMenuEntry<_MessageMenuSelection>>[
      _menuItem(
        value: _MessageMenuAction.copy,
        icon: Icons.content_copy,
        label: '复制消息',
      ),
    ];
    if (message.sender == 'user') {
      items.addAll(<PopupMenuEntry<_MessageMenuSelection>>[
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
      items.addAll(<PopupMenuEntry<_MessageMenuSelection>>[
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
        _menuItem(
          value: _MessageMenuAction.playVoice,
          icon: Icons.volume_up,
          label: '生成/播放语音',
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
    items.addAll(<PopupMenuEntry<_MessageMenuSelection>>[
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
    items.addAll(<PopupMenuEntry<_MessageMenuSelection>>[
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
    if (toolPkgItems.isNotEmpty) {
      items.add(const PopupMenuDivider());
      items.addAll(
        toolPkgItems.map(
          (item) => PopupMenuItem<_MessageMenuSelection>(
            value: _MessageMenuSelection.toolPkg(item),
            height: 36,
            child: Row(
              children: <Widget>[
                const Icon(Icons.extension_outlined, size: 16),
                const SizedBox(width: 12),
                Expanded(
                  child: Text(
                    item.title,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodyMedium,
                  ),
                ),
              ],
            ),
          ),
        ),
      );
    }
    return items;
  }

  /// Builds one built-in context-menu entry.
  PopupMenuItem<_MessageMenuSelection> _menuItem({
    required _MessageMenuAction value,
    required IconData icon,
    required String label,
  }) {
    return PopupMenuItem<_MessageMenuSelection>(
      value: _MessageMenuSelection.builtIn(value),
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

  /// Runs the chosen built-in command or ToolPkg menu callback.
  Future<void> _handleAction(_MessageMenuSelection selection) async {
    final toolPkgItem = selection.toolPkgItem;
    if (toolPkgItem != null) {
      await _runToolPkgMenuItem(toolPkgItem);
      return;
    }
    switch (selection.builtInAction!) {
      case _MessageMenuAction.copy:
        await showModalBottomSheet<void>(
          context: context,
          isScrollControlled: true,
          useSafeArea: true,
          builder: (context) => MessageCopyPreviewSheet(
            markdownText: cleanMessageContent(widget.message.copySourceText),
            splitMarkdownContent: widget.splitMarkdownContent,
          ),
        );
        break;
      case _MessageMenuAction.editAndResend:
        widget.onSelectMessageToEdit?.call(widget.message);
        break;
      case _MessageMenuAction.modifyMemory:
        widget.onSelectMessageToEdit?.call(widget.message);
        break;
      case _MessageMenuAction.rollback:
        widget.onRollbackToMessage?.call(widget.message.timestamp);
        break;
      case _MessageMenuAction.regenerate:
        await widget.onRegenerateMessage?.call(widget.message.timestamp);
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
      case _MessageMenuAction.playVoice:
        await widget.onPlayVoice?.call(widget.message);
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
        widget.onToggleMultiSelectMode?.call(widget.message.timestamp);
        break;
    }
  }

  /// Invokes a ToolPkg context-menu callback with the generated message JSON.
  Future<void> _runToolPkgMenuItem(
    core_proxy.ToolPkgChatMessageMenuItem item,
  ) async {
    final result = await widget.packageManager.invokeToolPkgChatMessageMenuItem(
      containerPackageName: item.containerPackageName,
      itemId: item.itemId,
      chatId: widget.chatId,
      messageIndex: widget.messageIndex,
      message: widget.message.toJson(),
    );
    final dialog = item.dialog;
    if (dialog != null) {
      await _showToolPkgMenuDialog(item: item, result: result);
    }
  }

  /// Opens the registered Compose DSL dialog with callback-provided state.
  Future<void> _showToolPkgMenuDialog({
    required core_proxy.ToolPkgChatMessageMenuItem item,
    required String? result,
  }) async {
    final dialog = item.dialog!;
    final response = result == null
        ? const <String, Object?>{}
        : _toolPkgJsonObject(jsonDecode(result), 'menu item result');
    final rawDialog = response['dialog'];
    final dialogResult = rawDialog == null
        ? const <String, Object?>{}
        : _toolPkgJsonObject(rawDialog, 'menu item dialog result');
    final state = <String, Object?>{
      'chatId': widget.chatId,
      'messageIndex': widget.messageIndex,
      'message': widget.message.toJson(),
      'menuItemId': item.itemId,
    };
    final rawState = dialogResult['state'];
    if (rawState != null) {
      state.addAll(_toolPkgJsonObject(rawState, 'menu item dialog state'));
    }
    final moduleSpec = <String, Object?>{
      'id': item.itemId,
      'runtime': 'compose_dsl',
      'screen': dialog.screen,
      'title': dialog.title,
      'toolPkgId': item.containerPackageName,
      'menuItemId': item.itemId,
    };
    final rawModuleSpec = dialogResult['moduleSpec'];
    if (rawModuleSpec != null) {
      moduleSpec.addAll(
        _toolPkgJsonObject(rawModuleSpec, 'menu item dialog moduleSpec'),
      );
    }
    final rawTitle = dialogResult['title'];
    final dialogTitle = switch (rawTitle) {
      null => dialog.title,
      String title => title,
      _ => throw StateError('ToolPkg menu item dialog title must be a string'),
    };
    final plugin = await widget.packageManager.getToolPkgContainerRuntime(
      containerPackageName: item.containerPackageName,
    );
    if (!mounted) {
      return;
    }
    if (plugin == null) {
      throw StateError(
        'ToolPkg container not found: ${item.containerPackageName}',
      );
    }
    await showDialog<void>(
      context: context,
      builder: (context) => AlertDialog(
        title: Text(dialogTitle),
        content: SizedBox(
          width: 760,
          height: 640,
          child: ToolPkgUiLauncherScreen(
            clients: widget.clients,
            plugin: plugin,
            initialRouteId: dialog.screen,
            showLauncherChrome: false,
            initialState: state,
            initialModuleSpec: moduleSpec,
          ),
        ),
      ),
    );
  }

  Future<void> _confirmDelete() async {
    final confirmed = await _confirm('确认删除', '确定删除这条消息？');
    if (!confirmed) {
      return;
    }
    await widget.onDeleteMessage?.call(widget.message.timestamp);
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

enum _MessageMenuAction {
  copy,
  editAndResend,
  modifyMemory,
  rollback,
  regenerate,
  deleteVariant,
  delete,
  reply,
  playVoice,
  insertSummary,
  createBranch,
  info,
  multiSelect,
}

/// Identifies one built-in or ToolPkg message context-menu selection.
class _MessageMenuSelection {
  const _MessageMenuSelection.builtIn(this.builtInAction) : toolPkgItem = null;

  const _MessageMenuSelection.toolPkg(this.toolPkgItem) : builtInAction = null;

  final _MessageMenuAction? builtInAction;
  final core_proxy.ToolPkgChatMessageMenuItem? toolPkgItem;
}

/// Converts a JSON object while rejecting values with non-string keys.
Map<String, Object?> _toolPkgJsonObject(Object? value, String fieldName) {
  if (value is! Map) {
    throw StateError('ToolPkg $fieldName must be an object');
  }
  final object = <String, Object?>{};
  for (final entry in value.entries) {
    final key = entry.key;
    if (key is! String) {
      throw StateError('ToolPkg $fieldName has a non-string key');
    }
    object[key] = entry.value;
  }
  return object;
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
