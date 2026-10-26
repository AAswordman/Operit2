// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../common/markdown/StreamMarkdownRenderer.dart';
import '../../../../common/ChatMessageIdentity.dart';
import '../../../../common/ChatResponsePresentation.dart';
import '../../viewmodel/ChatViewModel.dart';
import '../../../../../core/proxy/generated/CoreProxyModels.g.dart'
    as core_proxy;
import 'bubble/BubbleStyleChatMessage.dart';
import 'bubble/BubbleSurface.dart';
import 'cursor/CursorStyleChatMessage.dart';

/// Builds the same message widget for the transcript and appearance preview.
///
/// Theme values are resolved by the caller so cached transcript rows do not
/// gain an extra inherited-theme dependency or rebuild on unrelated updates.
Widget buildThemedChatMessage({
  Key? key,
  required ThemePreferenceSnapshot snapshot,
  required ColorScheme colorScheme,
  required ChatUiMessage message,
  VoidCallback? onIdentityTap,
  MarkdownContentSplitter? splitMarkdownContent,
  Future<void> Function(int timestamp)? onDeleteMessage,
  ValueChanged<ChatUiMessage>? onEditSummary,
  bool enableDialogs = true,
}) {
  final presentation = readChatResponsePresentation(message);
  if (presentation != null) {
    final structural = message.parts
        .where((part) => part.kind != core_proxy.MessagePartKind.markdown)
        .toList(growable: false);
    final groups = <({String key, List<core_proxy.MessagePart> parts})>[
      if (structural.isNotEmpty) (key: 'response-structure', parts: structural),
      for (var index = 0; index < presentation.bubbles.length; index++)
        (key: 'response-bubble-$index', parts: presentation.bubbles[index]),
    ];
    return Column(
      key: key,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var index = 0; index < groups.length; index++)
          Padding(
            key: ValueKey(groups[index].key),
            padding: EdgeInsets.only(top: index == 0 ? 0 : 8),
            child: buildThemedChatMessage(
              snapshot: snapshot,
              colorScheme: colorScheme,
              message: responseBubbleMessage(
                message,
                groups[index].parts,
                includeMetrics: index == groups.length - 1,
              ),
              onIdentityTap: onIdentityTap,
              splitMarkdownContent: splitMarkdownContent,
              onDeleteMessage: onDeleteMessage,
              onEditSummary: onEditSummary,
              enableDialogs: enableDialogs,
            ),
          ),
      ],
    );
  }
  final identityAvatarUri = readChatMessageIdentity(message)?.avatarUri;
  if (snapshot.chatStyle != UserPreferencesManager.CHAT_STYLE_BUBBLE) {
    return CursorStyleChatMessage(
      key: key,
      message: message,
      identityAvatarUri: identityAvatarUri,
      onIdentityTap: onIdentityTap,
      splitMarkdownContent: splitMarkdownContent,
      onDeleteMessage: onDeleteMessage,
      onEditSummary: onEditSummary,
      enableDialogs: enableDialogs,
    );
  }
  final colors = resolveChatMessageThemeColors(snapshot, colorScheme);
  return BubbleStyleChatMessage(
    key: key,
    message: message,
    userMessageColor: colors.userMessageColor,
    aiMessageColor: colors.aiMessageColor,
    userTextColor: colors.userTextColor,
    aiTextColor: colors.aiTextColor,
    systemMessageColor: colors.systemMessageColor,
    systemTextColor: colors.systemTextColor,
    transparentSurface: snapshot.transparentSurfaceEnabled,
    userBubbleImageStyle: _userBubbleImageStyle(snapshot),
    aiBubbleImageStyle: _aiBubbleImageStyle(snapshot),
    bubbleUserRoundedCornersEnabled: snapshot.bubbleUserRoundedCornersEnabled,
    bubbleAiRoundedCornersEnabled: snapshot.bubbleAiRoundedCornersEnabled,
    bubbleUserContentPaddingLeft: snapshot.bubbleUserContentPaddingLeft,
    bubbleUserContentPaddingRight: snapshot.bubbleUserContentPaddingRight,
    bubbleAiContentPaddingLeft: snapshot.bubbleAiContentPaddingLeft,
    bubbleAiContentPaddingRight: snapshot.bubbleAiContentPaddingRight,
    identityAvatarUri: identityAvatarUri,
    onIdentityTap: onIdentityTap,
    splitMarkdownContent: splitMarkdownContent,
    onDeleteMessage: onDeleteMessage,
    onEditSummary: onEditSummary,
    enableDialogs: enableDialogs,
  );
}

