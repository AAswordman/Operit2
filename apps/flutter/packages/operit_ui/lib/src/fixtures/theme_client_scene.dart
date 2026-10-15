import 'package:flutter/material.dart';

import '../model/theme_preference_defaults.dart';
import '../model/theme_preference_snapshot.dart';
import '../render/composer_chrome.dart';
import '../render/composer_surface.dart';
import '../render/desktop_sidebar_surface.dart';
import '../render/drawer_chrome.dart';
import '../render/history_chrome.dart';
import '../render/top_bar_chrome.dart';
import '../render/theme_layout_metrics.dart';
import '../render/theme_inspection_target.dart';
import '../render/bubble_surface.dart';

enum ThemeClientSceneMode {
  empty('empty'),
  conversation('conversation');

  const ThemeClientSceneMode(this.protocolValue);

  final String protocolValue;

  static ThemeClientSceneMode parse(Object? value) => switch (value) {
    'empty' => ThemeClientSceneMode.empty,
    'conversation' => ThemeClientSceneMode.conversation,
    _ => throw FormatException('Unknown client scene mode: $value'),
  };
}

/// Fixed client-shell fixture for visual comparison. The greeting, desktop
/// sidebar surface, history chrome, top-bar title, composer editor controls,
/// theme root and input surface are shared with the client. Runtime-owned
/// menu, drag/drop, swipe and message-state behavior is intentionally not
/// part of this deterministic scene, so the receipt remains non-parity until
/// the full client-shell screenshot matrix is accepted.
class ThemeClientScene extends StatelessWidget {
  const ThemeClientScene({
    super.key,
    required this.snapshot,
    this.userFontFamily,
    this.aiFontFamily,
    this.inspectEnabled = false,
    this.mode = ThemeClientSceneMode.conversation,
    this.selectedTargetId,
    this.onInspectTargetSelected = _ignoreInspectSelection,
  });

  static const String fixtureVersion = 'client-shell-v7';
  static const double sidebarWidth = 280;

  final ThemePreferenceSnapshot snapshot;
  final String? userFontFamily;
  final String? aiFontFamily;
  final bool inspectEnabled;
  final ThemeClientSceneMode mode;
  final String? selectedTargetId;
  final ValueChanged<String> onInspectTargetSelected;

  static void _ignoreInspectSelection(String _) {}

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final appearance = ThemeDrawerPalette.fromScheme(
      scheme,
      transparentSurface: snapshot.transparentSurfaceEnabled,
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        final compact = constraints.maxWidth < themeTabletBreakpoint;
        return Scaffold(
          body: compact
              ? _PreviewChatSurface(
                  snapshot: snapshot,
                  compact: true,
                  mode: mode,
                  inspectEnabled: inspectEnabled,
                  selectedTargetId: selectedTargetId,
                  onInspectTargetSelected: onInspectTargetSelected,
                  userFontFamily: userFontFamily,
                  aiFontFamily: aiFontFamily,
                )
              : Row(
                  children: <Widget>[
                    ThemeInspectTarget(
                      targetId: ThemeInspectableTargetId.sidebar,
                      label: '导航侧栏',
                      enabled: inspectEnabled,
                      selected:
                          selectedTargetId == ThemeInspectableTargetId.sidebar,
                      onSelected: onInspectTargetSelected,
                      child: SizedBox(
                        width: sidebarWidth,
                        child: ThemeDesktopSidebarSurface(
                          color: appearance.containerColor,
                          transparentSurface:
                              snapshot.transparentSurfaceEnabled,
                          child: _PreviewSidebar(appearance: appearance),
                        ),
                      ),
                    ),
                    Expanded(
                      child: _PreviewChatSurface(
                        snapshot: snapshot,
                        mode: mode,
                        inspectEnabled: inspectEnabled,
                        selectedTargetId: selectedTargetId,
                        onInspectTargetSelected: onInspectTargetSelected,
                        userFontFamily: userFontFamily,
                        aiFontFamily: aiFontFamily,
                      ),
                    ),
                  ],
                ),
          resizeToAvoidBottomInset: false,
        );
      },
    );
  }
}

