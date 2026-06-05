// ignore_for_file: file_names

import 'dart:async';
import 'dart:io';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../chat/components/style/bubble/BubbleSurface.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';

class AppearanceSettingsPanel extends StatelessWidget {
  const AppearanceSettingsPanel({super.key});

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final themeController = OperitTheme.of(context);
    final snapshot = themeController.themePreferenceSnapshot;
    return ListView(
      padding: const EdgeInsets.fromLTRB(28, 24, 28, 36),
      children: <Widget>[
        _SettingsHero(
          icon: Icons.palette_outlined,
          title: l10n.settingsCategoryAppearanceTitle,
          description: l10n.settingsCategoryAppearanceDescription,
        ),
        _SectionCard(
          title: l10n.settingsAppearanceThemeSection,
          children: <Widget>[
            _InfoLine(
              label: l10n.settingsAppearanceThemeMode,
              value: _themeModeLabel(l10n, themeController.themeMode),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceThemeTarget,
              value: _themeTargetLabel(l10n, themeController),
            ),
            _ThemeModeSelector(
              value: themeController.themeMode,
              onChanged: (themeMode) {
                unawaited(themeController.setThemeMode(themeMode));
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceMessageSurface,
              value: _messageSurfaceLabel(l10n, _surfaceFromSnapshot(snapshot)),
            ),
            _MessageSurfaceSelector(
              value: _surfaceFromSnapshot(snapshot),
              onChanged: (value) {
                unawaited(_applyMessageSurface(themeController, value));
              },
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceColorSection,
          children: <Widget>[
            _BodyText(l10n.settingsAppearanceColorDescription),
            _ThemeColorPresetSelector(
              selectedId: _selectedColorPresetId(snapshot),
              onChanged: (preset) {
                unawaited(
                  themeController.saveThemeSettings(
                    useCustomColors: preset.useCustomColors,
                    customPrimaryColor: preset.primaryColor,
                    customSecondaryColor: preset.secondaryColor,
                  ),
                );
              },
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed: () {
                  unawaited(
                    _showThemeColorDialog(context, themeController, snapshot),
                  );
                },
                icon: const Icon(Icons.colorize_outlined),
                label: Text(l10n.settingsAppearanceColorCustom),
              ),
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceBackgroundSection,
          children: <Widget>[
            _BodyText(l10n.settingsAppearanceBackgroundDescription),
            _InfoLine(
              label: l10n.settingsAppearanceBackgroundImage,
              value: _backgroundImageLabel(l10n, snapshot.backgroundImageUri),
            ),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.tonalIcon(
                  onPressed: () {
                    unawaited(_pickBackgroundImage(themeController));
                  },
                  icon: const Icon(Icons.image_outlined),
                  label: Text(l10n.settingsAppearanceBackgroundChooseImage),
                ),
                FilledButton.tonalIcon(
                  onPressed: () {
                    unawaited(_pickBackgroundVideo(themeController));
                  },
                  icon: const Icon(Icons.movie_creation_outlined),
                  label: Text(l10n.settingsAppearanceBackgroundChooseVideo),
                ),
                OutlinedButton.icon(
                  onPressed: snapshot.useBackgroundImage
                      ? () {
                          unawaited(
                            themeController.saveThemeSettings(
                              useBackgroundImage: false,
                            ),
                          );
                        }
                      : null,
                  icon: const Icon(Icons.visibility_off_outlined),
                  label: Text(l10n.settingsAppearanceBackgroundDisable),
                ),
              ],
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceBackgroundEnabled,
              value: snapshot.useBackgroundImage,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(useBackgroundImage: value),
                );
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceBackgroundOpacity,
              value: '${(snapshot.backgroundImageOpacity * 100).round()}%',
            ),
            Slider(
              value: snapshot.backgroundImageOpacity.clamp(0.1, 0.8),
              min: 0.1,
              max: 0.8,
              divisions: 70,
              label: '${(snapshot.backgroundImageOpacity * 100).round()}%',
              onChanged: (value) {
                themeController.previewThemeSettings(
                  backgroundImageOpacity: value,
                );
              },
              onChangeEnd: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    backgroundImageOpacity: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceBackgroundBlur,
              value: snapshot.useBackgroundBlur,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(useBackgroundBlur: value),
                );
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceBackgroundBlurRadius,
              value: snapshot.backgroundBlurRadius.round().toString(),
            ),
            Slider(
              value: snapshot.backgroundBlurRadius.clamp(0, 40),
              min: 0,
              max: 40,
              divisions: 40,
              label: snapshot.backgroundBlurRadius.round().toString(),
              onChanged: (value) {
                themeController.previewThemeSettings(
                  backgroundBlurRadius: value,
                );
              },
              onChangeEnd: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    backgroundBlurRadius: value,
                  ),
                );
              },
            ),
            if (snapshot.backgroundMediaType ==
                UserPreferencesManager.MEDIA_TYPE_VIDEO) ...<Widget>[
              _SettingSwitch(
                title: l10n.settingsAppearanceBackgroundVideoMuted,
                value: snapshot.videoBackgroundMuted,
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(
                      videoBackgroundMuted: value,
                    ),
                  );
                },
              ),
              _SettingSwitch(
                title: l10n.settingsAppearanceBackgroundVideoLoop,
                value: snapshot.videoBackgroundLoop,
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(
                      videoBackgroundLoop: value,
                    ),
                  );
                },
              ),
            ],
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceTextSection,
          children: <Widget>[
            _InfoLine(
              label: l10n.settingsAppearanceFontFamily,
              value: _fontFamilyLabel(l10n, snapshot),
            ),
            _FontFamilySelector(
              value: _fontFamilyPresetFromSnapshot(snapshot),
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    fontType: UserPreferencesManager.FONT_TYPE_SYSTEM,
                    systemFontName: _systemFontNameFromPreset(value),
                    useCustomFont: false,
                    customFontPath: '',
                  ),
                );
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceCustomFont,
              value: _customFontLabel(l10n, snapshot.customFontPath),
            ),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                FilledButton.tonalIcon(
                  onPressed: () {
                    unawaited(_pickCustomFont(themeController));
                  },
                  icon: const Icon(Icons.text_fields_outlined),
                  label: Text(l10n.settingsAppearanceChooseCustomFont),
                ),
                OutlinedButton.icon(
                  onPressed:
                      snapshot.customFontPath != null &&
                          snapshot.customFontPath!.isNotEmpty
                      ? () {
                          unawaited(
                            themeController.saveThemeSettings(
                              useCustomFont: false,
                              fontType: UserPreferencesManager.FONT_TYPE_SYSTEM,
                              customFontPath: '',
                            ),
                          );
                        }
                      : null,
                  icon: const Icon(Icons.format_clear_outlined),
                  label: Text(l10n.settingsAppearanceClearCustomFont),
                ),
              ],
            ),
            _InfoLine(
              label: l10n.settingsAppearanceFontScale,
              value: '${(snapshot.fontScale * 100).round()}%',
            ),
            Slider(
              value: snapshot.fontScale.clamp(0.85, 1.3),
              min: 0.85,
              max: 1.3,
              divisions: 45,
              label: '${(snapshot.fontScale * 100).round()}%',
              onChanged: (value) {
                themeController.previewThemeSettings(fontScale: value);
              },
              onChangeEnd: (value) {
                unawaited(themeController.saveThemeSettings(fontScale: value));
              },
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceAvatarSection,
          children: <Widget>[
            _InfoLine(
              label: l10n.settingsAppearanceAvatarShape,
              value: _avatarShapeLabel(l10n, snapshot.avatarShape),
            ),
            _AvatarShapeSelector(
              value: _avatarShapeFromSnapshot(snapshot.avatarShape),
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    avatarShape: _avatarShapeValue(value),
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowAvatars,
              value: snapshot.bubbleShowAvatar,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(bubbleShowAvatar: value),
                );
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceUserAvatar,
              value: _avatarImageLabel(l10n, snapshot.customUserAvatarUri),
            ),
            _AvatarActionRow(
              chooseLabel: l10n.settingsAppearanceChooseUserAvatar,
              clearLabel: l10n.settingsAppearanceClearUserAvatar,
              clearEnabled:
                  snapshot.customUserAvatarUri != null &&
                  snapshot.customUserAvatarUri!.isNotEmpty,
              onChoose: () {
                unawaited(_pickAvatarImage(themeController, isUser: true));
              },
              onClear: () {
                unawaited(
                  themeController.saveThemeSettings(customUserAvatarUri: ''),
                );
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceAiAvatar,
              value: _avatarImageLabel(l10n, snapshot.customAiAvatarUri),
            ),
            _AvatarActionRow(
              chooseLabel: l10n.settingsAppearanceChooseAiAvatar,
              clearLabel: l10n.settingsAppearanceClearAiAvatar,
              clearEnabled:
                  snapshot.customAiAvatarUri != null &&
                  snapshot.customAiAvatarUri!.isNotEmpty,
              onChoose: () {
                unawaited(_pickAvatarImage(themeController, isUser: false));
              },
              onClear: () {
                unawaited(
                  themeController.saveThemeSettings(customAiAvatarUri: ''),
                );
              },
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceChatDisplaySection,
          children: <Widget>[
            _InfoLine(
              label: l10n.settingsAppearanceMessageStyle,
              value: _messageStyleLabel(l10n, snapshot.chatStyle),
            ),
            _MessageStyleSelector(
              value: snapshot.chatStyle,
              onChanged: (value) {
                unawaited(themeController.saveThemeSettings(chatStyle: value));
              },
            ),
            _InfoLine(
              label: l10n.settingsAppearanceMessageColors,
              value: _messageColorPresetLabel(l10n, snapshot),
            ),
            _MessageColorPresetSelector(
              value: _messageColorPresetFromSnapshot(snapshot),
              onChanged: (value) {
                unawaited(_applyMessageColorPreset(themeController, value));
              },
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed: () {
                  unawaited(
                    _showMessageColorDialog(context, themeController, snapshot),
                  );
                },
                icon: const Icon(Icons.color_lens_outlined),
                label: Text(l10n.settingsAppearanceMessageColorsCustom),
              ),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceUserBubbleFont,
              value: _bubbleFontLabel(l10n, snapshot, isUser: true),
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed: () {
                  unawaited(
                    _showBubbleFontDialog(
                      context,
                      themeController,
                      snapshot,
                      isUser: true,
                    ),
                  );
                },
                icon: const Icon(Icons.text_fields_outlined),
                label: Text(l10n.settingsAppearanceAdjustUserBubbleFont),
              ),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceAiBubbleFont,
              value: _bubbleFontLabel(l10n, snapshot, isUser: false),
            ),
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed: () {
                  unawaited(
                    _showBubbleFontDialog(
                      context,
                      themeController,
                      snapshot,
                      isUser: false,
                    ),
                  );
                },
                icon: const Icon(Icons.text_fields_outlined),
                label: Text(l10n.settingsAppearanceAdjustAiBubbleFont),
              ),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceUserBubbleImage,
              value: _fileNameOrNoneLabel(
                l10n,
                snapshot.bubbleUserImageUri,
                snapshot.bubbleUserUseImage,
              ),
            ),
            _AvatarActionRow(
              chooseLabel: l10n.settingsAppearanceChooseUserBubbleImage,
              clearLabel: l10n.settingsAppearanceClearUserBubbleImage,
              clearEnabled:
                  snapshot.bubbleUserUseImage &&
                  snapshot.bubbleUserImageUri != null &&
                  snapshot.bubbleUserImageUri!.isNotEmpty,
              onChoose: () {
                unawaited(
                  _pickBubbleImage(
                    themeController,
                    snapshot: snapshot,
                    isUser: true,
                  ),
                );
              },
              onClear: () {
                unawaited(
                  themeController.saveThemeSettings(
                    bubbleUserUseImage: false,
                    bubbleUserImageUri: '',
                  ),
                );
              },
            ),
            if (snapshot.bubbleUserUseImage &&
                snapshot.bubbleUserImageUri != null &&
                snapshot.bubbleUserImageUri!.isNotEmpty) ...<Widget>[
              _InfoLine(
                label: l10n.settingsAppearanceBubbleImageRenderMode,
                value: _bubbleImageRenderModeLabel(
                  l10n,
                  snapshot.bubbleUserImageRenderMode,
                ),
              ),
              _BubbleImageRenderModeSelector(
                value: snapshot.bubbleUserImageRenderMode,
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(
                      bubbleUserImageRenderMode: value,
                    ),
                  );
                },
              ),
            ],
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed:
                    snapshot.bubbleUserUseImage &&
                        snapshot.bubbleUserImageUri != null &&
                        snapshot.bubbleUserImageUri!.isNotEmpty
                    ? () {
                        unawaited(
                          _showBubbleImageAdjustDialog(
                            context,
                            themeController,
                            snapshot,
                            isUser: true,
                          ),
                        );
                      }
                    : null,
                icon: const Icon(Icons.tune_outlined),
                label: Text(l10n.settingsAppearanceBubbleImageAdjustUser),
              ),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceAiBubbleImage,
              value: _fileNameOrNoneLabel(
                l10n,
                snapshot.bubbleAiImageUri,
                snapshot.bubbleAiUseImage,
              ),
            ),
            _AvatarActionRow(
              chooseLabel: l10n.settingsAppearanceChooseAiBubbleImage,
              clearLabel: l10n.settingsAppearanceClearAiBubbleImage,
              clearEnabled:
                  snapshot.bubbleAiUseImage &&
                  snapshot.bubbleAiImageUri != null &&
                  snapshot.bubbleAiImageUri!.isNotEmpty,
              onChoose: () {
                unawaited(
                  _pickBubbleImage(
                    themeController,
                    snapshot: snapshot,
                    isUser: false,
                  ),
                );
              },
              onClear: () {
                unawaited(
                  themeController.saveThemeSettings(
                    bubbleAiUseImage: false,
                    bubbleAiImageUri: '',
                  ),
                );
              },
            ),
            if (snapshot.bubbleAiUseImage &&
                snapshot.bubbleAiImageUri != null &&
                snapshot.bubbleAiImageUri!.isNotEmpty) ...<Widget>[
              _InfoLine(
                label: l10n.settingsAppearanceBubbleImageRenderMode,
                value: _bubbleImageRenderModeLabel(
                  l10n,
                  snapshot.bubbleAiImageRenderMode,
                ),
              ),
              _BubbleImageRenderModeSelector(
                value: snapshot.bubbleAiImageRenderMode,
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(
                      bubbleAiImageRenderMode: value,
                    ),
                  );
                },
              ),
            ],
            Align(
              alignment: Alignment.centerLeft,
              child: OutlinedButton.icon(
                onPressed:
                    snapshot.bubbleAiUseImage &&
                        snapshot.bubbleAiImageUri != null &&
                        snapshot.bubbleAiImageUri!.isNotEmpty
                    ? () {
                        unawaited(
                          _showBubbleImageAdjustDialog(
                            context,
                            themeController,
                            snapshot,
                            isUser: false,
                          ),
                        );
                      }
                    : null,
                icon: const Icon(Icons.tune_outlined),
                label: Text(l10n.settingsAppearanceBubbleImageAdjustAi),
              ),
            ),
            _InfoLine(
              label: l10n.settingsAppearanceMessageDensity,
              value: _messageDensityLabel(l10n, _densityFromSnapshot(snapshot)),
            ),
            _MessageDensitySelector(
              value: _densityFromSnapshot(snapshot),
              onChanged: (value) {
                final padding = value == _MessageDensity.compact ? 8.0 : 12.0;
                unawaited(
                  themeController.saveThemeSettings(
                    bubbleUserContentPaddingLeft: padding,
                    bubbleUserContentPaddingRight: padding,
                    bubbleAiContentPaddingLeft: padding,
                    bubbleAiContentPaddingRight: padding,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceWideLayout,
              value: snapshot.bubbleWideLayoutEnabled,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    bubbleWideLayoutEnabled: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceRoundedMessages,
              value:
                  snapshot.bubbleUserRoundedCornersEnabled &&
                  snapshot.bubbleAiRoundedCornersEnabled,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    bubbleUserRoundedCornersEnabled: value,
                    bubbleAiRoundedCornersEnabled: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowThinkingProcess,
              value: snapshot.showThinkingProcess,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(showThinkingProcess: value),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowRoleName,
              value: snapshot.showRoleName,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(showRoleName: value),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowUserName,
              value: snapshot.showUserName,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(showUserName: value),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowModelName,
              value: snapshot.showModelName,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(showModelName: value),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowModelProvider,
              value: snapshot.showModelProvider,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(showModelProvider: value),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowMessageTokenStats,
              value: snapshot.showMessageTokenStats,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    showMessageTokenStats: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowMessageTimingStats,
              value: snapshot.showMessageTimingStats,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    showMessageTimingStats: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowMessageTimestamp,
              value: snapshot.showMessageTimestamp,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    showMessageTimestamp: value,
                  ),
                );
              },
            ),
            _SettingSwitch(
              title: l10n.settingsAppearanceShowInputProcessingStatus,
              value: snapshot.showInputProcessingStatus,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(
                    showInputProcessingStatus: value,
                  ),
                );
              },
            ),
          ],
        ),
        Align(
          alignment: Alignment.centerLeft,
          child: OutlinedButton.icon(
            onPressed: () {
              unawaited(themeController.resetThemeSettings());
            },
            icon: const Icon(Icons.restart_alt),
            label: Text(l10n.settingsAppearanceResetTheme),
          ),
        ),
        _SectionCard(
          title: l10n.settingsAppearanceLanguageSection,
          children: <Widget>[
            _InfoLine(
              label: l10n.settingsAppearanceLanguage,
              value: l10n.localeName,
            ),
            _BodyText(l10n.settingsAppearanceLanguageDescription),
          ],
        ),
      ],
    );
  }
}

