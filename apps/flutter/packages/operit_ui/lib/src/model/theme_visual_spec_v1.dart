import 'dart:convert';

import 'package:flutter/material.dart';

import 'theme_preference_defaults.dart';
import 'theme_preference_snapshot.dart';

/// Projects a validated template into shared renderer inputs and editable appearance defaults.
class ThemeVisualSpecV1 {
  ThemeVisualSpecV1.fromCheckedContract(Map<String, Object?> json)
    : _json = jsonDecode(jsonEncode(json)) as Map<String, dynamic> {}

  final Map<String, Object?> _json;

  Map<String, dynamic> _section(String key) =>
      _json[key] as Map<String, dynamic>;

  /// Projects the declared palette into the renderer's complete color scheme.
  ColorScheme colorScheme(Brightness brightness) =>
      colorSchemeFromPalette(_section('palette'), brightness);

  /// Projects a saved configuration palette without depending on a ToolPkg.
  static ColorScheme colorSchemeFromPalette(
    Map<String, dynamic> palette,
    Brightness brightness,
  ) {
    final colors = palette[brightness.name] as Map<String, dynamic>;
    Color color(String role) => Color(_argb(colors[role] as String));
    return ColorScheme(
      brightness: brightness,
      primary: color('primary'),
      onPrimary: color('onPrimary'),
      primaryContainer: color('primaryContainer'),
      onPrimaryContainer: color('onPrimaryContainer'),
      primaryFixed: color('primaryFixed'),
      primaryFixedDim: color('primaryFixedDim'),
      onPrimaryFixed: color('onPrimaryFixed'),
      onPrimaryFixedVariant: color('onPrimaryFixedVariant'),
      secondary: color('secondary'),
      onSecondary: color('onSecondary'),
      secondaryContainer: color('secondaryContainer'),
      onSecondaryContainer: color('onSecondaryContainer'),
      secondaryFixed: color('secondaryFixed'),
      secondaryFixedDim: color('secondaryFixedDim'),
      onSecondaryFixed: color('onSecondaryFixed'),
      onSecondaryFixedVariant: color('onSecondaryFixedVariant'),
      tertiary: color('tertiary'),
      onTertiary: color('onTertiary'),
      tertiaryContainer: color('tertiaryContainer'),
      onTertiaryContainer: color('onTertiaryContainer'),
      tertiaryFixed: color('tertiaryFixed'),
      tertiaryFixedDim: color('tertiaryFixedDim'),
      onTertiaryFixed: color('onTertiaryFixed'),
      onTertiaryFixedVariant: color('onTertiaryFixedVariant'),
      error: color('error'),
      onError: color('onError'),
      errorContainer: color('errorContainer'),
      onErrorContainer: color('onErrorContainer'),
      surface: color('surface'),
      onSurface: color('onSurface'),
      surfaceDim: color('surfaceDim'),
      surfaceBright: color('surfaceBright'),
      surfaceContainerLowest: color('surfaceContainerLowest'),
      surfaceContainerLow: color('surfaceContainerLow'),
      surfaceContainer: color('surfaceContainer'),
      surfaceContainerHigh: color('surfaceContainerHigh'),
      surfaceContainerHighest: color('surfaceContainerHighest'),
      onSurfaceVariant: color('onSurfaceVariant'),
      outline: color('outline'),
      outlineVariant: color('outlineVariant'),
      shadow: color('shadow'),
      scrim: color('scrim'),
      inverseSurface: color('inverseSurface'),
      onInverseSurface: color('onInverseSurface'),
      inversePrimary: color('inversePrimary'),
      surfaceTint: color('surfaceTint'),
      // v1 fixes deprecated role aliases as well; SDK getters must not invent
      // another mapping for older application widgets that still read them.
      // ignore: deprecated_member_use
      background: color('surface'),
      // ignore: deprecated_member_use
      onBackground: color('onSurface'),
      // ignore: deprecated_member_use
      surfaceVariant: color('surfaceContainerHighest'),
    );
  }

