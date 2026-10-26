import 'package:flutter/material.dart';

class ThemePreferenceSnapshot {
  /// Creates the complete effective theme state for one UI surface.
  const ThemePreferenceSnapshot({
    required this.themeMode,
    this.themePaletteJson,
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
    required this.inputStyle,
    this.inputBackgroundColor,
    this.inputRoundedCornersEnabled = true,
    this.inputUseImage = false,
    this.inputImageUri,
    this.inputImageRenderMode = 'tiled_nine_slice',
    this.inputImageCropLeft = 0,
    this.inputImageCropTop = 0,
    this.inputImageCropRight = 0,
    this.inputImageCropBottom = 0,
    this.inputImageRepeatStart = 0.35,
    this.inputImageRepeatEnd = 0.65,
    this.inputImageRepeatYStart = 0.35,
    this.inputImageRepeatYEnd = 0.65,
    this.inputImageScale = 1,
    this.inputSettingsAsset,
    this.inputSettingsAssetSize = 18,
    this.inputAttachmentAsset,
    this.inputAttachmentAssetSize = 18,
    this.inputMicAsset,
    this.inputMicAssetSize = 26,
    this.inputSendAsset,
    this.inputSendAssetSize = 26,
    this.inputStopAsset,
    this.inputStopAssetSize = 26,
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

  final ThemeMode themeMode;
  final String? themePaletteJson;
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
  final String inputStyle;
  final int? inputBackgroundColor;
  final bool inputRoundedCornersEnabled;
  final bool inputUseImage;
  final String? inputImageUri;
  final String inputImageRenderMode;
  final double inputImageCropLeft;
  final double inputImageCropTop;
  final double inputImageCropRight;
  final double inputImageCropBottom;
  final double inputImageRepeatStart;
  final double inputImageRepeatEnd;
  final double inputImageRepeatYStart;
  final double inputImageRepeatYEnd;
  final double inputImageScale;
  final String? inputSettingsAsset;
  final double inputSettingsAssetSize;
  final String? inputAttachmentAsset;
  final double inputAttachmentAssetSize;
  final String? inputMicAsset;
  final double inputMicAssetSize;
  final String? inputSendAsset;
  final double inputSendAssetSize;
  final String? inputStopAsset;
  final double inputStopAssetSize;
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

  /// Encodes the complete effective theme state for another Flutter engine.
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'themeMode': themeMode.name,
      'themePaletteJson': themePaletteJson,
      'useCustomColors': useCustomColors,
      'customPrimaryColor': customPrimaryColor,
      'customSecondaryColor': customSecondaryColor,
      'useBackgroundImage': useBackgroundImage,
      'backgroundImageUri': backgroundImageUri,
      'backgroundMediaType': backgroundMediaType,
      'backgroundImageOpacity': backgroundImageOpacity,
      'videoBackgroundMuted': videoBackgroundMuted,
      'videoBackgroundLoop': videoBackgroundLoop,
      'useBackgroundBlur': useBackgroundBlur,
      'backgroundBlurRadius': backgroundBlurRadius,
      'transparentSurfaceEnabled': transparentSurfaceEnabled,
      'chatInputFloating': chatInputFloating,
      'inputStyle': inputStyle,
      'inputBackgroundColor': inputBackgroundColor,
      'inputRoundedCornersEnabled': inputRoundedCornersEnabled,
      'inputUseImage': inputUseImage,
      'inputImageUri': inputImageUri,
      'inputImageRenderMode': inputImageRenderMode,
      'inputImageCropLeft': inputImageCropLeft,
      'inputImageCropTop': inputImageCropTop,
      'inputImageCropRight': inputImageCropRight,
      'inputImageCropBottom': inputImageCropBottom,
      'inputImageRepeatStart': inputImageRepeatStart,
      'inputImageRepeatEnd': inputImageRepeatEnd,
      'inputImageRepeatYStart': inputImageRepeatYStart,
      'inputImageRepeatYEnd': inputImageRepeatYEnd,
      'inputImageScale': inputImageScale,
      'inputSettingsAsset': inputSettingsAsset,
      'inputSettingsAssetSize': inputSettingsAssetSize,
      'inputAttachmentAsset': inputAttachmentAsset,
      'inputAttachmentAssetSize': inputAttachmentAssetSize,
      'inputMicAsset': inputMicAsset,
      'inputMicAssetSize': inputMicAssetSize,
      'inputSendAsset': inputSendAsset,
      'inputSendAssetSize': inputSendAssetSize,
      'inputStopAsset': inputStopAsset,
      'inputStopAssetSize': inputStopAssetSize,
      'chatStyle': chatStyle,
      'bubbleShowAvatar': bubbleShowAvatar,
      'bubbleWideLayoutEnabled': bubbleWideLayoutEnabled,
      'cursorUserBubbleColor': cursorUserBubbleColor,
      'bubbleUserBubbleColor': bubbleUserBubbleColor,
      'bubbleAiBubbleColor': bubbleAiBubbleColor,
      'bubbleUserTextColor': bubbleUserTextColor,
      'bubbleAiTextColor': bubbleAiTextColor,
      'bubbleUserUseImage': bubbleUserUseImage,
      'bubbleAiUseImage': bubbleAiUseImage,
      'bubbleUserImageUri': bubbleUserImageUri,
      'bubbleAiImageUri': bubbleAiImageUri,
      'bubbleUserImageRenderMode': bubbleUserImageRenderMode,
      'bubbleAiImageRenderMode': bubbleAiImageRenderMode,
      'bubbleUserImageCropLeft': bubbleUserImageCropLeft,
      'bubbleUserImageCropTop': bubbleUserImageCropTop,
      'bubbleUserImageCropRight': bubbleUserImageCropRight,
      'bubbleUserImageCropBottom': bubbleUserImageCropBottom,
      'bubbleUserImageRepeatStart': bubbleUserImageRepeatStart,
      'bubbleUserImageRepeatEnd': bubbleUserImageRepeatEnd,
      'bubbleUserImageRepeatYStart': bubbleUserImageRepeatYStart,
      'bubbleUserImageRepeatYEnd': bubbleUserImageRepeatYEnd,
      'bubbleUserImageScale': bubbleUserImageScale,
      'bubbleAiImageCropLeft': bubbleAiImageCropLeft,
      'bubbleAiImageCropTop': bubbleAiImageCropTop,
      'bubbleAiImageCropRight': bubbleAiImageCropRight,
      'bubbleAiImageCropBottom': bubbleAiImageCropBottom,
      'bubbleAiImageRepeatStart': bubbleAiImageRepeatStart,
      'bubbleAiImageRepeatEnd': bubbleAiImageRepeatEnd,
      'bubbleAiImageRepeatYStart': bubbleAiImageRepeatYStart,
      'bubbleAiImageRepeatYEnd': bubbleAiImageRepeatYEnd,
      'bubbleAiImageScale': bubbleAiImageScale,
      'bubbleUserRoundedCornersEnabled': bubbleUserRoundedCornersEnabled,
      'bubbleAiRoundedCornersEnabled': bubbleAiRoundedCornersEnabled,
      'bubbleUserContentPaddingLeft': bubbleUserContentPaddingLeft,
      'bubbleUserContentPaddingRight': bubbleUserContentPaddingRight,
      'bubbleAiContentPaddingLeft': bubbleAiContentPaddingLeft,
      'bubbleAiContentPaddingRight': bubbleAiContentPaddingRight,
      'customUserAvatarUri': customUserAvatarUri,
      'avatarShape': avatarShape,
      'avatarCornerRadius': avatarCornerRadius,
      'useCustomFont': useCustomFont,
      'fontType': fontType,
      'systemFontName': systemFontName,
      'customFontPath': customFontPath,
      'fontScale': fontScale,
      'bubbleUserUseCustomFont': bubbleUserUseCustomFont,
      'bubbleUserFontType': bubbleUserFontType,
      'bubbleUserSystemFontName': bubbleUserSystemFontName,
      'bubbleUserCustomFontPath': bubbleUserCustomFontPath,
      'bubbleAiUseCustomFont': bubbleAiUseCustomFont,
      'bubbleAiFontType': bubbleAiFontType,
      'bubbleAiSystemFontName': bubbleAiSystemFontName,
      'bubbleAiCustomFontPath': bubbleAiCustomFontPath,
      'showThinkingProcess': showThinkingProcess,
      'showModelProvider': showModelProvider,
      'showModelName': showModelName,
      'showRoleName': showRoleName,
      'showUserName': showUserName,
      'showMessageTokenStats': showMessageTokenStats,
      'showMessageTimingStats': showMessageTimingStats,
      'showMessageTimestamp': showMessageTimestamp,
      'showInputProcessingStatus': showInputProcessingStatus,
    };
  }