Future<void> _pickBackgroundImage(OperitThemeController themeController) async {
  const imageGroup = XTypeGroup(
    label: 'image',
    extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[imageGroup]);
  if (file == null) {
    return;
  }
  await themeController.saveThemeSettings(
    useBackgroundImage: true,
    backgroundImageUri: file.path,
    backgroundMediaType: UserPreferencesManager.MEDIA_TYPE_IMAGE,
  );
}

Future<void> _pickBackgroundVideo(OperitThemeController themeController) async {
  const videoGroup = XTypeGroup(
    label: 'video',
    extensions: <String>['mp4', 'mov', 'm4v', 'webm', 'mkv', 'avi'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[videoGroup]);
  if (file == null) {
    return;
  }
  await themeController.saveThemeSettings(
    useBackgroundImage: true,
    backgroundImageUri: file.path,
    backgroundMediaType: UserPreferencesManager.MEDIA_TYPE_VIDEO,
    videoBackgroundMuted: true,
    videoBackgroundLoop: true,
  );
}

Future<void> _pickAvatarImage(
  OperitThemeController themeController, {
  required bool isUser,
}) async {
  const imageGroup = XTypeGroup(
    label: 'image',
    extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[imageGroup]);
  if (file == null) {
    return;
  }
  await themeController.saveThemeSettings(
    customUserAvatarUri: isUser ? file.path : null,
    customAiAvatarUri: isUser ? null : file.path,
  );
}

Future<void> _pickCustomFont(OperitThemeController themeController) async {
  const fontGroup = XTypeGroup(
    label: 'font',
    extensions: <String>['ttf', 'otf', 'ttc'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[fontGroup]);
  if (file == null) {
    return;
  }
  await themeController.saveThemeSettings(
    useCustomFont: true,
    fontType: UserPreferencesManager.FONT_TYPE_FILE,
    customFontPath: file.path,
  );
}

Future<void> _showBubbleFontDialog(
  BuildContext context,
  OperitThemeController themeController,
  ThemePreferenceSnapshot snapshot, {
  required bool isUser,
}) async {
  final l10n = AppLocalizations.of(context)!;
  var useCustomFont = isUser
      ? snapshot.bubbleUserUseCustomFont
      : snapshot.bubbleAiUseCustomFont;
  var fontType = isUser
      ? snapshot.bubbleUserFontType
      : snapshot.bubbleAiFontType;
  var systemFontName = isUser
      ? snapshot.bubbleUserSystemFontName
      : snapshot.bubbleAiSystemFontName;
  var customFontPath = isUser
      ? snapshot.bubbleUserCustomFontPath
      : snapshot.bubbleAiCustomFontPath;

  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          Future<void> pickFontFile() async {
            const fontGroup = XTypeGroup(
              label: 'font',
              extensions: <String>['ttf', 'otf', 'ttc'],
            );
            final file = await openFile(
              acceptedTypeGroups: <XTypeGroup>[fontGroup],
            );
            if (file == null) {
              return;
            }
            setDialogState(() {
              useCustomFont = true;
              fontType = UserPreferencesManager.FONT_TYPE_FILE;
              customFontPath = file.path;
            });
          }

          return AlertDialog(
            title: Text(
              isUser
                  ? l10n.settingsAppearanceAdjustUserBubbleFont
                  : l10n.settingsAppearanceAdjustAiBubbleFont,
            ),
            content: SizedBox(
              width: 420,
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  SwitchListTile(
                    contentPadding: EdgeInsets.zero,
                    title: Text(l10n.settingsAppearanceEnableBubbleFont),
                    value: useCustomFont,
                    onChanged: (value) {
                      setDialogState(() {
                        useCustomFont = value;
                      });
                    },
                  ),
                  _FontFamilySelector(
                    value: _fontFamilyPresetFromSystemName(systemFontName),
                    onChanged: (value) {
                      setDialogState(() {
                        useCustomFont = true;
                        fontType = UserPreferencesManager.FONT_TYPE_SYSTEM;
                        systemFontName = _systemFontNameFromPreset(value);
                      });
                    },
                  ),
                  _InfoLine(
                    label: l10n.settingsAppearanceCustomFont,
                    value: _customFontLabel(l10n, customFontPath),
                  ),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: <Widget>[
                      FilledButton.tonalIcon(
                        onPressed: () {
                          unawaited(pickFontFile());
                        },
                        icon: const Icon(Icons.text_fields_outlined),
                        label: Text(l10n.settingsAppearanceChooseCustomFont),
                      ),
                      OutlinedButton.icon(
                        onPressed:
                            customFontPath != null && customFontPath!.isNotEmpty
                            ? () {
                                setDialogState(() {
                                  customFontPath = '';
                                  fontType =
                                      UserPreferencesManager.FONT_TYPE_SYSTEM;
                                });
                              }
                            : null,
                        icon: const Icon(Icons.format_clear_outlined),
                        label: Text(l10n.settingsAppearanceClearCustomFont),
                      ),
                    ],
                  ),
                ],
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  unawaited(
                    isUser
                        ? themeController.saveThemeSettings(
                            bubbleUserUseCustomFont: useCustomFont,
                            bubbleUserFontType: fontType,
                            bubbleUserSystemFontName: systemFontName,
                            bubbleUserCustomFontPath: customFontPath,
                          )
                        : themeController.saveThemeSettings(
                            bubbleAiUseCustomFont: useCustomFont,
                            bubbleAiFontType: fontType,
                            bubbleAiSystemFontName: systemFontName,
                            bubbleAiCustomFontPath: customFontPath,
                          ),
                  );
                  Navigator.of(dialogContext).pop();
                },
                child: Text(l10n.save),
              ),
            ],
          );
        },
      );
    },
  );
}