/// Captures concrete theme-dependent colors used by one cached message row.
typedef ChatMessageThemeColors = ({
  Color userMessageColor,
  Color aiMessageColor,
  Color userTextColor,
  Color aiTextColor,
  Color systemMessageColor,
  Color systemTextColor,
});

/// Resolves concrete colors used by one cached message row.
ChatMessageThemeColors resolveChatMessageThemeColors(
  ThemePreferenceSnapshot snapshot,
  ColorScheme colorScheme,
) {
  return (
    userMessageColor:
        _optionalColor(snapshot.bubbleUserBubbleColor) ??
        colorScheme.primaryContainer,
    aiMessageColor:
        _optionalColor(snapshot.bubbleAiBubbleColor) ??
        colorScheme.surfaceContainerHighest,
    userTextColor:
        _optionalColor(snapshot.bubbleUserTextColor) ??
        colorScheme.onPrimaryContainer,
    aiTextColor:
        _optionalColor(snapshot.bubbleAiTextColor) ?? colorScheme.onSurface,
    systemMessageColor: colorScheme.surfaceContainerHighest,
    systemTextColor: colorScheme.onSurfaceVariant,
  );
}

/// Builds user bubble image settings from the active theme snapshot.
BubbleImageStyle? _userBubbleImageStyle(ThemePreferenceSnapshot snapshot) {
  final imagePath = snapshot.bubbleUserImageUri;
  if (!snapshot.bubbleUserUseImage || imagePath == null || imagePath.isEmpty) {
    return null;
  }
  return BubbleImageStyle(
    imagePath: imagePath,
    cropLeftRatio: snapshot.bubbleUserImageCropLeft,
    cropTopRatio: snapshot.bubbleUserImageCropTop,
    cropRightRatio: snapshot.bubbleUserImageCropRight,
    cropBottomRatio: snapshot.bubbleUserImageCropBottom,
    repeatXStartRatio: snapshot.bubbleUserImageRepeatStart,
    repeatXEndRatio: snapshot.bubbleUserImageRepeatEnd,
    repeatYStartRatio: snapshot.bubbleUserImageRepeatYStart,
    repeatYEndRatio: snapshot.bubbleUserImageRepeatYEnd,
    imageScale: snapshot.bubbleUserImageScale,
    renderMode: snapshot.bubbleUserImageRenderMode,
  );
}

/// Builds AI bubble image settings from the active theme snapshot.
BubbleImageStyle? _aiBubbleImageStyle(ThemePreferenceSnapshot snapshot) {
  final imagePath = snapshot.bubbleAiImageUri;
  if (!snapshot.bubbleAiUseImage || imagePath == null || imagePath.isEmpty) {
    return null;
  }
  return BubbleImageStyle(
    imagePath: imagePath,
    cropLeftRatio: snapshot.bubbleAiImageCropLeft,
    cropTopRatio: snapshot.bubbleAiImageCropTop,
    cropRightRatio: snapshot.bubbleAiImageCropRight,
    cropBottomRatio: snapshot.bubbleAiImageCropBottom,
    repeatXStartRatio: snapshot.bubbleAiImageRepeatStart,
    repeatXEndRatio: snapshot.bubbleAiImageRepeatEnd,
    repeatYStartRatio: snapshot.bubbleAiImageRepeatYStart,
    repeatYEndRatio: snapshot.bubbleAiImageRepeatYEnd,
    imageScale: snapshot.bubbleAiImageScale,
    renderMode: snapshot.bubbleAiImageRenderMode,
  );
}

/// Converts a stored ARGB color value into a Flutter color.
Color? _optionalColor(int? value) {
  return value == null ? null : Color(value);
}