  /// Projects editable template defaults while retaining user mode, scaling, avatars and visibility.
  /// Complete palette data stays part of the independent appearance snapshot.
  ThemePreferenceSnapshot toSnapshot(ThemePreferenceSnapshot userSettings) {
    final userJson = userSettings.toJson();
    final background = _section('background');
    final blur = background['blur'] as Map<String, dynamic>;
    final chat = _section('chat');
    final input = _section('input');
    final typography = _section('typography');
    final result = <String, Object?>{
      ...userJson,
      'themePaletteJson': jsonEncode(_section('palette')),
      'useCustomColors': true,
      'customPrimaryColor': colorScheme(Brightness.light).primary.toARGB32(),
      'customSecondaryColor': colorScheme(
        Brightness.light,
      ).secondary.toARGB32(),
      'useBackgroundImage': background['asset'] != null,
      'backgroundImageUri': background['asset'],
      'backgroundMediaType': ThemePreferenceDefaults.MEDIA_TYPE_IMAGE,
      'backgroundImageOpacity': background['opacity'],
      'videoBackgroundMuted': true,
      'videoBackgroundLoop': true,
      'useBackgroundBlur': blur['enabled'],
      'backgroundBlurRadius': blur['radius'],
      'transparentSurfaceEnabled': _section('surfaces')['transparent'],
      'chatStyle': chat['style'],
      'bubbleWideLayoutEnabled': chat['wideLayout'],
      'bubbleShowAvatar': chat['showAvatar'],
      'avatarShape': chat['avatarShape'],
      'avatarCornerRadius': chat['avatarCornerRadius'],
      'cursorUserBubbleColor': _nullableArgb(chat['cursorUserColor']),
      'chatInputFloating': input['floating'],
      'inputStyle': input['style'],
      'inputBackgroundColor': _nullableArgb(input['color']),
      'inputRoundedCornersEnabled': (input['roundedCorners'] as bool?) ?? true,
      'inputSettingsAsset': input['settingsAsset'] as String?,
      'inputSettingsAssetSize':
          (input['settingsAssetSize'] as num?)?.toDouble() ?? 18,
      'inputAttachmentAsset': input['attachmentAsset'] as String?,
      'inputAttachmentAssetSize':
          (input['attachmentAssetSize'] as num?)?.toDouble() ?? 18,
      'inputMicAsset': input['micAsset'] as String?,
      'inputMicAssetSize': (input['micAssetSize'] as num?)?.toDouble() ?? 26,
      'inputSendAsset': input['sendAsset'] as String?,
      'inputSendAssetSize': (input['sendAssetSize'] as num?)?.toDouble() ?? 26,
      'inputStopAsset': input['stopAsset'] as String?,
      'inputStopAssetSize': (input['stopAssetSize'] as num?)?.toDouble() ?? 26,
    };
    final inputImage = input['image'] as Map<String, dynamic>?;
    result['inputUseImage'] = inputImage != null;
    result['inputImageUri'] = inputImage?['asset'];
    if (inputImage == null) {
      result.addAll(<String, Object?>{
        'inputImageRenderMode': 'tiled_nine_slice',
        'inputImageCropLeft': 0,
        'inputImageCropTop': 0,
        'inputImageCropRight': 0,
        'inputImageCropBottom': 0,
        'inputImageRepeatStart': 0.35,
        'inputImageRepeatEnd': 0.65,
        'inputImageRepeatYStart': 0.35,
        'inputImageRepeatYEnd': 0.65,
        'inputImageScale': 1,
      });
    } else {
      final crop = inputImage['crop'] as Map<String, dynamic>;
      final slice = inputImage['slice'] as Map<String, dynamic>;
      result.addAll(<String, Object?>{
        'inputImageRenderMode': inputImage['renderMode'],
        'inputImageCropLeft': crop['left'],
        'inputImageCropTop': crop['top'],
        'inputImageCropRight': crop['right'],
        'inputImageCropBottom': crop['bottom'],
        'inputImageRepeatStart': slice['xStart'],
        'inputImageRepeatEnd': slice['xEnd'],
        'inputImageRepeatYStart': slice['yStart'],
        'inputImageRepeatYEnd': slice['yEnd'],
        'inputImageScale': inputImage['scale'],
      });
    }
    for (final role in <String>['user', 'ai']) {
      final prefix = role == 'user' ? 'bubbleUser' : 'bubbleAi';
      final bubble = chat[role] as Map<String, dynamic>;
      final image = bubble['image'] as Map<String, dynamic>?;
      final padding = bubble['padding'] as Map<String, dynamic>;
      result.addAll(<String, Object?>{
        '${prefix}BubbleColor': _nullableArgb(bubble['color']),
        '${prefix}TextColor': _nullableArgb(bubble['textColor']),
        '${prefix}UseImage': image != null,
        '${prefix}ImageUri': image?['asset'],
        '${prefix}RoundedCornersEnabled': bubble['roundedCorners'],
        '${prefix}ContentPaddingLeft': padding['left'],
        '${prefix}ContentPaddingRight': padding['right'],
      });
      if (image == null) {
        // Null declares "no image"; the inactive transport slots use v1 defaults.
        result.addAll(<String, Object?>{
          '${prefix}ImageRenderMode': 'tiled_nine_slice',
          '${prefix}ImageCropLeft': 0,
          '${prefix}ImageCropTop': 0,
          '${prefix}ImageCropRight': 0,
          '${prefix}ImageCropBottom': 0,
          '${prefix}ImageRepeatStart': 0.35,
          '${prefix}ImageRepeatEnd': 0.65,
          '${prefix}ImageRepeatYStart': 0.35,
          '${prefix}ImageRepeatYEnd': 0.65,
          '${prefix}ImageScale': 1,
        });
      } else {
        final crop = image['crop'] as Map<String, dynamic>;
        final slice = image['slice'] as Map<String, dynamic>;
        result.addAll(<String, Object?>{
          '${prefix}ImageRenderMode': image['renderMode'],
          '${prefix}ImageCropLeft': crop['left'],
          '${prefix}ImageCropTop': crop['top'],
          '${prefix}ImageCropRight': crop['right'],
          '${prefix}ImageCropBottom': crop['bottom'],
          '${prefix}ImageRepeatStart': slice['xStart'],
          '${prefix}ImageRepeatEnd': slice['xEnd'],
          '${prefix}ImageRepeatYStart': slice['yStart'],
          '${prefix}ImageRepeatYEnd': slice['yEnd'],
          '${prefix}ImageScale': image['scale'],
        });
      }
    }
    _setFont(result, '', typography['global'] as String);
    _setFont(result, 'bubbleUser', typography['user'] as String?);
    _setFont(result, 'bubbleAi', typography['ai'] as String?);
    return ThemePreferenceSnapshot.fromJson(result);
  }