Future<void> _pickBubbleImage(
  OperitThemeController themeController, {
  required ThemePreferenceSnapshot snapshot,
  required bool isUser,
}) async {
  const imageGroup = XTypeGroup(
    label: 'image',
    extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[imageGroup]);
  if (file == null) {
    return;
  }
  final useImage = !snapshot.transparentSurfaceEnabled;
  if (_isNinePatchPngPath(file.path)) {
    final autoParams = await _parseNinePatchBubbleParams(file.path);
    await themeController.saveThemeSettings(
      bubbleUserImageRenderMode: isUser
          ? UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
          : null,
      bubbleAiImageRenderMode: isUser
          ? null
          : UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
      bubbleUserUseImage: isUser ? useImage : null,
      bubbleAiUseImage: isUser ? null : useImage,
      bubbleUserImageUri: isUser ? file.path : null,
      bubbleAiImageUri: isUser ? null : file.path,
      bubbleUserImageCropLeft: isUser ? autoParams.cropLeftRatio : null,
      bubbleUserImageCropTop: isUser ? autoParams.cropTopRatio : null,
      bubbleUserImageCropRight: isUser ? autoParams.cropRightRatio : null,
      bubbleUserImageCropBottom: isUser ? autoParams.cropBottomRatio : null,
      bubbleUserImageRepeatStart: isUser ? autoParams.repeatXStartRatio : null,
      bubbleUserImageRepeatEnd: isUser ? autoParams.repeatXEndRatio : null,
      bubbleUserImageRepeatYStart: isUser ? autoParams.repeatYStartRatio : null,
      bubbleUserImageRepeatYEnd: isUser ? autoParams.repeatYEndRatio : null,
      bubbleUserImageScale: isUser ? 1 : null,
      bubbleAiImageCropLeft: isUser ? null : autoParams.cropLeftRatio,
      bubbleAiImageCropTop: isUser ? null : autoParams.cropTopRatio,
      bubbleAiImageCropRight: isUser ? null : autoParams.cropRightRatio,
      bubbleAiImageCropBottom: isUser ? null : autoParams.cropBottomRatio,
      bubbleAiImageRepeatStart: isUser ? null : autoParams.repeatXStartRatio,
      bubbleAiImageRepeatEnd: isUser ? null : autoParams.repeatXEndRatio,
      bubbleAiImageRepeatYStart: isUser ? null : autoParams.repeatYStartRatio,
      bubbleAiImageRepeatYEnd: isUser ? null : autoParams.repeatYEndRatio,
      bubbleAiImageScale: isUser ? null : 1,
    );
    return;
  }
  await themeController.saveThemeSettings(
    bubbleUserImageRenderMode: isUser
        ? UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE
        : null,
    bubbleAiImageRenderMode: isUser
        ? null
        : UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
    bubbleUserUseImage: isUser ? useImage : null,
    bubbleAiUseImage: isUser ? null : useImage,
    bubbleUserImageUri: isUser ? file.path : null,
    bubbleAiImageUri: isUser ? null : file.path,
  );
}

class _NinePatchBubbleAutoParams {
  const _NinePatchBubbleAutoParams({
    required this.cropLeftRatio,
    required this.cropTopRatio,
    required this.cropRightRatio,
    required this.cropBottomRatio,
    required this.repeatXStartRatio,
    required this.repeatXEndRatio,
    required this.repeatYStartRatio,
    required this.repeatYEndRatio,
  });

  final double cropLeftRatio;
  final double cropTopRatio;
  final double cropRightRatio;
  final double cropBottomRatio;
  final double repeatXStartRatio;
  final double repeatXEndRatio;
  final double repeatYStartRatio;
  final double repeatYEndRatio;
}