class _PreviewSidebar extends StatelessWidget {
  const _PreviewSidebar({required this.appearance});

  final ThemeDrawerPalette appearance;

  /// Builds the default workspace sidebar for the shared preview scene.
  @override
  Widget build(BuildContext context) {
    final dark = Theme.of(context).brightness == Brightness.dark;
    return Column(
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.only(top: 26),
          child: ThemeSidebarInfoCard(
            brandName: 'Operit',
            appearance: appearance,
          ),
        ),
        const SizedBox(height: 12),
        Padding(
          padding: const EdgeInsetsDirectional.fromSTEB(14, 0, 12, 8),
          child: Row(
            children: <Widget>[
              Expanded(
                child: ThemeUnifiedCreateBar(
                  onCreateConversation: () {},
                  onCreateGroup: () {},
                ),
              ),
              const SizedBox(width: 8),
              ThemeDrawerToolbarIconButton(
                icon: Icons.search_rounded,
                tooltip: '搜索对话',
                appearance: appearance,
                onClick: () {},
              ),
            ],
          ),
        ),
        Expanded(
          child: ListView(
            padding: const EdgeInsets.only(bottom: 14),
            children: <Widget>[
              _PreviewWorkspaceHeader(
                label: '未绑定工作区',
                count: 2,
                appearance: appearance,
              ),
              _PreviewGroup(
                label: '未分组',
                items: const <String>['New Chat'],
                selectedIndex: 0,
                appearance: appearance,
              ),
              _PreviewGroup(
                label: '主题设计',
                items: const <String>['打招呼'],
                selectedIndex: -1,
                appearance: appearance,
              ),
              _PreviewWorkspaceHeader(
                label: '主题工作台',
                count: 6,
                appearance: appearance,
              ),
              _PreviewGroup(
                label: '开发任务',
                items: const <String>['New Chat', '测试工具调用'],
                selectedIndex: -1,
                appearance: appearance,
              ),
              _PreviewGroup(
                label: '未分组',
                items: const <String>[
                  'New Chat',
                  'Greeting',
                  'Greeting',
                  'Greeting',
                ],
                selectedIndex: -1,
                appearance: appearance,
              ),
            ],
          ),
        ),
        Padding(
          padding: const EdgeInsets.fromLTRB(14, 8, 14, 16),
          child: Row(
            children: <Widget>[
              Expanded(
                child: ThemeBottomSidebarAction(
                  icon: Icons.inventory_2_outlined,
                  label: '包管理',
                  appearance: appearance,
                  onClick: () {},
                ),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: ThemeBottomSidebarAction(
                  icon: Icons.settings_outlined,
                  label: '设置',
                  appearance: appearance,
                  onClick: () {},
                ),
              ),
              const SizedBox(width: 8),
              ThemeBottomThemeToggleButton(
                appearance: appearance,
                darkThemeActive: dark,
                onToggle: () {},
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// Displays a workspace and its complete sample conversation count.
class _PreviewWorkspaceHeader extends StatelessWidget {
  /// Creates a workspace section matching the default client navigation.
  const _PreviewWorkspaceHeader({
    required this.label,
    required this.count,
    required this.appearance,
  });

  final String label;
  final int count;
  final ThemeDrawerPalette appearance;

  /// Renders the shared workspace header without a character avatar.
  @override
  Widget build(BuildContext context) {
    return ThemeHistorySectionHeader(
      label: label,
      count: count,
      expanded: true,
      appearance: appearance,
      workspaceStyle: true,
      onToggleExpanded: () {},
    );
  }
}

class _PreviewGroup extends StatelessWidget {
  const _PreviewGroup({
    required this.label,
    required this.items,
    required this.selectedIndex,
    required this.appearance,
  });

  final String label;
  final List<String> items;
  final int selectedIndex;
  final ThemeDrawerPalette appearance;

  /// Renders grouped conversations with the default workspace spacing.
  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: <Widget>[
        ThemeHistoryGroupHeader(
          label: label,
          pinned: false,
          workspaceStyle: true,
          expanded: true,
          appearance: appearance,
          showActions: false,
          action: ThemeHistoryMoreGlyph(
            color: appearance.itemColor.withValues(alpha: 0.78),
            side: 22,
          ),
          onToggleExpanded: () {},
        ),
        for (var index = 0; index < items.length; index += 1)
          _PreviewConversationItem(
            title: items[index],
            selected: index == selectedIndex,
            appearance: appearance,
          ),
      ],
    );
  }
}

class _PreviewConversationItem extends StatelessWidget {
  const _PreviewConversationItem({
    required this.title,
    required this.selected,
    required this.appearance,
  });

  final String title;
  final bool selected;
  final ThemeDrawerPalette appearance;

  /// Builds a conversation row using the shared workspace appearance.
  @override
  Widget build(BuildContext context) {
    return ThemeConversationRow(
      title: title,
      workspaceStyle: true,
      selected: selected,
      active: false,
      appearance: appearance,
      leading: ThemeConversationStatusHandle(
        isRunning: false,
        selected: selected,
        hovered: false,
        appearance: appearance,
      ),
      showActions: selected,
      action: ThemeHistoryMoreGlyph(
        color:
            (selected ? appearance.selectedContentColor : appearance.itemColor)
                .withValues(alpha: 0.78),
        side: 24,
      ),
      onTap: () {},
      onLongPress: () {},
    );
  }
}

class _PreviewChatSurface extends StatelessWidget {
  const _PreviewChatSurface({
    required this.snapshot,
    required this.mode,
    required this.inspectEnabled,
    required this.selectedTargetId,
    required this.onInspectTargetSelected,
    required this.userFontFamily,
    required this.aiFontFamily,
    this.compact = false,
  });

  final ThemePreferenceSnapshot snapshot;
  final ThemeClientSceneMode mode;
  final bool inspectEnabled;
  final String? selectedTargetId;
  final ValueChanged<String> onInspectTargetSelected;
  final String? userFontFamily;
  final String? aiFontFamily;
  final bool compact;

  /// Displays the default application title and shared chat surface.
  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final backgroundVisible =
        snapshot.useBackgroundImage &&
        snapshot.backgroundImageUri?.isNotEmpty == true;
    final contentColor = backgroundVisible || snapshot.transparentSurfaceEnabled
        ? Colors.transparent
        : scheme.surface;
    return Column(
      children: <Widget>[
        ThemeInspectTarget(
          targetId: ThemeInspectableTargetId.topBar,
          label: '对话顶部栏',
          enabled: inspectEnabled,
          selected: selectedTargetId == ThemeInspectableTargetId.topBar,
          onSelected: onInspectTargetSelected,
          child: ThemeTopBarChrome(
            title: ThemeTopBarTitle(
              primaryText: 'Operit',
              secondaryText: 'New Chat',
              contentColor: scheme.onSurface,
            ),
            navigationIcon: compact ? Icons.segment : Icons.chevron_left,
            navigationTooltip: compact ? 'Navigation' : 'Collapse sidebar',
            onNavigationPressed: () {},
            flipNavigationIcon: compact,
            contentColor: contentColor,
            contentForeground: scheme.onSurface,
            actions: <Widget>[
              ThemeWorkspaceTopBarButton(
                open: false,
                hasBoundWorkspace: false,
                onPressed: () {},
              ),
            ],
          ),
        ),
        Expanded(
          child: Center(
            child: SingleChildScrollView(
              padding: const EdgeInsets.symmetric(horizontal: 18, vertical: 20),
              child: ConstrainedBox(
                constraints: BoxConstraints(
                  maxWidth: snapshot.bubbleWideLayoutEnabled
                      ? themeChatWideContentMaxWidth
                      : themeChatContentMaxWidth,
                ),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.stretch,
                  children: <Widget>[
                    if (mode == ThemeClientSceneMode.conversation) ...<Widget>[
                      const SizedBox(height: 24),
                      Align(
                        alignment: Alignment.centerRight,
                        child: ThemeInspectTarget(
                          targetId: ThemeInspectableTargetId.userMessage,
                          label: '用户消息气泡',
                          enabled: inspectEnabled,
                          selected:
                              selectedTargetId ==
                              ThemeInspectableTargetId.userMessage,
                          onSelected: onInspectTargetSelected,
                          child: _PreviewBubble(
                            snapshot: snapshot,
                            user: true,
                            fontFamily: userFontFamily,
                          ),
                        ),
                      ),
                      const SizedBox(height: 14),
                      Align(
                        alignment: Alignment.centerLeft,
                        child: ThemeInspectTarget(
                          targetId: ThemeInspectableTargetId.aiMessage,
                          label: 'AI 消息气泡',
                          enabled: inspectEnabled,
                          selected:
                              selectedTargetId ==
                              ThemeInspectableTargetId.aiMessage,
                          onSelected: onInspectTargetSelected,
                          child: _PreviewBubble(
                            snapshot: snapshot,
                            user: false,
                            fontFamily: aiFontFamily,
                          ),
                        ),
                      ),
                    ],
                  ],
                ),
              ),
            ),
          ),
        ),
        ThemeInspectTarget(
          targetId: ThemeInspectableTargetId.composer,
          label: '消息输入框',
          enabled: inspectEnabled,
          selected: selectedTargetId == ThemeInspectableTargetId.composer,
          onSelected: onInspectTargetSelected,
          child: _PreviewComposer(snapshot: snapshot),
        ),
      ],
    );
  }
}

class _PreviewBubble extends StatelessWidget {
  const _PreviewBubble({
    required this.snapshot,
    required this.user,
    required this.fontFamily,
  });

  final ThemePreferenceSnapshot snapshot;
  final bool user;
  final String? fontFamily;

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final bubbleStyle =
        snapshot.chatStyle == ThemePreferenceDefaults.CHAT_STYLE_BUBBLE;
    final showsAvatar = bubbleStyle && snapshot.bubbleShowAvatar;
    final contentWidth = (MediaQuery.sizeOf(context).width - 36)
        .clamp(
          0.0,
          snapshot.bubbleWideLayoutEnabled
              ? themeChatWideContentMaxWidth
              : themeChatContentMaxWidth,
        )
        .toDouble();
    final messageInsets = user
        ? (showsAvatar ? 72.0 : 40.0)
        : (showsAvatar ? 40.0 : 8.0);
    final availableBubbleWidth = contentWidth - messageInsets;
    final maximumBubbleWidth = snapshot.bubbleWideLayoutEnabled
        ? availableBubbleWidth
        : availableBubbleWidth * 0.85;
    final color = user
        ? Color(
            (bubbleStyle
                    ? snapshot.bubbleUserBubbleColor
                    : snapshot.cursorUserBubbleColor) ??
                scheme.primaryContainer.toARGB32(),
          )
        : Color(
            snapshot.bubbleAiBubbleColor ??
                scheme.surfaceContainerHighest.toARGB32(),
          );
    final textColor = user
        ? Color(
            snapshot.bubbleUserTextColor ??
                scheme.onPrimaryContainer.toARGB32(),
          )
        : Color(snapshot.bubbleAiTextColor ?? scheme.onSurface.toARGB32());
    final imagePath = user
        ? snapshot.bubbleUserImageUri
        : snapshot.bubbleAiImageUri;
    final hasImage = user
        ? snapshot.bubbleUserUseImage
        : snapshot.bubbleAiUseImage;
    final image =
        bubbleStyle && hasImage && imagePath != null && imagePath.isNotEmpty
        ? BubbleImageStyle(
            imagePath: imagePath,
            cropLeftRatio: user
                ? snapshot.bubbleUserImageCropLeft
                : snapshot.bubbleAiImageCropLeft,
            cropTopRatio: user
                ? snapshot.bubbleUserImageCropTop
                : snapshot.bubbleAiImageCropTop,
            cropRightRatio: user
                ? snapshot.bubbleUserImageCropRight
                : snapshot.bubbleAiImageCropRight,
            cropBottomRatio: user
                ? snapshot.bubbleUserImageCropBottom
                : snapshot.bubbleAiImageCropBottom,
            repeatXStartRatio: user
                ? snapshot.bubbleUserImageRepeatStart
                : snapshot.bubbleAiImageRepeatStart,
            repeatXEndRatio: user
                ? snapshot.bubbleUserImageRepeatEnd
                : snapshot.bubbleAiImageRepeatEnd,
            repeatYStartRatio: user
                ? snapshot.bubbleUserImageRepeatYStart
                : snapshot.bubbleAiImageRepeatYStart,
            repeatYEndRatio: user
                ? snapshot.bubbleUserImageRepeatYEnd
                : snapshot.bubbleAiImageRepeatYEnd,
            imageScale: user
                ? snapshot.bubbleUserImageScale
                : snapshot.bubbleAiImageScale,
            renderMode: user
                ? snapshot.bubbleUserImageRenderMode
                : snapshot.bubbleAiImageRenderMode,
          )
        : null;
    final padding = bubbleStyle
        ? EdgeInsets.fromLTRB(
            user
                ? snapshot.bubbleUserContentPaddingLeft
                : snapshot.bubbleAiContentPaddingLeft,
            12,
            user
                ? snapshot.bubbleUserContentPaddingRight
                : snapshot.bubbleAiContentPaddingRight,
            12,
          )
        : user
        ? const EdgeInsets.all(16)
        : EdgeInsets.zero;
    final content = Padding(
      padding: bubbleStyle || user ? padding : const EdgeInsets.all(0),
      child: Text(
        user ? '帮我把主题工作台调整得更简洁一些。' : '可以。选择预览里的消息气泡，我会显示它可编辑的全部主题参数。',
        style: TextStyle(
          color: textColor,
          fontFamily: bubbleStyle ? fontFamily : null,
        ),
      ),
    );
    if (!user && !bubbleStyle) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 4),
        child: content,
      );
    }
    final roundedCornersEnabled = user
        ? snapshot.bubbleUserRoundedCornersEnabled
        : snapshot.bubbleAiRoundedCornersEnabled;
    final borderRadius = !roundedCornersEnabled
        ? bubbleStyle
              ? BorderRadius.zero
              : BorderRadius.circular(4)
        : BorderRadius.circular(user && !bubbleStyle ? 12 : 16);
    final bubble = ConstrainedBox(
      constraints: BoxConstraints(maxWidth: maximumBubbleWidth),
      child: BubbleSurface(
        color: color,
        borderRadius: borderRadius,
        imageStyle: image,
        transparentSurface: snapshot.transparentSurfaceEnabled,
        child: content,
      ),
    );
    if (!showsAvatar) return bubble;
    final avatar = ClipRRect(
      borderRadius:
          snapshot.avatarShape == ThemePreferenceDefaults.AVATAR_SHAPE_CIRCLE
          ? const BorderRadius.all(Radius.circular(16))
          : BorderRadius.circular(snapshot.avatarCornerRadius),
      child: ColoredBox(
        color: user ? scheme.primary : scheme.tertiaryContainer,
        child: SizedBox.square(
          dimension: 32,
          child: Center(
            child: Icon(
              user ? Icons.person : Icons.assistant,
              size: 22,
              color: user ? scheme.onPrimary : scheme.onTertiaryContainer,
            ),
          ),
        ),
      ),
    );
    return Row(
      mainAxisSize: MainAxisSize.min,
      children: user
          ? <Widget>[bubble, const SizedBox(width: 8), avatar]
          : <Widget>[avatar, const SizedBox(width: 8), bubble],
    );
  }
}