  Map<String, String?> get fontReferences => <String, String?>{
    'global': _section('typography')['global'] as String,
    'user': _section('typography')['user'] as String?,
    'ai': _section('typography')['ai'] as String?,
  };

  /// Returns the package-relative media references used by this normalized
  /// visual state. The Rust contract remains the authority for package
  /// closure; this projection only lets a renderer request the exact bytes it
  /// has already received from that contract boundary.
  Set<String> get assetReferences {
    final references = <String>{};
    final background = _section('background')['asset'];
    if (background is String && background.startsWith('assets/')) {
      references.add(background);
    }
    final chat = _section('chat');
    for (final role in <String>['user', 'ai']) {
      final image = (chat[role] as Map<String, dynamic>)['image'];
      if (image is Map<String, dynamic>) {
        final asset = image['asset'];
        if (asset is String && asset.startsWith('assets/')) {
          references.add(asset);
        }
      }
    }
    final input = _section('input');
    final inputImage = input['image'];
    if (inputImage is Map<String, dynamic>) {
      final asset = inputImage['asset'];
      if (asset is String && asset.startsWith('assets/')) {
        references.add(asset);
      }
    }
    for (final key in const <String>[
      'settingsAsset',
      'attachmentAsset',
      'micAsset',
      'sendAsset',
      'stopAsset',
    ]) {
      final asset = input[key];
      if (asset is String && asset.startsWith('assets/')) {
        references.add(asset);
      }
    }
    for (final reference in fontReferences.values) {
      if (reference != null && reference.startsWith('assets/')) {
        references.add(reference);
      }
    }
    return Set<String>.unmodifiable(references);
  }

  Set<String> get imageAssetReferences {
    final images = <String>{};
    final background = _section('background')['asset'];
    if (background is String && background.startsWith('assets/')) {
      images.add(background);
    }
    final chat = _section('chat');
    for (final role in <String>['user', 'ai']) {
      final image = (chat[role] as Map<String, dynamic>)['image'];
      if (image is Map<String, dynamic>) {
        final asset = image['asset'];
        if (asset is String && asset.startsWith('assets/')) {
          images.add(asset);
        }
      }
    }
    final input = _section('input');
    final inputImage = input['image'];
    if (inputImage is Map<String, dynamic>) {
      final asset = inputImage['asset'];
      if (asset is String && asset.startsWith('assets/')) {
        images.add(asset);
      }
    }
    for (final key in const <String>[
      'settingsAsset',
      'attachmentAsset',
      'micAsset',
      'sendAsset',
      'stopAsset',
    ]) {
      final asset = input[key];
      if (asset is String && asset.startsWith('assets/')) {
        images.add(asset);
      }
    }
    return Set<String>.unmodifiable(images);
  }
}

void _setFont(Map<String, Object?> target, String prefix, String? reference) {
  String key(String suffix) => prefix.isEmpty
      ? '${suffix[0].toLowerCase()}${suffix.substring(1)}'
      : '$prefix$suffix';
  final asset = reference != null && reference.startsWith('assets/');
  target[key('UseCustomFont')] = reference != null;
  target[key('FontType')] = asset ? 'file' : 'system';
  target[key('SystemFontName')] = asset ? null : reference;
  target[key('CustomFontPath')] = asset ? reference : null;
}

int? _nullableArgb(dynamic value) =>
    value == null ? null : _argb(value as String);

/// Public #RRGGBBAA is not Flutter's integer AARRGGBB representation.
int _argb(String rgba) {
  final rgb = int.parse(rgba.substring(1, 7), radix: 16);
  final alpha = int.parse(rgba.substring(7, 9), radix: 16);
  // Arithmetic keeps the unsigned 32-bit value identical on Dart VM and JS.
  return alpha * 0x1000000 + rgb;
}
