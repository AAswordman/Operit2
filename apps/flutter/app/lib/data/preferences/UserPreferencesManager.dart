// ignore_for_file: file_names, constant_identifier_names

import 'dart:convert';

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

class ThemePreferenceSnapshot {
  const ThemePreferenceSnapshot({
    required this.themeMode,
    required this.useSystemTheme,
    required this.useCustomColors,
    required this.customPrimaryColor,
    required this.customSecondaryColor,
    required this.useBackgroundImage,
    required this.backgroundImageUri,
    required this.backgroundMediaType,
    required this.backgroundImageOpacity,
    required this.videoBackgroundMuted,
    required this.videoBackgroundLoop,
    required this.useBackgroundBlur,
    required this.backgroundBlurRadius,
    required this.transparentSurfaceEnabled,
    required this.chatInputFloating,
    required this.chatStyle,
    required this.bubbleShowAvatar,
    required this.bubbleWideLayoutEnabled,
    required this.cursorUserBubbleColor,
    required this.bubbleUserBubbleColor,
    required this.bubbleAiBubbleColor,
    required this.bubbleUserTextColor,
    required this.bubbleAiTextColor,
    required this.bubbleUserUseImage,
    required this.bubbleAiUseImage,
    required this.bubbleUserImageUri,
    required this.bubbleAiImageUri,
    required this.bubbleUserImageRenderMode,
    required this.bubbleAiImageRenderMode,
    required this.bubbleUserImageCropLeft,
    required this.bubbleUserImageCropTop,
    required this.bubbleUserImageCropRight,
    required this.bubbleUserImageCropBottom,
    required this.bubbleUserImageRepeatStart,
    required this.bubbleUserImageRepeatEnd,
    required this.bubbleUserImageRepeatYStart,
    required this.bubbleUserImageRepeatYEnd,
    required this.bubbleUserImageScale,
    required this.bubbleAiImageCropLeft,
    required this.bubbleAiImageCropTop,
    required this.bubbleAiImageCropRight,
    required this.bubbleAiImageCropBottom,
    required this.bubbleAiImageRepeatStart,
    required this.bubbleAiImageRepeatEnd,
    required this.bubbleAiImageRepeatYStart,
    required this.bubbleAiImageRepeatYEnd,
    required this.bubbleAiImageScale,
    required this.bubbleUserRoundedCornersEnabled,
    required this.bubbleAiRoundedCornersEnabled,
    required this.bubbleUserContentPaddingLeft,
    required this.bubbleUserContentPaddingRight,
    required this.bubbleAiContentPaddingLeft,
    required this.bubbleAiContentPaddingRight,
    required this.customUserAvatarUri,
    required this.customAiAvatarUri,
    required this.avatarShape,
    required this.avatarCornerRadius,
    required this.useCustomFont,
    required this.fontType,
    required this.systemFontName,
    required this.customFontPath,
    required this.fontScale,
    required this.bubbleUserUseCustomFont,
    required this.bubbleUserFontType,
    required this.bubbleUserSystemFontName,
    required this.bubbleUserCustomFontPath,
    required this.bubbleAiUseCustomFont,
    required this.bubbleAiFontType,
    required this.bubbleAiSystemFontName,
    required this.bubbleAiCustomFontPath,
    required this.showThinkingProcess,
    required this.showModelProvider,
    required this.showModelName,
    required this.showRoleName,
    required this.showUserName,
    required this.showMessageTokenStats,
    required this.showMessageTimingStats,
    required this.showMessageTimestamp,
    required this.showInputProcessingStatus,
  });

  final String themeMode;
  final bool useSystemTheme;
  final bool useCustomColors;
  final int? customPrimaryColor;
  final int? customSecondaryColor;
  final bool useBackgroundImage;
  final String? backgroundImageUri;
  final String backgroundMediaType;
  final double backgroundImageOpacity;
  final bool videoBackgroundMuted;
  final bool videoBackgroundLoop;
  final bool useBackgroundBlur;
  final double backgroundBlurRadius;
  final bool transparentSurfaceEnabled;
  final bool chatInputFloating;
  final String chatStyle;
  final bool bubbleShowAvatar;
  final bool bubbleWideLayoutEnabled;
  final int? cursorUserBubbleColor;
  final int? bubbleUserBubbleColor;
  final int? bubbleAiBubbleColor;
  final int? bubbleUserTextColor;
  final int? bubbleAiTextColor;
  final bool bubbleUserUseImage;
  final bool bubbleAiUseImage;
  final String? bubbleUserImageUri;
  final String? bubbleAiImageUri;
  final String bubbleUserImageRenderMode;
  final String bubbleAiImageRenderMode;
  final double bubbleUserImageCropLeft;
  final double bubbleUserImageCropTop;
  final double bubbleUserImageCropRight;
  final double bubbleUserImageCropBottom;
  final double bubbleUserImageRepeatStart;
  final double bubbleUserImageRepeatEnd;
  final double bubbleUserImageRepeatYStart;
  final double bubbleUserImageRepeatYEnd;
  final double bubbleUserImageScale;
  final double bubbleAiImageCropLeft;
  final double bubbleAiImageCropTop;
  final double bubbleAiImageCropRight;
  final double bubbleAiImageCropBottom;
  final double bubbleAiImageRepeatStart;
  final double bubbleAiImageRepeatEnd;
  final double bubbleAiImageRepeatYStart;
  final double bubbleAiImageRepeatYEnd;
  final double bubbleAiImageScale;
  final bool bubbleUserRoundedCornersEnabled;
  final bool bubbleAiRoundedCornersEnabled;
  final double bubbleUserContentPaddingLeft;
  final double bubbleUserContentPaddingRight;
  final double bubbleAiContentPaddingLeft;
  final double bubbleAiContentPaddingRight;
  final String? customUserAvatarUri;
  final String? customAiAvatarUri;
  final String avatarShape;
  final double avatarCornerRadius;
  final bool useCustomFont;
  final String fontType;
  final String? systemFontName;
  final String? customFontPath;
  final double fontScale;
  final bool bubbleUserUseCustomFont;
  final String bubbleUserFontType;
  final String? bubbleUserSystemFontName;
  final String? bubbleUserCustomFontPath;
  final bool bubbleAiUseCustomFont;
  final String bubbleAiFontType;
  final String? bubbleAiSystemFontName;
  final String? bubbleAiCustomFontPath;
  final bool showThinkingProcess;
  final bool showModelProvider;
  final bool showModelName;
  final bool showRoleName;
  final bool showUserName;
  final bool showMessageTokenStats;
  final bool showMessageTimingStats;
  final bool showMessageTimestamp;
  final bool showInputProcessingStatus;

  ThemeMode get flutterThemeMode {
    if (useSystemTheme) {
      return ThemeMode.system;
    }
    return switch (themeMode) {
      UserPreferencesManager.THEME_MODE_LIGHT => ThemeMode.light,
      UserPreferencesManager.THEME_MODE_DARK => ThemeMode.dark,
      _ => throw FormatException('invalid theme mode preference: $themeMode'),
    };
  }
}

