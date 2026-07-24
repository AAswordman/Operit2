// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui' as ui;

import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:liquid_glass_widgets/widgets/shared/glass_effect.dart';

import '../../../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import '../../../../../../../l10n/generated/app_localizations.dart';
import '../../../../../../theme/OperitTheme.dart';
import '../../../../../packages/utils/PackageDisplayUtils.dart';
import '../../../../viewmodel/ChatViewModel.dart';
import '../../../ChatLayoutMetrics.dart';
import '../agent/AgentInputMenuPopup.dart';
import '../agent/AgentModelSelectorPopup.dart';
import '../common/PendingQueueMessageItem.dart';

class ClassicChatInputSection extends StatefulWidget {
  const ClassicChatInputSection({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.isLoading,
    required this.inputState,
    required this.viewModel,
    required this.currentChatId,
    required this.onSendMessage,
    required this.onQueueMessage,
    required this.onCancelMessage,
    required this.isSpeechRecording,
    required this.isSpeechTranscribing,
    required this.onSpeechInput,
    this.pendingQueueMessages = const <PendingQueueMessageItem>[],
    this.isPendingQueueExpanded = true,
    this.onPendingQueueExpandedChange,
    this.onDeletePendingQueueMessage,
    this.onEditPendingQueueMessage,
    this.onSendPendingQueueMessage,
    this.attachments = const <AttachmentInfo>[],
    this.onRemoveAttachment,
    this.onInsertAttachment,
    this.onAttachImage,
    this.onTakePhoto,
    this.onAttachMemory,
    this.onAttachFile,
    this.onAttachFiles,
    this.onAttachScreenContent,
    this.onAttachNotifications,
    this.onAttachLocation,
    this.onAttachPackage,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isLoading;
  final ChatInputProcessingState inputState;
  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onSendMessage;
  final VoidCallback onQueueMessage;
  final VoidCallback onCancelMessage;
  final bool isSpeechRecording;
  final bool isSpeechTranscribing;
  final VoidCallback onSpeechInput;
  final List<PendingQueueMessageItem> pendingQueueMessages;
  final bool isPendingQueueExpanded;
  final ValueChanged<bool>? onPendingQueueExpandedChange;
  final ValueChanged<int>? onDeletePendingQueueMessage;
  final ValueChanged<int>? onEditPendingQueueMessage;
  final ValueChanged<int>? onSendPendingQueueMessage;
  final List<AttachmentInfo> attachments;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;
  final VoidCallback? onAttachImage;
  final VoidCallback? onTakePhoto;
  final VoidCallback? onAttachMemory;
  final VoidCallback? onAttachFile;
  final ValueChanged<List<String>>? onAttachFiles;
  final VoidCallback? onAttachScreenContent;
  final VoidCallback? onAttachNotifications;
  final VoidCallback? onAttachLocation;
  final ValueChanged<String>? onAttachPackage;

  /// Creates the mutable state for the classic input section.
  @override
  State<ClassicChatInputSection> createState() =>
      _ClassicChatInputSectionState();
}

class _ClassicChatInputSectionState extends State<ClassicChatInputSection> {
  final GlobalKey _modelPopupTargetKey = GlobalKey();
  final GlobalKey _inputMenuPopupTargetKey = GlobalKey();
  final GlobalKey _attachmentPopupTargetKey = GlobalKey();
  OverlayEntry? _modelPopupEntry;
  OverlayEntry? _inputMenuPopupEntry;
  OverlayEntry? _attachmentPopupEntry;
  StreamSubscription<
    Map<core_proxy.FunctionType, core_proxy.FunctionModelBinding>
  >?
  _modelBindingSubscription;
  bool _draggingFiles = false;
  bool _inputExpanded = false;
  String _modelLabel = '';

  /// Starts text and model label listeners for the classic input.
  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_handleInputChanged);
    _watchCurrentModelLabel();
  }

