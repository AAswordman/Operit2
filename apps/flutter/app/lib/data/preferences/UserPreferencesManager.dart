// ignore_for_file: file_names, constant_identifier_names

import 'dart:convert';

import 'package:flutter/foundation.dart' show mapEquals;
import 'package:flutter/material.dart';
import 'package:operit_theme/operit_theme.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';

export 'package:operit_theme/operit_theme.dart' show ThemePreferenceSnapshot;

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

  static const ThemePreferenceSnapshot defaultThemePreferenceSnapshot =
      ThemePreferenceDefaults.snapshot;

  static const String _KEY_LONG_PASTED_TEXT_INPUT_ENABLED =
      'long_pasted_text_input_enabled';
  static const String _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD =
      'long_pasted_text_input_threshold';

  /// Observes committed user preferences, including changes synchronized from peers.
  Stream<Map<String, String>> preferencesFlow() {
    return _clients.preferencesPreferenceStorageManager
        .preferencesFlow(fileName: _fileName)
        .distinct(mapEquals);
  }

  /// Observes and validates the global long-paste settings from the shared store.
  Stream<LongPastedTextInputSettings> longPastedTextInputSettingsFlow() {
    return preferencesFlow()
        .map(
          (values) => <String, String>{
            for (final key in <String>[
              _KEY_LONG_PASTED_TEXT_INPUT_ENABLED,
              _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD,
            ])
              if (values[key] case final String value) key: value,
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
  String _decodeChatSidebarTab(String? value) {
    if (value == null) {
      return CHAT_SIDEBAR_WORKSPACE_TAB;
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
    final values = await _getStrings(<String>[_KEY_CHAT_SIDEBAR_TAB]);
    return _decodeChatSidebarTab(values[_KEY_CHAT_SIDEBAR_TAB]);
  }

  /// Persists a caller-owned opaque tab ID without interpreting package input.
  Future<void> saveChatSidebarTab(String tabId) async {
    if (tabId.trim().isEmpty) {
      throw ArgumentError.value(tabId, 'tabId', 'must not be blank');
    }
    await _setStrings(<String, String>{_KEY_CHAT_SIDEBAR_TAB: tabId});
  }

  /// Loads the global settings for converting long pasted text to attachments.
  Future<void> loadLongPastedTextInputSettings() async {
    final values = await _getStrings(<String>[
      _KEY_LONG_PASTED_TEXT_INPUT_ENABLED,
      _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD,
    ]);
    longPastedTextInputSettings.value = _decodeLongPastedTextInputSettings(
      values,
    );
  }

  /// Decodes persisted long-paste settings without concealing malformed values.
  LongPastedTextInputSettings _decodeLongPastedTextInputSettings(
    Map<String, String> values,
  ) {
    final enabledValue = values[_KEY_LONG_PASTED_TEXT_INPUT_ENABLED];
    final thresholdValue = values[_KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD];
    final settings = LongPastedTextInputSettings(
      enabled: enabledValue == null ? true : _decodeBool(enabledValue),
      threshold: thresholdValue == null
          ? defaultLongPastedTextInputThreshold
          : int.parse(thresholdValue),
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
    await _setStrings(<String, String>{
      _KEY_LONG_PASTED_TEXT_INPUT_ENABLED: enabled.toString(),
      _KEY_LONG_PASTED_TEXT_INPUT_THRESHOLD: threshold.toString(),
    });
    longPastedTextInputSettings.value = settings;
  }

  /// Reads only this CoreNode's global personal appearance. No role keys or migration.
  Future<ThemePreferenceSnapshot> resolveThemePreferenceSnapshot() async {
    final state = await _clients.application.nodeThemeState();
    final source = state.privateAppearanceJson;
    if (source == null) throw StateError('Node appearance has not been initialized');
    return ThemePreferenceSnapshot.fromJson(jsonDecode(source) as Map<String, dynamic>);
  }

  Future<void> saveNodeThemeSettings(Map<String, Object?> updates) async {
    await _clients.application.updateNodeThemeAppearance(
      updates: updates.map((key, value) => MapEntry(key, jsonEncode(value))),
    );
  }

  /// Atomically edits global node settings or the editable personal appearance.
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
    await saveNodeThemeSettings(<String, Object?>{
      if (themeMode != null) 'themeMode': themeMode!.name,
      if (useCustomColors != null) 'useCustomColors': useCustomColors,
      if (customPrimaryColor != null) 'customPrimaryColor': customPrimaryColor,
      if (customSecondaryColor != null) 'customSecondaryColor': customSecondaryColor,
      if (useBackgroundImage != null) 'useBackgroundImage': useBackgroundImage,
      if (backgroundImageUri != null) 'backgroundImageUri': backgroundImageUri,
      if (backgroundImageOpacity != null) 'backgroundImageOpacity': backgroundImageOpacity,
      if (backgroundMediaType != null) 'backgroundMediaType': backgroundMediaType,
      if (videoBackgroundMuted != null) 'videoBackgroundMuted': videoBackgroundMuted,
      if (videoBackgroundLoop != null) 'videoBackgroundLoop': videoBackgroundLoop,
      if (useBackgroundBlur != null) 'useBackgroundBlur': useBackgroundBlur,
      if (backgroundBlurRadius != null) 'backgroundBlurRadius': backgroundBlurRadius,
      if (transparentSurfaceEnabled != null) 'transparentSurfaceEnabled': transparentSurfaceEnabled,
      if (chatInputFloating != null) 'chatInputFloating': chatInputFloating,
      if (inputStyle != null) 'inputStyle': inputStyle,
      if (chatStyle != null) 'chatStyle': chatStyle,
      if (bubbleShowAvatar != null) 'bubbleShowAvatar': bubbleShowAvatar,
      if (bubbleWideLayoutEnabled != null) 'bubbleWideLayoutEnabled': bubbleWideLayoutEnabled,
      if (cursorUserBubbleColor != null) 'cursorUserBubbleColor': cursorUserBubbleColor,
      if (bubbleUserBubbleColor != null) 'bubbleUserBubbleColor': bubbleUserBubbleColor,
      if (bubbleAiBubbleColor != null) 'bubbleAiBubbleColor': bubbleAiBubbleColor,
      if (bubbleUserTextColor != null) 'bubbleUserTextColor': bubbleUserTextColor,
      if (bubbleAiTextColor != null) 'bubbleAiTextColor': bubbleAiTextColor,
      if (bubbleUserUseImage != null) 'bubbleUserUseImage': bubbleUserUseImage,
      if (bubbleAiUseImage != null) 'bubbleAiUseImage': bubbleAiUseImage,
      if (bubbleUserImageUri != null) 'bubbleUserImageUri': bubbleUserImageUri,
      if (bubbleAiImageUri != null) 'bubbleAiImageUri': bubbleAiImageUri,
      if (bubbleUserImageRenderMode != null) 'bubbleUserImageRenderMode': bubbleUserImageRenderMode,
      if (bubbleAiImageRenderMode != null) 'bubbleAiImageRenderMode': bubbleAiImageRenderMode,
      if (bubbleUserImageCropLeft != null) 'bubbleUserImageCropLeft': bubbleUserImageCropLeft,
      if (bubbleUserImageCropTop != null) 'bubbleUserImageCropTop': bubbleUserImageCropTop,
      if (bubbleUserImageCropRight != null) 'bubbleUserImageCropRight': bubbleUserImageCropRight,
      if (bubbleUserImageCropBottom != null) 'bubbleUserImageCropBottom': bubbleUserImageCropBottom,
      if (bubbleUserImageRepeatStart != null) 'bubbleUserImageRepeatStart': bubbleUserImageRepeatStart,
      if (bubbleUserImageRepeatEnd != null) 'bubbleUserImageRepeatEnd': bubbleUserImageRepeatEnd,
      if (bubbleUserImageRepeatYStart != null) 'bubbleUserImageRepeatYStart': bubbleUserImageRepeatYStart,
      if (bubbleUserImageRepeatYEnd != null) 'bubbleUserImageRepeatYEnd': bubbleUserImageRepeatYEnd,
      if (bubbleUserImageScale != null) 'bubbleUserImageScale': bubbleUserImageScale,
      if (bubbleAiImageCropLeft != null) 'bubbleAiImageCropLeft': bubbleAiImageCropLeft,
      if (bubbleAiImageCropTop != null) 'bubbleAiImageCropTop': bubbleAiImageCropTop,
      if (bubbleAiImageCropRight != null) 'bubbleAiImageCropRight': bubbleAiImageCropRight,
      if (bubbleAiImageCropBottom != null) 'bubbleAiImageCropBottom': bubbleAiImageCropBottom,
      if (bubbleAiImageRepeatStart != null) 'bubbleAiImageRepeatStart': bubbleAiImageRepeatStart,
      if (bubbleAiImageRepeatEnd != null) 'bubbleAiImageRepeatEnd': bubbleAiImageRepeatEnd,
      if (bubbleAiImageRepeatYStart != null) 'bubbleAiImageRepeatYStart': bubbleAiImageRepeatYStart,
      if (bubbleAiImageRepeatYEnd != null) 'bubbleAiImageRepeatYEnd': bubbleAiImageRepeatYEnd,
      if (bubbleAiImageScale != null) 'bubbleAiImageScale': bubbleAiImageScale,
      if (bubbleUserRoundedCornersEnabled != null) 'bubbleUserRoundedCornersEnabled': bubbleUserRoundedCornersEnabled,
      if (bubbleAiRoundedCornersEnabled != null) 'bubbleAiRoundedCornersEnabled': bubbleAiRoundedCornersEnabled,
      if (bubbleUserContentPaddingLeft != null) 'bubbleUserContentPaddingLeft': bubbleUserContentPaddingLeft,
      if (bubbleUserContentPaddingRight != null) 'bubbleUserContentPaddingRight': bubbleUserContentPaddingRight,
      if (bubbleAiContentPaddingLeft != null) 'bubbleAiContentPaddingLeft': bubbleAiContentPaddingLeft,
      if (bubbleAiContentPaddingRight != null) 'bubbleAiContentPaddingRight': bubbleAiContentPaddingRight,
      if (showThinkingProcess != null) 'showThinkingProcess': showThinkingProcess,
      if (showModelProvider != null) 'showModelProvider': showModelProvider,
      if (showModelName != null) 'showModelName': showModelName,
      if (showRoleName != null) 'showRoleName': showRoleName,
      if (showUserName != null) 'showUserName': showUserName,
      if (showMessageTokenStats != null) 'showMessageTokenStats': showMessageTokenStats,
      if (showMessageTimingStats != null) 'showMessageTimingStats': showMessageTimingStats,
      if (showMessageTimestamp != null) 'showMessageTimestamp': showMessageTimestamp,
      if (avatarShape != null) 'avatarShape': avatarShape,
      if (avatarCornerRadius != null) 'avatarCornerRadius': avatarCornerRadius,
      if (showInputProcessingStatus != null) 'showInputProcessingStatus': showInputProcessingStatus,
      if (useCustomFont != null) 'useCustomFont': useCustomFont,
      if (fontType != null) 'fontType': fontType,
      if (systemFontName != null) 'systemFontName': systemFontName,
      if (customFontPath != null) 'customFontPath': customFontPath,
      if (fontScale != null) 'fontScale': fontScale,
      if (bubbleUserUseCustomFont != null) 'bubbleUserUseCustomFont': bubbleUserUseCustomFont,
      if (bubbleUserFontType != null) 'bubbleUserFontType': bubbleUserFontType,
      if (bubbleUserSystemFontName != null) 'bubbleUserSystemFontName': bubbleUserSystemFontName,
      if (bubbleUserCustomFontPath != null) 'bubbleUserCustomFontPath': bubbleUserCustomFontPath,
      if (bubbleAiUseCustomFont != null) 'bubbleAiUseCustomFont': bubbleAiUseCustomFont,
      if (bubbleAiFontType != null) 'bubbleAiFontType': bubbleAiFontType,
      if (bubbleAiSystemFontName != null) 'bubbleAiSystemFontName': bubbleAiSystemFontName,
      if (bubbleAiCustomFontPath != null) 'bubbleAiCustomFontPath': bubbleAiCustomFontPath,
    });
  }

  Future<void> saveGlobalUserAvatarSettings({
    required String customUserAvatarUri,
  }) => saveNodeThemeSettings(<String, Object?>{'customUserAvatarUri': customUserAvatarUri});

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

}

/// Decodes one strict boolean preference value.
bool _decodeBool(String value) {
  return switch (value) {
    'true' => true,
    'false' => false,
    _ => throw FormatException('invalid boolean preference: $value'),
  };
}
