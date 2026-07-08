// ignore_for_file: file_names

import 'dart:async';
import 'dart:ui' as ui;

import 'package:desktop_drop/desktop_drop.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:liquid_glass_widgets/widgets/shared/glass_effect.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitTheme.dart';
import '../../packages/utils/PackageDisplayUtils.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatLayoutMetrics.dart';
import 'style/input/agent/AgentInputMenuPopup.dart';
import 'style/input/agent/AgentModelSelectorPopup.dart';

class PendingQueueMessageItem {
  const PendingQueueMessageItem({required this.id, required this.text});

  final int id;
  final String text;
}

class AgentChatInputSection extends StatefulWidget {
  const AgentChatInputSection({
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
    this.onSettings,
    this.onModelSelector,
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
  final VoidCallback? onSettings;
  final VoidCallback? onModelSelector;

  @override
  State<AgentChatInputSection> createState() => _AgentChatInputSectionState();
}

class _AgentChatInputSectionState extends State<AgentChatInputSection> {
  final LayerLink _modelPopupLink = LayerLink();
  final LayerLink _inputMenuPopupLink = LayerLink();
  final GlobalKey _modelPopupTargetKey = GlobalKey();
  final GlobalKey _inputMenuPopupTargetKey = GlobalKey();
  final GlobalKey _attachmentPopupTargetKey = GlobalKey();
  OverlayEntry? _modelPopupEntry;
  OverlayEntry? _inputMenuPopupEntry;
  OverlayEntry? _attachmentPopupEntry;
  StreamSubscription<Map<core_proxy.FunctionType, core_proxy.FunctionModelBinding>>?
  _modelBindingSubscription;
  bool _draggingFiles = false;
  String _modelLabel = '';

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_handleInputChanged);
    _watchCurrentModelLabel();
  }