bool _isNinePatchPngPath(String path) {
  return path.toLowerCase().endsWith('.9.png');
}

Future<_NinePatchBubbleAutoParams> _parseNinePatchBubbleParams(
  String path,
) async {
  final bytes = await File(path).readAsBytes();
  final codec = await ui.instantiateImageCodec(bytes);
  final frame = await codec.getNextFrame();
  final image = frame.image;
  final width = image.width;
  final height = image.height;
  if (width < 3 || height < 3) {
    image.dispose();
    throw StateError('nine-patch bubble image must be at least 3x3 pixels');
  }
  final byteData = await image.toByteData(format: ui.ImageByteFormat.rawRgba);
  image.dispose();
  if (byteData == null) {
    throw StateError('nine-patch bubble image pixels are unavailable');
  }

  final innerWidth = width - 2;
  final innerHeight = height - 2;
  final topMarkers = <int>[];
  final leftMarkers = <int>[];
  for (var x = 0; x < innerWidth; x++) {
    if (_isNinePatchMarker(byteData, width, x + 1, 0)) {
      topMarkers.add(x);
    }
  }
  for (var y = 0; y < innerHeight; y++) {
    if (_isNinePatchMarker(byteData, width, 0, y + 1)) {
      leftMarkers.add(y);
    }
  }
  final xRange = _buildNinePatchRange(topMarkers, innerWidth);
  final yRange = _buildNinePatchRange(leftMarkers, innerHeight);

  return _NinePatchBubbleAutoParams(
    cropLeftRatio: (1 / width).clamp(0.0, 0.45),
    cropTopRatio: (1 / height).clamp(0.0, 0.45),
    cropRightRatio: (1 / width).clamp(0.0, 0.45),
    cropBottomRatio: (1 / height).clamp(0.0, 0.45),
    repeatXStartRatio: xRange.$1,
    repeatXEndRatio: xRange.$2,
    repeatYStartRatio: yRange.$1,
    repeatYEndRatio: yRange.$2,
  );
}

bool _isNinePatchMarker(ByteData bytes, int width, int x, int y) {
  final offset = ((y * width + x) * 4);
  final red = bytes.getUint8(offset);
  final green = bytes.getUint8(offset + 1);
  final blue = bytes.getUint8(offset + 2);
  final alpha = bytes.getUint8(offset + 3);
  return alpha >= 0x80 && red < 32 && green < 32 && blue < 32;
}

(double, double) _buildNinePatchRange(List<int> marked, int innerSize) {
  if (marked.isEmpty || innerSize <= 0) {
    throw StateError('nine-patch bubble image is missing stretch markers');
  }
  final start = (marked.first / innerSize).clamp(0.0, 1.0);
  final endExclusive = ((marked.last + 1) / innerSize).clamp(0.0, 1.0);
  return (start, endExclusive);
}

Future<void> _showBubbleImageAdjustDialog(
  BuildContext context,
  OperitThemeController themeController,
  ThemePreferenceSnapshot snapshot, {
  required bool isUser,
}) async {
  final l10n = AppLocalizations.of(context)!;
  final imagePath = isUser
      ? snapshot.bubbleUserImageUri
      : snapshot.bubbleAiImageUri;
  if (imagePath == null || imagePath.isEmpty) {
    throw StateError('bubble image path is required for adjustment');
  }
  var cropLeft = isUser
      ? snapshot.bubbleUserImageCropLeft
      : snapshot.bubbleAiImageCropLeft;
  var cropTop = isUser
      ? snapshot.bubbleUserImageCropTop
      : snapshot.bubbleAiImageCropTop;
  var cropRight = isUser
      ? snapshot.bubbleUserImageCropRight
      : snapshot.bubbleAiImageCropRight;
  var cropBottom = isUser
      ? snapshot.bubbleUserImageCropBottom
      : snapshot.bubbleAiImageCropBottom;
  var repeatStart = isUser
      ? snapshot.bubbleUserImageRepeatStart
      : snapshot.bubbleAiImageRepeatStart;
  var repeatEnd = isUser
      ? snapshot.bubbleUserImageRepeatEnd
      : snapshot.bubbleAiImageRepeatEnd;
  var repeatYStart = isUser
      ? snapshot.bubbleUserImageRepeatYStart
      : snapshot.bubbleAiImageRepeatYStart;
  var repeatYEnd = isUser
      ? snapshot.bubbleUserImageRepeatYEnd
      : snapshot.bubbleAiImageRepeatYEnd;
  var imageScale = isUser
      ? snapshot.bubbleUserImageScale
      : snapshot.bubbleAiImageScale;

  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          void update(VoidCallback change) {
            setDialogState(change);
          }

          final colorScheme = Theme.of(context).colorScheme;
          final previewColor = isUser
              ? snapshot.bubbleUserBubbleColor == null
                    ? colorScheme.primaryContainer
                    : Color(snapshot.bubbleUserBubbleColor!)
              : snapshot.bubbleAiBubbleColor == null
              ? colorScheme.surfaceContainerHighest
              : Color(snapshot.bubbleAiBubbleColor!);
          final previewTextColor = isUser
              ? snapshot.bubbleUserTextColor == null
                    ? colorScheme.onPrimaryContainer
                    : Color(snapshot.bubbleUserTextColor!)
              : snapshot.bubbleAiTextColor == null
              ? colorScheme.onSurface
              : Color(snapshot.bubbleAiTextColor!);
          final previewStyle = BubbleImageStyle(
            imagePath: imagePath,
            cropLeftRatio: cropLeft,
            cropTopRatio: cropTop,
            cropRightRatio: cropRight,
            cropBottomRatio: cropBottom,
            repeatXStartRatio: repeatStart,
            repeatXEndRatio: repeatEnd,
            repeatYStartRatio: repeatYStart,
            repeatYEndRatio: repeatYEnd,
            imageScale: imageScale,
            renderMode: isUser
                ? snapshot.bubbleUserImageRenderMode
                : snapshot.bubbleAiImageRenderMode,
            showSliceGuides: true,
          );

          return AlertDialog(
            title: Text(
              isUser
                  ? l10n.settingsAppearanceBubbleImageAdjustUser
                  : l10n.settingsAppearanceBubbleImageAdjustAi,
            ),
            content: SingleChildScrollView(
              child: SizedBox(
                width: 420,
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: <Widget>[
                    _DialogSectionTitle(
                      l10n.settingsAppearanceBubbleImagePreview,
                    ),
                    SizedBox(
                      height: 112,
                      width: double.infinity,
                      child: BubbleSurface(
                        color: previewColor,
                        borderRadius: BorderRadius.circular(
                          isUser
                              ? snapshot.bubbleUserRoundedCornersEnabled
                                    ? 12
                                    : 4
                              : snapshot.bubbleAiRoundedCornersEnabled
                              ? 12
                              : 4,
                        ),
                        imageStyle: previewStyle,
                        child: Padding(
                          padding: EdgeInsets.fromLTRB(
                            isUser
                                ? snapshot.bubbleUserContentPaddingLeft
                                : snapshot.bubbleAiContentPaddingLeft,
                            12,
                            isUser
                                ? snapshot.bubbleUserContentPaddingRight
                                : snapshot.bubbleAiContentPaddingRight,
                            12,
                          ),
                          child: Align(
                            alignment: Alignment.centerLeft,
                            child: Text(
                              l10n.settingsAppearanceBubbleImagePreviewText,
                              style: Theme.of(context).textTheme.bodyMedium
                                  ?.copyWith(color: previewTextColor),
                            ),
                          ),
                        ),
                      ),
                    ),
                    _DialogSectionTitle(l10n.settingsAppearanceBubbleImageCrop),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageCropLeft,
                      value: cropLeft,
                      min: 0,
                      max: 0.45,
                      onChanged: (value) => update(() => cropLeft = value),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageCropTop,
                      value: cropTop,
                      min: 0,
                      max: 0.45,
                      onChanged: (value) => update(() => cropTop = value),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageCropRight,
                      value: cropRight,
                      min: 0,
                      max: 0.45,
                      onChanged: (value) => update(() => cropRight = value),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageCropBottom,
                      value: cropBottom,
                      min: 0,
                      max: 0.45,
                      onChanged: (value) => update(() => cropBottom = value),
                    ),
                    _DialogSectionTitle(
                      l10n.settingsAppearanceBubbleImageRepeat,
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageRepeatStart,
                      value: repeatStart,
                      min: 0.05,
                      max: 0.9,
                      onChanged: (value) => update(() {
                        repeatStart = value;
                        if (repeatEnd <= repeatStart + 0.01) {
                          repeatEnd = (repeatStart + 0.01).clamp(0.06, 0.95);
                        }
                      }),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageRepeatEnd,
                      value: repeatEnd,
                      min: repeatStart + 0.01,
                      max: 0.95,
                      onChanged: (value) => update(() => repeatEnd = value),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageRepeatYStart,
                      value: repeatYStart,
                      min: 0.05,
                      max: 0.9,
                      onChanged: (value) => update(() {
                        repeatYStart = value;
                        if (repeatYEnd <= repeatYStart + 0.01) {
                          repeatYEnd = (repeatYStart + 0.01).clamp(0.06, 0.95);
                        }
                      }),
                    ),
                    _PercentSlider(
                      label: l10n.settingsAppearanceBubbleImageRepeatYEnd,
                      value: repeatYEnd,
                      min: repeatYStart + 0.01,
                      max: 0.95,
                      onChanged: (value) => update(() => repeatYEnd = value),
                    ),
                    _DialogSectionTitle(
                      l10n.settingsAppearanceBubbleImageScale,
                    ),
                    _ValueSlider(
                      label: l10n.settingsAppearanceBubbleImageScale,
                      value: imageScale,
                      min: 0.2,
                      max: 3,
                      divisions: 28,
                      onChanged: (value) => update(() => imageScale = value),
                    ),
                  ],
                ),
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  unawaited(
                    isUser
                        ? themeController.saveThemeSettings(
                            bubbleUserImageCropLeft: cropLeft,
                            bubbleUserImageCropTop: cropTop,
                            bubbleUserImageCropRight: cropRight,
                            bubbleUserImageCropBottom: cropBottom,
                            bubbleUserImageRepeatStart: repeatStart,
                            bubbleUserImageRepeatEnd: repeatEnd,
                            bubbleUserImageRepeatYStart: repeatYStart,
                            bubbleUserImageRepeatYEnd: repeatYEnd,
                            bubbleUserImageScale: imageScale,
                          )
                        : themeController.saveThemeSettings(
                            bubbleAiImageCropLeft: cropLeft,
                            bubbleAiImageCropTop: cropTop,
                            bubbleAiImageCropRight: cropRight,
                            bubbleAiImageCropBottom: cropBottom,
                            bubbleAiImageRepeatStart: repeatStart,
                            bubbleAiImageRepeatEnd: repeatEnd,
                            bubbleAiImageRepeatYStart: repeatYStart,
                            bubbleAiImageRepeatYEnd: repeatYEnd,
                            bubbleAiImageScale: imageScale,
                          ),
                  );
                  Navigator.of(dialogContext).pop();
                },
                child: Text(l10n.save),
              ),
            ],
          );
        },
      );
    },
  );
}