  /// Decodes a serialized effective theme state from another Flutter engine.
  factory ThemePreferenceSnapshot.fromJson(Map<String, Object?> json) {
    /// Converts an optional JSON number into a nullable integer.
    int? optionalInt(String key) {
      final value = json[key];
      return value == null ? null : (value as num).toInt();
    }

    /// Converts a required JSON number into a double.
    double requiredDouble(String key) => (json[key] as num).toDouble();

    /// Reads one required string from the serialized snapshot.
    String requiredString(String key) => json[key] as String;

    /// Reads one optional string from the serialized snapshot.
    String? optionalString(String key) => json[key] as String?;

    /// Reads one required boolean from the serialized snapshot.
    bool requiredBool(String key) => json[key] as bool;

    return ThemePreferenceSnapshot(
      themeMode: ThemeMode.values.byName(requiredString('themeMode')),
      themePaletteJson: optionalString('themePaletteJson'),
      useCustomColors: requiredBool('useCustomColors'),
      customPrimaryColor: optionalInt('customPrimaryColor'),
      customSecondaryColor: optionalInt('customSecondaryColor'),
      useBackgroundImage: requiredBool('useBackgroundImage'),
      backgroundImageUri: optionalString('backgroundImageUri'),
      backgroundMediaType: requiredString('backgroundMediaType'),
      backgroundImageOpacity: requiredDouble('backgroundImageOpacity'),
      videoBackgroundMuted: requiredBool('videoBackgroundMuted'),
      videoBackgroundLoop: requiredBool('videoBackgroundLoop'),
      useBackgroundBlur: requiredBool('useBackgroundBlur'),
      backgroundBlurRadius: requiredDouble('backgroundBlurRadius'),
      transparentSurfaceEnabled: requiredBool('transparentSurfaceEnabled'),
      chatInputFloating: requiredBool('chatInputFloating'),
      inputStyle: requiredString('inputStyle'),
      inputBackgroundColor: optionalInt('inputBackgroundColor'),
      inputRoundedCornersEnabled:
          (json['inputRoundedCornersEnabled'] as bool?) ?? true,
      inputUseImage: (json['inputUseImage'] as bool?) ?? false,
      inputImageUri: optionalString('inputImageUri'),
      inputImageRenderMode:
          (json['inputImageRenderMode'] as String?) ?? 'tiled_nine_slice',
      inputImageCropLeft: (json['inputImageCropLeft'] as num?)?.toDouble() ?? 0,
      inputImageCropTop: (json['inputImageCropTop'] as num?)?.toDouble() ?? 0,
      inputImageCropRight:
          (json['inputImageCropRight'] as num?)?.toDouble() ?? 0,
      inputImageCropBottom:
          (json['inputImageCropBottom'] as num?)?.toDouble() ?? 0,
      inputImageRepeatStart:
          (json['inputImageRepeatStart'] as num?)?.toDouble() ?? 0.35,
      inputImageRepeatEnd:
          (json['inputImageRepeatEnd'] as num?)?.toDouble() ?? 0.65,
      inputImageRepeatYStart:
          (json['inputImageRepeatYStart'] as num?)?.toDouble() ?? 0.35,
      inputImageRepeatYEnd:
          (json['inputImageRepeatYEnd'] as num?)?.toDouble() ?? 0.65,
      inputImageScale: (json['inputImageScale'] as num?)?.toDouble() ?? 1,
      inputSettingsAsset: optionalString('inputSettingsAsset'),
      inputSettingsAssetSize:
          (json['inputSettingsAssetSize'] as num?)?.toDouble() ?? 18,
      inputAttachmentAsset: optionalString('inputAttachmentAsset'),
      inputAttachmentAssetSize:
          (json['inputAttachmentAssetSize'] as num?)?.toDouble() ?? 18,
      inputMicAsset: optionalString('inputMicAsset'),
      inputMicAssetSize: (json['inputMicAssetSize'] as num?)?.toDouble() ?? 26,
      inputSendAsset: optionalString('inputSendAsset'),
      inputSendAssetSize:
          (json['inputSendAssetSize'] as num?)?.toDouble() ?? 26,
      inputStopAsset: optionalString('inputStopAsset'),
      inputStopAssetSize:
          (json['inputStopAssetSize'] as num?)?.toDouble() ?? 26,
      chatStyle: requiredString('chatStyle'),
      bubbleShowAvatar: requiredBool('bubbleShowAvatar'),
      bubbleWideLayoutEnabled: requiredBool('bubbleWideLayoutEnabled'),
      cursorUserBubbleColor: optionalInt('cursorUserBubbleColor'),
      bubbleUserBubbleColor: optionalInt('bubbleUserBubbleColor'),
      bubbleAiBubbleColor: optionalInt('bubbleAiBubbleColor'),
      bubbleUserTextColor: optionalInt('bubbleUserTextColor'),
      bubbleAiTextColor: optionalInt('bubbleAiTextColor'),
      bubbleUserUseImage: requiredBool('bubbleUserUseImage'),
      bubbleAiUseImage: requiredBool('bubbleAiUseImage'),
      bubbleUserImageUri: optionalString('bubbleUserImageUri'),
      bubbleAiImageUri: optionalString('bubbleAiImageUri'),
      bubbleUserImageRenderMode: requiredString('bubbleUserImageRenderMode'),
      bubbleAiImageRenderMode: requiredString('bubbleAiImageRenderMode'),
      bubbleUserImageCropLeft: requiredDouble('bubbleUserImageCropLeft'),
      bubbleUserImageCropTop: requiredDouble('bubbleUserImageCropTop'),
      bubbleUserImageCropRight: requiredDouble('bubbleUserImageCropRight'),
      bubbleUserImageCropBottom: requiredDouble('bubbleUserImageCropBottom'),
      bubbleUserImageRepeatStart: requiredDouble('bubbleUserImageRepeatStart'),
      bubbleUserImageRepeatEnd: requiredDouble('bubbleUserImageRepeatEnd'),
      bubbleUserImageRepeatYStart: requiredDouble(
        'bubbleUserImageRepeatYStart',
      ),
      bubbleUserImageRepeatYEnd: requiredDouble('bubbleUserImageRepeatYEnd'),
      bubbleUserImageScale: requiredDouble('bubbleUserImageScale'),
      bubbleAiImageCropLeft: requiredDouble('bubbleAiImageCropLeft'),
      bubbleAiImageCropTop: requiredDouble('bubbleAiImageCropTop'),
      bubbleAiImageCropRight: requiredDouble('bubbleAiImageCropRight'),
      bubbleAiImageCropBottom: requiredDouble('bubbleAiImageCropBottom'),
      bubbleAiImageRepeatStart: requiredDouble('bubbleAiImageRepeatStart'),
      bubbleAiImageRepeatEnd: requiredDouble('bubbleAiImageRepeatEnd'),
      bubbleAiImageRepeatYStart: requiredDouble('bubbleAiImageRepeatYStart'),
      bubbleAiImageRepeatYEnd: requiredDouble('bubbleAiImageRepeatYEnd'),
      bubbleAiImageScale: requiredDouble('bubbleAiImageScale'),
      bubbleUserRoundedCornersEnabled: requiredBool(
        'bubbleUserRoundedCornersEnabled',
      ),
      bubbleAiRoundedCornersEnabled: requiredBool(
        'bubbleAiRoundedCornersEnabled',
      ),
      bubbleUserContentPaddingLeft: requiredDouble(
        'bubbleUserContentPaddingLeft',
      ),
      bubbleUserContentPaddingRight: requiredDouble(
        'bubbleUserContentPaddingRight',
      ),
      bubbleAiContentPaddingLeft: requiredDouble('bubbleAiContentPaddingLeft'),
      bubbleAiContentPaddingRight: requiredDouble(
        'bubbleAiContentPaddingRight',
      ),
      customUserAvatarUri: optionalString('customUserAvatarUri'),
      avatarShape: requiredString('avatarShape'),
      avatarCornerRadius: requiredDouble('avatarCornerRadius'),
      useCustomFont: requiredBool('useCustomFont'),
      fontType: requiredString('fontType'),
      systemFontName: optionalString('systemFontName'),
      customFontPath: optionalString('customFontPath'),
      fontScale: requiredDouble('fontScale'),
      bubbleUserUseCustomFont: requiredBool('bubbleUserUseCustomFont'),
      bubbleUserFontType: requiredString('bubbleUserFontType'),
      bubbleUserSystemFontName: optionalString('bubbleUserSystemFontName'),
      bubbleUserCustomFontPath: optionalString('bubbleUserCustomFontPath'),
      bubbleAiUseCustomFont: requiredBool('bubbleAiUseCustomFont'),
      bubbleAiFontType: requiredString('bubbleAiFontType'),
      bubbleAiSystemFontName: optionalString('bubbleAiSystemFontName'),
      bubbleAiCustomFontPath: optionalString('bubbleAiCustomFontPath'),
      showThinkingProcess: requiredBool('showThinkingProcess'),
      showModelProvider: requiredBool('showModelProvider'),
      showModelName: requiredBool('showModelName'),
      showRoleName: requiredBool('showRoleName'),
      showUserName: requiredBool('showUserName'),
      showMessageTokenStats: requiredBool('showMessageTokenStats'),
      showMessageTimingStats: requiredBool('showMessageTimingStats'),
      showMessageTimestamp: requiredBool('showMessageTimestamp'),
      showInputProcessingStatus: requiredBool('showInputProcessingStatus'),
    );
  }
}