  @override
  void didUpdateWidget(covariant AgentChatInputSection oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.controller != widget.controller) {
      oldWidget.controller.removeListener(_handleInputChanged);
      widget.controller.addListener(_handleInputChanged);
    }
  }

  void _handleInputChanged() {
    if (mounted) {
      setState(() {});
    }
  }

  void _toggleSettingsPopup() {
    widget.onModelSelector?.call();
    if (_modelPopupEntry == null) {
      _dismissInputMenuPopup();
      _dismissAttachmentPopup();
      _showModelSettingsPopup();
    } else {
      _dismissModelSettingsPopup();
    }
  }

  void _openInputMenuPopup() {
    widget.onSettings?.call();
    if (_inputMenuPopupEntry == null) {
      _dismissModelSettingsPopup();
      _dismissAttachmentPopup();
      _showInputMenuPopup();
    } else {
      _dismissInputMenuPopup();
    }
  }

  void _openAttachmentPopup() {
    if (_attachmentPopupEntry == null) {
      _dismissModelSettingsPopup();
      _dismissInputMenuPopup();
      _showAttachmentPopup();
    } else {
      _dismissAttachmentPopup();
    }
  }

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
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissModelSettingsPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: placement.left,
              bottom: placement.bottom,
              width: placement.width,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: ConstrainedBox(
                  constraints: BoxConstraints(maxHeight: placement.maxHeight),
                  child: AgentModelSelectorPopup(
                    viewModel: widget.viewModel,
                    onDismiss: _dismissModelSettingsPopup,
                    onModelChanged: _handleModelChanged,
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_modelPopupEntry!);
  }

  void _showInputMenuPopup() {
    final overlay = Overlay.of(context);
    _inputMenuPopupEntry = OverlayEntry(
      builder: (context) {
        final placement = _popupPlacement(
          context,
          targetKey: _inputMenuPopupTargetKey,
          alignEnd: true,
          maxWidth: 300,
        );
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissInputMenuPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: placement.left,
              bottom: placement.bottom,
              width: placement.width,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: ConstrainedBox(
                  constraints: BoxConstraints(maxHeight: placement.maxHeight),
                  child: AgentInputMenuPopup(
                    viewModel: widget.viewModel,
                    currentChatId: widget.currentChatId,
                    onDismiss: _dismissInputMenuPopup,
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_inputMenuPopupEntry!);
  }

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
        return Stack(
          children: <Widget>[
            Positioned.fill(
              child: GestureDetector(
                behavior: HitTestBehavior.translucent,
                onTap: _dismissAttachmentPopup,
                child: const SizedBox.expand(),
              ),
            ),
            Positioned(
              left: placement.left,
              bottom: placement.bottom,
              width: placement.width,
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onTap: () {},
                child: ConstrainedBox(
                  constraints: BoxConstraints(maxHeight: placement.maxHeight),
                  child: _AttachmentSelectorPopupPanel(
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
                    onAttachLocation: _runAttachmentAction(
                      widget.onAttachLocation,
                    ),
                    onAttachPackage: _showAttachmentPackageSelector,
                  ),
                ),
              ),
            ),
          ],
        );
      },
    );
    overlay.insert(_attachmentPopupEntry!);
  }

  void _setDraggingFiles(bool dragging) {
    if (_draggingFiles == dragging) {
      return;
    }
    setState(() {
      _draggingFiles = dragging;
    });
  }

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
  }

  void _handleModelChanged(String modelId) {
    setState(() {
      _modelLabel = _formatModelLabel(modelId);
    });
  }

  String _formatModelLabel(String modelId) {
    return modelId.length > 26 ? '${modelId.substring(0, 26)}...' : modelId;
  }

  VoidCallback? _runAttachmentAction(VoidCallback? action) {
    if (action == null) {
      return null;
    }
    return () {
      action();
      _dismissAttachmentPopup();
    };
  }

  Future<void> _showAttachmentPackageSelector() async {
    _dismissAttachmentPopup();
    final packageName = await showDialog<String>(
      context: context,
      builder: (context) {
        return _AttachmentPackageSelectorDialog(viewModel: widget.viewModel);
      },
    );
    if (packageName == null || packageName.trim().isEmpty) {
      return;
    }
    widget.onAttachPackage?.call(packageName);
  }

  _PopupPlacement _popupPlacement(
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
    final targetLeft = targetRect.left;
    final targetRight = targetRect.right;
    final desiredLeft = alignEnd ? targetRight - width : targetLeft;
    final maxLeft = screenSize.width - rightPadding - width;
    final left = desiredLeft.clamp(horizontalPadding, maxLeft).toDouble();
    final targetTop = targetRect.top;
    final bottom = screenSize.height - targetTop + 8;
    final maxHeight = (targetTop - mediaQuery.padding.top - 20).clamp(
      96.0,
      420.0,
    );
    return _PopupPlacement(
      left: left,
      bottom: bottom,
      width: width,
      maxHeight: maxHeight.toDouble(),
    );
  }

  Rect _targetRect(GlobalKey targetKey) {
    final renderObject = targetKey.currentContext?.findRenderObject();
    if (renderObject is! RenderBox || !renderObject.hasSize) {
      throw StateError('Popup target is not laid out.');
    }
    final topLeft = renderObject.localToGlobal(Offset.zero);
    return topLeft & renderObject.size;
  }

  void _dismissModelSettingsPopup() {
    _modelPopupEntry?.remove();
    _modelPopupEntry = null;
  }

  void _dismissInputMenuPopup() {
    _inputMenuPopupEntry?.remove();
    _inputMenuPopupEntry = null;
  }

  void _dismissAttachmentPopup() {
    _attachmentPopupEntry?.remove();
    _attachmentPopupEntry = null;
  }

  @override
  void dispose() {
    widget.controller.removeListener(_handleInputChanged);
    _modelBindingSubscription?.cancel();
    _dismissModelSettingsPopup();
    _dismissInputMenuPopup();
    _dismissAttachmentPopup();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final themePreferenceSnapshot = OperitTheme.of(
      context,
    ).themePreferenceSnapshot;
    final processing = widget.isLoading || widget.inputState.isProcessing;
    final hasDraftText = widget.controller.text.trim().isNotEmpty;
    final canSendMessage = hasDraftText || widget.attachments.isNotEmpty;
    final showCancelAction = processing && !hasDraftText;
    final showQueueAction = processing && hasDraftText;
    final processingStatus = _inputProcessingStatus(l10n, widget.inputState);
    final showProcessingStatus =
        themePreferenceSnapshot.showInputProcessingStatus &&
        widget.inputState.isProcessing &&
        processingStatus.isNotEmpty;
    const inputCardBorderRadius = BorderRadius.vertical(
      top: Radius.circular(20),
    );
    const inputCardShape = RoundedRectangleBorder(
      borderRadius: inputCardBorderRadius,
    );

    return Material(
      color: Colors.transparent,
      child: Align(
        alignment: Alignment.bottomCenter,
        child: ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: themePreferenceSnapshot.bubbleWideLayoutEnabled
                ? chatWideContentMaxWidth
                : chatContentMaxWidth,
          ),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              if (widget.pendingQueueMessages.isNotEmpty)
                Padding(
                  padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
                  child: _PendingMessageQueuePanel(
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
                  padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
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
              _InputSurface(
                color: colorScheme.surfaceContainer,
                shape: inputCardShape,
                borderRadius: inputCardBorderRadius,
                transparentSurface:
                    themePreferenceSnapshot.transparentSurfaceEnabled,
                width: double.infinity,
                margin: const EdgeInsets.only(top: 4),
                child: Padding(
                  padding: const EdgeInsets.fromLTRB(12, 14, 12, 8),
                  child: _InputBody(
                    controller: widget.controller,
                    focusNode: widget.focusNode,
                    inputState: widget.inputState,
                    modelLabel: _modelLabel,
                    modelSelectorLink: _modelPopupLink,
                    modelSelectorKey: _modelPopupTargetKey,
                    settingsLink: _inputMenuPopupLink,
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
                    attachments: widget.attachments,
                    onRemoveAttachment: widget.onRemoveAttachment,
                    onInsertAttachment: widget.onInsertAttachment,
                    onAttachFiles: widget.onAttachFiles,
                    draggingFiles: _draggingFiles,
                    onDraggingFilesChanged: _setDraggingFiles,
                    onAttach: _openAttachmentPopup,
                    onSettings: _openInputMenuPopup,
                    onModelSelector: _toggleSettingsPopup,
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}

class _InputSurface extends StatelessWidget {
  const _InputSurface({
    required this.color,
    required this.shape,
    required this.borderRadius,
    required this.child,
    required this.width,
    required this.margin,
    required this.transparentSurface,
  });

  final Color color;
  final ShapeBorder shape;
  final BorderRadius borderRadius;
  final Widget child;
  final double width;
  final EdgeInsetsGeometry margin;
  final bool transparentSurface;

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
                    blurRadius: 18,
                    spreadRadius: 1,
                    offset: const Offset(0, -4),
                  ),
                  BoxShadow(
                    color: Colors.black.withValues(alpha: 0.035),
                    blurRadius: 5,
                    spreadRadius: 0,
                    offset: const Offset(0, -1),
                  ),
                ],
        ),
        child: child,
      ),
    );
    final Widget surface;
    if (transparentSurface) {
      final settings = _inputTransparentGlassSettings(context);
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

LiquidGlassSettings _inputTransparentGlassSettings(BuildContext context) {
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

class _InputBody extends StatelessWidget {
  const _InputBody({
    required this.controller,
    required this.focusNode,
    required this.inputState,
    required this.modelLabel,
    required this.modelSelectorLink,
    required this.modelSelectorKey,
    required this.settingsLink,
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
    required this.attachments,
    required this.onRemoveAttachment,
    required this.onInsertAttachment,
    required this.onAttachFiles,
    required this.draggingFiles,
    required this.onDraggingFilesChanged,
    required this.onAttach,
    required this.onSettings,
    required this.onModelSelector,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final ChatInputProcessingState inputState;
  final String modelLabel;
  final LayerLink modelSelectorLink;
  final GlobalKey modelSelectorKey;
  final LayerLink settingsLink;
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
  final List<AttachmentInfo> attachments;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;
  final ValueChanged<List<String>>? onAttachFiles;
  final bool draggingFiles;
  final ValueChanged<bool> onDraggingFilesChanged;
  final VoidCallback? onAttach;
  final VoidCallback? onSettings;
  final VoidCallback? onModelSelector;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final inputContent = Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        if (attachments.isNotEmpty)
          _AttachmentStrip(
            attachments: attachments,
            onRemoveAttachment: onRemoveAttachment,
            onInsertAttachment: onInsertAttachment,
          ),
        _DesktopEnterSendShortcuts(
          controller: controller,
          canSendMessage: canSendMessage,
          onSendMessage: onSendMessage,
          child: TextField(
            controller: controller,
            focusNode: focusNode,
            minLines: 1,
            maxLines: 6,
            enabled: true,
            textInputAction: TextInputAction.newline,
            style: theme.textTheme.bodyMedium?.copyWith(height: 20 / 14),
            decoration: InputDecoration(
              hintText: l10n.askOperitHint,
              hintStyle: theme.textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
              filled: false,
              fillColor: Colors.transparent,
              suffixIcon: IconButton(
                onPressed: () {},
                icon: const Icon(Icons.fullscreen),
                color: colorScheme.onSurfaceVariant,
                tooltip: l10n.fullscreenInput,
              ),
              border: InputBorder.none,
              enabledBorder: InputBorder.none,
              focusedBorder: InputBorder.none,
              contentPadding: const EdgeInsets.fromLTRB(16, 10, 8, 8),
            ),
            onSubmitted: (_) {
              if (canSendMessage) {
                onSendMessage();
              }
            },
          ),
        ),
        const SizedBox(height: 8),
        Row(
          children: <Widget>[
            Expanded(
              child: Align(
                alignment: Alignment.centerLeft,
                child: CompositedTransformTarget(
                  key: modelSelectorKey,
                  link: modelSelectorLink,
                  child: InkWell(
                    borderRadius: BorderRadius.circular(12),
                    onTap: onModelSelector,
                    child: Container(
                      constraints: const BoxConstraints(maxWidth: 220),
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 6,
                      ),
                      decoration: BoxDecoration(
                        border: Border.all(
                          color: colorScheme.outline.withValues(alpha: 0.2),
                        ),
                        borderRadius: BorderRadius.circular(12),
                      ),
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: <Widget>[
                          Flexible(
                            child: Text(
                              modelLabel,
                              maxLines: 1,
                              overflow: TextOverflow.ellipsis,
                              style: theme.textTheme.bodyMedium?.copyWith(
                                color: colorScheme.onSurface,
                              ),
                            ),
                          ),
                          const SizedBox(width: 4),
                          Icon(
                            Icons.keyboard_arrow_down,
                            size: 18,
                            color: colorScheme.onSurfaceVariant,
                          ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
            const SizedBox(width: 6),
            CompositedTransformTarget(
              key: settingsKey,
              link: settingsLink,
              child: _IconTapTarget(
                icon: Icons.tune_outlined,
                color: colorScheme.onSurfaceVariant,
                onTap: onSettings,
                targetSize: 34,
                tooltip: l10n.settings,
              ),
            ),
            const SizedBox(width: 8),
            KeyedSubtree(
              key: attachmentKey,
              child: _IconTapTarget(
                icon: Icons.add,
                color: colorScheme.onSurfaceVariant.withValues(alpha: 0.9),
                onTap: onAttach,
                size: 24,
                tooltip: l10n.addAttachment,
              ),
            ),
            const SizedBox(width: 6),
            _ActionButton(
              processing: processing,
              progress: _progressFor(inputState),
              background: _actionBackground(
                colorScheme,
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: canSendMessage,
              ),
              foreground: _actionForeground(
                colorScheme,
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: canSendMessage,
              ),
              icon: _actionIcon(
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: canSendMessage,
              ),
              tooltip: showCancelAction
                  ? l10n.cancel
                  : showQueueAction
                  ? l10n.chatQueueAddMessage
                  : (canSendMessage ? l10n.send : ''),
              onPressed: () {
                if (showCancelAction) {
                  onCancelMessage();
                } else if (showQueueAction) {
                  onQueueMessage();
                } else if (canSendMessage) {
                  onSendMessage();
                }
              },
            ),
          ],
        ),
      ],
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

class _AttachmentSelectorPopupPanel extends StatelessWidget {
  const _AttachmentSelectorPopupPanel({
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

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    final items = <_AttachmentPanelItem>[
      _AttachmentPanelItem(
        icon: Icons.image,
        label: l10n.attachmentPhoto,
        onTap: onAttachImage,
      ),
      _AttachmentPanelItem(
        icon: Icons.photo_camera,
        label: l10n.attachmentCamera,
        onTap: onTakePhoto,
      ),
      _AttachmentPanelItem(
        icon: Icons.memory,
        label: l10n.attachmentMemory,
        onTap: onAttachMemory,
      ),
      _AttachmentPanelItem(
        icon: Icons.description,
        label: l10n.attachmentFile,
        onTap: onAttachFile,
      ),
      _AttachmentPanelItem(
        icon: Icons.screenshot_monitor,
        label: l10n.attachmentScreenContent,
        onTap: onAttachScreenContent,
      ),
      _AttachmentPanelItem(
        icon: Icons.notifications,
        label: l10n.attachmentNotifications,
        onTap: onAttachNotifications,
      ),
      _AttachmentPanelItem(
        icon: Icons.location_on,
        label: l10n.attachmentLocation,
        onTap: onAttachLocation,
      ),
      _AttachmentPanelItem(
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
              _AttachmentPanelItemButton(
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

class _AttachmentPanelItem {
  const _AttachmentPanelItem({
    required this.icon,
    required this.label,
    required this.onTap,
  });

  final IconData icon;
  final String label;
  final VoidCallback? onTap;
}

class _AttachmentPanelItemButton extends StatelessWidget {
  const _AttachmentPanelItemButton({
    required this.item,
    required this.iconColor,
    required this.textStyle,
    required this.textColor,
  });

  final _AttachmentPanelItem item;
  final Color iconColor;
  final TextStyle? textStyle;
  final Color textColor;

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

enum _AttachmentPackageKind { package, skill, mcp }

class _AttachmentPackageOption {
  const _AttachmentPackageOption({
    required this.packageName,
    required this.title,
    required this.description,
    required this.kind,
  });

  final String packageName;
  final String title;
  final String description;
  final _AttachmentPackageKind kind;
}

class _AttachmentPackageSelectorDialog extends StatefulWidget {
  const _AttachmentPackageSelectorDialog({required this.viewModel});

  final ChatViewModel viewModel;

  @override
  State<_AttachmentPackageSelectorDialog> createState() =>
      _AttachmentPackageSelectorDialogState();
}

class _AttachmentPackageSelectorDialogState
    extends State<_AttachmentPackageSelectorDialog> {
  final TextEditingController _searchController = TextEditingController();
  late final Future<List<_AttachmentPackageOption>> _future =
      _loadAttachmentPackageOptions(widget.viewModel);

  @override
  void initState() {
    super.initState();
    _searchController.addListener(_onSearchChanged);
  }

  @override
  void dispose() {
    _searchController.removeListener(_onSearchChanged);
    _searchController.dispose();
    super.dispose();
  }

  void _onSearchChanged() {
    if (mounted) {
      setState(() {});
    }
  }

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
        child: FutureBuilder<List<_AttachmentPackageOption>>(
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
            final options = snapshot.data ?? const <_AttachmentPackageOption>[];
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

            final filteredOptions = _filteredPackageOptions(options);
            return ConstrainedBox(
              constraints: const BoxConstraints(maxHeight: 400),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  TextField(
                    controller: _searchController,
                    maxLines: 1,
                    decoration: InputDecoration(
                      hintText: l10n.attachmentPackageSearchPlaceholder,
                      prefixIcon: Icon(
                        Icons.search,
                        color: colorScheme.onSurfaceVariant,
                      ),
                      suffixIcon: _searchController.text.isEmpty
                          ? null
                          : IconButton(
                              onPressed: _searchController.clear,
                              icon: const Icon(Icons.clear),
                              tooltip: l10n.clearSearch,
                            ),
                    ),
                  ),
                  const SizedBox(height: 12),
                  if (filteredOptions.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 24),
                      child: Text(
                        l10n.attachmentPackageSearchEmpty,
                        textAlign: TextAlign.center,
                        style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ),
                    )
                  else
                    Flexible(
                      child: ListView.builder(
                        shrinkWrap: true,
                        itemCount: filteredOptions.length,
                        itemBuilder: (context, index) {
                          final option = filteredOptions[index];
                          return _AttachmentPackageOptionTile(
                            option: option,
                            onTap: () =>
                                Navigator.of(context).pop(option.packageName),
                          );
                        },
                      ),
                    ),
                ],
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

  List<_AttachmentPackageOption> _filteredPackageOptions(
    List<_AttachmentPackageOption> options,
  ) {
    final query = _searchController.text.trim().toLowerCase();
    if (query.isEmpty) {
      return options;
    }
    return options
        .where((option) {
          return option.title.toLowerCase().contains(query) ||
              option.packageName.toLowerCase().contains(query) ||
              option.description.toLowerCase().contains(query);
        })
        .toList(growable: false);
  }
}

class _AttachmentPackageOptionTile extends StatelessWidget {
  const _AttachmentPackageOptionTile({
    required this.option,
    required this.onTap,
  });

  final _AttachmentPackageOption option;
  final VoidCallback onTap;

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
                      _buildAttachmentPackageSubtitle(
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

Future<List<_AttachmentPackageOption>> _loadAttachmentPackageOptions(
  ChatViewModel viewModel,
) async {
  final packageManager = viewModel.clients.permissionsPackToolPackageManager;
  final permissionsSkillRuntimeSkillRepository = viewModel.clients.permissionsSkillRuntimeSkillRepository;
  final permissionsMcpRuntimeMcpLocalServer = viewModel.clients.permissionsMcpRuntimeMcpLocalServer;
  final options = <String, _AttachmentPackageOption>{};

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
      () => _AttachmentPackageOption(
        packageName: packageName,
        title: toolPackageDisplayName(entry.value),
        description: localizedText(entry.value.description),
        kind: _AttachmentPackageKind.package,
      ),
    );
  }

  final skillPackages = await permissionsSkillRuntimeSkillRepository.getAiVisibleSkillPackages();
  final skillEntries = skillPackages.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in skillEntries) {
    options.putIfAbsent(
      entry.key,
      () => _AttachmentPackageOption(
        packageName: entry.key,
        title: entry.key,
        description: entry.value.description,
        kind: _AttachmentPackageKind.skill,
      ),
    );
  }

  final mcpServers = await permissionsMcpRuntimeMcpLocalServer.getAllMcpServers();
  final mcpMetadata = await permissionsMcpRuntimeMcpLocalServer.getAllPluginMetadata();
  final mcpEntries = mcpServers.entries.toList()
    ..sort((left, right) => left.key.compareTo(right.key));
  for (final entry in mcpEntries) {
    final metadata = mcpMetadata[entry.key];
    options.putIfAbsent(
      entry.key,
      () => _AttachmentPackageOption(
        packageName: entry.key,
        title: _mcpPackageTitle(entry.key, metadata),
        description: metadata?.description ?? '',
        kind: _AttachmentPackageKind.mcp,
      ),
    );
  }

  return options.values.toList(growable: false);
}

String _mcpPackageTitle(
  String serverName,
  core_proxy.PluginMetadata? metadata,
) {
  final title = metadata?.name.trim() ?? '';
  if (title.isEmpty) {
    return serverName;
  }
  return title;
}

String _buildAttachmentPackageSubtitle(
  AppLocalizations l10n,
  _AttachmentPackageOption option,
) {
  final typeLabel = switch (option.kind) {
    _AttachmentPackageKind.package => l10n.attachmentPackageKindPackage,
    _AttachmentPackageKind.skill => l10n.attachmentPackageKindSkill,
    _AttachmentPackageKind.mcp => l10n.attachmentPackageKindMcp,
  };
  if (option.description.trim().isEmpty) {
    return typeLabel;
  }
  return '$typeLabel · ${option.description}';
}

class _PendingMessageQueuePanel extends StatelessWidget {
  const _PendingMessageQueuePanel({
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
                  return _PendingQueueMessageTile(
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

class _PendingQueueMessageTile extends StatelessWidget {
  const _PendingQueueMessageTile({
    required this.item,
    required this.onDeleteMessage,
    required this.onEditMessage,
    required this.onSendMessage,
  });

  final PendingQueueMessageItem item;
  final ValueChanged<int>? onDeleteMessage;
  final ValueChanged<int>? onEditMessage;
  final ValueChanged<int>? onSendMessage;

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
            _QueueIconAction(
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
            _QueueIconAction(
              icon: Icons.send,
              tooltip: l10n.send,
              color: actionColor,
              onTap: onSendMessage == null
                  ? null
                  : () => onSendMessage!(item.id),
            ),
            _QueueIconAction(
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

class _AttachmentStrip extends StatelessWidget {
  const _AttachmentStrip({
    required this.attachments,
    required this.onRemoveAttachment,
    required this.onInsertAttachment,
  });

  final List<AttachmentInfo> attachments;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      height: 32,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.fromLTRB(12, 0, 12, 6),
        itemCount: attachments.length,
        separatorBuilder: (_, _) => const SizedBox(width: 6),
        itemBuilder: (context, index) {
          final attachment = attachments[index];
          return _AttachmentChip(
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

class _AttachmentChip extends StatelessWidget {
  const _AttachmentChip({
    required this.attachment,
    required this.colorScheme,
    required this.onRemoveAttachment,
    required this.onInsertAttachment,
  });

  final AttachmentInfo attachment;
  final ColorScheme colorScheme;
  final ValueChanged<String>? onRemoveAttachment;
  final ValueChanged<AttachmentInfo>? onInsertAttachment;

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
                    _attachmentIcon(attachment.mimeType),
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
                _AttachmentRemoveButton(
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

class _AttachmentRemoveButton extends StatelessWidget {
  const _AttachmentRemoveButton({
    required this.tooltip,
    required this.color,
    required this.onTap,
  });

  final String tooltip;
  final Color color;
  final VoidCallback? onTap;

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

class _QueueIconAction extends StatelessWidget {
  const _QueueIconAction({
    required this.icon,
    required this.tooltip,
    required this.color,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final Color color;
  final VoidCallback? onTap;

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

IconData _attachmentIcon(String mimeType) {
  if (mimeType.startsWith('image/')) {
    return Icons.image;
  }
  return Icons.description;
}

class _DesktopEnterSendShortcuts extends StatelessWidget {
  const _DesktopEnterSendShortcuts({
    required this.controller,
    required this.canSendMessage,
    required this.onSendMessage,
    required this.child,
  });

  final TextEditingController controller;
  final bool canSendMessage;
  final VoidCallback onSendMessage;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    if (!_usesDesktopEnterSend) {
      return child;
    }
    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        const SingleActivator(LogicalKeyboardKey.enter): _sendIfReady,
        const SingleActivator(LogicalKeyboardKey.numpadEnter): _sendIfReady,
        const SingleActivator(LogicalKeyboardKey.enter, control: true):
            _insertNewline,
        const SingleActivator(LogicalKeyboardKey.numpadEnter, control: true):
            _insertNewline,
      },
      child: child,
    );
  }

  bool get _usesDesktopEnterSend {
    return defaultTargetPlatform == TargetPlatform.windows ||
        defaultTargetPlatform == TargetPlatform.macOS;
  }

  void _sendIfReady() {
    if (canSendMessage) {
      onSendMessage();
    }
  }

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

String _inputProcessingStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _inputProcessingMessage(l10n, state.message);
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
    'ToolProgress' => _toolProgressStatus(l10n, state),
    _ => '',
  };
}

String _toolProgressStatus(
  AppLocalizations l10n,
  ChatInputProcessingState state,
) {
  final message = _inputProcessingMessage(l10n, state.message);
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

String _inputProcessingMessage(AppLocalizations l10n, String key) {
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

class _PopupPlacement {
  const _PopupPlacement({
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

class _IconTapTarget extends StatelessWidget {
  const _IconTapTarget({
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

class _ActionButton extends StatelessWidget {
  const _ActionButton({
    required this.processing,
    required this.progress,
    required this.background,
    required this.foreground,
    required this.icon,
    required this.tooltip,
    required this.onPressed,
  });

  final bool processing;
  final double progress;
  final Color background;
  final Color foreground;
  final IconData icon;
  final String tooltip;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final button = SizedBox(
      width: 40,
      height: 40,
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
                width: 36,
                height: 36,
                child: Icon(icon, size: 18, color: foreground),
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

double _progressFor(ChatInputProcessingState state) {
  return switch (state.kind) {
    'Processing' => 0.3,
    'Connecting' => 0.6,
    'Summarizing' => 0.05,
    'ToolProgress' => state.progress.clamp(0, 1),
    _ => 1,
  };
}

Color _actionBackground(
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

Color _actionForeground(
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

IconData _actionIcon({
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