class _ThemeColorPreset {
  const _ThemeColorPreset({
    required this.id,
    required this.primaryColor,
    required this.secondaryColor,
    required this.useCustomColors,
  });

  final String id;
  final int? primaryColor;
  final int? secondaryColor;
  final bool useCustomColors;
}

const List<_ThemeColorPreset> _themeColorPresets = <_ThemeColorPreset>[
  _ThemeColorPreset(
    id: 'default',
    primaryColor: null,
    secondaryColor: null,
    useCustomColors: false,
  ),
  _ThemeColorPreset(
    id: 'sky',
    primaryColor: 0xFF4C9EEB,
    secondaryColor: 0xFF32B8C6,
    useCustomColors: true,
  ),
  _ThemeColorPreset(
    id: 'matcha',
    primaryColor: 0xFF5C8D48,
    secondaryColor: 0xFFB08B42,
    useCustomColors: true,
  ),
  _ThemeColorPreset(
    id: 'ember',
    primaryColor: 0xFFE46F3D,
    secondaryColor: 0xFF9C6A2F,
    useCustomColors: true,
  ),
  _ThemeColorPreset(
    id: 'rose',
    primaryColor: 0xFFD85C7F,
    secondaryColor: 0xFF8E6AD8,
    useCustomColors: true,
  ),
];

String _selectedColorPresetId(ThemePreferenceSnapshot snapshot) {
  if (!snapshot.useCustomColors) {
    return 'default';
  }
  for (final preset in _themeColorPresets) {
    if (preset.useCustomColors &&
        preset.primaryColor == snapshot.customPrimaryColor &&
        preset.secondaryColor == snapshot.customSecondaryColor) {
      return preset.id;
    }
  }
  return 'custom';
}