  /// Rebinds listeners when the input controller changes.
  @override
  void didUpdateWidget(covariant ClassicChatInputSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_handleInputChanged);
      widget.controller.addListener(_handleInputChanged);
    }
  }

  /// Refreshes action button state after text changes.
  void _handleInputChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  /// Toggles the input height while preserving the active draft and focus.
  void _toggleInputExpansion() {
    setState(() {
      _inputExpanded = !_inputExpanded;
    });
    widget.focusNode.requestFocus();
  }

  /// Toggles the model selector popup from the classic menu.
  void _toggleModelSettingsPopup() {
    if (_modelPopupEntry == null) {
      _dismissAttachmentPopup();
      _showModelSettingsPopup();
    } else {
      _dismissModelSettingsPopup();
    }
  }

  /// Toggles the classic settings popup.
  void _toggleInputMenuPopup() {
    if (_inputMenuPopupEntry == null) {
      _dismissModelSettingsPopup();
      _dismissAttachmentPopup();
      _showInputMenuPopup();
    } else {
      _dismissInputMenuPopup();
    }
  }

  /// Toggles the attachment popup.
  void _toggleAttachmentPopup() {
    if (_attachmentPopupEntry == null) {
      _dismissModelSettingsPopup();
      _dismissInputMenuPopup();
      _showAttachmentPopup();
    } else {
      _dismissAttachmentPopup();
    }
  }

  /// Shows the model selector popup above the classic settings strip.
  void _showModelSettingsPopup() {
    final overlay = Overlay.of(context);
    _modelPopupEntry = OverlayEntry(
      builder: (context) {
        final placement = _popupPlacement(
          context,
          targetKey: _modelPopupTargetKey,
          alignEnd: false,
          maxWidth: 300,
        );
        return _ClassicPopupShell(
          left: placement.left,
          bottom: placement.bottom,
          width: placement.width,
          maxHeight: placement.maxHeight,
          onDismiss: _dismissModelSettingsPopup,
          child: AgentModelSelectorPopup(
            viewModel: widget.viewModel,
            onDismiss: _dismissModelSettingsPopup,
            onModelChanged: _handleModelChanged,
          ),
        );
      },
    );
    overlay.insert(_modelPopupEntry!);
  }

  /// Shows the input settings popup above the classic settings strip.
  void _showInputMenuPopup() {
    final overlay = Overlay.of(context);
    _inputMenuPopupEntry = OverlayEntry(
      builder: (context) {
        final placement = _popupPlacement(
          context,
          targetKey: _inputMenuPopupTargetKey,
          alignEnd: false,
          maxWidth: 300,
        );
        return _ClassicPopupShell(
          left: placement.left,
          bottom: placement.bottom,
          width: placement.width,
          maxHeight: placement.maxHeight,
          onDismiss: _dismissInputMenuPopup,
          child: AgentInputMenuPopup(
            viewModel: widget.viewModel,
            currentChatId: widget.currentChatId,
            onDismiss: _dismissInputMenuPopup,
            leadingChildren: <Widget>[
              _ClassicModelMenuRow(
                targetKey: _modelPopupTargetKey,
                modelLabel: _modelLabel,
                onTap: _toggleModelSettingsPopup,
              ),
              const Divider(height: 1),
            ],
          ),
        );
      },
    );
    overlay.insert(_inputMenuPopupEntry!);
  }

  /// Shows the attachment popup above the classic input row.
  void _showAttachmentPopup() {
    final overlay = Overlay.of(context);
    _attachmentPopupEntry = OverlayEntry(
      builder: (context) {
        final placement = _popupPlacement(
          context,
          targetKey: _attachmentPopupTargetKey,
          alignEnd: true,
          maxWidth: 200,
        );
        return _ClassicPopupShell(
          left: placement.left,
          bottom: placement.bottom,
          width: placement.width,
          maxHeight: placement.maxHeight,
          onDismiss: _dismissAttachmentPopup,
          child: _ClassicAttachmentPopupPanel(
            onAttachImage: _runAttachmentAction(widget.onAttachImage),
            onTakePhoto: _runAttachmentAction(widget.onTakePhoto),
            onAttachMemory: _runAttachmentAction(widget.onAttachMemory),
            onAttachFile: _runAttachmentAction(widget.onAttachFile),
            onAttachScreenContent: _runAttachmentAction(
              widget.onAttachScreenContent,
            ),
            onAttachNotifications: _runAttachmentAction(
              widget.onAttachNotifications,
            ),
            onAttachLocation: _runAttachmentAction(widget.onAttachLocation),
            onAttachPackage: _showAttachmentPackageSelector,
          ),
        );
      },
    );
    overlay.insert(_attachmentPopupEntry!);
  }

  /// Resolves popup placement from a target key.
  _ClassicPopupPlacement _popupPlacement(
    BuildContext context, {
    required GlobalKey targetKey,
    required bool alignEnd,
    required double maxWidth,
  }) {
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final horizontalPadding = 12.0 + mediaQuery.padding.left;
    final rightPadding = 12.0 + mediaQuery.padding.right;
    final availableWidth = screenSize.width - horizontalPadding - rightPadding;
    final width = availableWidth < maxWidth ? availableWidth : maxWidth;
    final targetRect = _targetRect(targetKey);
    final desiredLeft = alignEnd ? targetRect.right - width : targetRect.left;
    final maxLeft = screenSize.width - rightPadding - width;
    final left = desiredLeft.clamp(horizontalPadding, maxLeft).toDouble();
    final bottom = screenSize.height - targetRect.top + 8;
    final maxHeight = (targetRect.top - mediaQuery.padding.top - 20).clamp(
      96.0,
      420.0,
    );
    return _ClassicPopupPlacement(
      left: left,
      bottom: bottom,
      width: width,
      maxHeight: maxHeight.toDouble(),
    );
  }

  /// Reads the screen-space rectangle for a popup target.
  Rect _targetRect(GlobalKey targetKey) {
    final renderObject = targetKey.currentContext?.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) {
      throw StateError('Classic popup target is not laid out.');
    }
    final topLeft = renderObject.localToGlobal(Offset.zero);
    return topLeft & renderObject.size;
  }

  /// Updates the file drag hover state.
  void _setDraggingFiles(bool dragging) {
    if (_draggingFiles == dragging) {
      return;
    }
    setState(() {
      _draggingFiles = dragging;
    });
  }

  /// Loads and watches the current chat model label.
  Future<void> _watchCurrentModelLabel() async {
    final clients = widget.viewModel.clients;
    await clients.preferencesModelConfigManager.initializeIfNeeded();
    await clients.preferencesFunctionalConfigManager.initializeIfNeeded();
    final binding = await clients.preferencesFunctionalConfigManager
        .getModelBindingForFunction(functionType: core_proxy.FunctionType.chat);
    await _applyModelBinding(binding);
    if (!mounted) {
      return;
    }
    _modelBindingSubscription = clients.preferencesFunctionalConfigManager
        .functionModelBindingFlowChanges()
        .listen((bindings) {
          _applyModelBinding(bindings[core_proxy.FunctionType.chat]!);
        });
  }

  /// Applies the selected model binding to the visible model label.
  Future<void> _applyModelBinding(
    core_proxy.FunctionModelBinding binding,
  ) async {
    final config = await widget.viewModel.clients.preferencesModelConfigManager
        .getResolvedModelConfig(
          providerId: binding.providerId,
          modelId: binding.modelId,
        );
    if (!mounted) {
      return;
    }
    setState(() {
      _modelLabel = _formatModelLabel(config.modelId);
    });
    _inputMenuPopupEntry?.markNeedsBuild();
  }

  /// Updates the visible model label after selector changes.
  void _handleModelChanged(String modelId) {
    setState(() {
      _modelLabel = _formatModelLabel(modelId);
    });
    _inputMenuPopupEntry?.markNeedsBuild();
  }

  /// Shortens long model labels for the classic settings strip.
  String _formatModelLabel(String modelId) {
    return modelId.length > 26 ? '${modelId.substring(0, 26)}...' : modelId;
  }

  /// Wraps an attachment action so the popup closes after selection.
  VoidCallback? _runAttachmentAction(VoidCallback? action) {
    if (action == null) {
      return null;
    }
    return () {
      action();
      _dismissAttachmentPopup();
    };
  }

  /// Opens the package selector dialog for package attachments.
  Future<void> _showAttachmentPackageSelector() async {
    _dismissAttachmentPopup();
    final packageName = await showDialog<String>(
      context: context,
      builder: (context) {
        return _ClassicAttachmentPackageSelectorDialog(
          viewModel: widget.viewModel,
        );
      },
    );
    if (packageName == null || packageName.trim().isEmpty) {
      return;
    }
    widget.onAttachPackage?.call(packageName);
  }

  /// Removes the model selector popup.
  void _dismissModelSettingsPopup() {
    _modelPopupEntry?.remove();
    _modelPopupEntry = null;
  }

  /// Removes the input settings popup.
  void _dismissInputMenuPopup() {
    _dismissModelSettingsPopup();
    _inputMenuPopupEntry?.remove();
    _inputMenuPopupEntry = null;
  }

  /// Removes the attachment popup.
  void _dismissAttachmentPopup() {
    _attachmentPopupEntry?.remove();
    _attachmentPopupEntry = null;
  }

  /// Releases listeners and removes open popups.
  @override
  void dispose() {
    widget.controller.removeListener(_handleInputChanged);
    _modelBindingSubscription?.cancel();
    _dismissModelSettingsPopup();
    _dismissInputMenuPopup();
    _dismissAttachmentPopup();
    super.dispose();
  }

  /// Builds the classic chat input section.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final snapshot = OperitTheme.of(context).themePreferenceSnapshot;
    final processing = widget.isLoading || widget.inputState.isProcessing;
    final hasDraftText = widget.controller.text.trim().isNotEmpty;
    final canSendMessage = hasDraftText || widget.attachments.isNotEmpty;
    final showCancelAction = processing && !hasDraftText;
    final showQueueAction = processing && hasDraftText;
    final processingStatus = _classicInputProcessingStatus(
      l10n,
      widget.inputState,
    );
    final showProcessingStatus =
        snapshot.showInputProcessingStatus &&
        widget.inputState.isProcessing &&
        processingStatus.isNotEmpty;
    final borderRadius = snapshot.chatInputFloating
        ? BorderRadius.circular(22)
        : BorderRadius.zero;
    final surfaceShape = RoundedRectangleBorder(borderRadius: borderRadius);
    return Material(
      color: Colors.transparent,
      child: Align(
        alignment: Alignment.bottomCenter,
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: snapshot.bubbleWideLayoutEnabled
                ? chatWideContentMaxWidth
                : chatContentMaxWidth,
          ),
          child: _ClassicInputSurface(
            color: colorScheme.surface,
            shape: surfaceShape,
            borderRadius: borderRadius,
            transparentSurface: snapshot.transparentSurfaceEnabled,
            width: double.infinity,
            margin: snapshot.chatInputFloating
                ? const EdgeInsets.fromLTRB(8, 0, 8, 6)
                : EdgeInsets.zero,
            child: Padding(
              padding: EdgeInsets.symmetric(
                horizontal: snapshot.chatInputFloating ? 14 : 22,
                vertical: snapshot.chatInputFloating ? 6 : 8,
              ),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (widget.pendingQueueMessages.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 8),
                      child: _ClassicPendingMessageQueuePanel(
                        queuedMessages: widget.pendingQueueMessages,
                        expanded: widget.isPendingQueueExpanded,
                        onExpandedChange: widget.onPendingQueueExpandedChange,
                        onDeleteMessage: widget.onDeletePendingQueueMessage,
                        onEditMessage: widget.onEditPendingQueueMessage,
                        onSendMessage: widget.onSendPendingQueueMessage,
                      ),
                    ),
                  if (showProcessingStatus)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 6),
                      child: Align(
                        alignment: Alignment.centerLeft,
                        child: Text(
                          processingStatus,
                          style: theme.textTheme.bodySmall?.copyWith(
                            color: colorScheme.onSurface.withValues(alpha: 0.8),
                          ),
                        ),
                      ),
                    ),
                  if (widget.attachments.isNotEmpty)
                    Padding(
                      padding: const EdgeInsets.only(bottom: 6),
                      child: _ClassicAttachmentStrip(
                        attachments: widget.attachments,
                        onRemoveAttachment: widget.onRemoveAttachment,
                        onInsertAttachment: widget.onInsertAttachment,
                      ),
                    ),
                  _ClassicInputBody(
                    controller: widget.controller,
                    focusNode: widget.focusNode,
                    inputState: widget.inputState,
                    settingsKey: _inputMenuPopupTargetKey,
                    attachmentKey: _attachmentPopupTargetKey,
                    processing: processing,
                    hasDraftText: hasDraftText,
                    canSendMessage: canSendMessage,
                    showCancelAction: showCancelAction,
                    showQueueAction: showQueueAction,
                    onSendMessage: widget.onSendMessage,
                    onQueueMessage: widget.onQueueMessage,
                    onCancelMessage: widget.onCancelMessage,
                    isSpeechRecording: widget.isSpeechRecording,
                    isSpeechTranscribing: widget.isSpeechTranscribing,
                    onSpeechInput: widget.onSpeechInput,
                    inputExpanded: _inputExpanded,
                    onToggleInputExpansion: _toggleInputExpansion,
                    onAttachFiles: widget.onAttachFiles,
                    draggingFiles: _draggingFiles,
                    onDraggingFilesChanged: _setDraggingFiles,
                    onSettings: _toggleInputMenuPopup,
                    onAttach: _toggleAttachmentPopup,
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ClassicInputBody extends StatelessWidget {
  const _ClassicInputBody({
    required this.controller,
    required this.focusNode,
    required this.inputState,
    required this.settingsKey,
    required this.attachmentKey,
    required this.processing,
    required this.hasDraftText,
    required this.canSendMessage,
    required this.showCancelAction,
    required this.showQueueAction,
    required this.onSendMessage,
    required this.onQueueMessage,
    required this.onCancelMessage,
    required this.isSpeechRecording,
    required this.isSpeechTranscribing,
    required this.onSpeechInput,
    required this.inputExpanded,
    required this.onToggleInputExpansion,
    required this.onAttachFiles,
    required this.draggingFiles,
    required this.onDraggingFilesChanged,
    required this.onSettings,
    required this.onAttach,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final ChatInputProcessingState inputState;
  final GlobalKey settingsKey;
  final GlobalKey attachmentKey;
  final bool processing;
  final bool hasDraftText;
  final bool canSendMessage;
  final bool showCancelAction;
  final bool showQueueAction;
  final VoidCallback onSendMessage;
  final VoidCallback onQueueMessage;
  final VoidCallback onCancelMessage;
  final bool isSpeechRecording;
  final bool isSpeechTranscribing;
  final VoidCallback onSpeechInput;
  final bool inputExpanded;
  final VoidCallback onToggleInputExpansion;
  final ValueChanged<List<String>>? onAttachFiles;
  final bool draggingFiles;
  final ValueChanged<bool> onDraggingFilesChanged;
  final VoidCallback onSettings;
  final VoidCallback? onAttach;

  /// Builds the traditional text field and action buttons.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final outline = colorScheme.outline.withValues(
      alpha: hasDraftText ? 0.68 : 0.42,
    );
    final inputBorder = OutlineInputBorder(
      borderRadius: BorderRadius.circular(14),
      borderSide: BorderSide(color: outline, width: 1),
    );
    final inputContent = AnimatedSize(
      duration: const Duration(milliseconds: 220),
      reverseDuration: const Duration(milliseconds: 220),
      curve: Curves.easeOutCubic,
      alignment: Alignment.bottomCenter,
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: <Widget>[
          Expanded(
            child: _ClassicDesktopEnterSendShortcuts(
              controller: controller,
              canSendMessage: canSendMessage,
              onSendMessage: onSendMessage,
              enabled: !inputExpanded,
              child: TextField(
                controller: controller,
                focusNode: focusNode,
                minLines: inputExpanded ? 10 : 1,
                maxLines: inputExpanded ? 16 : 5,
                readOnly: isSpeechRecording || isSpeechTranscribing,
                textInputAction: TextInputAction.newline,
                style: theme.textTheme.bodyMedium?.copyWith(height: 20 / 14),
                decoration: InputDecoration(
                  hintText: l10n.askOperitHint,
                  hintStyle: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                  filled: true,
                  fillColor: colorScheme.surface.withValues(alpha: 0.84),
                  border: inputBorder,
                  enabledBorder: inputBorder,
                  focusedBorder: inputBorder.copyWith(
                    borderSide: BorderSide(
                      color: colorScheme.primary,
                      width: 1.2,
                    ),
                  ),
                  contentPadding: const EdgeInsets.fromLTRB(14, 9, 4, 9),
                  prefixIconConstraints: const BoxConstraints(
                    minWidth: 34,
                    minHeight: 34,
                  ),
                  prefixIcon: KeyedSubtree(
                    key: settingsKey,
                    child: _ClassicIconTapTarget(
                      icon: Icons.tune_outlined,
                      color: colorScheme.onSurfaceVariant,
                      onTap: onSettings,
                      size: 18,
                      targetSize: 34,
                      tooltip: l10n.settings,
                    ),
                  ),
                  suffixIconConstraints: const BoxConstraints(
                    minWidth: 34,
                    minHeight: 34,
                  ),
                  suffixIcon: _ClassicIconTapTarget(
                    icon: inputExpanded
                        ? Icons.fullscreen_exit
                        : Icons.fullscreen,
                    color: colorScheme.onSurfaceVariant,
                    onTap: onToggleInputExpansion,
                    size: 18,
                    targetSize: 34,
                    tooltip: inputExpanded
                        ? l10n.collapseInput
                        : l10n.expandInput,
                  ),
                ),
                onSubmitted: (_) {
                  if (!inputExpanded && canSendMessage) {
                    onSendMessage();
                  }
                },
              ),
            ),
          ),
          const SizedBox(width: 8),
          KeyedSubtree(
            key: attachmentKey,
            child: Material(
              color: colorScheme.surfaceContainerHighest,
              shape: const CircleBorder(),
              clipBehavior: Clip.antiAlias,
              child: _ClassicIconTapTarget(
                icon: Icons.add,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.9),
                onTap: onAttach,
                size: 20,
                targetSize: 34,
                tooltip: l10n.addAttachment,
              ),
            ),
          ),
          const SizedBox(width: 8),
          _ClassicActionButton(
            processing: processing || isSpeechTranscribing,
            progress: isSpeechTranscribing
                ? null
                : _classicProgressFor(inputState),
            background: isSpeechRecording
                ? colorScheme.error
                : _classicActionBackground(
                    colorScheme,
                    showCancelAction: showCancelAction,
                    showQueueAction: showQueueAction,
                    canSend: canSendMessage,
                  ),
            foreground: isSpeechRecording
                ? colorScheme.onError
                : _classicActionForeground(
                    colorScheme,
                    showCancelAction: showCancelAction,
                    showQueueAction: showQueueAction,
                    canSend: canSendMessage,
                  ),
            icon: isSpeechRecording
                ? Icons.stop
                : isSpeechTranscribing
                ? Icons.graphic_eq
                : _classicActionIcon(
                    showCancelAction: showCancelAction,
                    showQueueAction: showQueueAction,
                    canSend: canSendMessage,
                  ),
            tooltip: isSpeechRecording
                ? '停止录音'
                : isSpeechTranscribing
                ? '正在识别'
                : showCancelAction
                ? l10n.cancel
                : showQueueAction
                ? l10n.chatQueueAddMessage
                : (canSendMessage ? l10n.send : ''),
            onPressed: isSpeechTranscribing
                ? null
                : () {
                    if (showCancelAction) {
                      onCancelMessage();
                    } else if (showQueueAction) {
                      onQueueMessage();
                    } else if (canSendMessage) {
                      onSendMessage();
                    } else {
                      onSpeechInput();
                    }
                  },
          ),
        ],
      ),
    );
    return DropTarget(
      enable: onAttachFiles != null,
      onDragEntered: (_) => onDraggingFilesChanged(true),
      onDragExited: (_) => onDraggingFilesChanged(false),
      onDragDone: (details) {
        onDraggingFilesChanged(false);
        final paths = details.files.map((file) => file.path).toList();
        onAttachFiles?.call(paths);
      },
      child: Stack(
        children: <Widget>[
          inputContent,
          Positioned.fill(
            child: IgnorePointer(
              child: AnimatedOpacity(
                opacity: draggingFiles ? 1 : 0,
                duration: const Duration(milliseconds: 120),
                child: DecoratedBox(
                  decoration: BoxDecoration(
                    color: colorScheme.primary.withValues(alpha: 0.08),
                    borderRadius: BorderRadius.circular(18),
                    border: Border.all(
                      color: colorScheme.primary.withValues(alpha: 0.55),
                      width: 1.4,
                    ),
                  ),
                  child: Center(
                    child: Icon(
                      Icons.attach_file,
                      color: colorScheme.primary.withValues(alpha: 0.9),
                    ),
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

class _ClassicInputSurface extends StatelessWidget {
  const _ClassicInputSurface({
    required this.color,
    required this.shape,
    required this.borderRadius,
    required this.transparentSurface,
    required this.width,
    required this.margin,
    required this.child,
  });

  final Color color;
  final ShapeBorder shape;
  final BorderRadius borderRadius;
  final bool transparentSurface;
  final double width;
  final EdgeInsetsGeometry margin;
  final Widget child;

  /// Builds the classic input surface with opaque or glass styling.
  @override
  Widget build(BuildContext context) {
    final effectiveColor = transparentSurface ? Colors.transparent : color;
    final decorated = SizedBox(
      width: width,
      child: DecoratedBox(
        decoration: ShapeDecoration(
          color: effectiveColor,
          shape: shape,
          shadows: transparentSurface
              ? const <BoxShadow>[]
              : <BoxShadow>[
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.08),
                    blurRadius: 14,
                    offset: const Offset(0, -3),
                  ),
                ],
        ),
        child: child,
      ),
    );
    final Widget surface;
    if (transparentSurface) {
      final settings = _classicInputGlassSettings(context);
      surface = ClipPath(
        clipper: ShapeBorderClipper(shape: shape),
        child: Stack(
          children: <Widget>[
            Positioned.fill(
              child: BackdropFilter(
                filter: ui.ImageFilter.blur(
                  sigmaX: settings.blur,
                  sigmaY: settings.blur,
                ),
                child: const ColoredBox(color: Colors.transparent),
              ),
            ),
            Positioned.fill(
              child: GlassEffect(
                quality: GlassQuality.standard,
                shape: LiquidRoundedSuperellipse(
                  borderRadius: borderRadius.topLeft.x,
                ),
                settings: settings,
                interactionIntensity: 0.85,
                ambientRim: 0.11,
                baseAlphaMultiplier: 0.75,
                edgeAlphaMultiplier: 0.75,
                rimThickness: 1.48,
                rimSmoothing: 8,
                child: const SizedBox.expand(),
              ),
            ),
            decorated,
          ],
        ),
      );
    } else {
      surface = decorated;
    }
    return Padding(padding: margin, child: surface);
  }
}

class _ClassicPopupShell extends StatelessWidget {
  const _ClassicPopupShell({
    required this.left,
    required this.bottom,
    required this.width,
    required this.maxHeight,
    required this.onDismiss,
    required this.child,
  });

  final double left;
  final double bottom;
  final double width;
  final double maxHeight;
  final VoidCallback onDismiss;
  final Widget child;

  /// Builds a dismissible positioned popup shell.
  @override
  Widget build(BuildContext context) {
    return Stack(
      children: <Widget>[
        Positioned.fill(
          child: GestureDetector(
            behavior: HitTestBehavior.translucent,
            onTap: onDismiss,
            child: const SizedBox.expand(),
          ),
        ),
        Positioned(
          left: left,
          bottom: bottom,
          width: width,
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () {},
            child: ConstrainedBox(
              constraints: BoxConstraints(maxHeight: maxHeight),
              child: child,
            ),
          ),
        ),
      ],
    );
  }
}

class _ClassicModelMenuRow extends StatelessWidget {
  const _ClassicModelMenuRow({
    required this.targetKey,
    required this.modelLabel,
    required this.onTap,
  });

  final GlobalKey targetKey;
  final String modelLabel;
  final VoidCallback onTap;

  /// Builds the model selector entry shown inside the classic menu.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return InkWell(
      key: targetKey,
      onTap: onTap,
      child: ConstrainedBox(
        constraints: const BoxConstraints(minHeight: 40),
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(Icons.memory_outlined, size: 17, color: colorScheme.primary),
              const SizedBox(width: 12),
              Text(l10n.model, style: theme.textTheme.bodySmall),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  modelLabel,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  textAlign: TextAlign.end,
                  style: theme.textTheme.bodySmall!.copyWith(
                    color: colorScheme.primary,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
              const SizedBox(width: 6),
              Icon(
                Icons.chevron_right,
                size: 20,
                color: colorScheme.onSurfaceVariant,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ClassicPopupPlacement {
  const _ClassicPopupPlacement({
    required this.left,
    required this.bottom,
    required this.width,
    required this.maxHeight,
  });

  final double left;
  final double bottom;
  final double width;
  final double maxHeight;
}

class _ClassicAttachmentPopupPanel extends StatelessWidget {
  const _ClassicAttachmentPopupPanel({
    required this.onAttachImage,
    required this.onTakePhoto,
    required this.onAttachMemory,
    required this.onAttachFile,
    required this.onAttachScreenContent,
    required this.onAttachNotifications,
    required this.onAttachLocation,
    required this.onAttachPackage,
  });

  final VoidCallback? onAttachImage;
  final VoidCallback? onTakePhoto;
  final VoidCallback? onAttachMemory;
  final VoidCallback? onAttachFile;
  final VoidCallback? onAttachScreenContent;
  final VoidCallback? onAttachNotifications;
  final VoidCallback? onAttachLocation;
  final VoidCallback onAttachPackage;

  /// Builds the classic attachment action popup.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final items = <_ClassicAttachmentPanelItem>[
      _ClassicAttachmentPanelItem(
        icon: Icons.image,
        label: l10n.attachmentPhoto,
        onTap: onAttachImage,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.photo_camera,
        label: l10n.attachmentCamera,
        onTap: onTakePhoto,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.memory,
        label: l10n.attachmentMemory,
        onTap: onAttachMemory,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.description,
        label: l10n.attachmentFile,
        onTap: onAttachFile,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.screenshot_monitor,
        label: l10n.attachmentScreenContent,
        onTap: onAttachScreenContent,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.notifications,
        label: l10n.attachmentNotifications,
        onTap: onAttachNotifications,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.location_on,
        label: l10n.attachmentLocation,
        onTap: onAttachLocation,
      ),
      _ClassicAttachmentPanelItem(
        icon: Icons.auto_awesome,
        label: l10n.attachmentPackage,
        onTap: onAttachPackage,
      ),
    ];
    return Material(
      color: colorScheme.surfaceContainer,
      elevation: 4,
      borderRadius: BorderRadius.circular(8),
      clipBehavior: Clip.antiAlias,
      child: SingleChildScrollView(
        padding: const EdgeInsets.symmetric(vertical: 4),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            for (final item in items)
              _ClassicAttachmentPanelItemButton(
                item: item,
                iconColor: colorScheme.onSurfaceVariant.withValues(alpha: 0.7),
                textStyle: theme.textTheme.bodyMedium,
                textColor: colorScheme.onSurface,
              ),
          ],
        ),
      ),
    );
  }
}

class _ClassicAttachmentPanelItem {
  const _ClassicAttachmentPanelItem({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;
}

class _ClassicAttachmentPanelItemButton extends StatelessWidget {
  const _ClassicAttachmentPanelItemButton({
    required this.item,
    required this.iconColor,
    required this.textStyle,
    required this.textColor,
  });

  final _ClassicAttachmentPanelItem item;
  final Color iconColor;
  final TextStyle? textStyle;
  final Color textColor;

  /// Builds one attachment popup action row.
  @override
  Widget build(BuildContext context) {
    return InkWell(
      onTap: item.onTap,
      child: SizedBox(
        height: 36,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 12),
          child: Row(
            children: <Widget>[
              Icon(item.icon, size: 16, color: iconColor),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  item.label,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: textStyle?.copyWith(color: textColor),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ClassicAttachmentPackageSelectorDialog extends StatefulWidget {
  const _ClassicAttachmentPackageSelectorDialog({required this.viewModel});

  final ChatViewModel viewModel;

  /// Creates the mutable state for the package selector dialog.
  @override
  State<_ClassicAttachmentPackageSelectorDialog> createState() =>
      _ClassicAttachmentPackageSelectorDialogState();
}

class _ClassicAttachmentPackageSelectorDialogState
    extends State<_ClassicAttachmentPackageSelectorDialog> {
  late final Future<List<_ClassicAttachmentPackageOption>> _future =
      _loadClassicAttachmentPackageOptions(widget.viewModel);

  /// Builds the package selector dialog.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return AlertDialog(
      title: Text(
        l10n.attachmentPackageSelectTitle,
        style: Theme.of(context).textTheme.titleMedium,
      ),
      content: SizedBox(
        width: 420,
        child: FutureBuilder<List<_ClassicAttachmentPackageOption>>(
          future: _future,
          builder: (context, snapshot) {
            if (snapshot.hasError) {
              Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
            }
            if (snapshot.connectionState != ConnectionState.done) {
              return const SizedBox(
                height: 120,
                child: Center(child: CircularProgressIndicator()),
              );
            }
            final options = snapshot.requireData;
            if (options.isEmpty) {
              return Padding(
                padding: const EdgeInsets.symmetric(vertical: 24),
                child: Text(
                  l10n.attachmentPackageEmpty,
                  textAlign: TextAlign.center,
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              );
            }
            return ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 400),
              child: ListView.builder(
                shrinkWrap: true,
                itemCount: options.length,
                itemBuilder: (context, index) {
                  final option = options[index];
                  return _ClassicAttachmentPackageOptionTile(
                    option: option,
                    onTap: () => Navigator.of(context).pop(option.packageName),
                  );
                },
              ),
            );
          },
        ),
      ),
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
      ],
    );
  }
}

enum _ClassicAttachmentPackageKind { package, skill, mcp }

class _ClassicAttachmentPackageOption {
  const _ClassicAttachmentPackageOption({
    required this.packageName,
    required this.title,
    required this.description,
    required this.kind,
  });

  final String packageName;
  final String title;
  final String description;
  final _ClassicAttachmentPackageKind kind;
}

class _ClassicAttachmentPackageOptionTile extends StatelessWidget {
  const _ClassicAttachmentPackageOptionTile({
    required this.option,
    required this.onTap,
  });

  final _ClassicAttachmentPackageOption option;
  final VoidCallback onTap;

  /// Builds one package option tile.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(8),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 10),
          child: Row(
            children: <Widget>[
              Icon(Icons.auto_awesome, size: 20, color: colorScheme.primary),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    Text(
                      option.title,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodyMedium?.copyWith(
                        color: colorScheme.onSurface,
                      ),
                    ),
                    Text(
                      _classicAttachmentPackageSubtitle(
                        AppLocalizations.of(context)!,
                        option,
                      ),
                      maxLines: 2,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _ClassicPendingMessageQueuePanel extends StatelessWidget {
  const _ClassicPendingMessageQueuePanel({
    required this.queuedMessages,
    required this.expanded,
    required this.onExpandedChange,
    required this.onDeleteMessage,
    required this.onEditMessage,
    required this.onSendMessage,
  });

  final List<PendingQueueMessageItem> queuedMessages;
  final bool expanded;
  final ValueChanged<bool>? onExpandedChange;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;

  /// Builds the pending queue panel for the classic input.
  @override
  Widget build(BuildContext context) {
    if (queuedMessages.isEmpty) {
      return const SizedBox.shrink();
    }
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return Material(
      color: colorScheme.surfaceContainerHighest,
      borderRadius: BorderRadius.circular(8),
      clipBehavior: Clip.antiAlias,
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          InkWell(
            onTap: onExpandedChange == null
                ? null
                : () => onExpandedChange!(!expanded),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
              child: Row(
                children: <Widget>[
                  Expanded(
                    child: Text(
                      l10n.chatPendingQueueTitle(queuedMessages.length),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: theme.textTheme.labelLarge?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ),
                  Icon(
                    expanded
                        ? Icons.keyboard_arrow_up
                        : Icons.keyboard_arrow_down,
                    color: colorScheme.onSurfaceVariant,
                  ),
                ],
              ),
            ),
          ),
          if (expanded)
            ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 220),
              child: ListView.separated(
                padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
                shrinkWrap: true,
                itemCount: queuedMessages.length,
                separatorBuilder: (_, _) => const SizedBox(height: 8),
                itemBuilder: (context, index) {
                  final item = queuedMessages[index];
                  return _ClassicPendingQueueMessageTile(
                    item: item,
                    onDeleteMessage: onDeleteMessage,
                    onEditMessage: onEditMessage,
                    onSendMessage: onSendMessage,
                  );
                },
              ),
            ),
        ],
      ),
    );
  }
}

class _ClassicPendingQueueMessageTile extends StatelessWidget {
  const _ClassicPendingQueueMessageTile({
    required this.item,
    required this.onDeleteMessage,
    required this.onEditMessage,
    required this.onSendMessage,
  });

  final PendingQueueMessageItem item;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;

  /// Builds one pending queue message row.
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final actionColor = colorScheme.onSurfaceVariant.withValues(alpha: 0.9);
    return Material(
      color: colorScheme.surface,
      borderRadius: BorderRadius.circular(8),
      clipBehavior: Clip.antiAlias,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
        child: Row(
          children: <Widget>[
            _ClassicQueueIconAction(
              icon: Icons.edit,
              tooltip: l10n.edit,
              color: actionColor,
              onTap: onEditMessage == null
                  ? null
                  : () => onEditMessage!(item.id),
            ),
            Expanded(
              child: Text(
                item.text,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: theme.textTheme.bodySmall?.copyWith(
                  color: colorScheme.onSurface,
                ),
              ),
            ),
            _ClassicQueueIconAction(
              icon: Icons.send,
              tooltip: l10n.send,
              color: actionColor,
              onTap: onSendMessage == null
                  ? null
                  : () => onSendMessage!(item.id),
            ),
            _ClassicQueueIconAction(
              icon: Icons.delete,
              tooltip: l10n.delete,
              color: actionColor,
              onTap: onDeleteMessage == null
                  ? null
                  : () => onDeleteMessage!(item.id),
            ),
          ],
        ),
      ),
    );
  }
}

class _ClassicAttachmentStrip extends StatelessWidget {
  const _ClassicAttachmentStrip({
    required this.attachments,
    required this.onRemoveAttachment,
    required this.onInsertAttachment,
  });

  final List<AttachmentInfo> attachments;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;

  /// Builds the horizontal attachment chip strip.
  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      height: 32,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        itemCount: attachments.length,
        separatorBuilder: (_, _) => const SizedBox(width: 6),
        itemBuilder: (context, index) {
          final attachment = attachments[index];
          return _ClassicAttachmentChip(
            attachment: attachment,
            colorScheme: colorScheme,
            onRemoveAttachment: onRemoveAttachment,
            onInsertAttachment: onInsertAttachment,
          );
        },
      ),
    );
  }
}

class _ClassicAttachmentChip extends StatelessWidget {
  const _ClassicAttachmentChip({
    required this.attachment,
    required this.colorScheme,
    required this.onRemoveAttachment,
    required this.onInsertAttachment,
  });

  final AttachmentInfo attachment;
  final ColorScheme colorScheme;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;

  /// Builds one attachment chip.
  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: attachment.fileName,
      child: Material(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.7),
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(13),
          side: BorderSide(color: colorScheme.outline.withValues(alpha: 0.5)),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onInsertAttachment == null
              ? null
              : () => onInsertAttachment!(attachment),
          child: SizedBox(
            height: 26,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.only(left: 6),
                  child: Icon(
                    _classicAttachmentIcon(attachment.mimeType),
                    size: 14,
                    color: colorScheme.primary,
                  ),
                ),
                const SizedBox(width: 4),
                ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 80),
                  child: Text(
                    attachment.fileName,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: colorScheme.onSurface,
                    ),
                  ),
                ),
                _ClassicAttachmentRemoveButton(
                  tooltip: AppLocalizations.of(context)!.close,
                  color: colorScheme.onSurfaceVariant,
                  onTap: onRemoveAttachment == null
                      ? null
                      : () => onRemoveAttachment!(attachment.filePath),
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _ClassicAttachmentRemoveButton extends StatelessWidget {
  const _ClassicAttachmentRemoveButton({
    required this.tooltip,
    required this.color,
    required this.onTap,
  });

  final String tooltip;
  final Color color;
  final VoidCallback? onTap;

  /// Builds the remove button inside one attachment chip.
  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 8,
        containedInkWell: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(3, 2, 5, 2),
          child: Icon(Icons.close, size: 10, color: color),
        ),
      ),
    );
  }
}