class UserPreferencesManager {
  const UserPreferencesManager({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  final GeneratedCoreProxyClients _clients;

  static const String DEFAULT_PROFILE_ID = 'default';
  static const String THEME_MODE_LIGHT = 'light';
  static const String THEME_MODE_DARK = 'dark';
  static const String MEDIA_TYPE_IMAGE = 'image';
  static const String MEDIA_TYPE_VIDEO = 'video';
  static const String CHAT_STYLE_CURSOR = 'cursor';
  static const String CHAT_STYLE_BUBBLE = 'bubble';
  static const String BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE =
      'tiled_nine_slice';
  static const String BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH = 'nine_patch';
  static const String AVATAR_SHAPE_CIRCLE = 'circle';
  static const String AVATAR_SHAPE_SQUARE = 'square';
  static const String ON_COLOR_MODE_AUTO = 'auto';
  static const String ON_COLOR_MODE_LIGHT = 'light';
  static const String ON_COLOR_MODE_DARK = 'dark';
  static const String FONT_TYPE_SYSTEM = 'system';
  static const String FONT_TYPE_FILE = 'file';
  static const String SYSTEM_FONT_DEFAULT = 'default';
  static const String SYSTEM_FONT_SERIF = 'serif';
  static const String SYSTEM_FONT_SANS_SERIF = 'sans-serif';
  static const String SYSTEM_FONT_MONOSPACE = 'monospace';
  static const String SYSTEM_FONT_CURSIVE = 'cursive';

  static const String _fileName = 'user_preferences.preferences.json';
  static const String _ACTIVE_PROFILE_ID = 'active_profile_id';
  static const String _PROFILE_LIST = 'profile_list';
  static const String _BIRTH_DATE_LOCKED = 'birth_date_locked';
  static const String _GENDER_LOCKED = 'gender_locked';
  static const String _PERSONALITY_LOCKED = 'personality_locked';
  static const String _IDENTITY_LOCKED = 'identity_locked';
  static const String _OCCUPATION_LOCKED = 'occupation_locked';
  static const String _AI_STYLE_LOCKED = 'ai_style_locked';

  static const ThemePreferenceSnapshot defaultThemePreferenceSnapshot =
      ThemePreferenceSnapshot(
        themeMode: THEME_MODE_LIGHT,
        useSystemTheme: true,
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
        customAiAvatarUri: null,
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

  static const String _THEME_MODE = 'theme_mode';
  static const String _USE_SYSTEM_THEME = 'use_system_theme';
  static const String _CUSTOM_PRIMARY_COLOR = 'custom_primary_color';
  static const String _CUSTOM_SECONDARY_COLOR = 'custom_secondary_color';
  static const String _USE_CUSTOM_COLORS = 'use_custom_colors';
  static const String _USE_BACKGROUND_IMAGE = 'use_background_image';
  static const String _BACKGROUND_IMAGE_URI = 'background_image_uri';
  static const String _BACKGROUND_IMAGE_OPACITY = 'background_image_opacity';
  static const String _BACKGROUND_MEDIA_TYPE = 'background_media_type';
  static const String _VIDEO_BACKGROUND_MUTED = 'video_background_muted';
  static const String _VIDEO_BACKGROUND_LOOP = 'video_background_loop';
  static const String _TRANSPARENT_SURFACE_ENABLED =
      'transparent_surface_enabled';
  static const String _CHAT_INPUT_FLOATING = 'chat_input_floating';
  static const String _USE_BACKGROUND_BLUR = 'use_background_blur';
  static const String _BACKGROUND_BLUR_RADIUS = 'background_blur_radius';
  static const String _USE_CUSTOM_FONT = 'use_custom_font';
  static const String _FONT_TYPE = 'font_type';
  static const String _SYSTEM_FONT_NAME = 'system_font_name';
  static const String _CUSTOM_FONT_PATH = 'custom_font_path';
  static const String _FONT_SCALE = 'font_scale';
  static const String _CHAT_STYLE = 'chat_style';
  static const String _BUBBLE_SHOW_AVATAR = 'bubble_show_avatar';
  static const String _BUBBLE_WIDE_LAYOUT_ENABLED =
      'bubble_wide_layout_enabled';
  static const String _CURSOR_USER_BUBBLE_COLOR = 'cursor_user_bubble_color';
  static const String _BUBBLE_USER_BUBBLE_COLOR = 'bubble_user_bubble_color';
  static const String _BUBBLE_AI_BUBBLE_COLOR = 'bubble_ai_bubble_color';
  static const String _BUBBLE_USER_TEXT_COLOR = 'bubble_user_text_color';
  static const String _BUBBLE_AI_TEXT_COLOR = 'bubble_ai_text_color';
  static const String _BUBBLE_USER_USE_CUSTOM_FONT =
      'bubble_user_use_custom_font';
  static const String _BUBBLE_USER_FONT_TYPE = 'bubble_user_font_type';
  static const String _BUBBLE_USER_SYSTEM_FONT_NAME =
      'bubble_user_system_font_name';
  static const String _BUBBLE_USER_CUSTOM_FONT_PATH =
      'bubble_user_custom_font_path';
  static const String _BUBBLE_AI_USE_CUSTOM_FONT = 'bubble_ai_use_custom_font';
  static const String _BUBBLE_AI_FONT_TYPE = 'bubble_ai_font_type';
  static const String _BUBBLE_AI_SYSTEM_FONT_NAME =
      'bubble_ai_system_font_name';
  static const String _BUBBLE_AI_CUSTOM_FONT_PATH =
      'bubble_ai_custom_font_path';
  static const String _BUBBLE_USER_USE_IMAGE = 'bubble_user_use_image';
  static const String _BUBBLE_AI_USE_IMAGE = 'bubble_ai_use_image';
  static const String _BUBBLE_USER_IMAGE_URI = 'bubble_user_image_uri';
  static const String _BUBBLE_AI_IMAGE_URI = 'bubble_ai_image_uri';
  static const String _BUBBLE_USER_IMAGE_CROP_LEFT =
      'bubble_user_image_crop_left';
  static const String _BUBBLE_USER_IMAGE_CROP_TOP =
      'bubble_user_image_crop_top';
  static const String _BUBBLE_USER_IMAGE_CROP_RIGHT =
      'bubble_user_image_crop_right';
  static const String _BUBBLE_USER_IMAGE_CROP_BOTTOM =
      'bubble_user_image_crop_bottom';
  static const String _BUBBLE_USER_IMAGE_REPEAT_START =
      'bubble_user_image_repeat_start';
  static const String _BUBBLE_USER_IMAGE_REPEAT_END =
      'bubble_user_image_repeat_end';
  static const String _BUBBLE_USER_IMAGE_REPEAT_Y_START =
      'bubble_user_image_repeat_y_start';
  static const String _BUBBLE_USER_IMAGE_REPEAT_Y_END =
      'bubble_user_image_repeat_y_end';
  static const String _BUBBLE_USER_IMAGE_SCALE = 'bubble_user_image_scale';
  static const String _BUBBLE_AI_IMAGE_CROP_LEFT = 'bubble_ai_image_crop_left';
  static const String _BUBBLE_AI_IMAGE_CROP_TOP = 'bubble_ai_image_crop_top';
  static const String _BUBBLE_AI_IMAGE_CROP_RIGHT =
      'bubble_ai_image_crop_right';
  static const String _BUBBLE_AI_IMAGE_CROP_BOTTOM =
      'bubble_ai_image_crop_bottom';
  static const String _BUBBLE_AI_IMAGE_REPEAT_START =
      'bubble_ai_image_repeat_start';
  static const String _BUBBLE_AI_IMAGE_REPEAT_END =
      'bubble_ai_image_repeat_end';
  static const String _BUBBLE_AI_IMAGE_REPEAT_Y_START =
      'bubble_ai_image_repeat_y_start';
  static const String _BUBBLE_AI_IMAGE_REPEAT_Y_END =
      'bubble_ai_image_repeat_y_end';
  static const String _BUBBLE_AI_IMAGE_SCALE = 'bubble_ai_image_scale';
  static const String _BUBBLE_USER_IMAGE_RENDER_MODE =
      'bubble_user_image_render_mode';
  static const String _BUBBLE_AI_IMAGE_RENDER_MODE =
      'bubble_ai_image_render_mode';
  static const String _BUBBLE_USER_ROUNDED_CORNERS_ENABLED =
      'bubble_rounded_corners_enabled';
  static const String _BUBBLE_AI_ROUNDED_CORNERS_ENABLED =
      'bubble_ai_rounded_corners_enabled';
  static const String _BUBBLE_USER_CONTENT_PADDING_LEFT =
      'bubble_content_padding_left';
  static const String _BUBBLE_USER_CONTENT_PADDING_RIGHT =
      'bubble_content_padding_right';
  static const String _BUBBLE_AI_CONTENT_PADDING_LEFT =
      'bubble_ai_content_padding_left';
  static const String _BUBBLE_AI_CONTENT_PADDING_RIGHT =
      'bubble_ai_content_padding_right';
  static const String _KEY_SHOW_THINKING_PROCESS = 'show_thinking_process';
  static const String _KEY_SHOW_MODEL_PROVIDER = 'show_model_provider';
  static const String _KEY_SHOW_MODEL_NAME = 'show_model_name';
  static const String _KEY_SHOW_ROLE_NAME = 'show_role_name';
  static const String _KEY_SHOW_USER_NAME = 'show_user_name';
  static const String _KEY_SHOW_MESSAGE_TOKEN_STATS =
      'show_message_token_stats';
  static const String _KEY_SHOW_MESSAGE_TIMING_STATS =
      'show_message_timing_stats';
  static const String _KEY_SHOW_MESSAGE_TIMESTAMP = 'show_message_timestamp';
  static const String _KEY_CUSTOM_USER_AVATAR_URI = 'custom_user_avatar_uri';
  static const String _KEY_CUSTOM_AI_AVATAR_URI = 'custom_ai_avatar_uri';
  static const String _KEY_AVATAR_SHAPE = 'avatar_shape';
  static const String _KEY_AVATAR_CORNER_RADIUS = 'avatar_corner_radius';
  static const String _KEY_SHOW_INPUT_PROCESSING_STATUS =
      'show_input_processing_status';

  static const List<String> _stringThemeKeys = <String>[
    _THEME_MODE,
    _BACKGROUND_IMAGE_URI,
    _BACKGROUND_MEDIA_TYPE,
    _CHAT_STYLE,
    _KEY_CUSTOM_USER_AVATAR_URI,
    _KEY_CUSTOM_AI_AVATAR_URI,
    _KEY_AVATAR_SHAPE,
    _FONT_TYPE,
    _SYSTEM_FONT_NAME,
    _CUSTOM_FONT_PATH,
    _BUBBLE_USER_FONT_TYPE,
    _BUBBLE_USER_SYSTEM_FONT_NAME,
    _BUBBLE_USER_CUSTOM_FONT_PATH,
    _BUBBLE_AI_FONT_TYPE,
    _BUBBLE_AI_SYSTEM_FONT_NAME,
    _BUBBLE_AI_CUSTOM_FONT_PATH,
    _BUBBLE_USER_IMAGE_URI,
    _BUBBLE_AI_IMAGE_URI,
    _BUBBLE_USER_IMAGE_RENDER_MODE,
    _BUBBLE_AI_IMAGE_RENDER_MODE,
  ];

  static const List<String> _booleanThemeKeys = <String>[
    _USE_SYSTEM_THEME,
    _USE_CUSTOM_COLORS,
    _USE_BACKGROUND_IMAGE,
    _VIDEO_BACKGROUND_MUTED,
    _VIDEO_BACKGROUND_LOOP,
    _TRANSPARENT_SURFACE_ENABLED,
    _CHAT_INPUT_FLOATING,
    _USE_BACKGROUND_BLUR,
    _BUBBLE_SHOW_AVATAR,
    _BUBBLE_WIDE_LAYOUT_ENABLED,
    _BUBBLE_USER_USE_IMAGE,
    _BUBBLE_AI_USE_IMAGE,
    _BUBBLE_USER_ROUNDED_CORNERS_ENABLED,
    _BUBBLE_AI_ROUNDED_CORNERS_ENABLED,
    _KEY_SHOW_THINKING_PROCESS,
    _KEY_SHOW_INPUT_PROCESSING_STATUS,
    _USE_CUSTOM_FONT,
    _BUBBLE_USER_USE_CUSTOM_FONT,
    _BUBBLE_AI_USE_CUSTOM_FONT,
    _KEY_SHOW_MODEL_PROVIDER,
    _KEY_SHOW_MODEL_NAME,
    _KEY_SHOW_ROLE_NAME,
    _KEY_SHOW_USER_NAME,
    _KEY_SHOW_MESSAGE_TOKEN_STATS,
    _KEY_SHOW_MESSAGE_TIMING_STATS,
    _KEY_SHOW_MESSAGE_TIMESTAMP,
  ];

  static const List<String> _intThemeKeys = <String>[
    _CUSTOM_PRIMARY_COLOR,
    _CUSTOM_SECONDARY_COLOR,
    _CURSOR_USER_BUBBLE_COLOR,
    _BUBBLE_USER_BUBBLE_COLOR,
    _BUBBLE_AI_BUBBLE_COLOR,
    _BUBBLE_USER_TEXT_COLOR,
    _BUBBLE_AI_TEXT_COLOR,
  ];

  static const List<String> _floatThemeKeys = <String>[
    _BACKGROUND_IMAGE_OPACITY,
    _BACKGROUND_BLUR_RADIUS,
    _KEY_AVATAR_CORNER_RADIUS,
    _FONT_SCALE,
    _BUBBLE_USER_IMAGE_CROP_LEFT,
    _BUBBLE_USER_IMAGE_CROP_TOP,
    _BUBBLE_USER_IMAGE_CROP_RIGHT,
    _BUBBLE_USER_IMAGE_CROP_BOTTOM,
    _BUBBLE_USER_IMAGE_REPEAT_START,
    _BUBBLE_USER_IMAGE_REPEAT_END,
    _BUBBLE_USER_IMAGE_REPEAT_Y_START,
    _BUBBLE_USER_IMAGE_REPEAT_Y_END,
    _BUBBLE_USER_IMAGE_SCALE,
    _BUBBLE_AI_IMAGE_CROP_LEFT,
    _BUBBLE_AI_IMAGE_CROP_TOP,
    _BUBBLE_AI_IMAGE_CROP_RIGHT,
    _BUBBLE_AI_IMAGE_CROP_BOTTOM,
    _BUBBLE_AI_IMAGE_REPEAT_START,
    _BUBBLE_AI_IMAGE_REPEAT_END,
    _BUBBLE_AI_IMAGE_REPEAT_Y_START,
    _BUBBLE_AI_IMAGE_REPEAT_Y_END,
    _BUBBLE_AI_IMAGE_SCALE,
    _BUBBLE_USER_CONTENT_PADDING_LEFT,
    _BUBBLE_USER_CONTENT_PADDING_RIGHT,
    _BUBBLE_AI_CONTENT_PADDING_LEFT,
    _BUBBLE_AI_CONTENT_PADDING_RIGHT,
  ];

  static const List<String> _themeKeys = <String>[
    ..._stringThemeKeys,
    ..._booleanThemeKeys,
    ..._intThemeKeys,
    ..._floatThemeKeys,
  ];

  static const List<String> _resetThemeKeys = <String>[
    _THEME_MODE,
    _USE_SYSTEM_THEME,
    _CUSTOM_PRIMARY_COLOR,
    _CUSTOM_SECONDARY_COLOR,
    _USE_CUSTOM_COLORS,
    _USE_BACKGROUND_IMAGE,
    _BACKGROUND_IMAGE_URI,
    _BACKGROUND_IMAGE_OPACITY,
    _BACKGROUND_MEDIA_TYPE,
    _VIDEO_BACKGROUND_MUTED,
    _VIDEO_BACKGROUND_LOOP,
    _TRANSPARENT_SURFACE_ENABLED,
    _CHAT_INPUT_FLOATING,
    _USE_BACKGROUND_BLUR,
    _BACKGROUND_BLUR_RADIUS,
    _CHAT_STYLE,
    _BUBBLE_SHOW_AVATAR,
    _BUBBLE_WIDE_LAYOUT_ENABLED,
    _CURSOR_USER_BUBBLE_COLOR,
    _BUBBLE_USER_BUBBLE_COLOR,
    _BUBBLE_AI_BUBBLE_COLOR,
    _BUBBLE_USER_TEXT_COLOR,
    _BUBBLE_AI_TEXT_COLOR,
    _BUBBLE_USER_USE_CUSTOM_FONT,
    _BUBBLE_USER_FONT_TYPE,
    _BUBBLE_USER_SYSTEM_FONT_NAME,
    _BUBBLE_USER_CUSTOM_FONT_PATH,
    _BUBBLE_AI_USE_CUSTOM_FONT,
    _BUBBLE_AI_FONT_TYPE,
    _BUBBLE_AI_SYSTEM_FONT_NAME,
    _BUBBLE_AI_CUSTOM_FONT_PATH,
    _BUBBLE_USER_USE_IMAGE,
    _BUBBLE_AI_USE_IMAGE,
    _BUBBLE_USER_IMAGE_URI,
    _BUBBLE_AI_IMAGE_URI,
    _BUBBLE_USER_IMAGE_CROP_LEFT,
    _BUBBLE_USER_IMAGE_CROP_TOP,
    _BUBBLE_USER_IMAGE_CROP_RIGHT,
    _BUBBLE_USER_IMAGE_CROP_BOTTOM,
    _BUBBLE_USER_IMAGE_REPEAT_START,
    _BUBBLE_USER_IMAGE_REPEAT_END,
    _BUBBLE_USER_IMAGE_REPEAT_Y_START,
    _BUBBLE_USER_IMAGE_REPEAT_Y_END,
    _BUBBLE_USER_IMAGE_SCALE,
    _BUBBLE_AI_IMAGE_CROP_LEFT,
    _BUBBLE_AI_IMAGE_CROP_TOP,
    _BUBBLE_AI_IMAGE_CROP_RIGHT,
    _BUBBLE_AI_IMAGE_CROP_BOTTOM,
    _BUBBLE_AI_IMAGE_REPEAT_START,
    _BUBBLE_AI_IMAGE_REPEAT_END,
    _BUBBLE_AI_IMAGE_REPEAT_Y_START,
    _BUBBLE_AI_IMAGE_REPEAT_Y_END,
    _BUBBLE_AI_IMAGE_SCALE,
    _BUBBLE_USER_IMAGE_RENDER_MODE,
    _BUBBLE_AI_IMAGE_RENDER_MODE,
    _BUBBLE_USER_ROUNDED_CORNERS_ENABLED,
    _BUBBLE_AI_ROUNDED_CORNERS_ENABLED,
    _BUBBLE_USER_CONTENT_PADDING_LEFT,
    _BUBBLE_USER_CONTENT_PADDING_RIGHT,
    _BUBBLE_AI_CONTENT_PADDING_LEFT,
    _BUBBLE_AI_CONTENT_PADDING_RIGHT,
    _KEY_SHOW_THINKING_PROCESS,
    _KEY_SHOW_MODEL_PROVIDER,
    _KEY_SHOW_MODEL_NAME,
    _KEY_SHOW_ROLE_NAME,
    _KEY_SHOW_USER_NAME,
    _KEY_SHOW_MESSAGE_TOKEN_STATS,
    _KEY_SHOW_MESSAGE_TIMING_STATS,
    _KEY_SHOW_MESSAGE_TIMESTAMP,
    _KEY_CUSTOM_USER_AVATAR_URI,
    _KEY_CUSTOM_AI_AVATAR_URI,
    _KEY_AVATAR_SHAPE,
    _KEY_AVATAR_CORNER_RADIUS,
    _KEY_SHOW_INPUT_PROCESSING_STATUS,
    _USE_CUSTOM_FONT,
    _FONT_TYPE,
    _SYSTEM_FONT_NAME,
    _CUSTOM_FONT_PATH,
    _FONT_SCALE,
  ];

  Future<String> themeMode() async {
    return await _stringValue(_THEME_MODE) ?? THEME_MODE_LIGHT;
  }

  Future<void> initializeIfNeeded({required String defaultProfileName}) async {
    final profiles = await profileListFlowSnapshot();
    if (profiles.isEmpty || !profiles.contains(DEFAULT_PROFILE_ID)) {
      await createProfile(name: defaultProfileName, isDefault: true);
    }
  }

  Future<String> activeProfileIdFlowSnapshot() {
    return activeProfileId();
  }

  Future<String> activeProfileId() async {
    return await _getString(_ACTIVE_PROFILE_ID) ?? DEFAULT_PROFILE_ID;
  }

  Future<List<String>> profileListFlowSnapshot() async {
    final profileListJson = await _getString(_PROFILE_LIST) ?? '[]';
    final profileList = (jsonDecode(profileListJson) as List<Object?>)
        .map((profileId) => profileId as String)
        .toList();
    if (!profileList.contains(DEFAULT_PROFILE_ID)) {
      profileList.insert(0, DEFAULT_PROFILE_ID);
    }
    return profileList;
  }

  Future<core_proxy.PreferenceProfile> getUserPreferencesFlowSnapshot({
    String profileId = '',
  }) async {
    final targetProfileId = profileId.isEmpty
        ? await activeProfileId()
        : profileId;
    return getProfile(profileId: targetProfileId);
  }

  Future<Map<String, bool>> categoryLockStatusFlowSnapshot() async {
    return <String, bool>{
      'birthDate': await _booleanValue(_BIRTH_DATE_LOCKED, false),
      'gender': await _booleanValue(_GENDER_LOCKED, false),
      'personality': await _booleanValue(_PERSONALITY_LOCKED, false),
      'identity': await _booleanValue(_IDENTITY_LOCKED, false),
      'occupation': await _booleanValue(_OCCUPATION_LOCKED, false),
      'aiStyle': await _booleanValue(_AI_STYLE_LOCKED, false),
    };
  }

  Future<bool> isCategoryLocked({required String category}) async {
    final lockStatusMap = await categoryLockStatusFlowSnapshot();
    return lockStatusMap[category] ?? false;
  }

  Future<void> setCategoryLocked({
    required String category,
    required bool locked,
  }) async {
    switch (category) {
      case 'birthDate':
        await _setString(_BIRTH_DATE_LOCKED, locked.toString());
      case 'gender':
        await _setString(_GENDER_LOCKED, locked.toString());
      case 'personality':
        await _setString(_PERSONALITY_LOCKED, locked.toString());
      case 'identity':
        await _setString(_IDENTITY_LOCKED, locked.toString());
      case 'occupation':
        await _setString(_OCCUPATION_LOCKED, locked.toString());
      case 'aiStyle':
        await _setString(_AI_STYLE_LOCKED, locked.toString());
    }
  }

  Future<String> createProfile({
    required String name,
    bool isDefault = false,
  }) async {
    final profileId = isDefault
        ? DEFAULT_PROFILE_ID
        : 'profile_${DateTime.now().millisecondsSinceEpoch}';
    final newProfile = core_proxy.PreferenceProfile(
      id: profileId,
      name: name,
      birthDate: 0,
      gender: '',
      personality: '',
      identity: '',
      occupation: '',
      aiStyle: '',
      isInitialized: false,
    );
    final currentListJson = await _getString(_PROFILE_LIST) ?? '[]';
    final currentList = (jsonDecode(currentListJson) as List<Object?>)
        .map((profileId) => profileId as String)
        .toList();
    if (!currentList.contains(profileId)) {
      currentList.add(profileId);
    }
    await _setString(_PROFILE_LIST, jsonEncode(currentList));
    await updateProfile(newProfile);
    await _setString(_BIRTH_DATE_LOCKED, true.toString());
    return profileId;
  }

  Future<void> setActiveProfile({required String profileId}) {
    return _setString(_ACTIVE_PROFILE_ID, profileId);
  }

  Future<core_proxy.PreferenceProfile> getProfile({
    required String profileId,
  }) async {
    final profileJson = await _getString(_profileKey(profileId));
    if (profileJson != null) {
      return core_proxy.PreferenceProfile.fromJson(
        Map<String, Object?>.from(jsonDecode(profileJson) as Map),
      );
    }
    return _createDefaultProfile(profileId);
  }

  Future<void> updateProfile(core_proxy.PreferenceProfile profile) {
    return _setString(_profileKey(profile.id), jsonEncode(profile.toJson()));
  }

  Future<core_proxy.PreferenceProfile> updateProfileCategory({
    String profileId = '',
    int? birthDate,
    String? gender,
    String? personality,
    String? identity,
    String? occupation,
    String? aiStyle,
  }) async {
    final targetProfileId = profileId.isEmpty
        ? await activeProfileId()
        : profileId;
    final currentProfile = await getProfile(profileId: targetProfileId);
    final updatedProfile = core_proxy.PreferenceProfile(
      id: currentProfile.id,
      name: currentProfile.name,
      birthDate:
          birthDate != null && !(await isCategoryLocked(category: 'birthDate'))
          ? birthDate
          : currentProfile.birthDate,
      gender: gender != null && !(await isCategoryLocked(category: 'gender'))
          ? gender
          : currentProfile.gender,
      personality:
          personality != null &&
              !(await isCategoryLocked(category: 'personality'))
          ? personality
          : currentProfile.personality,
      identity:
          identity != null && !(await isCategoryLocked(category: 'identity'))
          ? identity
          : currentProfile.identity,
      occupation:
          occupation != null &&
              !(await isCategoryLocked(category: 'occupation'))
          ? occupation
          : currentProfile.occupation,
      aiStyle: aiStyle != null && !(await isCategoryLocked(category: 'aiStyle'))
          ? aiStyle
          : currentProfile.aiStyle,
      isInitialized: true,
    );
    await updateProfile(updatedProfile);
    return updatedProfile;
  }

  Future<bool> useSystemTheme() {
    return _booleanValue(_USE_SYSTEM_THEME, true);
  }

  Future<ThemeMode> flutterThemeMode() async {
    return (await resolveThemePreferenceSnapshot()).flutterThemeMode;
  }

  Future<ThemePreferenceSnapshot> resolveThemePreferenceSnapshot({
    String? characterCardId,
    String? characterGroupId,
  }) async {
    final normalizedGroupId = _normalizedThemeId(characterGroupId);
    final normalizedCardId = _normalizedThemeId(characterCardId);
    final groupPrefix = normalizedGroupId == null
        ? null
        : _characterGroupThemePrefix(normalizedGroupId);
    final cardPrefix = normalizedCardId == null
        ? null
        : _characterCardThemePrefix(normalizedCardId);
    final keys = <String>[
      ..._themeKeys,
      if (groupPrefix != null) ..._prefixedThemeKeys(groupPrefix),
      if (cardPrefix != null) ..._prefixedThemeKeys(cardPrefix),
    ];
    final preferences = await _getStrings(keys);
    String? prefix;

    if (groupPrefix != null && _containsThemePrefix(preferences, groupPrefix)) {
      prefix = groupPrefix;
    } else if (cardPrefix != null &&
        _containsThemePrefix(preferences, cardPrefix)) {
      prefix = cardPrefix;
    }

    String? stringValue(String key, {String? defaultValue}) {
      return preferences[_keyWithPrefix(key, prefix)] ?? defaultValue;
    }

    bool booleanValue(String key, bool defaultValue) {
      final value = stringValue(key);
      return value == null ? defaultValue : _decodeBool(value);
    }

    int? intValue(String key) {
      final value = stringValue(key);
      return value == null ? null : int.parse(value);
    }

    double doubleValue(String key, double defaultValue) {
      final value = stringValue(key);
      return value == null ? defaultValue : double.parse(value);
    }

    return ThemePreferenceSnapshot(
      themeMode: stringValue(_THEME_MODE) ?? THEME_MODE_LIGHT,
      useSystemTheme: booleanValue(_USE_SYSTEM_THEME, true),
      useCustomColors: booleanValue(_USE_CUSTOM_COLORS, false),
      customPrimaryColor: intValue(_CUSTOM_PRIMARY_COLOR),
      customSecondaryColor: intValue(_CUSTOM_SECONDARY_COLOR),
      useBackgroundImage: booleanValue(_USE_BACKGROUND_IMAGE, false),
      backgroundImageUri: stringValue(_BACKGROUND_IMAGE_URI),
      backgroundMediaType:
          stringValue(_BACKGROUND_MEDIA_TYPE) ?? MEDIA_TYPE_IMAGE,
      backgroundImageOpacity: doubleValue(_BACKGROUND_IMAGE_OPACITY, 0.3),
      videoBackgroundMuted: booleanValue(_VIDEO_BACKGROUND_MUTED, true),
      videoBackgroundLoop: booleanValue(_VIDEO_BACKGROUND_LOOP, true),
      useBackgroundBlur: booleanValue(_USE_BACKGROUND_BLUR, false),
      backgroundBlurRadius: doubleValue(_BACKGROUND_BLUR_RADIUS, 12),
      transparentSurfaceEnabled: booleanValue(
        _TRANSPARENT_SURFACE_ENABLED,
        false,
      ),
      chatInputFloating: booleanValue(_CHAT_INPUT_FLOATING, false),
      chatStyle: stringValue(_CHAT_STYLE) ?? CHAT_STYLE_CURSOR,
      bubbleShowAvatar: booleanValue(_BUBBLE_SHOW_AVATAR, true),
      bubbleWideLayoutEnabled: booleanValue(_BUBBLE_WIDE_LAYOUT_ENABLED, false),
      cursorUserBubbleColor: intValue(_CURSOR_USER_BUBBLE_COLOR),
      bubbleUserBubbleColor: intValue(_BUBBLE_USER_BUBBLE_COLOR),
      bubbleAiBubbleColor: intValue(_BUBBLE_AI_BUBBLE_COLOR),
      bubbleUserTextColor: intValue(_BUBBLE_USER_TEXT_COLOR),
      bubbleAiTextColor: intValue(_BUBBLE_AI_TEXT_COLOR),
      bubbleUserUseImage: booleanValue(_BUBBLE_USER_USE_IMAGE, false),
      bubbleAiUseImage: booleanValue(_BUBBLE_AI_USE_IMAGE, false),
      bubbleUserImageUri: stringValue(_BUBBLE_USER_IMAGE_URI),
      bubbleAiImageUri: stringValue(_BUBBLE_AI_IMAGE_URI),
      bubbleUserImageRenderMode:
          stringValue(_BUBBLE_USER_IMAGE_RENDER_MODE) ??
          BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
      bubbleAiImageRenderMode:
          stringValue(_BUBBLE_AI_IMAGE_RENDER_MODE) ??
          BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
      bubbleUserImageCropLeft: doubleValue(_BUBBLE_USER_IMAGE_CROP_LEFT, 0),
      bubbleUserImageCropTop: doubleValue(_BUBBLE_USER_IMAGE_CROP_TOP, 0),
      bubbleUserImageCropRight: doubleValue(_BUBBLE_USER_IMAGE_CROP_RIGHT, 0),
      bubbleUserImageCropBottom: doubleValue(_BUBBLE_USER_IMAGE_CROP_BOTTOM, 0),
      bubbleUserImageRepeatStart: doubleValue(
        _BUBBLE_USER_IMAGE_REPEAT_START,
        0.35,
      ),
      bubbleUserImageRepeatEnd: doubleValue(
        _BUBBLE_USER_IMAGE_REPEAT_END,
        0.65,
      ),
      bubbleUserImageRepeatYStart: doubleValue(
        _BUBBLE_USER_IMAGE_REPEAT_Y_START,
        0.35,
      ),
      bubbleUserImageRepeatYEnd: doubleValue(
        _BUBBLE_USER_IMAGE_REPEAT_Y_END,
        0.65,
      ),
      bubbleUserImageScale: doubleValue(_BUBBLE_USER_IMAGE_SCALE, 1),
      bubbleAiImageCropLeft: doubleValue(_BUBBLE_AI_IMAGE_CROP_LEFT, 0),
      bubbleAiImageCropTop: doubleValue(_BUBBLE_AI_IMAGE_CROP_TOP, 0),
      bubbleAiImageCropRight: doubleValue(_BUBBLE_AI_IMAGE_CROP_RIGHT, 0),
      bubbleAiImageCropBottom: doubleValue(_BUBBLE_AI_IMAGE_CROP_BOTTOM, 0),
      bubbleAiImageRepeatStart: doubleValue(
        _BUBBLE_AI_IMAGE_REPEAT_START,
        0.35,
      ),
      bubbleAiImageRepeatEnd: doubleValue(_BUBBLE_AI_IMAGE_REPEAT_END, 0.65),
      bubbleAiImageRepeatYStart: doubleValue(
        _BUBBLE_AI_IMAGE_REPEAT_Y_START,
        0.35,
      ),
      bubbleAiImageRepeatYEnd: doubleValue(_BUBBLE_AI_IMAGE_REPEAT_Y_END, 0.65),
      bubbleAiImageScale: doubleValue(_BUBBLE_AI_IMAGE_SCALE, 1),
      bubbleUserRoundedCornersEnabled: booleanValue(
        _BUBBLE_USER_ROUNDED_CORNERS_ENABLED,
        true,
      ),
      bubbleAiRoundedCornersEnabled: booleanValue(
        _BUBBLE_AI_ROUNDED_CORNERS_ENABLED,
        true,
      ),
      bubbleUserContentPaddingLeft: doubleValue(
        _BUBBLE_USER_CONTENT_PADDING_LEFT,
        12,
      ),
      bubbleUserContentPaddingRight: doubleValue(
        _BUBBLE_USER_CONTENT_PADDING_RIGHT,
        12,
      ),
      bubbleAiContentPaddingLeft: doubleValue(
        _BUBBLE_AI_CONTENT_PADDING_LEFT,
        12,
      ),
      bubbleAiContentPaddingRight: doubleValue(
        _BUBBLE_AI_CONTENT_PADDING_RIGHT,
        12,
      ),
      customUserAvatarUri: stringValue(_KEY_CUSTOM_USER_AVATAR_URI),
      customAiAvatarUri: stringValue(_KEY_CUSTOM_AI_AVATAR_URI),
      avatarShape: stringValue(_KEY_AVATAR_SHAPE) ?? AVATAR_SHAPE_CIRCLE,
      avatarCornerRadius: doubleValue(_KEY_AVATAR_CORNER_RADIUS, 8),
      useCustomFont: booleanValue(_USE_CUSTOM_FONT, false),
      fontType: stringValue(_FONT_TYPE) ?? FONT_TYPE_SYSTEM,
      systemFontName: stringValue(_SYSTEM_FONT_NAME),
      customFontPath: stringValue(_CUSTOM_FONT_PATH),
      fontScale: doubleValue(_FONT_SCALE, 1.0),
      bubbleUserUseCustomFont: booleanValue(
        _BUBBLE_USER_USE_CUSTOM_FONT,
        false,
      ),
      bubbleUserFontType:
          stringValue(_BUBBLE_USER_FONT_TYPE) ?? FONT_TYPE_SYSTEM,
      bubbleUserSystemFontName:
          stringValue(_BUBBLE_USER_SYSTEM_FONT_NAME) ?? SYSTEM_FONT_DEFAULT,
      bubbleUserCustomFontPath: stringValue(_BUBBLE_USER_CUSTOM_FONT_PATH),
      bubbleAiUseCustomFont: booleanValue(_BUBBLE_AI_USE_CUSTOM_FONT, false),
      bubbleAiFontType: stringValue(_BUBBLE_AI_FONT_TYPE) ?? FONT_TYPE_SYSTEM,
      bubbleAiSystemFontName:
          stringValue(_BUBBLE_AI_SYSTEM_FONT_NAME) ?? SYSTEM_FONT_DEFAULT,
      bubbleAiCustomFontPath: stringValue(_BUBBLE_AI_CUSTOM_FONT_PATH),
      showThinkingProcess: booleanValue(_KEY_SHOW_THINKING_PROCESS, true),
      showModelProvider: booleanValue(_KEY_SHOW_MODEL_PROVIDER, false),
      showModelName: booleanValue(_KEY_SHOW_MODEL_NAME, false),
      showRoleName: booleanValue(_KEY_SHOW_ROLE_NAME, true),
      showUserName: booleanValue(_KEY_SHOW_USER_NAME, true),
      showMessageTokenStats: booleanValue(_KEY_SHOW_MESSAGE_TOKEN_STATS, false),
      showMessageTimingStats: booleanValue(
        _KEY_SHOW_MESSAGE_TIMING_STATS,
        false,
      ),
      showMessageTimestamp: booleanValue(_KEY_SHOW_MESSAGE_TIMESTAMP, false),
      showInputProcessingStatus: booleanValue(
        _KEY_SHOW_INPUT_PROCESSING_STATUS,
        true,
      ),
    );
  }

  Future<void> saveThemeSettings({
    String? themeMode,
    bool? useSystemTheme,
    bool? useCustomColors,
    int? customPrimaryColor,
    int? customSecondaryColor,
    bool? useBackgroundImage,
    String? backgroundImageUri,
    double? backgroundImageOpacity,
    String? backgroundMediaType,
    bool? videoBackgroundMuted,
    bool? videoBackgroundLoop,
    bool? useBackgroundBlur,
    double? backgroundBlurRadius,
    bool? transparentSurfaceEnabled,
    bool? chatInputFloating,
    String? chatStyle,
    bool? bubbleShowAvatar,
    bool? bubbleWideLayoutEnabled,
    int? cursorUserBubbleColor,
    int? bubbleUserBubbleColor,
    int? bubbleAiBubbleColor,
    int? bubbleUserTextColor,
    int? bubbleAiTextColor,
    bool? bubbleUserUseImage,
    bool? bubbleAiUseImage,
    String? bubbleUserImageUri,
    String? bubbleAiImageUri,
    String? bubbleUserImageRenderMode,
    String? bubbleAiImageRenderMode,
    double? bubbleUserImageCropLeft,
    double? bubbleUserImageCropTop,
    double? bubbleUserImageCropRight,
    double? bubbleUserImageCropBottom,
    double? bubbleUserImageRepeatStart,
    double? bubbleUserImageRepeatEnd,
    double? bubbleUserImageRepeatYStart,
    double? bubbleUserImageRepeatYEnd,
    double? bubbleUserImageScale,
    double? bubbleAiImageCropLeft,
    double? bubbleAiImageCropTop,
    double? bubbleAiImageCropRight,
    double? bubbleAiImageCropBottom,
    double? bubbleAiImageRepeatStart,
    double? bubbleAiImageRepeatEnd,
    double? bubbleAiImageRepeatYStart,
    double? bubbleAiImageRepeatYEnd,
    double? bubbleAiImageScale,
    bool? bubbleUserRoundedCornersEnabled,
    bool? bubbleAiRoundedCornersEnabled,
    double? bubbleUserContentPaddingLeft,
    double? bubbleUserContentPaddingRight,
    double? bubbleAiContentPaddingLeft,
    double? bubbleAiContentPaddingRight,
    bool? showThinkingProcess,
    bool? showModelProvider,
    bool? showModelName,
    bool? showRoleName,
    bool? showUserName,
    bool? showMessageTokenStats,
    bool? showMessageTimingStats,
    bool? showMessageTimestamp,
    String? customUserAvatarUri,
    String? customAiAvatarUri,
    String? avatarShape,
    double? avatarCornerRadius,
    bool? showInputProcessingStatus,
    bool? useCustomFont,
    String? fontType,
    String? systemFontName,
    String? customFontPath,
    double? fontScale,
    bool? bubbleUserUseCustomFont,
    String? bubbleUserFontType,
    String? bubbleUserSystemFontName,
    String? bubbleUserCustomFontPath,
    bool? bubbleAiUseCustomFont,
    String? bubbleAiFontType,
    String? bubbleAiSystemFontName,
    String? bubbleAiCustomFontPath,
  }) async {
    final values = <String, String>{};

    void setIfPresent(String key, Object? value) {
      if (value == null) {
        return;
      }
      values[key] = value.toString();
    }

    setIfPresent(_THEME_MODE, themeMode);
    setIfPresent(_USE_SYSTEM_THEME, useSystemTheme);
    setIfPresent(_CUSTOM_PRIMARY_COLOR, customPrimaryColor);
    setIfPresent(_CUSTOM_SECONDARY_COLOR, customSecondaryColor);
    setIfPresent(_USE_CUSTOM_COLORS, useCustomColors);
    setIfPresent(_USE_BACKGROUND_IMAGE, useBackgroundImage);
    setIfPresent(_BACKGROUND_IMAGE_URI, backgroundImageUri);
    setIfPresent(_BACKGROUND_IMAGE_OPACITY, backgroundImageOpacity);
    setIfPresent(_BACKGROUND_MEDIA_TYPE, backgroundMediaType);
    setIfPresent(_VIDEO_BACKGROUND_MUTED, videoBackgroundMuted);
    setIfPresent(_VIDEO_BACKGROUND_LOOP, videoBackgroundLoop);
    setIfPresent(_USE_BACKGROUND_BLUR, useBackgroundBlur);
    setIfPresent(_BACKGROUND_BLUR_RADIUS, backgroundBlurRadius);
    setIfPresent(_TRANSPARENT_SURFACE_ENABLED, transparentSurfaceEnabled);
    setIfPresent(_CHAT_INPUT_FLOATING, chatInputFloating);
    setIfPresent(_CHAT_STYLE, chatStyle);
    setIfPresent(_BUBBLE_SHOW_AVATAR, bubbleShowAvatar);
    setIfPresent(_BUBBLE_WIDE_LAYOUT_ENABLED, bubbleWideLayoutEnabled);
    setIfPresent(_CURSOR_USER_BUBBLE_COLOR, cursorUserBubbleColor);
    setIfPresent(_BUBBLE_USER_BUBBLE_COLOR, bubbleUserBubbleColor);
    setIfPresent(_BUBBLE_AI_BUBBLE_COLOR, bubbleAiBubbleColor);
    setIfPresent(_BUBBLE_USER_TEXT_COLOR, bubbleUserTextColor);
    setIfPresent(_BUBBLE_AI_TEXT_COLOR, bubbleAiTextColor);
    setIfPresent(_BUBBLE_USER_USE_IMAGE, bubbleUserUseImage);
    setIfPresent(_BUBBLE_AI_USE_IMAGE, bubbleAiUseImage);
    setIfPresent(_BUBBLE_USER_IMAGE_URI, bubbleUserImageUri);
    setIfPresent(_BUBBLE_AI_IMAGE_URI, bubbleAiImageUri);
    setIfPresent(_BUBBLE_USER_IMAGE_RENDER_MODE, bubbleUserImageRenderMode);
    setIfPresent(_BUBBLE_AI_IMAGE_RENDER_MODE, bubbleAiImageRenderMode);
    setIfPresent(_BUBBLE_USER_IMAGE_CROP_LEFT, bubbleUserImageCropLeft);
    setIfPresent(_BUBBLE_USER_IMAGE_CROP_TOP, bubbleUserImageCropTop);
    setIfPresent(_BUBBLE_USER_IMAGE_CROP_RIGHT, bubbleUserImageCropRight);
    setIfPresent(_BUBBLE_USER_IMAGE_CROP_BOTTOM, bubbleUserImageCropBottom);
    setIfPresent(_BUBBLE_USER_IMAGE_REPEAT_START, bubbleUserImageRepeatStart);
    setIfPresent(_BUBBLE_USER_IMAGE_REPEAT_END, bubbleUserImageRepeatEnd);
    setIfPresent(
      _BUBBLE_USER_IMAGE_REPEAT_Y_START,
      bubbleUserImageRepeatYStart,
    );
    setIfPresent(_BUBBLE_USER_IMAGE_REPEAT_Y_END, bubbleUserImageRepeatYEnd);
    setIfPresent(_BUBBLE_USER_IMAGE_SCALE, bubbleUserImageScale);
    setIfPresent(_BUBBLE_AI_IMAGE_CROP_LEFT, bubbleAiImageCropLeft);
    setIfPresent(_BUBBLE_AI_IMAGE_CROP_TOP, bubbleAiImageCropTop);
    setIfPresent(_BUBBLE_AI_IMAGE_CROP_RIGHT, bubbleAiImageCropRight);
    setIfPresent(_BUBBLE_AI_IMAGE_CROP_BOTTOM, bubbleAiImageCropBottom);
    setIfPresent(_BUBBLE_AI_IMAGE_REPEAT_START, bubbleAiImageRepeatStart);
    setIfPresent(_BUBBLE_AI_IMAGE_REPEAT_END, bubbleAiImageRepeatEnd);
    setIfPresent(_BUBBLE_AI_IMAGE_REPEAT_Y_START, bubbleAiImageRepeatYStart);
    setIfPresent(_BUBBLE_AI_IMAGE_REPEAT_Y_END, bubbleAiImageRepeatYEnd);
    setIfPresent(_BUBBLE_AI_IMAGE_SCALE, bubbleAiImageScale);
    setIfPresent(
      _BUBBLE_USER_ROUNDED_CORNERS_ENABLED,
      bubbleUserRoundedCornersEnabled,
    );
    setIfPresent(
      _BUBBLE_AI_ROUNDED_CORNERS_ENABLED,
      bubbleAiRoundedCornersEnabled,
    );
    setIfPresent(
      _BUBBLE_USER_CONTENT_PADDING_LEFT,
      bubbleUserContentPaddingLeft,
    );
    setIfPresent(
      _BUBBLE_USER_CONTENT_PADDING_RIGHT,
      bubbleUserContentPaddingRight,
    );
    setIfPresent(_BUBBLE_AI_CONTENT_PADDING_LEFT, bubbleAiContentPaddingLeft);
    setIfPresent(_BUBBLE_AI_CONTENT_PADDING_RIGHT, bubbleAiContentPaddingRight);
    setIfPresent(_KEY_SHOW_THINKING_PROCESS, showThinkingProcess);
    setIfPresent(_KEY_SHOW_MODEL_PROVIDER, showModelProvider);
    setIfPresent(_KEY_SHOW_MODEL_NAME, showModelName);
    setIfPresent(_KEY_SHOW_ROLE_NAME, showRoleName);
    setIfPresent(_KEY_SHOW_USER_NAME, showUserName);
    setIfPresent(_KEY_SHOW_MESSAGE_TOKEN_STATS, showMessageTokenStats);
    setIfPresent(_KEY_SHOW_MESSAGE_TIMING_STATS, showMessageTimingStats);
    setIfPresent(_KEY_SHOW_MESSAGE_TIMESTAMP, showMessageTimestamp);
    setIfPresent(_KEY_CUSTOM_USER_AVATAR_URI, customUserAvatarUri);
    setIfPresent(_KEY_CUSTOM_AI_AVATAR_URI, customAiAvatarUri);
    setIfPresent(_KEY_AVATAR_SHAPE, avatarShape);
    setIfPresent(_KEY_AVATAR_CORNER_RADIUS, avatarCornerRadius);
    setIfPresent(_KEY_SHOW_INPUT_PROCESSING_STATUS, showInputProcessingStatus);
    setIfPresent(_USE_CUSTOM_FONT, useCustomFont);
    setIfPresent(_FONT_TYPE, fontType);
    setIfPresent(_SYSTEM_FONT_NAME, systemFontName);
    setIfPresent(_CUSTOM_FONT_PATH, customFontPath);
    setIfPresent(_FONT_SCALE, fontScale);
    setIfPresent(_BUBBLE_USER_USE_CUSTOM_FONT, bubbleUserUseCustomFont);
    setIfPresent(_BUBBLE_USER_FONT_TYPE, bubbleUserFontType);
    setIfPresent(_BUBBLE_USER_SYSTEM_FONT_NAME, bubbleUserSystemFontName);
    setIfPresent(_BUBBLE_USER_CUSTOM_FONT_PATH, bubbleUserCustomFontPath);
    setIfPresent(_BUBBLE_AI_USE_CUSTOM_FONT, bubbleAiUseCustomFont);
    setIfPresent(_BUBBLE_AI_FONT_TYPE, bubbleAiFontType);
    setIfPresent(_BUBBLE_AI_SYSTEM_FONT_NAME, bubbleAiSystemFontName);
    setIfPresent(_BUBBLE_AI_CUSTOM_FONT_PATH, bubbleAiCustomFontPath);

    if (values.isNotEmpty) {
      await _setStrings(values);
    }
  }

  Future<void> saveThemeSnapshot(ThemePreferenceSnapshot snapshot) async {
    await _removeStrings(_resetThemeKeys);
    await saveThemeSettings(
      themeMode: snapshot.themeMode,
      useSystemTheme: snapshot.useSystemTheme,
      useCustomColors: snapshot.useCustomColors,
      customPrimaryColor: snapshot.customPrimaryColor,
      customSecondaryColor: snapshot.customSecondaryColor,
      useBackgroundImage: snapshot.useBackgroundImage,
      backgroundImageUri: snapshot.backgroundImageUri,
      backgroundImageOpacity: snapshot.backgroundImageOpacity,
      backgroundMediaType: snapshot.backgroundMediaType,
      videoBackgroundMuted: snapshot.videoBackgroundMuted,
      videoBackgroundLoop: snapshot.videoBackgroundLoop,
      useBackgroundBlur: snapshot.useBackgroundBlur,
      backgroundBlurRadius: snapshot.backgroundBlurRadius,
      transparentSurfaceEnabled: snapshot.transparentSurfaceEnabled,
      chatInputFloating: snapshot.chatInputFloating,
      chatStyle: snapshot.chatStyle,
      bubbleShowAvatar: snapshot.bubbleShowAvatar,
      bubbleWideLayoutEnabled: snapshot.bubbleWideLayoutEnabled,
      cursorUserBubbleColor: snapshot.cursorUserBubbleColor,
      bubbleUserBubbleColor: snapshot.bubbleUserBubbleColor,
      bubbleAiBubbleColor: snapshot.bubbleAiBubbleColor,
      bubbleUserTextColor: snapshot.bubbleUserTextColor,
      bubbleAiTextColor: snapshot.bubbleAiTextColor,
      bubbleUserUseImage: snapshot.bubbleUserUseImage,
      bubbleAiUseImage: snapshot.bubbleAiUseImage,
      bubbleUserImageUri: snapshot.bubbleUserImageUri,
      bubbleAiImageUri: snapshot.bubbleAiImageUri,
      bubbleUserImageRenderMode: snapshot.bubbleUserImageRenderMode,
      bubbleAiImageRenderMode: snapshot.bubbleAiImageRenderMode,
      bubbleUserImageCropLeft: snapshot.bubbleUserImageCropLeft,
      bubbleUserImageCropTop: snapshot.bubbleUserImageCropTop,
      bubbleUserImageCropRight: snapshot.bubbleUserImageCropRight,
      bubbleUserImageCropBottom: snapshot.bubbleUserImageCropBottom,
      bubbleUserImageRepeatStart: snapshot.bubbleUserImageRepeatStart,
      bubbleUserImageRepeatEnd: snapshot.bubbleUserImageRepeatEnd,
      bubbleUserImageRepeatYStart: snapshot.bubbleUserImageRepeatYStart,
      bubbleUserImageRepeatYEnd: snapshot.bubbleUserImageRepeatYEnd,
      bubbleUserImageScale: snapshot.bubbleUserImageScale,
      bubbleAiImageCropLeft: snapshot.bubbleAiImageCropLeft,
      bubbleAiImageCropTop: snapshot.bubbleAiImageCropTop,
      bubbleAiImageCropRight: snapshot.bubbleAiImageCropRight,
      bubbleAiImageCropBottom: snapshot.bubbleAiImageCropBottom,
      bubbleAiImageRepeatStart: snapshot.bubbleAiImageRepeatStart,
      bubbleAiImageRepeatEnd: snapshot.bubbleAiImageRepeatEnd,
      bubbleAiImageRepeatYStart: snapshot.bubbleAiImageRepeatYStart,
      bubbleAiImageRepeatYEnd: snapshot.bubbleAiImageRepeatYEnd,
      bubbleAiImageScale: snapshot.bubbleAiImageScale,
      bubbleUserRoundedCornersEnabled: snapshot.bubbleUserRoundedCornersEnabled,
      bubbleAiRoundedCornersEnabled: snapshot.bubbleAiRoundedCornersEnabled,
      bubbleUserContentPaddingLeft: snapshot.bubbleUserContentPaddingLeft,
      bubbleUserContentPaddingRight: snapshot.bubbleUserContentPaddingRight,
      bubbleAiContentPaddingLeft: snapshot.bubbleAiContentPaddingLeft,
      bubbleAiContentPaddingRight: snapshot.bubbleAiContentPaddingRight,
      showThinkingProcess: snapshot.showThinkingProcess,
      showModelProvider: snapshot.showModelProvider,
      showModelName: snapshot.showModelName,
      showRoleName: snapshot.showRoleName,
      showUserName: snapshot.showUserName,
      showMessageTokenStats: snapshot.showMessageTokenStats,
      showMessageTimingStats: snapshot.showMessageTimingStats,
      showMessageTimestamp: snapshot.showMessageTimestamp,
      customUserAvatarUri: snapshot.customUserAvatarUri,
      customAiAvatarUri: snapshot.customAiAvatarUri,
      avatarShape: snapshot.avatarShape,
      avatarCornerRadius: snapshot.avatarCornerRadius,
      showInputProcessingStatus: snapshot.showInputProcessingStatus,
      useCustomFont: snapshot.useCustomFont,
      fontType: snapshot.fontType,
      systemFontName: snapshot.systemFontName,
      customFontPath: snapshot.customFontPath,
      fontScale: snapshot.fontScale,
      bubbleUserUseCustomFont: snapshot.bubbleUserUseCustomFont,
      bubbleUserFontType: snapshot.bubbleUserFontType,
      bubbleUserSystemFontName: snapshot.bubbleUserSystemFontName,
      bubbleUserCustomFontPath: snapshot.bubbleUserCustomFontPath,
      bubbleAiUseCustomFont: snapshot.bubbleAiUseCustomFont,
      bubbleAiFontType: snapshot.bubbleAiFontType,
      bubbleAiSystemFontName: snapshot.bubbleAiSystemFontName,
      bubbleAiCustomFontPath: snapshot.bubbleAiCustomFontPath,
    );
  }

  Future<void> saveFlutterThemeMode(ThemeMode themeMode) {
    return switch (themeMode) {
      ThemeMode.system => saveThemeSettings(useSystemTheme: true),
      ThemeMode.light => saveThemeSettings(
        themeMode: THEME_MODE_LIGHT,
        useSystemTheme: false,
      ),
      ThemeMode.dark => saveThemeSettings(
        themeMode: THEME_MODE_DARK,
        useSystemTheme: false,
      ),
    };
  }

  Future<void> resetThemeSettings() async {
    await _removeStrings(_resetThemeKeys);
  }

  Future<void> resetMessageColorSettings() async {
    await _removeStrings(<String>[
      _CURSOR_USER_BUBBLE_COLOR,
      _BUBBLE_USER_BUBBLE_COLOR,
      _BUBBLE_AI_BUBBLE_COLOR,
      _BUBBLE_USER_TEXT_COLOR,
      _BUBBLE_AI_TEXT_COLOR,
    ]);
  }

  Future<void> resetMessageColorSettingsForCharacterCard(
    String characterCardId,
  ) {
    return _resetMessageColorSettingsWithPrefix(
      _characterCardThemePrefix(characterCardId),
    );
  }

  Future<void> resetMessageColorSettingsForCharacterGroup(
    String characterGroupId,
  ) {
    return _resetMessageColorSettingsWithPrefix(
      _characterGroupThemePrefix(characterGroupId),
    );
  }

  Future<void> copyCurrentThemeToCharacterCard(String characterCardId) {
    return _copyCurrentThemeToPrefix(
      _characterCardThemePrefix(characterCardId),
    );
  }

  Future<void> cloneThemeBetweenCharacterCards(
    String sourceCharacterCardId,
    String targetCharacterCardId,
  ) {
    return _cloneThemeBetweenPrefixes(
      _characterCardThemePrefix(sourceCharacterCardId),
      _characterCardThemePrefix(targetCharacterCardId),
    );
  }

  Future<void> switchToCharacterCardTheme(String characterCardId) {
    return _switchToThemeByPrefix(_characterCardThemePrefix(characterCardId));
  }

  Future<void> saveCurrentThemeToCharacterCard(String characterCardId) {
    return copyCurrentThemeToCharacterCard(characterCardId);
  }

  Future<void> deleteCharacterCardTheme(String characterCardId) {
    return _deleteThemeByPrefix(_characterCardThemePrefix(characterCardId));
  }

  Future<bool> hasCharacterCardTheme(String characterCardId) {
    return _hasThemePrefix(_characterCardThemePrefix(characterCardId));
  }

  Future<void> copyCurrentThemeToCharacterGroup(String characterGroupId) {
    return _copyCurrentThemeToPrefix(
      _characterGroupThemePrefix(characterGroupId),
    );
  }

  Future<void> cloneThemeBetweenCharacterGroups(
    String sourceCharacterGroupId,
    String targetCharacterGroupId,
  ) {
    return _cloneThemeBetweenPrefixes(
      _characterGroupThemePrefix(sourceCharacterGroupId),
      _characterGroupThemePrefix(targetCharacterGroupId),
    );
  }

  Future<void> switchToCharacterGroupTheme(String characterGroupId) {
    return _switchToThemeByPrefix(_characterGroupThemePrefix(characterGroupId));
  }

  Future<void> saveCurrentThemeToCharacterGroup(String characterGroupId) {
    return copyCurrentThemeToCharacterGroup(characterGroupId);
  }

  Future<void> deleteCharacterGroupTheme(String characterGroupId) {
    return _deleteThemeByPrefix(_characterGroupThemePrefix(characterGroupId));
  }

  Future<bool> hasCharacterGroupTheme(String characterGroupId) {
    return _hasThemePrefix(_characterGroupThemePrefix(characterGroupId));
  }

  Future<void> _copyCurrentThemeToPrefix(String prefix) async {
    final values = await _getStrings(_themeKeys);
    await _setStrings(
      values.map((key, value) => MapEntry('$prefix$key', value)),
    );
  }

  Future<void> _cloneThemeBetweenPrefixes(
    String sourcePrefix,
    String targetPrefix,
  ) async {
    final sourceKeys = _prefixedThemeKeys(sourcePrefix);
    final values = await _getStrings(sourceKeys);
    await _setStrings(
      values.map(
        (key, value) => MapEntry(
          '$targetPrefix${key.substring(sourcePrefix.length)}',
          value,
        ),
      ),
    );
  }

  Future<void> _switchToThemeByPrefix(String prefix) async {
    final sourceKeys = _prefixedThemeKeys(prefix);
    final values = await _getStrings(sourceKeys);
    await _removeStrings(_themeKeys);
    await _setStrings(
      values.map((key, value) => MapEntry(key.substring(prefix.length), value)),
    );
  }

  Future<void> _deleteThemeByPrefix(String prefix) async {
    await _removeStrings(_prefixedThemeKeys(prefix));
  }

  Future<void> _resetMessageColorSettingsWithPrefix(String prefix) async {
    await _removeStrings(
      <String>[
        _CURSOR_USER_BUBBLE_COLOR,
        _BUBBLE_USER_BUBBLE_COLOR,
        _BUBBLE_AI_BUBBLE_COLOR,
        _BUBBLE_USER_TEXT_COLOR,
        _BUBBLE_AI_TEXT_COLOR,
      ].map((key) => '$prefix$key').toList(),
    );
  }

  Future<bool> _hasThemePrefix(String prefix) async {
    return _containsThemePrefix(
      await _getStrings(_prefixedThemeKeys(prefix)),
      prefix,
    );
  }

  Future<String?> _stringValue(String key, {String? prefix}) {
    return _getString(_keyWithPrefix(key, prefix));
  }

  Future<bool> _booleanValue(
    String key,
    bool defaultValue, {
    String? prefix,
  }) async {
    final value = await _stringValue(key, prefix: prefix);
    return value == null ? defaultValue : _decodeBool(value);
  }

  Future<Map<String, String>> _getStrings(List<String> keys) {
    return _clients.preferencesPreferenceStorageManager.getPreferences(
      fileName: _fileName,
      keys: keys,
    );
  }

  Future<void> _setStrings(Map<String, String> values) {
    return _clients.preferencesPreferenceStorageManager.setPreferences(
      fileName: _fileName,
      values: values,
    );
  }

  Future<void> _removeStrings(List<String> keys) {
    return _clients.preferencesPreferenceStorageManager.removePreferences(
      fileName: _fileName,
      keys: keys,
    );
  }

  Future<String?> _getString(String key) {
    return _clients.preferencesPreferenceStorageManager.getPreference(
      fileName: _fileName,
      key: key,
    );
  }

  Future<void> _setString(String key, String value) {
    return _clients.preferencesPreferenceStorageManager.setPreference(
      fileName: _fileName,
      key: key,
      value: value,
    );
  }
}

bool _decodeBool(String value) {
  return switch (value) {
    'true' => true,
    'false' => false,
    _ => throw FormatException('invalid boolean preference: $value'),
  };
}

String? _normalizedThemeId(String? value) {
  final normalized = value?.trim();
  return normalized == null || normalized.isEmpty ? null : normalized;
}

String _characterCardThemePrefix(String characterCardId) {
  return 'character_card_theme_${characterCardId}_';
}

String _characterGroupThemePrefix(String characterGroupId) {
  return 'character_group_theme_${characterGroupId}_';
}

String _keyWithPrefix(String key, String? prefix) {
  return prefix == null ? key : '$prefix$key';
}

List<String> _prefixedThemeKeys(String prefix) {
  return UserPreferencesManager._themeKeys.map((key) => '$prefix$key').toList();
}

bool _containsThemePrefix(Map<String, String> preferences, String prefix) {
  for (final key in UserPreferencesManager._themeKeys) {
    if (preferences.containsKey('$prefix$key')) {
      return true;
    }
  }
  return false;
}

String _profileKey(String profileId) {
  return 'profile_$profileId';
}

core_proxy.PreferenceProfile _createDefaultProfile(String profileId) {
  return core_proxy.PreferenceProfile(
    id: profileId,
    name: profileId == UserPreferencesManager.DEFAULT_PROFILE_ID
        ? 'Default'
        : profileId,
    birthDate: 0,
    gender: '',
    personality: '',
    identity: '',
    occupation: '',
    aiStyle: '',
    isInitialized: false,
  );
}