String _backgroundImageLabel(AppLocalizations l10n, String? imagePath) {
  if (imagePath == null || imagePath.isEmpty) {
    return l10n.settingsAppearanceBackgroundNone;
  }
  final normalized = imagePath.replaceAll('\\', '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

String _avatarImageLabel(AppLocalizations l10n, String? imagePath) {
  if (imagePath == null || imagePath.isEmpty) {
    return l10n.settingsAppearanceAvatarDefault;
  }
  final normalized = imagePath.replaceAll('\\', '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

String _customFontLabel(AppLocalizations l10n, String? fontPath) {
  if (fontPath == null || fontPath.isEmpty) {
    return l10n.settingsAppearanceFontDefault;
  }
  final normalized = fontPath.replaceAll('\\', '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

String _bubbleFontLabel(
  AppLocalizations l10n,
  ThemePreferenceSnapshot snapshot, {
  required bool isUser,
}) {
  final useCustomFont = isUser
      ? snapshot.bubbleUserUseCustomFont
      : snapshot.bubbleAiUseCustomFont;
  final fontType = isUser
      ? snapshot.bubbleUserFontType
      : snapshot.bubbleAiFontType;
  final systemFontName = isUser
      ? snapshot.bubbleUserSystemFontName
      : snapshot.bubbleAiSystemFontName;
  final customFontPath = isUser
      ? snapshot.bubbleUserCustomFontPath
      : snapshot.bubbleAiCustomFontPath;
  if (!useCustomFont) {
    return l10n.settingsAppearanceFontDefault;
  }
  if (fontType == UserPreferencesManager.FONT_TYPE_FILE &&
      customFontPath != null &&
      customFontPath.isNotEmpty) {
    return l10n.settingsAppearanceFontCustom;
  }
  return switch (_fontFamilyPresetFromSystemName(systemFontName)) {
    _FontFamilyPreset.defaultFont => l10n.settingsAppearanceFontDefault,
    _FontFamilyPreset.serif => l10n.settingsAppearanceFontSerif,
    _FontFamilyPreset.monospace => l10n.settingsAppearanceFontMonospace,
  };
}

String _fileNameOrNoneLabel(
  AppLocalizations l10n,
  String? imagePath,
  bool enabled,
) {
  if (!enabled || imagePath == null || imagePath.isEmpty) {
    return l10n.settingsAppearanceBackgroundNone;
  }
  final normalized = imagePath.replaceAll('\\', '/');
  return normalized.substring(normalized.lastIndexOf('/') + 1);
}

class _ThemeColorPresetSelector extends StatelessWidget {
  const _ThemeColorPresetSelector({
    required this.selectedId,
    required this.onChanged,
  });

  final String selectedId;
  final ValueChanged<_ThemeColorPreset> onChanged;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 8,
      runSpacing: 8,
      children: <Widget>[
        for (final preset in _themeColorPresets)
          ChoiceChip(
            selected: selectedId == preset.id,
            label: Text(_themeColorPresetLabel(context, preset.id)),
            avatar: _ThemeColorDot(preset: preset),
            onSelected: (_) => onChanged(preset),
          ),
      ],
    );
  }
}

class _ThemeColorDot extends StatelessWidget {
  const _ThemeColorDot({required this.preset});

  final _ThemeColorPreset preset;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final color = preset.primaryColor == null
        ? colorScheme.primary
        : Color(preset.primaryColor!);
    return Container(
      width: 14,
      height: 14,
      decoration: BoxDecoration(color: color, shape: BoxShape.circle),
    );
  }
}

String _themeColorPresetLabel(BuildContext context, String id) {
  final l10n = AppLocalizations.of(context)!;
  return switch (id) {
    'default' => l10n.settingsAppearanceColorDefault,
    'sky' => l10n.settingsAppearanceColorSky,
    'matcha' => l10n.settingsAppearanceColorMatcha,
    'ember' => l10n.settingsAppearanceColorEmber,
    'rose' => l10n.settingsAppearanceColorRose,
    _ => id,
  };
}

enum _AvatarShapePreset { circle, square }

class _AvatarShapeSelector extends StatelessWidget {
  const _AvatarShapeSelector({required this.value, required this.onChanged});

  final _AvatarShapePreset value;
  final ValueChanged<_AvatarShapePreset> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<_AvatarShapePreset>(
        showSelectedIcon: false,
        segments: <ButtonSegment<_AvatarShapePreset>>[
          ButtonSegment<_AvatarShapePreset>(
            value: _AvatarShapePreset.circle,
            label: Text(l10n.settingsAppearanceAvatarShapeCircle),
          ),
          ButtonSegment<_AvatarShapePreset>(
            value: _AvatarShapePreset.square,
            label: Text(l10n.settingsAppearanceAvatarShapeSquare),
          ),
        ],
        selected: <_AvatarShapePreset>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

_AvatarShapePreset _avatarShapeFromSnapshot(String avatarShape) {
  return avatarShape == UserPreferencesManager.AVATAR_SHAPE_SQUARE
      ? _AvatarShapePreset.square
      : _AvatarShapePreset.circle;
}

String _avatarShapeValue(_AvatarShapePreset value) {
  return switch (value) {
    _AvatarShapePreset.circle => UserPreferencesManager.AVATAR_SHAPE_CIRCLE,
    _AvatarShapePreset.square => UserPreferencesManager.AVATAR_SHAPE_SQUARE,
  };
}

String _avatarShapeLabel(AppLocalizations l10n, String avatarShape) {
  return switch (_avatarShapeFromSnapshot(avatarShape)) {
    _AvatarShapePreset.circle => l10n.settingsAppearanceAvatarShapeCircle,
    _AvatarShapePreset.square => l10n.settingsAppearanceAvatarShapeSquare,
  };
}

class _MessageStyleSelector extends StatelessWidget {
  const _MessageStyleSelector({required this.value, required this.onChanged});

  final String value;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<String>(
        showSelectedIcon: false,
        segments: <ButtonSegment<String>>[
          ButtonSegment<String>(
            value: UserPreferencesManager.CHAT_STYLE_CURSOR,
            label: Text(l10n.settingsAppearanceMessageStyleClean),
          ),
          ButtonSegment<String>(
            value: UserPreferencesManager.CHAT_STYLE_BUBBLE,
            label: Text(l10n.settingsAppearanceMessageStyleCard),
          ),
        ],
        selected: <String>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

enum _MessageColorPreset { theme, sky, matcha, ink, custom }

class _MessageColorPresetValues {
  const _MessageColorPresetValues({
    required this.cursorUserBubbleColor,
    required this.bubbleUserBubbleColor,
    required this.bubbleAiBubbleColor,
    required this.bubbleUserTextColor,
    required this.bubbleAiTextColor,
  });

  final int cursorUserBubbleColor;
  final int bubbleUserBubbleColor;
  final int bubbleAiBubbleColor;
  final int bubbleUserTextColor;
  final int bubbleAiTextColor;
}

const Map<_MessageColorPreset, _MessageColorPresetValues>
_messageColorPresetValues = <_MessageColorPreset, _MessageColorPresetValues>{
  _MessageColorPreset.sky: _MessageColorPresetValues(
    cursorUserBubbleColor: 0xFFE3F2FD,
    bubbleUserBubbleColor: 0xFFE3F2FD,
    bubbleAiBubbleColor: 0xFFF4F8FF,
    bubbleUserTextColor: 0xFF0F2F43,
    bubbleAiTextColor: 0xFF17212F,
  ),
  _MessageColorPreset.matcha: _MessageColorPresetValues(
    cursorUserBubbleColor: 0xFFE7F5E9,
    bubbleUserBubbleColor: 0xFFE7F5E9,
    bubbleAiBubbleColor: 0xFFFFF7E6,
    bubbleUserTextColor: 0xFF17351F,
    bubbleAiTextColor: 0xFF2F2718,
  ),
  _MessageColorPreset.ink: _MessageColorPresetValues(
    cursorUserBubbleColor: 0xFF253142,
    bubbleUserBubbleColor: 0xFF253142,
    bubbleAiBubbleColor: 0xFF111827,
    bubbleUserTextColor: 0xFFF8FAFC,
    bubbleAiTextColor: 0xFFF8FAFC,
  ),
};

const List<_MessageColorPreset> _messageColorPresetChoices =
    <_MessageColorPreset>[
      _MessageColorPreset.theme,
      _MessageColorPreset.sky,
      _MessageColorPreset.matcha,
      _MessageColorPreset.ink,
    ];

class _MessageColorPresetSelector extends StatelessWidget {
  const _MessageColorPresetSelector({
    required this.value,
    required this.onChanged,
  });

  final _MessageColorPreset value;
  final ValueChanged<_MessageColorPreset> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        children: <Widget>[
          for (final preset in _messageColorPresetChoices)
            ChoiceChip(
              selected: value == preset,
              label: Text(_messageColorPresetName(l10n, preset)),
              avatar: _MessageColorDot(preset: preset),
              onSelected: (_) => onChanged(preset),
            ),
        ],
      ),
    );
  }
}

class _MessageColorDot extends StatelessWidget {
  const _MessageColorDot({required this.preset});

  final _MessageColorPreset preset;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final userColor = switch (preset) {
      _MessageColorPreset.theme ||
      _MessageColorPreset.custom => colorScheme.primaryContainer,
      _ => Color(_messageColorPresetValues[preset]!.bubbleUserBubbleColor),
    };
    final aiColor = switch (preset) {
      _MessageColorPreset.theme ||
      _MessageColorPreset.custom => colorScheme.surfaceContainerHighest,
      _ => Color(_messageColorPresetValues[preset]!.bubbleAiBubbleColor),
    };
    return Container(
      width: 16,
      height: 16,
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        gradient: LinearGradient(colors: <Color>[userColor, aiColor]),
      ),
    );
  }
}

Future<void> _applyMessageColorPreset(
  OperitThemeController themeController,
  _MessageColorPreset preset,
) async {
  if (preset == _MessageColorPreset.theme) {
    return themeController.resetMessageColorSettings();
  }
  final values = _messageColorPresetValues[preset]!;
  await themeController.saveThemeSettings(
    cursorUserBubbleColor: values.cursorUserBubbleColor,
    bubbleUserBubbleColor: values.bubbleUserBubbleColor,
    bubbleAiBubbleColor: values.bubbleAiBubbleColor,
    bubbleUserTextColor: values.bubbleUserTextColor,
    bubbleAiTextColor: values.bubbleAiTextColor,
  );
}

Future<void> _showThemeColorDialog(
  BuildContext context,
  OperitThemeController themeController,
  ThemePreferenceSnapshot snapshot,
) async {
  final l10n = AppLocalizations.of(context)!;
  final colorScheme = Theme.of(context).colorScheme;
  final primaryController = TextEditingController(
    text: _hexColorText(
      Color(snapshot.customPrimaryColor ?? colorScheme.primary.toARGB32()),
    ),
  );
  final secondaryController = TextEditingController(
    text: _hexColorText(
      Color(snapshot.customSecondaryColor ?? colorScheme.secondary.toARGB32()),
    ),
  );
  String? errorText;
  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          return AlertDialog(
            title: Text(l10n.settingsAppearanceCustomColorsTitle),
            content: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _HexColorField(
                  label: l10n.settingsAppearancePrimaryColor,
                  controller: primaryController,
                ),
                const SizedBox(height: 12),
                _HexColorField(
                  label: l10n.settingsAppearanceSecondaryColor,
                  controller: secondaryController,
                ),
                if (errorText != null) ...<Widget>[
                  const SizedBox(height: 12),
                  Text(errorText!, style: TextStyle(color: colorScheme.error)),
                ],
              ],
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  try {
                    final primaryColor = _parseHexColor(primaryController.text);
                    final secondaryColor = _parseHexColor(
                      secondaryController.text,
                    );
                    unawaited(
                      themeController.saveThemeSettings(
                        useCustomColors: true,
                        customPrimaryColor: primaryColor,
                        customSecondaryColor: secondaryColor,
                      ),
                    );
                    Navigator.of(dialogContext).pop();
                  } on FormatException {
                    setDialogState(() {
                      errorText = l10n.settingsAppearanceHexColorInvalid;
                    });
                  }
                },
                child: Text(l10n.save),
              ),
            ],
          );
        },
      );
    },
  );
}

Future<void> _showMessageColorDialog(
  BuildContext context,
  OperitThemeController themeController,
  ThemePreferenceSnapshot snapshot,
) async {
  final l10n = AppLocalizations.of(context)!;
  final colorScheme = Theme.of(context).colorScheme;
  final cursorUserController = TextEditingController(
    text: _hexColorText(
      Color(
        snapshot.cursorUserBubbleColor ??
            colorScheme.primaryContainer.toARGB32(),
      ),
    ),
  );
  final userBubbleController = TextEditingController(
    text: _hexColorText(
      Color(
        snapshot.bubbleUserBubbleColor ??
            colorScheme.primaryContainer.toARGB32(),
      ),
    ),
  );
  final aiBubbleController = TextEditingController(
    text: _hexColorText(
      Color(
        snapshot.bubbleAiBubbleColor ??
            colorScheme.surfaceContainerHighest.toARGB32(),
      ),
    ),
  );
  final userTextController = TextEditingController(
    text: _hexColorText(
      Color(
        snapshot.bubbleUserTextColor ??
            colorScheme.onPrimaryContainer.toARGB32(),
      ),
    ),
  );
  final aiTextController = TextEditingController(
    text: _hexColorText(
      Color(snapshot.bubbleAiTextColor ?? colorScheme.onSurface.toARGB32()),
    ),
  );
  String? errorText;
  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          return AlertDialog(
            title: Text(l10n.settingsAppearanceCustomMessageColorsTitle),
            content: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  _HexColorField(
                    label: l10n.settingsAppearanceCursorUserBubbleColor,
                    controller: cursorUserController,
                  ),
                  const SizedBox(height: 12),
                  _HexColorField(
                    label: l10n.settingsAppearanceUserBubbleColor,
                    controller: userBubbleController,
                  ),
                  const SizedBox(height: 12),
                  _HexColorField(
                    label: l10n.settingsAppearanceAiBubbleColor,
                    controller: aiBubbleController,
                  ),
                  const SizedBox(height: 12),
                  _HexColorField(
                    label: l10n.settingsAppearanceUserTextColor,
                    controller: userTextController,
                  ),
                  const SizedBox(height: 12),
                  _HexColorField(
                    label: l10n.settingsAppearanceAiTextColor,
                    controller: aiTextController,
                  ),
                  if (errorText != null) ...<Widget>[
                    const SizedBox(height: 12),
                    Text(
                      errorText!,
                      style: TextStyle(color: colorScheme.error),
                    ),
                  ],
                ],
              ),
            ),
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  try {
                    final cursorUserColor = _parseHexColor(
                      cursorUserController.text,
                    );
                    final userBubbleColor = _parseHexColor(
                      userBubbleController.text,
                    );
                    final aiBubbleColor = _parseHexColor(
                      aiBubbleController.text,
                    );
                    final userTextColor = _parseHexColor(
                      userTextController.text,
                    );
                    final aiTextColor = _parseHexColor(aiTextController.text);
                    unawaited(
                      themeController.saveThemeSettings(
                        cursorUserBubbleColor: cursorUserColor,
                        bubbleUserBubbleColor: userBubbleColor,
                        bubbleAiBubbleColor: aiBubbleColor,
                        bubbleUserTextColor: userTextColor,
                        bubbleAiTextColor: aiTextColor,
                      ),
                    );
                    Navigator.of(dialogContext).pop();
                  } on FormatException {
                    setDialogState(() {
                      errorText = l10n.settingsAppearanceHexColorInvalid;
                    });
                  }
                },
                child: Text(l10n.save),
              ),
            ],
          );
        },
      );
    },
  );
}