class _ClassicQueueIconAction extends StatelessWidget {
  const _ClassicQueueIconAction({
    required this.icon,
    required this.tooltip,
    required this.color,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final Color color;
  final VoidCallback? onTap;

  /// Builds one icon action in the pending queue.
  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 16,
        child: SizedBox(
          width: 28,
          height: 28,
          child: Icon(icon, size: 16, color: color),
        ),
      ),
    );
  }
}

class _ClassicIconTapTarget extends StatelessWidget {
  const _ClassicIconTapTarget({
    required this.icon,
    required this.color,
    required this.onTap,
    required this.tooltip,
    this.size = 20,
    this.targetSize = 36,
  });

  final IconData icon;
  final Color color;
  final VoidCallback? onTap;
  final String tooltip;
  final double size;
  final double targetSize;

  /// Builds a fixed-size icon tap target.
  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: InkResponse(
        onTap: onTap,
        radius: 20,
        child: SizedBox(
          width: targetSize,
          height: targetSize,
          child: Icon(icon, size: size, color: color),
        ),
      ),
    );
  }
}

class _ClassicActionButton extends StatelessWidget {
  const _ClassicActionButton({
    required this.processing,
    required this.progress,
    required this.background,
    required this.foreground,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final bool processing;
  final double? progress;
  final Color background;
  final Color foreground;
  final IconData icon;
  final String tooltip;
  final VoidCallback? onPressed;

  /// Builds the classic send, queue, or cancel button.
  @override
  Widget build(BuildContext context) {
    final button = SizedBox(
      width: 36,
      height: 36,
      child: Stack(
        alignment: Alignment.center,
        children: <Widget>[
          Material(
            color: background,
            shape: const CircleBorder(),
            child: InkWell(
              customBorder: const CircleBorder(),
              onTap: onPressed,
              child: SizedBox(
                width: 32,
                height: 32,
                child: Icon(icon, size: 17, color: foreground),
              ),
            ),
          ),
          if (processing)
            Positioned.fill(
              child: IgnorePointer(
                child: CircularProgressIndicator(
                  value: progress,
                  strokeWidth: 2.4,
                  color: foreground.withValues(alpha: 0.9),
                  backgroundColor: foreground.withValues(alpha: 0.24),
                ),
              ),
            ),
        ],
      ),
    );
    if (tooltip.isEmpty) {
      return button;
    }
    return Tooltip(message: tooltip, child: button);
  }
}

class _ClassicDesktopEnterSendShortcuts extends StatelessWidget {
  const _ClassicDesktopEnterSendShortcuts({
    required this.controller,
    required this.canSendMessage,
    required this.onSendMessage,
    required this.enabled,
    required this.child,
  });

  final TextEditingController controller;
  final bool canSendMessage;
  final VoidCallback onSendMessage;
  final bool enabled;
  final Widget child;

  /// Builds desktop enter-to-send shortcuts around the text field.
  @override
  Widget build(BuildContext context) {
    return CallbackShortcuts(
      bindings: enabled && _usesDesktopEnterSend
          ? <ShortcutActivator, VoidCallback>{
              const SingleActivator(LogicalKeyboardKey.enter): _sendIfReady,
              const SingleActivator(LogicalKeyboardKey.numpadEnter):
                  _sendIfReady,
              const SingleActivator(LogicalKeyboardKey.enter, control: true):
                  _insertNewline,
              const SingleActivator(
                LogicalKeyboardKey.numpadEnter,
                control: true,
              ): _insertNewline,
            }
          : const <ShortcutActivator, VoidCallback>{},
      child: child,
    );
  }

  bool get _usesDesktopEnterSend {
    return defaultTargetPlatform == TargetPlatform.windows ||
        defaultTargetPlatform == TargetPlatform.macOS;
  }

  /// Sends the draft when there is content.
  void _sendIfReady() {
    if (canSendMessage) {
      onSendMessage();
    }
  }

  /// Inserts a newline at the current selection.
  void _insertNewline() {
    final value = controller.value;
    final text = value.text;
    final selection = value.selection;
    final range = selection.isValid
        ? selection
        : TextSelection.collapsed(offset: text.length);
    final nextText = text.replaceRange(range.start, range.end, '\n');
    controller.value = TextEditingValue(
      text: nextText,
      selection: TextSelection.collapsed(offset: range.start + 1),
      composing: TextRange.empty,
    );
  }
}

/// Builds the glass settings used by the classic transparent input surface.
LiquidGlassSettings _classicInputGlassSettings(BuildContext context) {
  final dark = Theme.of(context).brightness == Brightness.dark;
  return LiquidGlassSettings(
    glassColor: dark ? const Color(0x08FFFFFF) : const Color(0x07FFFFFF),
    thickness: 80,
    blur: 8,
    chromaticAberration: 0.18,
    lightIntensity: 0.42,
    ambientStrength: 0.2,
    refractiveIndex: 1.16,
    saturation: 1.04,
    glowIntensity: 0.17,
    specularSharpness: GlassSpecularSharpness.soft,
    standardOpacityMultiplier: 0.12,
  );
}

/// Chooses an icon for the attachment MIME type.
IconData _classicAttachmentIcon(String mimeType) {
  if (mimeType.startsWith('image/')) {
    return Icons.image;
  }
  return Icons.description;
}

/// Converts a processing state into a visible status message.
String _classicInputProcessingStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _classicInputProcessingMessage(l10n, state.message);
  if (message.isNotEmpty) {
    return message;
  }
  return switch (state.kind) {
    'Processing' => l10n.processingMessage,
    'Connecting' => l10n.connectingAiService,
    'Receiving' => l10n.receivingAiResponse,
    'Summarizing' => l10n.summarizingMemories,
    'ExecutingPlan' => l10n.executingPlan,
    'ExecutingTool' => l10n.executingTool(state.toolName),
    'ProcessingToolResult' => l10n.processingToolResult(state.toolName),
    'ToolProgress' => _classicToolProgressStatus(l10n, state),
    _ => '',
  };
}

/// Converts tool progress into a visible status message.
String _classicToolProgressStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _classicInputProcessingMessage(l10n, state.message);
  if (message.isNotEmpty) {
    return state.toolName.isEmpty
        ? message
        : l10n.toolStatusWithName(state.toolName, message);
  }
  if (state.toolName.isEmpty) {
    return l10n.toolRunning;
  }
  return l10n.toolRunningWithName(state.toolName);
}

