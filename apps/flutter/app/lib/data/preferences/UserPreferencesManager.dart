// ignore_for_file: file_names, constant_identifier_names

import 'package:flutter/foundation.dart' show mapEquals;
import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';

export 'package:operit_ui/operit_ui.dart' show ThemePreferenceSnapshot;

class LongPastedTextInputSettings {
  /// Creates the global settings for converting long pasted text to attachments.
  const LongPastedTextInputSettings({
    required this.enabled,
    required this.threshold,
  });

  final bool enabled;
  final int threshold;
}

class UserPreferencesManager {
  const UserPreferencesManager({
    GeneratedCoreProxyClients clients = const GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(),
    ),
  }) : _clients = clients;

  final GeneratedCoreProxyClients _clients;

  static const String DEFAULT_PROFILE_ID = 'default';
  static const String MEDIA_TYPE_IMAGE = 'image';
  static const String MEDIA_TYPE_VIDEO = 'video';
  static const String CHAT_STYLE_CURSOR = 'cursor';
  static const String CHAT_STYLE_BUBBLE = 'bubble';
  static const String INPUT_STYLE_CLASSIC = 'classic';
  static const String INPUT_STYLE_AGENT = 'agent';

  /// Names the only built-in conversation tab, independent of old grouping values.
  static const String CHAT_SIDEBAR_WORKSPACE_TAB = 'workspace';
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
  static const int longPastedTextInputMinimumThreshold = 1000;
  static const int longPastedTextInputMaximumThreshold = 20000;
  static const int longPastedTextInputThresholdStep = 500;
  static const int defaultLongPastedTextInputThreshold = 3000;

  static final ValueNotifier<LongPastedTextInputSettings>
  longPastedTextInputSettings = ValueNotifier<LongPastedTextInputSettings>(
    const LongPastedTextInputSettings(
      enabled: true,
      threshold: defaultLongPastedTextInputThreshold,
    ),
  );

  static const String _fileName = 'user_preferences.preferences.json';
  static const String _KEY_CHAT_SIDEBAR_TAB = 'chat_sidebar_tab_v1';

  /// Maps the shared snapshot onto the existing Space preference keys.
  static const Map<String, String> _appearancePreferenceKeys = <String, String>{
    'themeMode': 'theme_mode',
    'themePaletteJson': 'theme_palette_json',
    'useCustomColors': 'use_custom_colors',
    'customPrimaryColor': 'custom_primary_color',
    'customSecondaryColor': 'custom_secondary_color',
    'useBackgroundImage': 'use_background_image',
    'backgroundImageUri': 'background_image_uri',
    'backgroundMediaType': 'background_media_type',
    'backgroundImageOpacity': 'background_image_opacity',
    'videoBackgroundMuted': 'video_background_muted',
    'videoBackgroundLoop': 'video_background_loop',
    'useBackgroundBlur': 'use_background_blur',
    'backgroundBlurRadius': 'background_blur_radius',
    'transparentSurfaceEnabled': 'transparent_surface_enabled',
    'chatInputFloating': 'chat_input_floating',
    'inputStyle': 'input_style',
    'inputBackgroundColor': 'input_background_color',
    'inputRoundedCornersEnabled': 'input_rounded_corners_enabled',
    'inputUseImage': 'input_use_image',
    'inputImageUri': 'input_image_uri',
    'inputImageRenderMode': 'input_image_render_mode',
    'inputImageCropLeft': 'input_image_crop_left',
    'inputImageCropTop': 'input_image_crop_top',
    'inputImageCropRight': 'input_image_crop_right',
    'inputImageCropBottom': 'input_image_crop_bottom',
    'inputImageRepeatStart': 'input_image_repeat_start',
    'inputImageRepeatEnd': 'input_image_repeat_end',
    'inputImageRepeatYStart': 'input_image_repeat_y_start',
    'inputImageRepeatYEnd': 'input_image_repeat_y_end',
    'inputImageScale': 'input_image_scale',
    'inputSettingsAsset': 'input_settings_asset',
    'inputSettingsAssetSize': 'input_settings_asset_size',
    'inputAttachmentAsset': 'input_attachment_asset',
    'inputAttachmentAssetSize': 'input_attachment_asset_size',
    'inputMicAsset': 'input_mic_asset',
    'inputMicAssetSize': 'input_mic_asset_size',
    'inputSendAsset': 'input_send_asset',
    'inputSendAssetSize': 'input_send_asset_size',
    'inputStopAsset': 'input_stop_asset',
    'inputStopAssetSize': 'input_stop_asset_size',
    'chatStyle': 'chat_style',
    'bubbleShowAvatar': 'bubble_show_avatar',
    'bubbleWideLayoutEnabled': 'bubble_wide_layout_enabled',
    'cursorUserBubbleColor': 'cursor_user_bubble_color',
    'bubbleUserBubbleColor': 'bubble_user_bubble_color',
    'bubbleAiBubbleColor': 'bubble_ai_bubble_color',
    'bubbleUserTextColor': 'bubble_user_text_color',
    'bubbleAiTextColor': 'bubble_ai_text_color',
    'bubbleUserUseImage': 'bubble_user_use_image',
    'bubbleAiUseImage': 'bubble_ai_use_image',
    'bubbleUserImageUri': 'bubble_user_image_uri',
    'bubbleAiImageUri': 'bubble_ai_image_uri',
    'bubbleUserImageRenderMode': 'bubble_user_image_render_mode',
    'bubbleAiImageRenderMode': 'bubble_ai_image_render_mode',
    'bubbleUserImageCropLeft': 'bubble_user_image_crop_left',
    'bubbleUserImageCropTop': 'bubble_user_image_crop_top',
    'bubbleUserImageCropRight': 'bubble_user_image_crop_right',
    'bubbleUserImageCropBottom': 'bubble_user_image_crop_bottom',
    'bubbleUserImageRepeatStart': 'bubble_user_image_repeat_start',
    'bubbleUserImageRepeatEnd': 'bubble_user_image_repeat_end',
    'bubbleUserImageRepeatYStart': 'bubble_user_image_repeat_y_start',
    'bubbleUserImageRepeatYEnd': 'bubble_user_image_repeat_y_end',
    'bubbleUserImageScale': 'bubble_user_image_scale',
    'bubbleAiImageCropLeft': 'bubble_ai_image_crop_left',
    'bubbleAiImageCropTop': 'bubble_ai_image_crop_top',
    'bubbleAiImageCropRight': 'bubble_ai_image_crop_right',
    'bubbleAiImageCropBottom': 'bubble_ai_image_crop_bottom',
    'bubbleAiImageRepeatStart': 'bubble_ai_image_repeat_start',
    'bubbleAiImageRepeatEnd': 'bubble_ai_image_repeat_end',
    'bubbleAiImageRepeatYStart': 'bubble_ai_image_repeat_y_start',
    'bubbleAiImageRepeatYEnd': 'bubble_ai_image_repeat_y_end',
    'bubbleAiImageScale': 'bubble_ai_image_scale',
    'bubbleUserRoundedCornersEnabled': 'bubble_rounded_corners_enabled',
    'bubbleAiRoundedCornersEnabled': 'bubble_ai_rounded_corners_enabled',
    'bubbleUserContentPaddingLeft': 'bubble_content_padding_left',
    'bubbleUserContentPaddingRight': 'bubble_content_padding_right',
    'bubbleAiContentPaddingLeft': 'bubble_ai_content_padding_left',
    'bubbleAiContentPaddingRight': 'bubble_ai_content_padding_right',
    'customUserAvatarUri': 'custom_user_avatar_uri',
    'avatarShape': 'avatar_shape',
    'avatarCornerRadius': 'avatar_corner_radius',
    'useCustomFont': 'use_custom_font',
    'fontType': 'font_type',
    'systemFontName': 'system_font_name',
    'customFontPath': 'custom_font_path',
    'fontScale': 'font_scale',
    'bubbleUserUseCustomFont': 'bubble_user_use_custom_font',
    'bubbleUserFontType': 'bubble_user_font_type',
    'bubbleUserSystemFontName': 'bubble_user_system_font_name',
    'bubbleUserCustomFontPath': 'bubble_user_custom_font_path',
    'bubbleAiUseCustomFont': 'bubble_ai_use_custom_font',
    'bubbleAiFontType': 'bubble_ai_font_type',
    'bubbleAiSystemFontName': 'bubble_ai_system_font_name',
    'bubbleAiCustomFontPath': 'bubble_ai_custom_font_path',
    'showThinkingProcess': 'show_thinking_process',
    'showModelProvider': 'show_model_provider',
    'showModelName': 'show_model_name',
    'showRoleName': 'show_role_name',
    'showUserName': 'show_user_name',
    'showMessageTokenStats': 'show_message_token_stats',
    'showMessageTimingStats': 'show_message_timing_stats',
    'showMessageTimestamp': 'show_message_timestamp',
    'showInputProcessingStatus': 'show_input_processing_status',
  };

  static const ThemePreferenceSnapshot defaultThemePreferenceSnapshot =
      ThemePreferenceDefaults.snapshot;

  static const String _KEY_LONG_PASTED_TEXT_INPUT_ENABLED =
      'long_pasted_text_input_enabled';
  static const String _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD =
      'long_pasted_text_input_threshold';

  /// Observes committed user preferences, including changes synchronized from peers.
  Stream<Map<String, Object?>> preferencesFlow() {
    return _clients.preferencesPreferenceStorageManager
        .preferencesFlow(fileName: _fileName)
        .distinct(mapEquals);
  }

  /// Observes and validates the global long-paste settings from the shared store.
  Stream<LongPastedTextInputSettings> longPastedTextInputSettingsFlow() {
    return preferencesFlow()
        .map(
          (values) => <String, Object?>{
            for (final key in <String>[
              _KEY_LONG_PASTED_TEXT_INPUT_ENABLED,
              _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD,
            ])
              if (values.containsKey(key)) key: values[key],
          },
        )
        .distinct(mapEquals)
        .map(_decodeLongPastedTextInputSettings);
  }

  /// Observes the new opaque tab selection without consulting legacy grouping keys.
  Stream<String> chatSidebarTabFlow() => preferencesFlow()
      .map((values) => _decodeChatSidebarTab(values[_KEY_CHAT_SIDEBAR_TAB]))
      .distinct();

  /// Defines workspace as the initial state of the new key and rejects corrupt values.
  String _decodeChatSidebarTab(Object? value) {
    if (value == null) {
      return CHAT_SIDEBAR_WORKSPACE_TAB;
    }
    if (value is! String) {
      throw const FormatException("chat sidebar tab must be a string");
    }
    if (value.trim().isEmpty) {
      throw const FormatException(
        'The persisted chat sidebar tab ID is blank.',
      );
    }
    return value;
  }

  /// Loads the opaque tab selection without translating legacy character preferences.
  Future<String> loadChatSidebarTab() async {
    final values = await _getValues(<String>[_KEY_CHAT_SIDEBAR_TAB]);
    return _decodeChatSidebarTab(values[_KEY_CHAT_SIDEBAR_TAB]);
  }

  /// Persists a caller-owned opaque tab ID without interpreting package input.
  Future<void> saveChatSidebarTab(String tabId) async {
    if (tabId.trim().isEmpty) {
      throw ArgumentError.value(tabId, 'tabId', 'must not be blank');
    }
    await _setValues(<String, Object?>{_KEY_CHAT_SIDEBAR_TAB: tabId});
  }

  /// Loads the global settings for converting long pasted text to attachments.
  Future<void> loadLongPastedTextInputSettings() async {
    final values = await _getValues(<String>[
      _KEY_LONG_PASTED_TEXT_INPUT_ENABLED,
      _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD,
    ]);
    longPastedTextInputSettings.value = _decodeLongPastedTextInputSettings(
      values,
    );
  }

  /// Decodes persisted long-paste settings without concealing malformed values.
  LongPastedTextInputSettings _decodeLongPastedTextInputSettings(
    Map<String, Object?> values,
  ) {
    final enabledValue = values[_KEY_LONG_PASTED_TEXT_INPUT_ENABLED];
    final thresholdValue = values[_KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD];
    final settings = LongPastedTextInputSettings(
      enabled: enabledValue == null ? true : _decodeBool(enabledValue),
      threshold: thresholdValue == null
          ? defaultLongPastedTextInputThreshold
          : thresholdValue as int,
    );
    if (settings.threshold < longPastedTextInputMinimumThreshold ||
        settings.threshold > longPastedTextInputMaximumThreshold) {
      throw StateError(
        'long pasted text threshold is outside the supported range: '
        '${settings.threshold}',
      );
    }
    return settings;
  }

  /// Persists the global settings for converting long pasted text to attachments.
  Future<void> saveLongPastedTextInputSettings({
    required bool enabled,
    required int threshold,
  }) async {
    if (threshold < longPastedTextInputMinimumThreshold ||
        threshold > longPastedTextInputMaximumThreshold) {
      throw ArgumentError.value(
        threshold,
        'threshold',
        'must be within the supported long pasted text range',
      );
    }
    final settings = LongPastedTextInputSettings(
      enabled: enabled,
      threshold: threshold,
    );
    await _setValues(<String, Object?>{
      _KEY_LONG_PASTED_TEXT_INPUT_ENABLED: enabled,
      _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD: threshold,
    });
    longPastedTextInputSettings.value = settings;
  }

  /// Reads personal appearance from the original synchronized Space preferences.
  Future<ThemePreferenceSnapshot> resolveThemePreferenceSnapshot() async {
    final preferences = await _getValues(<String>[
      ..._appearancePreferenceKeys.values,
      'use_system_theme',
    ]);
    final snapshot = defaultThemePreferenceSnapshot.toJson();
    for (final entry in _appearancePreferenceKeys.entries) {
      final value = preferences[entry.value];
      if (value != null) snapshot[entry.key] = value;
    }
    final resolved = ThemePreferenceSnapshot.fromJson(snapshot);
    final legacySystemTheme = preferences['use_system_theme'];
    if (legacySystemTheme == null) return resolved;
    if (_decodeBool(legacySystemTheme)) {
      snapshot['themeMode'] = ThemeMode.system.name;
    }
    await _clients.application.themeConfigManager().patchAppearance(
      initialSnapshot: defaultThemePreferenceSnapshot.toJson(),
      values: <String, Object?>{'theme_mode': snapshot['themeMode']},
      removeKeys: const <String>[],
    );
    return ThemePreferenceSnapshot.fromJson(snapshot);
  }

  /// Atomically patches shared appearance and its active saved configuration.
  Future<void> saveAppearanceFields(Map<String, Object?> updates) async {
    final values = <String, Object?>{};
    final removeKeys = <String>[];
    for (final entry in updates.entries) {
      final key = _appearancePreferenceKeys[entry.key];
      if (key == null)
        throw ArgumentError('Unknown appearance field: ${entry.key}');
      if (entry.value == null) {
        removeKeys.add(key);
      } else {
        values[key] = entry.value;
      }
    }
    await _clients.application.themeConfigManager().patchAppearance(
      initialSnapshot: defaultThemePreferenceSnapshot.toJson(),
      values: values,
      removeKeys: removeKeys,
    );
  }

  /// Atomically edits synchronized personal appearance using the released keys.
  Future<void> saveThemeSettings({
    ThemeMode? themeMode,
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
    String? inputStyle,
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
    await saveAppearanceFields(<String, Object?>{
      if (useCustomColors != null ||
          customPrimaryColor != null ||
          customSecondaryColor != null)
        'themePaletteJson': null,
      if (themeMode != null) 'themeMode': themeMode.name,
      if (useCustomColors != null) 'useCustomColors': useCustomColors,
      if (customPrimaryColor != null) 'customPrimaryColor': customPrimaryColor,
      if (customSecondaryColor != null)
        'customSecondaryColor': customSecondaryColor,
      if (useBackgroundImage != null) 'useBackgroundImage': useBackgroundImage,
      if (backgroundImageUri != null) 'backgroundImageUri': backgroundImageUri,
      if (backgroundImageOpacity != null)
        'backgroundImageOpacity': backgroundImageOpacity,
      if (backgroundMediaType != null)
        'backgroundMediaType': backgroundMediaType,
      if (videoBackgroundMuted != null)
        'videoBackgroundMuted': videoBackgroundMuted,
      if (videoBackgroundLoop != null)
        'videoBackgroundLoop': videoBackgroundLoop,
      if (useBackgroundBlur != null) 'useBackgroundBlur': useBackgroundBlur,
      if (backgroundBlurRadius != null)
        'backgroundBlurRadius': backgroundBlurRadius,
      if (transparentSurfaceEnabled != null)
        'transparentSurfaceEnabled': transparentSurfaceEnabled,
      if (chatInputFloating != null) 'chatInputFloating': chatInputFloating,
      if (inputStyle != null) 'inputStyle': inputStyle,
      if (chatStyle != null) 'chatStyle': chatStyle,
      if (bubbleShowAvatar != null) 'bubbleShowAvatar': bubbleShowAvatar,
      if (bubbleWideLayoutEnabled != null)
        'bubbleWideLayoutEnabled': bubbleWideLayoutEnabled,
      if (cursorUserBubbleColor != null)
        'cursorUserBubbleColor': cursorUserBubbleColor,
      if (bubbleUserBubbleColor != null)
        'bubbleUserBubbleColor': bubbleUserBubbleColor,
      if (bubbleAiBubbleColor != null)
        'bubbleAiBubbleColor': bubbleAiBubbleColor,
      if (bubbleUserTextColor != null)
        'bubbleUserTextColor': bubbleUserTextColor,
      if (bubbleAiTextColor != null) 'bubbleAiTextColor': bubbleAiTextColor,
      if (bubbleUserUseImage != null) 'bubbleUserUseImage': bubbleUserUseImage,
      if (bubbleAiUseImage != null) 'bubbleAiUseImage': bubbleAiUseImage,
      if (bubbleUserImageUri != null) 'bubbleUserImageUri': bubbleUserImageUri,
      if (bubbleAiImageUri != null) 'bubbleAiImageUri': bubbleAiImageUri,
      if (bubbleUserImageRenderMode != null)
        'bubbleUserImageRenderMode': bubbleUserImageRenderMode,
      if (bubbleAiImageRenderMode != null)
        'bubbleAiImageRenderMode': bubbleAiImageRenderMode,
      if (bubbleUserImageCropLeft != null)
        'bubbleUserImageCropLeft': bubbleUserImageCropLeft,
      if (bubbleUserImageCropTop != null)
        'bubbleUserImageCropTop': bubbleUserImageCropTop,
      if (bubbleUserImageCropRight != null)
        'bubbleUserImageCropRight': bubbleUserImageCropRight,
      if (bubbleUserImageCropBottom != null)
        'bubbleUserImageCropBottom': bubbleUserImageCropBottom,
      if (bubbleUserImageRepeatStart != null)
        'bubbleUserImageRepeatStart': bubbleUserImageRepeatStart,
      if (bubbleUserImageRepeatEnd != null)
        'bubbleUserImageRepeatEnd': bubbleUserImageRepeatEnd,
      if (bubbleUserImageRepeatYStart != null)
        'bubbleUserImageRepeatYStart': bubbleUserImageRepeatYStart,
      if (bubbleUserImageRepeatYEnd != null)
        'bubbleUserImageRepeatYEnd': bubbleUserImageRepeatYEnd,
      if (bubbleUserImageScale != null)
        'bubbleUserImageScale': bubbleUserImageScale,
      if (bubbleAiImageCropLeft != null)
        'bubbleAiImageCropLeft': bubbleAiImageCropLeft,
      if (bubbleAiImageCropTop != null)
        'bubbleAiImageCropTop': bubbleAiImageCropTop,
      if (bubbleAiImageCropRight != null)
        'bubbleAiImageCropRight': bubbleAiImageCropRight,
      if (bubbleAiImageCropBottom != null)
        'bubbleAiImageCropBottom': bubbleAiImageCropBottom,
      if (bubbleAiImageRepeatStart != null)
        'bubbleAiImageRepeatStart': bubbleAiImageRepeatStart,
      if (bubbleAiImageRepeatEnd != null)
        'bubbleAiImageRepeatEnd': bubbleAiImageRepeatEnd,
      if (bubbleAiImageRepeatYStart != null)
        'bubbleAiImageRepeatYStart': bubbleAiImageRepeatYStart,
      if (bubbleAiImageRepeatYEnd != null)
        'bubbleAiImageRepeatYEnd': bubbleAiImageRepeatYEnd,
      if (bubbleAiImageScale != null) 'bubbleAiImageScale': bubbleAiImageScale,
      if (bubbleUserRoundedCornersEnabled != null)
        'bubbleUserRoundedCornersEnabled': bubbleUserRoundedCornersEnabled,
      if (bubbleAiRoundedCornersEnabled != null)
        'bubbleAiRoundedCornersEnabled': bubbleAiRoundedCornersEnabled,
      if (bubbleUserContentPaddingLeft != null)
        'bubbleUserContentPaddingLeft': bubbleUserContentPaddingLeft,
      if (bubbleUserContentPaddingRight != null)
        'bubbleUserContentPaddingRight': bubbleUserContentPaddingRight,
      if (bubbleAiContentPaddingLeft != null)
        'bubbleAiContentPaddingLeft': bubbleAiContentPaddingLeft,
      if (bubbleAiContentPaddingRight != null)
        'bubbleAiContentPaddingRight': bubbleAiContentPaddingRight,
      if (showThinkingProcess != null)
        'showThinkingProcess': showThinkingProcess,
      if (showModelProvider != null) 'showModelProvider': showModelProvider,
      if (showModelName != null) 'showModelName': showModelName,
      if (showRoleName != null) 'showRoleName': showRoleName,
      if (showUserName != null) 'showUserName': showUserName,
      if (showMessageTokenStats != null)
        'showMessageTokenStats': showMessageTokenStats,
      if (showMessageTimingStats != null)
        'showMessageTimingStats': showMessageTimingStats,
      if (showMessageTimestamp != null)
        'showMessageTimestamp': showMessageTimestamp,
      if (avatarShape != null) 'avatarShape': avatarShape,
      if (avatarCornerRadius != null) 'avatarCornerRadius': avatarCornerRadius,
      if (showInputProcessingStatus != null)
        'showInputProcessingStatus': showInputProcessingStatus,
      if (useCustomFont != null) 'useCustomFont': useCustomFont,
      if (fontType != null) 'fontType': fontType,
      if (systemFontName != null) 'systemFontName': systemFontName,
      if (customFontPath != null) 'customFontPath': customFontPath,
      if (fontScale != null) 'fontScale': fontScale,
      if (bubbleUserUseCustomFont != null)
        'bubbleUserUseCustomFont': bubbleUserUseCustomFont,
      if (bubbleUserFontType != null) 'bubbleUserFontType': bubbleUserFontType,
      if (bubbleUserSystemFontName != null)
        'bubbleUserSystemFontName': bubbleUserSystemFontName,
      if (bubbleUserCustomFontPath != null)
        'bubbleUserCustomFontPath': bubbleUserCustomFontPath,
      if (bubbleAiUseCustomFont != null)
        'bubbleAiUseCustomFont': bubbleAiUseCustomFont,
      if (bubbleAiFontType != null) 'bubbleAiFontType': bubbleAiFontType,
      if (bubbleAiSystemFontName != null)
        'bubbleAiSystemFontName': bubbleAiSystemFontName,
      if (bubbleAiCustomFontPath != null)
        'bubbleAiCustomFontPath': bubbleAiCustomFontPath,
    });
  }

  /// Resets shared appearance while preserving the independent user avatar.
  Future<void> resetThemeSettings() => _clients.application
      .themeConfigManager()
      .patchAppearance(
        initialSnapshot: defaultThemePreferenceSnapshot.toJson(),
        values: const <String, Object?>{},
        removeKeys: <String>[
          for (final entry in _appearancePreferenceKeys.entries)
            if (entry.key != 'customUserAvatarUri') entry.value,
        ],
      )
      .then((_) {});

  /// Explicitly replaces unreadable appearance while retaining the user avatar and saved configurations.
  Future<void> restoreDefaultAppearance() async {
    final values = await _getValues(const <String>['custom_user_avatar_uri']);
    final snapshot = defaultThemePreferenceSnapshot.toJson();
    if (values.containsKey('custom_user_avatar_uri')) {
      snapshot['customUserAvatarUri'] = values['custom_user_avatar_uri'];
    }
    await _clients.application.themeConfigManager().useCustomAppearance(
      snapshot: snapshot,
    );
  }

  /// Saves the independent user avatar through the ordinary preference owner.
  Future<void> saveGlobalUserAvatarSettings({
    required String customUserAvatarUri,
  }) => saveAppearanceFields(<String, Object?>{
    'customUserAvatarUri': customUserAvatarUri,
  });

  Future<Map<String, Object?>> _getValues(List<String> keys) {
    return _clients.preferencesPreferenceStorageManager.getPreferences(
      fileName: _fileName,
      keys: keys,
    );
  }

  Future<void> _setValues(Map<String, Object?> values) {
    return _clients.preferencesPreferenceStorageManager.setPreferences(
      fileName: _fileName,
      values: values,
    );
  }
}

/// Decodes one strict boolean preference value.
bool _decodeBool(Object value) {
  return switch (value) {
    true => true,
    false => false,
    _ => throw FormatException('invalid boolean preference: $value'),
  };
}