class _HexColorField extends StatelessWidget {
  const _HexColorField({required this.label, required this.controller});

  final String label;
  final TextEditingController controller;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return TextField(
      controller: controller,
      decoration: InputDecoration(
        labelText: label,
        hintText: l10n.settingsAppearanceHexColorHint,
      ),
      textCapitalization: TextCapitalization.characters,
      style: const TextStyle(fontFamily: 'monospace'),
    );
  }
}

String _hexColorText(Color color) {
  final value = color.toARGB32() & 0xFFFFFF;
  return '#${value.toRadixString(16).padLeft(6, '0').toUpperCase()}';
}

int _parseHexColor(String input) {
  final normalized = input.trim().replaceFirst('#', '');
  final valid = RegExp(r'^[0-9a-fA-F]{6}$').hasMatch(normalized);
  if (!valid) {
    throw const FormatException('invalid hex color');
  }
  return 0xFF000000 | int.parse(normalized, radix: 16);
}

_MessageColorPreset _messageColorPresetFromSnapshot(
  ThemePreferenceSnapshot snapshot,
) {
  for (final entry in _messageColorPresetValues.entries) {
    final values = entry.value;
    if (snapshot.cursorUserBubbleColor == values.cursorUserBubbleColor &&
        snapshot.bubbleUserBubbleColor == values.bubbleUserBubbleColor &&
        snapshot.bubbleAiBubbleColor == values.bubbleAiBubbleColor &&
        snapshot.bubbleUserTextColor == values.bubbleUserTextColor &&
        snapshot.bubbleAiTextColor == values.bubbleAiTextColor) {
      return entry.key;
    }
  }
  if (snapshot.cursorUserBubbleColor != null ||
      snapshot.bubbleUserBubbleColor != null ||
      snapshot.bubbleAiBubbleColor != null ||
      snapshot.bubbleUserTextColor != null ||
      snapshot.bubbleAiTextColor != null) {
    return _MessageColorPreset.custom;
  }
  return _MessageColorPreset.theme;
}

String _messageColorPresetLabel(
  AppLocalizations l10n,
  ThemePreferenceSnapshot snapshot,
) {
  return _messageColorPresetName(
    l10n,
    _messageColorPresetFromSnapshot(snapshot),
  );
}

String _messageColorPresetName(
  AppLocalizations l10n,
  _MessageColorPreset preset,
) {
  return switch (preset) {
    _MessageColorPreset.theme => l10n.settingsAppearanceMessageColorsTheme,
    _MessageColorPreset.sky => l10n.settingsAppearanceMessageColorsSky,
    _MessageColorPreset.matcha => l10n.settingsAppearanceMessageColorsMatcha,
    _MessageColorPreset.ink => l10n.settingsAppearanceMessageColorsInk,
    _MessageColorPreset.custom => l10n.settingsAppearanceMessageColorsCustom,
  };
}

enum _MessageSurface { normal, transparent }

class _MessageSurfaceSelector extends StatelessWidget {
  const _MessageSurfaceSelector({required this.value, required this.onChanged});