/// Maps internal processing message keys to localized text.
String _classicInputProcessingMessage(AppLocalizations l10n, String key) {
  const memberReplyingPrefix = 'role_response_planner_member_replying|';
  if (key.startsWith(memberReplyingPrefix)) {
    return l10n.roleResponsePlannerMemberReplying(
      key.substring(memberReplyingPrefix.length),
    );
  }
  return switch (key) {
    'enhanced_processing_input' => l10n.processingInput,
    'enhanced_processing_message' => l10n.processingMessage,
    'enhanced_connecting_service' => l10n.connectingAiService,
    'enhanced_receiving_response' => l10n.receivingAiResponse,
    'enhanced_receiving_tool_result' => l10n.receivingToolResultAiResponse,
    'role_response_planner_planning' => l10n.roleResponsePlannerPlanning,
    'role_response_planner_failed' => l10n.roleResponsePlannerFailed,
    'message_processing' => l10n.processingMessage,
    'message_summarizing' => l10n.summarizingMemories,
    _ => key,
  };
}

/// Resolves a progress value for the classic action button ring.
double _classicProgressFor(ChatInputProcessingState state) {
  return switch (state.kind) {
    'Processing' => 0.3,
    'Connecting' => 0.6,
    'Summarizing' => 0.05,
    'ToolProgress' => state.progress.clamp(0, 1),
    _ => 1,
  };
}