class _PreviewComposer extends StatefulWidget {
  const _PreviewComposer({required this.snapshot});
  final ThemePreferenceSnapshot snapshot;
  @override
  State<_PreviewComposer> createState() => _PreviewComposerState();
}

class _PreviewComposerState extends State<_PreviewComposer> {
  final TextEditingController _controller = TextEditingController();
  final FocusNode _focusNode = FocusNode();
  int _previewActionState = 0;

  @override
  void dispose() {
    _controller.dispose();
    _focusNode.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final snapshot = widget.snapshot;
    final scheme = Theme.of(context).colorScheme;
    final body = switch (snapshot.inputStyle) {
      ThemePreferenceDefaults.INPUT_STYLE_AGENT => ThemeAgentComposerBody(
        editor: ThemeComposerTextEditor(
          controller: _controller,
          focusNode: _focusNode,
          hintText: '向 Operit 提问',
          inputExpanded: false,
          readOnly: true,
          onSubmitted: (_) {},
          onToggleExpansion: () {},
          expandTooltip: '展开输入',
          collapseTooltip: '收起输入',
        ),
        modelSelector: ThemeComposerModelSelector(
          identity: ThemeComposerModelIdentity(
            label: 'gpt-6-luna',
            style: Theme.of(context).textTheme.bodyMedium!.copyWith(
              color: scheme.onSurface,
              height: 1,
              leadingDistribution: TextLeadingDistribution.even,
            ),
            chevronColor: scheme.onSurfaceVariant,
            expanded: false,
            logoBuilder: (size) =>
                ThemeComposerProviderInitial(initial: 'G', size: size),
          ),
          expanded: false,
          onTap: () {},
        ),
        settings: ThemeComposerIconTarget(
          icon: Icons.tune_outlined,
          color: scheme.onSurfaceVariant,
          onTap: () {},
          tooltip: '设置',
          imageAsset: snapshot.inputSettingsAsset,
          imageSize: snapshot.inputSettingsAssetSize,
          targetSize: 34,
        ),
        attachment: ThemeComposerIconTarget(
          icon: Icons.add,
          color: scheme.onSurfaceVariant.withValues(alpha: 0.9),
          onTap: () {},
          tooltip: '添加附件',
          imageAsset: snapshot.inputAttachmentAsset,
          imageSize: snapshot.inputAttachmentAssetSize,
          size: 24,
        ),
        action: ThemeComposerActionButton(
          processing: _previewActionState == 2,
          progress: _previewActionState == 2 ? 0.35 : null,
          background: _previewActionState == 2 ? scheme.error : scheme.primary,
          foreground: _previewActionState == 2
              ? scheme.onError
              : scheme.onPrimary,
          icon: _previewActionState == 2
              ? Icons.close
              : _previewActionState == 1
              ? Icons.arrow_upward
              : Icons.mic,
          tooltip: _previewActionState == 2
              ? '停止（点击切换状态预览）'
              : _previewActionState == 1
              ? '发送（点击切换状态预览）'
              : '语音输入（点击切换状态预览）',
          imageAsset: resolveThemeInputActionAsset(
            snapshot,
            isStop: _previewActionState == 2,
            isSend: _previewActionState == 1,
          ),
          imageSize: resolveThemeInputActionAssetSize(
            snapshot,
            isStop: _previewActionState == 2,
            isSend: _previewActionState == 1,
          ),
          onPressed: () => setState(
            () => _previewActionState = (_previewActionState + 1) % 3,
          ),
        ),
      ),
      ThemePreferenceDefaults.INPUT_STYLE_CLASSIC => ThemeClassicComposerBody(
        inputExpanded: false,
        editor: ThemeClassicComposerTextEditor(
          controller: _controller,
          focusNode: _focusNode,
          hintText: '向 Operit 提问',
          inputExpanded: false,
          readOnly: true,
          hasDraftText: false,
          settings: ThemeComposerIconTarget(
            icon: Icons.tune_outlined,
            color: scheme.onSurfaceVariant,
            onTap: () {},
            tooltip: '设置',
            imageAsset: snapshot.inputSettingsAsset,
            imageSize: snapshot.inputSettingsAssetSize,
            size: 18,
            targetSize: 34,
          ),
          onSubmitted: (_) {},
          onToggleExpansion: () {},
          expandTooltip: '展开输入',
          collapseTooltip: '收起输入',
        ),
        attachment: Material(
          color: (snapshot.inputAttachmentAsset?.isNotEmpty ?? false)
              ? Colors.transparent
              : scheme.surfaceContainerHighest,
          shape: (snapshot.inputAttachmentAsset?.isNotEmpty ?? false)
              ? const RoundedRectangleBorder()
              : const CircleBorder(),
          clipBehavior: Clip.antiAlias,
          child: ThemeComposerIconTarget(
            icon: Icons.add,
            color: scheme.onSurfaceVariant.withValues(alpha: 0.9),
            onTap: () {},
            tooltip: '添加附件',
            imageAsset: snapshot.inputAttachmentAsset,
            imageSize: snapshot.inputAttachmentAssetSize,
            targetSize: 34,
          ),
        ),
        action: ThemeComposerActionButton(
          processing: _previewActionState == 2,
          progress: _previewActionState == 2 ? 0.35 : null,
          background: _previewActionState == 2 ? scheme.error : scheme.primary,
          foreground: _previewActionState == 2
              ? scheme.onError
              : scheme.onPrimary,
          icon: _previewActionState == 2
              ? Icons.close
              : _previewActionState == 1
              ? Icons.arrow_upward
              : Icons.mic,
          tooltip: _previewActionState == 2
              ? '停止（点击切换状态预览）'
              : _previewActionState == 1
              ? '发送（点击切换状态预览）'
              : '语音输入（点击切换状态预览）',
          imageAsset: resolveThemeInputActionAsset(
            snapshot,
            isStop: _previewActionState == 2,
            isSend: _previewActionState == 1,
          ),
          imageSize: resolveThemeInputActionAssetSize(
            snapshot,
            isStop: _previewActionState == 2,
            isSend: _previewActionState == 1,
          ),
          onPressed: () => setState(
            () => _previewActionState = (_previewActionState + 1) % 3,
          ),
          buttonSize: 36,
          contentSize: 32,
          iconSize: 17,
        ),
      ),
      _ => throw FormatException(
        'Unsupported theme input style: ${snapshot.inputStyle}',
      ),
    };
    return Align(
      alignment: Alignment.bottomCenter,
      child: ConstrainedBox(
        constraints: BoxConstraints(
          maxWidth: snapshot.bubbleWideLayoutEnabled
              ? themeChatWideContentMaxWidth
              : themeChatContentMaxWidth,
        ),
        child: ThemeComposerCard(
          inputStyle: snapshot.inputStyle,
          floating: snapshot.chatInputFloating,
          transparentSurface: snapshot.transparentSurfaceEnabled,
          backgroundColor: snapshot.inputBackgroundColor != null
              ? Color(snapshot.inputBackgroundColor!)
              : null,
          roundedCorners: snapshot.inputRoundedCornersEnabled,
          imageStyle: buildThemeInputImageStyle(snapshot),
          child: body,
        ),
      ),
    );
  }
}