  final _MessageSurface value;
  final ValueChanged<_MessageSurface> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<_MessageSurface>(
        showSelectedIcon: false,
        segments: <ButtonSegment<_MessageSurface>>[
          ButtonSegment<_MessageSurface>(
            value: _MessageSurface.normal,
            label: Text(l10n.settingsAppearanceMessageSurfaceNormal),
          ),
          ButtonSegment<_MessageSurface>(
            value: _MessageSurface.transparent,
            label: Text(l10n.settingsAppearanceMessageSurfaceTransparent),
          ),
        ],
        selected: <_MessageSurface>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

_MessageSurface _surfaceFromSnapshot(ThemePreferenceSnapshot snapshot) {
  return snapshot.transparentSurfaceEnabled
      ? _MessageSurface.transparent
      : _MessageSurface.normal;
}

String _messageSurfaceLabel(AppLocalizations l10n, _MessageSurface surface) {
  return switch (surface) {
    _MessageSurface.normal => l10n.settingsAppearanceMessageSurfaceNormal,
    _MessageSurface.transparent =>
      l10n.settingsAppearanceMessageSurfaceTransparent,
  };
}

Future<void> _applyMessageSurface(
  OperitThemeController themeController,
  _MessageSurface surface,
) async {
  final transparent = surface == _MessageSurface.transparent;
  await themeController.saveThemeSettings(
    transparentSurfaceEnabled: transparent,
  );
}

class _BubbleImageRenderModeSelector extends StatelessWidget {
  const _BubbleImageRenderModeSelector({
    required this.value,
    required this.onChanged,
  });

  final String value;
  final ValueChanged<String> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<String>(
        showSelectedIcon: false,
        segments: <ButtonSegment<String>>[
          ButtonSegment<String>(
            value: UserPreferencesManager
                .BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
            label: Text(l10n.settingsAppearanceBubbleImageTiledNineSlice),
          ),
          ButtonSegment<String>(
            value: UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
            label: Text(l10n.settingsAppearanceBubbleImageNinePatch),
          ),
        ],
        selected: <String>{_bubbleImageRenderModeValue(value)},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

String _bubbleImageRenderModeValue(String value) {
  return value == UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
      ? UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
      : UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE;
}

String _bubbleImageRenderModeLabel(AppLocalizations l10n, String value) {
  return _bubbleImageRenderModeValue(value) ==
          UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
      ? l10n.settingsAppearanceBubbleImageNinePatch
      : l10n.settingsAppearanceBubbleImageTiledNineSlice;
}

enum _FontFamilyPreset { defaultFont, serif, monospace }

class _FontFamilySelector extends StatelessWidget {
  const _FontFamilySelector({required this.value, required this.onChanged});

  final _FontFamilyPreset value;
  final ValueChanged<_FontFamilyPreset> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<_FontFamilyPreset>(
        showSelectedIcon: false,
        segments: <ButtonSegment<_FontFamilyPreset>>[
          ButtonSegment<_FontFamilyPreset>(
            value: _FontFamilyPreset.defaultFont,
            label: Text(l10n.settingsAppearanceFontDefault),
          ),
          ButtonSegment<_FontFamilyPreset>(
            value: _FontFamilyPreset.serif,
            label: Text(l10n.settingsAppearanceFontSerif),
          ),
          ButtonSegment<_FontFamilyPreset>(
            value: _FontFamilyPreset.monospace,
            label: Text(l10n.settingsAppearanceFontMonospace),
          ),
        ],
        selected: <_FontFamilyPreset>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

_FontFamilyPreset _fontFamilyPresetFromSystemName(String? systemFontName) {
  return switch (systemFontName) {
    UserPreferencesManager.SYSTEM_FONT_SERIF => _FontFamilyPreset.serif,
    UserPreferencesManager.SYSTEM_FONT_MONOSPACE => _FontFamilyPreset.monospace,
    _ => _FontFamilyPreset.defaultFont,
  };
}

_FontFamilyPreset _fontFamilyPresetFromSnapshot(
  ThemePreferenceSnapshot snapshot,
) {
  return _fontFamilyPresetFromSystemName(snapshot.systemFontName);
}

String _systemFontNameFromPreset(_FontFamilyPreset value) {
  return switch (value) {
    _FontFamilyPreset.serif => UserPreferencesManager.SYSTEM_FONT_SERIF,
    _FontFamilyPreset.monospace => UserPreferencesManager.SYSTEM_FONT_MONOSPACE,
    _FontFamilyPreset.defaultFont => UserPreferencesManager.SYSTEM_FONT_DEFAULT,
  };
}

String _fontFamilyLabel(
  AppLocalizations l10n,
  ThemePreferenceSnapshot snapshot,
) {
  if (snapshot.fontType == UserPreferencesManager.FONT_TYPE_FILE &&
      snapshot.customFontPath != null &&
      snapshot.customFontPath!.isNotEmpty) {
    return l10n.settingsAppearanceFontCustom;
  }
  return switch (_fontFamilyPresetFromSnapshot(snapshot)) {
    _FontFamilyPreset.defaultFont => l10n.settingsAppearanceFontDefault,
    _FontFamilyPreset.serif => l10n.settingsAppearanceFontSerif,
    _FontFamilyPreset.monospace => l10n.settingsAppearanceFontMonospace,
  };
}

enum _MessageDensity { comfortable, compact }

class _MessageDensitySelector extends StatelessWidget {
  const _MessageDensitySelector({required this.value, required this.onChanged});

  final _MessageDensity value;
  final ValueChanged<_MessageDensity> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<_MessageDensity>(
        showSelectedIcon: false,
        segments: <ButtonSegment<_MessageDensity>>[
          ButtonSegment<_MessageDensity>(
            value: _MessageDensity.comfortable,
            label: Text(l10n.settingsAppearanceMessageDensityComfortable),
          ),
          ButtonSegment<_MessageDensity>(
            value: _MessageDensity.compact,
            label: Text(l10n.settingsAppearanceMessageDensityCompact),
          ),
        ],
        selected: <_MessageDensity>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

_MessageDensity _densityFromSnapshot(ThemePreferenceSnapshot snapshot) {
  final average =
      (snapshot.bubbleUserContentPaddingLeft +
          snapshot.bubbleUserContentPaddingRight +
          snapshot.bubbleAiContentPaddingLeft +
          snapshot.bubbleAiContentPaddingRight) /
      4;
  return average <= 10 ? _MessageDensity.compact : _MessageDensity.comfortable;
}

String _messageStyleLabel(AppLocalizations l10n, String value) {
  return switch (value) {
    UserPreferencesManager.CHAT_STYLE_BUBBLE =>
      l10n.settingsAppearanceMessageStyleCard,
    _ => l10n.settingsAppearanceMessageStyleClean,
  };
}

String _messageDensityLabel(AppLocalizations l10n, _MessageDensity value) {
  return switch (value) {
    _MessageDensity.comfortable =>
      l10n.settingsAppearanceMessageDensityComfortable,
    _MessageDensity.compact => l10n.settingsAppearanceMessageDensityCompact,
  };
}

class _ThemeModeSelector extends StatelessWidget {
  const _ThemeModeSelector({required this.value, required this.onChanged});

  final ThemeMode value;
  final ValueChanged<ThemeMode> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: SegmentedButton<ThemeMode>(
        showSelectedIcon: false,
        segments: <ButtonSegment<ThemeMode>>[
          ButtonSegment<ThemeMode>(
            value: ThemeMode.system,
            icon: const Icon(Icons.brightness_auto_outlined),
            label: Text(l10n.settingsAppearanceThemeSystem),
          ),
          ButtonSegment<ThemeMode>(
            value: ThemeMode.light,
            icon: const Icon(Icons.light_mode_outlined),
            label: Text(l10n.settingsAppearanceThemeLight),
          ),
          ButtonSegment<ThemeMode>(
            value: ThemeMode.dark,
            icon: const Icon(Icons.dark_mode_outlined),
            label: Text(l10n.settingsAppearanceThemeDark),
          ),
        ],
        selected: <ThemeMode>{value},
        onSelectionChanged: (selection) => onChanged(selection.single),
      ),
    );
  }
}

String _themeModeLabel(AppLocalizations l10n, ThemeMode mode) {
  return switch (mode) {
    ThemeMode.system => l10n.settingsAppearanceThemeSystem,
    ThemeMode.light => l10n.settingsAppearanceThemeLight,
    ThemeMode.dark => l10n.settingsAppearanceThemeDark,
  };
}

String _themeTargetLabel(
  AppLocalizations l10n,
  OperitThemeController themeController,
) {
  final name = themeController.activeThemeTargetName;
  if (name == null) {
    return l10n.settingsAppearanceThemeTargetGlobal;
  }
  if (themeController.isActiveThemeTargetGroup) {
    return l10n.settingsAppearanceThemeTargetGroup(name);
  }
  return l10n.settingsAppearanceThemeTargetCharacter(name);
}

class _SettingsHero extends StatelessWidget {
  const _SettingsHero({
    required this.icon,
    required this.title,
    required this.description,
  });

  final IconData icon;
  final String title;
  final String description;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    return Padding(
      padding: const EdgeInsets.only(bottom: 18),
      child: Row(
        children: <Widget>[
          CircleAvatar(
            radius: 24,
            backgroundColor: colorScheme.primaryContainer,
            child: Icon(icon, color: colorScheme.onPrimaryContainer),
          ),
          const SizedBox(width: 14),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: <Widget>[
                Text(
                  title,
                  style: theme.textTheme.headlineSmall?.copyWith(
                    fontWeight: FontWeight.w800,
                  ),
                ),
                const SizedBox(height: 4),
                Text(
                  description,
                  style: theme.textTheme.bodyMedium?.copyWith(
                    color: colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _SectionCard extends StatelessWidget {
  const _SectionCard({required this.title, required this.children});

  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(18);
    return Padding(
      padding: const EdgeInsets.only(bottom: 14),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        child: Padding(
          padding: const EdgeInsets.fromLTRB(18, 16, 18, 14),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: <Widget>[
              Text(title, style: const TextStyle(fontWeight: FontWeight.w800)),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _InfoLine extends StatelessWidget {
  const _InfoLine({required this.label, required this.value});

  final String label;
  final String value;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 9),
      child: Row(
        children: <Widget>[
          Expanded(child: Text(label)),
          const SizedBox(width: 12),
          Flexible(
            child: Text(
              value,
              textAlign: TextAlign.end,
              style: TextStyle(color: colorScheme.onSurfaceVariant),
            ),
          ),
        ],
      ),
    );
  }
}

class _BodyText extends StatelessWidget {
  const _BodyText(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Text(
        text,
        style: TextStyle(color: Theme.of(context).colorScheme.onSurfaceVariant),
      ),
    );
  }
}

class _SettingSwitch extends StatelessWidget {
  const _SettingSwitch({
    required this.title,
    required this.value,
    required this.onChanged,
  });

  final String title;
  final bool value;
  final ValueChanged<bool> onChanged;

  @override
  Widget build(BuildContext context) {
    return SwitchListTile(
      contentPadding: EdgeInsets.zero,
      title: Text(title),
      value: value,
      onChanged: onChanged,
    );
  }
}

class _DialogSectionTitle extends StatelessWidget {
  const _DialogSectionTitle(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 10, bottom: 4),
      child: Text(
        text,
        style: Theme.of(
          context,
        ).textTheme.titleSmall?.copyWith(fontWeight: FontWeight.w800),
      ),
    );
  }
}

class _PercentSlider extends StatelessWidget {
  const _PercentSlider({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.onChanged,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    return _ValueSlider(
      label: label,
      value: value,
      min: min,
      max: max,
      divisions: ((max - min) * 100).round(),
      valueText: '${(value * 100).round()}%',
      onChanged: onChanged,
    );
  }
}

class _ValueSlider extends StatelessWidget {
  const _ValueSlider({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.divisions,
    required this.onChanged,
    this.valueText,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final int divisions;
  final String? valueText;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    final text = valueText ?? value.toStringAsFixed(2);
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Column(
        children: <Widget>[
          Row(
            children: <Widget>[
              Expanded(child: Text(label)),
              Text(
                text,
                style: TextStyle(
                  color: Theme.of(context).colorScheme.onSurfaceVariant,
                ),
              ),
            ],
          ),
          Slider(
            value: value.clamp(min, max),
            min: min,
            max: max,
            divisions: divisions,
            label: text,
            onChanged: onChanged,
          ),
        ],
      ),
    );
  }
}

class _AvatarActionRow extends StatelessWidget {
  const _AvatarActionRow({
    required this.chooseLabel,
    required this.clearLabel,
    required this.clearEnabled,
    required this.onChoose,
    required this.onClear,
  });

  final String chooseLabel;
  final String clearLabel;
  final bool clearEnabled;
  final VoidCallback onChoose;
  final VoidCallback onClear;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(bottom: 8),
      child: Wrap(
        spacing: 8,
        runSpacing: 8,
        children: <Widget>[
          FilledButton.tonalIcon(
            onPressed: onChoose,
            icon: const Icon(Icons.image_outlined),
            label: Text(chooseLabel),
          ),
          OutlinedButton.icon(
            onPressed: clearEnabled ? onClear : null,
            icon: const Icon(Icons.person_off_outlined),
            label: Text(clearLabel),
          ),
        ],
      ),
    );
  }
}