/// Resolves the classic action button background color.
Color _classicActionBackground(
  ColorScheme colorScheme, {
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return colorScheme.error;
  }
  if (showQueueAction) {
    return colorScheme.tertiary;
  }
  if (canSend) {
    return colorScheme.primary;
  }
  return colorScheme.primary;
}

/// Resolves the classic action button foreground color.
Color _classicActionForeground(
  ColorScheme colorScheme, {
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return colorScheme.onError;
  }
  if (showQueueAction) {
    return colorScheme.onTertiary;
  }
  return colorScheme.onPrimary;
}

/// Resolves the classic action button icon.
IconData _classicActionIcon({
  required bool showCancelAction,
  required bool showQueueAction,
  required bool canSend,
}) {
  if (showCancelAction) {
    return Icons.close;
  }
  if (showQueueAction) {
    return Icons.add;
  }
  if (canSend) {
    return Icons.send;
  }
  return Icons.mic;
}

/// Loads package attachment choices from package, skill, and MCP sources.
Future<List<_ClassicAttachmentPackageOption>>
_loadClassicAttachmentPackageOptions(ChatViewModel viewModel) async {
  final packageManager = viewModel.clients.application.packageManager();
  final skillRepository = viewModel.clients.application.skillRepository();
  final permissionsMcpRuntimeMcpLocalServer =
      viewModel.clients.permissionsMcpRuntimeMcpLocalServer;
  final options = <String, _ClassicAttachmentPackageOption>{};

  final availablePackages = await packageManager.getAvailablePackages();
  final packageEntries = availablePackages.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in packageEntries) {
    final packageName = entry.key;
    final isContainer = await packageManager.isToolPkgContainer(
      packageName: packageName,
    );
    if (isContainer) {
      continue;
    }
    options.putIfAbsent(
      packageName,
      () => _ClassicAttachmentPackageOption(
        packageName: packageName,
        title: toolPackageDisplayName(entry.value),
        description: localizedText(entry.value.description),
        kind: _ClassicAttachmentPackageKind.package,
      ),
    );
  }

  final skillPackages = await skillRepository.getAiVisibleSkillPackages();
  final skillEntries = skillPackages.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in skillEntries) {
    options.putIfAbsent(
      entry.key,
      () => _ClassicAttachmentPackageOption(
        packageName: entry.key,
        title: entry.key,
        description: entry.value.description,
        kind: _ClassicAttachmentPackageKind.skill,
      ),
    );
  }

  final mcpServers = await permissionsMcpRuntimeMcpLocalServer
      .getAllMcpServers();
  final mcpMetadata = await permissionsMcpRuntimeMcpLocalServer
      .getAllPluginMetadata();
  final mcpEntries = mcpServers.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in mcpEntries) {
    final metadata = mcpMetadata[entry.key];
    options.putIfAbsent(
      entry.key,
      () => _ClassicAttachmentPackageOption(
        packageName: entry.key,
        title: _classicMcpPackageTitle(entry.key, metadata),
        description: metadata?.description ?? '',
        kind: _ClassicAttachmentPackageKind.mcp,
      ),
    );
  }

  return options.values.toList(growable: false);
}

/// Resolves the visible MCP package title.
String _classicMcpPackageTitle(
  String serverName,
  core_proxy.PluginMetadata? metadata,
) {
  final title = metadata?.name.trim() ?? '';
  if (title.isEmpty) {
    return serverName;
  }
  return title;
}

/// Builds the subtitle for a package attachment option.
String _classicAttachmentPackageSubtitle(
  AppLocalizations l10n,
  _ClassicAttachmentPackageOption option,
) {
  final typeLabel = switch (option.kind) {
    _ClassicAttachmentPackageKind.package => l10n.attachmentPackageKindPackage,
    _ClassicAttachmentPackageKind.skill => l10n.attachmentPackageKindSkill,
    _ClassicAttachmentPackageKind.mcp => l10n.attachmentPackageKindMcp,
  };
  if (option.description.trim().isEmpty) {
    return typeLabel;
  }
  return '$typeLabel · ${option.description}';
}
