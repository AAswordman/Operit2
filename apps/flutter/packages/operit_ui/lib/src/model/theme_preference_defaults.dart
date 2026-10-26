// ignore_for_file: constant_identifier_names

import 'package:flutter/material.dart';

import 'theme_preference_snapshot.dart';

/// Internal effective-state constants; not the public theme package contract.
class ThemePreferenceDefaults {
  static const String MEDIA_TYPE_IMAGE = 'image';
  static const String MEDIA_TYPE_VIDEO = 'video';
  static const String CHAT_STYLE_CURSOR = 'cursor';
  static const String CHAT_STYLE_BUBBLE = 'bubble';
  static const String INPUT_STYLE_CLASSIC = 'classic';
  static const String INPUT_STYLE_AGENT = 'agent';
  static const String BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE =
      'tiled_nine_slice';
  static const String BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH = 'nine_patch';
  static const String AVATAR_SHAPE_CIRCLE = 'circle';
  static const String AVATAR_SHAPE_SQUARE = 'square';
  static const String FONT_TYPE_SYSTEM = 'system';
  static const String FONT_TYPE_FILE = 'file';
  static const String SYSTEM_FONT_DEFAULT = 'default';
  static const String SYSTEM_FONT_SERIF = 'serif';
  static const String SYSTEM_FONT_SANS_SERIF = 'sans-serif';
  static const String SYSTEM_FONT_MONOSPACE = 'monospace';
  static const String SYSTEM_FONT_CURSIVE = 'cursive';
  static const ThemePreferenceSnapshot snapshot = ThemePreferenceSnapshot(
    themeMode: ThemeMode.system,
    useCustomColors: false,
    customPrimaryColor: null,
    customSecondaryColor: null,
    useBackgroundImage: false,
    backgroundImageUri: null,
    backgroundMediaType: MEDIA_TYPE_IMAGE,
    backgroundImageOpacity: 0.3,
    videoBackgroundMuted: true,
    videoBackgroundLoop: true,
    useBackgroundBlur: false,
    backgroundBlurRadius: 12,
    transparentSurfaceEnabled: false,
    chatInputFloating: false,
    inputStyle: INPUT_STYLE_AGENT,
    inputBackgroundColor: null,
    inputRoundedCornersEnabled: true,
    inputUseImage: false,
    inputImageUri: null,
    inputImageRenderMode: BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
    inputImageCropLeft: 0,
    inputImageCropTop: 0,
    inputImageCropRight: 0,
    inputImageCropBottom: 0,
    inputImageRepeatStart: 0.35,
    inputImageRepeatEnd: 0.65,
    inputImageRepeatYStart: 0.35,
    inputImageRepeatYEnd: 0.65,
    inputImageScale: 1,
    inputSettingsAsset: null,
    inputSettingsAssetSize: 18,
    inputAttachmentAsset: null,
    inputAttachmentAssetSize: 18,
    inputMicAsset: null,
    inputMicAssetSize: 26,
    inputSendAsset: null,
    inputSendAssetSize: 26,
    inputStopAsset: null,
    inputStopAssetSize: 26,
    chatStyle: CHAT_STYLE_CURSOR,
    bubbleShowAvatar: true,
    bubbleWideLayoutEnabled: false,
    cursorUserBubbleColor: null,
    bubbleUserBubbleColor: null,
    bubbleAiBubbleColor: null,
    bubbleUserTextColor: null,
    bubbleAiTextColor: null,
    bubbleUserUseImage: false,
    bubbleAiUseImage: false,
    bubbleUserImageUri: null,
    bubbleAiImageUri: null,
    bubbleUserImageRenderMode: BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
    bubbleAiImageRenderMode: BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
    bubbleUserImageCropLeft: 0,
    bubbleUserImageCropTop: 0,
    bubbleUserImageCropRight: 0,
    bubbleUserImageCropBottom: 0,
    bubbleUserImageRepeatStart: 0.35,
    bubbleUserImageRepeatEnd: 0.65,
    bubbleUserImageRepeatYStart: 0.35,
    bubbleUserImageRepeatYEnd: 0.65,
    bubbleUserImageScale: 1,
    bubbleAiImageCropLeft: 0,
    bubbleAiImageCropTop: 0,
    bubbleAiImageCropRight: 0,
    bubbleAiImageCropBottom: 0,
    bubbleAiImageRepeatStart: 0.35,
    bubbleAiImageRepeatEnd: 0.65,
    bubbleAiImageRepeatYStart: 0.35,
    bubbleAiImageRepeatYEnd: 0.65,
    bubbleAiImageScale: 1,
    bubbleUserRoundedCornersEnabled: true,
    bubbleAiRoundedCornersEnabled: true,
    bubbleUserContentPaddingLeft: 12,
    bubbleUserContentPaddingRight: 12,
    bubbleAiContentPaddingLeft: 12,
    bubbleAiContentPaddingRight: 12,
    customUserAvatarUri: null,
    avatarShape: AVATAR_SHAPE_CIRCLE,
    avatarCornerRadius: 8,
    useCustomFont: false,
    fontType: FONT_TYPE_SYSTEM,
    systemFontName: null,
    customFontPath: null,
    fontScale: 1,
    bubbleUserUseCustomFont: false,
    bubbleUserFontType: FONT_TYPE_SYSTEM,
    bubbleUserSystemFontName: SYSTEM_FONT_DEFAULT,
    bubbleUserCustomFontPath: null,
    bubbleAiUseCustomFont: false,
    bubbleAiFontType: FONT_TYPE_SYSTEM,
    bubbleAiSystemFontName: SYSTEM_FONT_DEFAULT,
    bubbleAiCustomFontPath: null,
    showThinkingProcess: true,
    showModelProvider: false,
    showModelName: false,
    showRoleName: true,
    showUserName: true,
    showMessageTokenStats: false,
    showMessageTimingStats: false,
    showMessageTimestamp: false,
    showInputProcessingStatus: true,
  );
}
