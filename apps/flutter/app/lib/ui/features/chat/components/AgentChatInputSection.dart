// ignore_for_file: file_names

import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:liquid_glass_widgets/liquid_glass_widgets.dart';
import 'package:liquid_glass_widgets/widgets/shared/glass_effect.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitTheme.dart';
import '../viewmodel/ChatViewModel.dart';
import 'ChatLayoutMetrics.dart';
import 'style/input/agent/AgentInputMenuPopup.dart';
import 'style/input/agent/AgentModelSelectorPopup.dart';

class AgentChatInputSection extends StatefulWidget {
  const AgentChatInputSection({
    super.key,
    required this.controller,
    required this.focusNode,
    required this.isLoading,
    required this.inputState,
    required this.modelLabel,
    required this.viewModel,
    required this.currentChatId,
    required this.onSendMessage,
    required this.onCancelMessage,
    required this.onModelChanged,
    this.onAttach,
    this.onSettings,
    this.onModelSelector,
  });

  final TextEditingController controller;
  final FocusNode focusNode;
  final bool isLoading;
  final ChatInputProcessingState inputState;
  final String modelLabel;
  final ChatViewModel viewModel;
  final String? currentChatId;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;
  final ValueChanged<String> onModelChanged;
  final VoidCallback? onAttach;
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
  OverlayEntry? _modelPopupEntry;
  OverlayEntry? _inputMenuPopupEntry;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_handleInputChanged);
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
      _showModelSettingsPopup();
    } else {
      _dismissModelSettingsPopup();
    }
  }

  void _openInputMenuPopup() {
    widget.onSettings?.call();
    if (_inputMenuPopupEntry == null) {
      _dismissModelSettingsPopup();
      _showInputMenuPopup();
    } else {
      _dismissInputMenuPopup();
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
                    onModelChanged: widget.onModelChanged,
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

  _PopupPlacement _popupPlacement(
    BuildContext context, {
    required GlobalKey targetKey,
    required bool alignEnd,
  }) {
    final mediaQuery = MediaQuery.of(context);
    final screenSize = mediaQuery.size;
    final horizontalPadding = 12.0 + mediaQuery.padding.left;
    final rightPadding = 12.0 + mediaQuery.padding.right;
    final availableWidth = screenSize.width - horizontalPadding - rightPadding;
    final width = availableWidth < 300.0 ? availableWidth : 300.0;
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

  @override
  void dispose() {
    widget.controller.removeListener(_handleInputChanged);
    _dismissModelSettingsPopup();
    _dismissInputMenuPopup();
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
                  padding: const EdgeInsets.symmetric(
                    horizontal: 12,
                    vertical: 8,
                  ),
                  child: _InputBody(
                    controller: widget.controller,
                    focusNode: widget.focusNode,
                    inputState: widget.inputState,
                    modelLabel: widget.modelLabel,
                    modelSelectorLink: _modelPopupLink,
                    modelSelectorKey: _modelPopupTargetKey,
                    settingsLink: _inputMenuPopupLink,
                    settingsKey: _inputMenuPopupTargetKey,
                    processing: processing,
                    hasDraftText: hasDraftText,
                    showCancelAction: showCancelAction,
                    showQueueAction: showQueueAction,
                    onSendMessage: widget.onSendMessage,
                    onCancelMessage: widget.onCancelMessage,
                    onAttach: widget.onAttach,
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
    required this.processing,
    required this.hasDraftText,
    required this.showCancelAction,
    required this.showQueueAction,
    required this.onSendMessage,
    required this.onCancelMessage,
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
  final bool processing;
  final bool hasDraftText;
  final bool showCancelAction;
  final bool showQueueAction;
  final VoidCallback onSendMessage;
  final VoidCallback onCancelMessage;
  final VoidCallback? onAttach;
  final VoidCallback? onSettings;
  final VoidCallback? onModelSelector;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final l10n = AppLocalizations.of(context)!;
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        _DesktopEnterSendShortcuts(
          controller: controller,
          hasDraftText: hasDraftText,
          processing: processing,
          onSendMessage: onSendMessage,
          child: TextField(
            controller: controller,
            focusNode: focusNode,
            minLines: 1,
            maxLines: 6,
            enabled: true,
            textInputAction: TextInputAction.newline,
            style: theme.textTheme.bodyMedium?.copyWith(
              fontSize: 14,
              height: 20 / 14,
            ),
            decoration: InputDecoration(
              hintText: l10n.askOperitHint,
              hintStyle: theme.textTheme.bodyMedium?.copyWith(
                color: colorScheme.onSurfaceVariant,
                fontSize: 14,
              ),
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
              if (hasDraftText && !processing) {
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
            _IconTapTarget(
              icon: Icons.add,
              color: colorScheme.onSurfaceVariant.withValues(alpha: 0.9),
              onTap: onAttach,
              size: 24,
              tooltip: l10n.addAttachment,
            ),
            const SizedBox(width: 6),
            _ActionButton(
              processing: processing,
              progress: _progressFor(inputState),
              background: _actionBackground(
                colorScheme,
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: hasDraftText,
              ),
              foreground: _actionForeground(
                colorScheme,
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: hasDraftText,
              ),
              icon: _actionIcon(
                showCancelAction: showCancelAction,
                showQueueAction: showQueueAction,
                canSend: hasDraftText,
              ),
              tooltip: showCancelAction
                  ? l10n.cancel
                  : (hasDraftText ? l10n.send : ''),
              onPressed: () {
                if (showCancelAction) {
                  onCancelMessage();
                } else if (hasDraftText) {
                  onSendMessage();
                }
              },
            ),
          ],
        ),
      ],
    );
  }
}

class _DesktopEnterSendShortcuts extends StatelessWidget {
  const _DesktopEnterSendShortcuts({
    required this.controller,
    required this.hasDraftText,
    required this.processing,
    required this.onSendMessage,
    required this.child,
  });

  final TextEditingController controller;
  final bool hasDraftText;
  final bool processing;
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
    if (hasDraftText && !processing) {
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
