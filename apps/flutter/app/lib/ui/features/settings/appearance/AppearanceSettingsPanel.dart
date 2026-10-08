// ignore_for_file: file_names

import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:file_selector/file_selector.dart';
import 'package:flutter/material.dart';

import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../chat/components/style/bubble/BubbleSurface.dart';
import '../../../common/components/OperitDialog.dart';
import '../../../theme/OperitGlassSurface.dart';
import '../../../theme/OperitTheme.dart';
import '../../../theme/OperitThemeAssets.dart';
import '../components/SettingsControlStyles.dart';
import 'ChatAppearancePreview.dart';

enum _AppearanceSettingsTab { theme, background, bubbles, interaction }

const List<_AppearanceSettingsTab> _appearanceSettingsTabs =
    <_AppearanceSettingsTab>[
      _AppearanceSettingsTab.theme,
      _AppearanceSettingsTab.background,
      _AppearanceSettingsTab.bubbles,
      _AppearanceSettingsTab.interaction,
    ];

class AppearanceSettingsPanel extends StatelessWidget {
  const AppearanceSettingsPanel({super.key});

  /// Builds the tabbed appearance settings editor with responsive layouts.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final themeController = OperitTheme.of(context);
    final snapshot = themeController.themePreferenceSnapshot;
    return DefaultTabController(
      length: _appearanceSettingsTabs.length,
      child: Column(
        children: <Widget>[
          const _AppearanceHeaderBar(),
          Expanded(
            child: TabBarView(
              children: <Widget>[
                _buildThemeTab(context, l10n, themeController, snapshot),
                _buildBackgroundTab(context, l10n, themeController, snapshot),
                _buildBubblesTab(context, l10n, themeController, snapshot),
                _buildInteractionTab(context, l10n, themeController, snapshot),
              ],
            ),
          ),
        ],
      ),
    );
  }

  Widget _buildThemeTab(
    BuildContext context,
    AppLocalizations l10n,
    OperitThemeController themeController,
    ThemePreferenceSnapshot snapshot,
  ) {
    final hasCustomFont =
        snapshot.customFontPath != null && snapshot.customFontPath!.isNotEmpty;
    return _ResponsivePreviewSplitTabView(
      previewCard: _ChatAppearanceLivePreviewCard(
        snapshot: snapshot,
        showInputPreview: false,
      ),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsAppearanceThemeSection,
          icon: Icons.palette_outlined,
          children: <Widget>[
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceThemeMode,
              badge: _themeModeLabel(l10n, themeController.themeMode),
              child: _ThemeModeSelector(
                value: themeController.themeMode,
                onChanged: (themeMode) {
                  unawaited(themeController.setThemeMode(themeMode));
                },
              ),
            ),
            const SizedBox(height: 10),
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceMessageSurface,
              badge: _messageSurfaceLabel(l10n, _surfaceFromSnapshot(snapshot)),
              child: _MessageSurfaceSelector(
                value: _surfaceFromSnapshot(snapshot),
                onChanged: (value) {
                  unawaited(_applyMessageSurface(themeController, value));
                },
              ),
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceColorSection,
          icon: Icons.color_lens_outlined,
          children: <Widget>[
            _BodyText(l10n.settingsAppearanceColorDescription),
            const SizedBox(height: 4),
            _ThemeColorPresetSelector(
              selectedId: _selectedColorPresetId(snapshot),
              snapshot: snapshot,
              onChanged: (preset) {
                unawaited(
                  themeController.saveThemeSettings(
                    useCustomColors: preset.useCustomColors,
                    customPrimaryColor: preset.primaryColor,
                    customSecondaryColor: preset.secondaryColor,
                  ),
                );
              },
              onCustomTap: () {
                unawaited(
                  _showThemeColorDialog(context, themeController, snapshot),
                );
              },
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceTextSection,
          icon: Icons.text_fields_outlined,
          children: <Widget>[
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceFontFamily,
              badge: _fontFamilyLabel(l10n, snapshot),
              child: _FontFamilySelector(
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
            ),
            const SizedBox(height: 10),
            _CompactAssetTile(
              icon: Icons.font_download_outlined,
              title: l10n.settingsAppearanceCustomFont,
              subtitle: _customFontLabel(l10n, snapshot.customFontPath),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () {
                    unawaited(_pickCustomFont(themeController));
                  },
                  icon: const Icon(Icons.folder_open_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceChooseCustomFont),
                ),
                if (hasCustomFont)
                  SettingsEntityIconButton(
                    tooltip: l10n.settingsAppearanceClearCustomFont,
                    icon: Icons.close,
                    onPressed: () {
                      unawaited(
                        themeController.saveThemeSettings(
                          useCustomFont: false,
                          fontType: UserPreferencesManager.FONT_TYPE_SYSTEM,
                          customFontPath: '',
                        ),
                      );
                    },
                  ),
              ],
            ),
            const SizedBox(height: 8),
            SettingsSliderRow(
              icon: Icons.format_size_outlined,
              label: l10n.settingsAppearanceFontScale,
              value: snapshot.fontScale.clamp(0.85, 1.3),
              min: 0.85,
              max: 1.3,
              divisions: 45,
              valueText: '${(snapshot.fontScale * 100).round()}%',
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
          title: l10n.settingsAppearanceLanguageSection,
          icon: Icons.language_outlined,
          action: SettingsInfoBadge(label: l10n.localeName),
          children: <Widget>[
            _BodyText(l10n.settingsAppearanceLanguageDescription),
            const SizedBox(height: 8),
            Align(
              alignment: Alignment.centerRight,
              child: OutlinedButton.icon(
                style: SettingsControlStyles.sectionTextButton(),
                onPressed: () {
                  unawaited(themeController.resetThemeSettings());
                },
                icon: const Icon(Icons.restart_alt, size: 16),
                label: Text(l10n.settingsAppearanceResetTheme),
              ),
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildBackgroundTab(
    BuildContext context,
    AppLocalizations l10n,
    OperitThemeController themeController,
    ThemePreferenceSnapshot snapshot,
  ) {
    final hasBackgroundMedia =
        snapshot.backgroundImageUri != null &&
        snapshot.backgroundImageUri!.isNotEmpty;
    final isVideo =
        snapshot.backgroundMediaType == UserPreferencesManager.MEDIA_TYPE_VIDEO;
    return _ResponsivePreviewSplitTabView(
      previewCard: _ChatAppearanceLivePreviewCard(
        snapshot: snapshot,
        showInputPreview: false,
      ),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsAppearanceBackgroundSection,
          icon: Icons.wallpaper_outlined,
          children: <Widget>[
            _BodyText(l10n.settingsAppearanceBackgroundDescription),
            const SizedBox(height: 4),
            SettingsSwitchRow(
              icon: Icons.visibility_outlined,
              title: l10n.settingsAppearanceBackgroundEnabled,
              value: snapshot.useBackgroundImage,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(useBackgroundImage: value),
                );
              },
            ),
            const SizedBox(height: 8),
            _CompactAssetTile(
              icon: isVideo
                  ? Icons.movie_creation_outlined
                  : Icons.image_outlined,
              title: l10n.settingsAppearanceBackgroundImage,
              subtitle: _backgroundImageLabel(
                l10n,
                snapshot.backgroundImageUri,
              ),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () {
                    unawaited(_pickBackgroundImage(context, themeController));
                  },
                  icon: const Icon(Icons.image_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceBackgroundChooseImage),
                ),
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () {
                    unawaited(_pickBackgroundVideo(themeController));
                  },
                  icon: const Icon(Icons.movie_creation_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceBackgroundChooseVideo),
                ),
                if (hasBackgroundMedia && snapshot.useBackgroundImage)
                  SettingsEntityIconButton(
                    tooltip: l10n.settingsAppearanceBackgroundDisable,
                    icon: Icons.hide_image_outlined,
                    onPressed: () {
                      unawaited(
                        themeController.saveThemeSettings(
                          useBackgroundImage: false,
                        ),
                      );
                    },
                  ),
              ],
            ),
          ],
        ),
        if (isVideo)
          _SectionCard(
            title: l10n.settingsAppearanceBackgroundVideoSection,
            icon: Icons.smart_display_outlined,
            children: <Widget>[
              Wrap(
                spacing: 8,
                runSpacing: 8,
                children: <Widget>[
                  SettingsToggleChip(
                    icon: Icons.volume_off_outlined,
                    label: l10n.settingsAppearanceBackgroundVideoMuted,
                    selected: snapshot.videoBackgroundMuted,
                    onSelected: (value) {
                      unawaited(
                        themeController.saveThemeSettings(
                          videoBackgroundMuted: value,
                        ),
                      );
                    },
                  ),
                  SettingsToggleChip(
                    icon: Icons.repeat_outlined,
                    label: l10n.settingsAppearanceBackgroundVideoLoop,
                    selected: snapshot.videoBackgroundLoop,
                    onSelected: (value) {
                      unawaited(
                        themeController.saveThemeSettings(
                          videoBackgroundLoop: value,
                        ),
                      );
                    },
                  ),
                ],
              ),
            ],
          ),
        _SectionCard(
          title: l10n.settingsAppearanceBackgroundEffectsSection,
          icon: Icons.auto_fix_high_outlined,
          children: <Widget>[
            SettingsSliderRow(
              icon: Icons.opacity_outlined,
              label: l10n.settingsAppearanceBackgroundOpacity,
              value: snapshot.backgroundImageOpacity.clamp(0.1, 0.8),
              min: 0.1,
              max: 0.8,
              divisions: 70,
              valueText: '${(snapshot.backgroundImageOpacity * 100).round()}%',
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
            const Divider(height: 14),
            SettingsSwitchRow(
              icon: Icons.blur_on_outlined,
              title: l10n.settingsAppearanceBackgroundBlur,
              value: snapshot.useBackgroundBlur,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(useBackgroundBlur: value),
                );
              },
            ),
            if (snapshot.useBackgroundBlur) ...<Widget>[
              const SizedBox(height: 4),
              SettingsSliderRow(
                icon: Icons.tune_outlined,
                label: l10n.settingsAppearanceBackgroundBlurRadius,
                value: snapshot.backgroundBlurRadius.clamp(0, 40),
                min: 0,
                max: 40,
                divisions: 40,
                valueText: '${snapshot.backgroundBlurRadius.round()} px',
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
            ],
          ],
        ),
      ],
    );
  }

  Widget _buildBubblesTab(
    BuildContext context,
    AppLocalizations l10n,
    OperitThemeController themeController,
    ThemePreferenceSnapshot snapshot,
  ) {
    final userImageActive =
        snapshot.bubbleUserUseImage &&
        snapshot.bubbleUserImageUri != null &&
        snapshot.bubbleUserImageUri!.isNotEmpty;
    final aiImageActive =
        snapshot.bubbleAiUseImage &&
        snapshot.bubbleAiImageUri != null &&
        snapshot.bubbleAiImageUri!.isNotEmpty;

    return _ResponsivePreviewSplitTabView(
      previewCard: _ChatAppearanceLivePreviewCard(
        snapshot: snapshot,
        showInputPreview: false,
      ),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsAppearanceBubbleStyleSection,
          icon: Icons.chat_bubble_outline,
          children: <Widget>[
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceMessageStyle,
              badge: _messageStyleLabel(l10n, snapshot.chatStyle),
              child: _MessageStyleSelector(
                value: snapshot.chatStyle,
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(chatStyle: value),
                  );
                },
              ),
            ),
            const SizedBox(height: 10),
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceMessageDensity,
              badge: _messageDensityLabel(l10n, _densityFromSnapshot(snapshot)),
              child: _MessageDensitySelector(
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
            ),
            const SizedBox(height: 10),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                SettingsToggleChip(
                  icon: Icons.aspect_ratio_outlined,
                  label: l10n.settingsAppearanceWideLayout,
                  selected: snapshot.bubbleWideLayoutEnabled,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        bubbleWideLayoutEnabled: value,
                      ),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.rounded_corner_outlined,
                  label: l10n.settingsAppearanceRoundedMessages,
                  selected:
                      snapshot.bubbleUserRoundedCornersEnabled &&
                      snapshot.bubbleAiRoundedCornersEnabled,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        bubbleUserRoundedCornersEnabled: value,
                        bubbleAiRoundedCornersEnabled: value,
                      ),
                    );
                  },
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceMessageColors,
          icon: Icons.format_color_fill_outlined,
          action: SettingsInfoBadge(
            label: _messageColorPresetLabel(l10n, snapshot),
          ),
          children: <Widget>[
            _MessageColorPresetSelector(
              value: _messageColorPresetFromSnapshot(snapshot),
              onChanged: (value) {
                unawaited(_applyMessageColorPreset(themeController, value));
              },
              onCustomTap: () {
                unawaited(
                  _showMessageColorDialog(context, themeController, snapshot),
                );
              },
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceBubbleFontSection,
          icon: Icons.font_download_outlined,
          children: <Widget>[
            _CompactAssetTile(
              icon: Icons.person_outline,
              title: l10n.settingsAppearanceUserBubbleFont,
              subtitle: _bubbleFontLabel(l10n, snapshot, isUser: true),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
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
                  icon: const Icon(Icons.tune_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceAdjustUserBubbleFont),
                ),
              ],
            ),
            const SizedBox(height: 8),
            _CompactAssetTile(
              icon: Icons.smart_toy_outlined,
              title: l10n.settingsAppearanceAiBubbleFont,
              subtitle: _bubbleFontLabel(l10n, snapshot, isUser: false),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
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
                  icon: const Icon(Icons.tune_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceAdjustAiBubbleFont),
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceBubbleImageSection,
          icon: Icons.layers_outlined,
          children: <Widget>[
            _CompactAssetTile(
              icon: Icons.person_outline,
              title: l10n.settingsAppearanceUserBubbleImage,
              subtitle: _fileNameOrNoneLabel(
                l10n,
                snapshot.bubbleUserImageUri,
                snapshot.bubbleUserUseImage,
              ),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () {
                    unawaited(
                      _pickBubbleImage(
                        themeController,
                        snapshot: snapshot,
                        isUser: true,
                      ),
                    );
                  },
                  icon: const Icon(Icons.image_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceChooseUserBubbleImage),
                ),
                if (userImageActive) ...<Widget>[
                  OutlinedButton.icon(
                    style: SettingsControlStyles.sectionTextButton(),
                    onPressed: () {
                      unawaited(
                        _showBubbleImageAdjustDialog(
                          context,
                          themeController,
                          snapshot,
                          isUser: true,
                        ),
                      );
                    },
                    icon: const Icon(Icons.tune_outlined, size: 16),
                    label: Text(l10n.settingsAppearanceBubbleImageAdjustUser),
                  ),
                  SettingsEntityIconButton(
                    tooltip: l10n.settingsAppearanceClearUserBubbleImage,
                    icon: Icons.layers_clear_outlined,
                    onPressed: () {
                      unawaited(
                        themeController.saveThemeSettings(
                          bubbleUserUseImage: false,
                          bubbleUserImageUri: '',
                        ),
                      );
                    },
                  ),
                ],
              ],
              bottom: userImageActive
                  ? _BubbleImageRenderModeSelector(
                      value: snapshot.bubbleUserImageRenderMode,
                      onChanged: (value) {
                        unawaited(
                          themeController.saveThemeSettings(
                            bubbleUserImageRenderMode: value,
                          ),
                        );
                      },
                    )
                  : null,
            ),
            const SizedBox(height: 8),
            _CompactAssetTile(
              icon: Icons.smart_toy_outlined,
              title: l10n.settingsAppearanceAiBubbleImage,
              subtitle: _fileNameOrNoneLabel(
                l10n,
                snapshot.bubbleAiImageUri,
                snapshot.bubbleAiUseImage,
              ),
              actions: <Widget>[
                FilledButton.tonalIcon(
                  style: SettingsControlStyles.sectionTextButton(),
                  onPressed: () {
                    unawaited(
                      _pickBubbleImage(
                        themeController,
                        snapshot: snapshot,
                        isUser: false,
                      ),
                    );
                  },
                  icon: const Icon(Icons.image_outlined, size: 16),
                  label: Text(l10n.settingsAppearanceChooseAiBubbleImage),
                ),
                if (aiImageActive) ...<Widget>[
                  OutlinedButton.icon(
                    style: SettingsControlStyles.sectionTextButton(),
                    onPressed: () {
                      unawaited(
                        _showBubbleImageAdjustDialog(
                          context,
                          themeController,
                          snapshot,
                          isUser: false,
                        ),
                      );
                    },
                    icon: const Icon(Icons.tune_outlined, size: 16),
                    label: Text(l10n.settingsAppearanceBubbleImageAdjustAi),
                  ),
                  SettingsEntityIconButton(
                    tooltip: l10n.settingsAppearanceClearAiBubbleImage,
                    icon: Icons.layers_clear_outlined,
                    onPressed: () {
                      unawaited(
                        themeController.saveThemeSettings(
                          bubbleAiUseImage: false,
                          bubbleAiImageUri: '',
                        ),
                      );
                    },
                  ),
                ],
              ],
              bottom: aiImageActive
                  ? _BubbleImageRenderModeSelector(
                      value: snapshot.bubbleAiImageRenderMode,
                      onChanged: (value) {
                        unawaited(
                          themeController.saveThemeSettings(
                            bubbleAiImageRenderMode: value,
                          ),
                        );
                      },
                    )
                  : null,
            ),
          ],
        ),
      ],
    );
  }

  Widget _buildInteractionTab(
    BuildContext context,
    AppLocalizations l10n,
    OperitThemeController themeController,
    ThemePreferenceSnapshot snapshot,
  ) {
    return _ResponsivePreviewSplitTabView(
      previewCard: _ChatAppearanceLivePreviewCard(
        snapshot: snapshot,
        showInputPreview: true,
      ),
      children: <Widget>[
        _SectionCard(
          title: l10n.settingsAppearanceAvatarSection,
          icon: Icons.account_circle_outlined,
          children: <Widget>[
            SettingsSwitchRow(
              icon: Icons.visibility_outlined,
              title: l10n.settingsAppearanceShowAvatars,
              value: snapshot.bubbleShowAvatar,
              onChanged: (value) {
                unawaited(
                  themeController.saveThemeSettings(bubbleShowAvatar: value),
                );
              },
            ),
            if (snapshot.bubbleShowAvatar) ...<Widget>[
              const SizedBox(height: 8),
              _AppearanceFieldBlock(
                label: l10n.settingsAppearanceAvatarShape,
                badge: _avatarShapeLabel(l10n, snapshot.avatarShape),
                child: _AvatarShapeSelector(
                  value: _avatarShapeFromSnapshot(snapshot.avatarShape),
                  onChanged: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        avatarShape: _avatarShapeValue(value),
                      ),
                    );
                  },
                ),
              ),
            ],
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceInputSection,
          icon: Icons.keyboard_outlined,
          children: <Widget>[
            _AppearanceFieldBlock(
              label: l10n.settingsAppearanceInputStyle,
              badge: _inputStyleLabel(l10n, snapshot.inputStyle),
              child: _InputStyleSelector(
                value: _inputStyleValue(snapshot.inputStyle),
                onChanged: (value) {
                  unawaited(
                    themeController.saveThemeSettings(inputStyle: value),
                  );
                },
              ),
            ),
            const SizedBox(height: 8),
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                SettingsToggleChip(
                  icon: Icons.vertical_align_bottom_outlined,
                  label: l10n.settingsAppearanceInputFloating,
                  selected: snapshot.chatInputFloating,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        chatInputFloating: value,
                      ),
                    );
                  },
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceInteractionStatusSection,
          icon: Icons.psychology_outlined,
          children: <Widget>[
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                SettingsToggleChip(
                  icon: Icons.psychology_outlined,
                  label: l10n.settingsAppearanceShowThinkingProcess,
                  selected: snapshot.showThinkingProcess,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showThinkingProcess: value,
                      ),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.hourglass_top_outlined,
                  label: l10n.settingsAppearanceShowInputProcessingStatus,
                  selected: snapshot.showInputProcessingStatus,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showInputProcessingStatus: value,
                      ),
                    );
                  },
                ),
              ],
            ),
          ],
        ),
        _SectionCard(
          title: l10n.settingsAppearanceMessageDisplaySection,
          icon: Icons.tune_outlined,
          children: <Widget>[
            Wrap(
              spacing: 8,
              runSpacing: 8,
              children: <Widget>[
                SettingsToggleChip(
                  icon: Icons.badge_outlined,
                  label: l10n.settingsAppearanceShowRoleName,
                  selected: snapshot.showRoleName,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(showRoleName: value),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.person_outline,
                  label: l10n.settingsAppearanceShowUserName,
                  selected: snapshot.showUserName,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(showUserName: value),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.memory_outlined,
                  label: l10n.settingsAppearanceShowModelName,
                  selected: snapshot.showModelName,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(showModelName: value),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.cloud_outlined,
                  label: l10n.settingsAppearanceShowModelProvider,
                  selected: snapshot.showModelProvider,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showModelProvider: value,
                      ),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.data_usage_outlined,
                  label: l10n.settingsAppearanceShowMessageTokenStats,
                  selected: snapshot.showMessageTokenStats,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showMessageTokenStats: value,
                      ),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.timer_outlined,
                  label: l10n.settingsAppearanceShowMessageTimingStats,
                  selected: snapshot.showMessageTimingStats,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showMessageTimingStats: value,
                      ),
                    );
                  },
                ),
                SettingsToggleChip(
                  icon: Icons.schedule_outlined,
                  label: l10n.settingsAppearanceShowMessageTimestamp,
                  selected: snapshot.showMessageTimestamp,
                  onSelected: (value) {
                    unawaited(
                      themeController.saveThemeSettings(
                        showMessageTimestamp: value,
                      ),
                    );
                  },
                ),
              ],
            ),
          ],
        ),
      ],
    );
  }
}

class _AppearanceHeaderBar extends StatelessWidget {
  /// Creates the independent appearance-category header.
  const _AppearanceHeaderBar();

  /// Builds the responsive appearance tabs without plugin-owned target selection.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final colorScheme = Theme.of(context).colorScheme;
    return LayoutBuilder(
      builder: (context, constraints) {
        final useHorizontalHeader = constraints.maxWidth >= 680;
        if (useHorizontalHeader) {
          return Column(
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Padding(
                padding: const EdgeInsets.fromLTRB(16, 10, 16, 6),
                child: Row(
                  children: <Widget>[
                    Expanded(
                      child: Material(
                        color: Colors.transparent,
                        child: TabBar(
                          isScrollable: true,
                          tabAlignment: TabAlignment.start,
                          dividerColor: Colors.transparent,
                          tabs: <Widget>[
                            for (final tab in _appearanceSettingsTabs)
                              Tab(text: _appearanceSettingsTabLabel(l10n, tab)),
                          ],
                        ),
                      ),
                    ),
                  ],
                ),
              ),
              Divider(
                height: 1,
                color: colorScheme.outlineVariant.withValues(alpha: 0.22),
              ),
            ],
          );
        }

        return Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Material(
              color: Colors.transparent,
              child: TabBar(
                isScrollable: true,
                tabAlignment: TabAlignment.start,
                tabs: <Widget>[
                  for (final tab in _appearanceSettingsTabs)
                    Tab(text: _appearanceSettingsTabLabel(l10n, tab)),
                ],
              ),
            ),
          ],
        );
      },
    );
  }
}

class _ResponsivePreviewSplitTabView extends StatefulWidget {
  /// Creates the responsive independent appearance controls and their preview.
  const _ResponsivePreviewSplitTabView({
    required this.previewCard,
    required this.children,
  });

  final Widget previewCard;
  final List<Widget> children;

  @override
  State<_ResponsivePreviewSplitTabView> createState() =>
      _ResponsivePreviewSplitTabViewState();
}

class _ResponsivePreviewSplitTabViewState
    extends State<_ResponsivePreviewSplitTabView> {
  bool _mobilePreviewExpanded = false;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 820) {
          final previewWidth = (constraints.maxWidth * 0.40).clamp(
            340.0,
            420.0,
          );
          return Align(
            alignment: Alignment.topCenter,
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 1200),
              child: Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(
                    child: ListView(
                      padding: const EdgeInsets.fromLTRB(16, 12, 8, 24),
                      children: widget.children,
                    ),
                  ),
                  SizedBox(
                    width: previewWidth,
                    child: SingleChildScrollView(
                      padding: const EdgeInsets.fromLTRB(8, 12, 16, 24),
                      child: widget.previewCard,
                    ),
                  ),
                ],
              ),
            ),
          );
        }

        final l10n = AppLocalizations.of(context)!;
        final colorScheme = Theme.of(context).colorScheme;
        return ListView(
          padding: const EdgeInsets.fromLTRB(16, 10, 16, 20),
          children: <Widget>[
            Padding(
              padding: const EdgeInsets.only(bottom: 8),
              child: Material(
                color: colorScheme.surfaceContainerHighest.withValues(
                  alpha: 0.35,
                ),
                shape: RoundedRectangleBorder(
                  borderRadius: BorderRadius.circular(12),
                  side: BorderSide(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.22),
                  ),
                ),
                child: InkWell(
                  borderRadius: BorderRadius.circular(12),
                  onTap: () {
                    setState(() {
                      _mobilePreviewExpanded = !_mobilePreviewExpanded;
                    });
                  },
                  child: Padding(
                    padding: const EdgeInsets.symmetric(
                      horizontal: 14,
                      vertical: 10,
                    ),
                    child: Row(
                      children: <Widget>[
                        Icon(
                          Icons.visibility_outlined,
                          size: 18,
                          color: colorScheme.primary,
                        ),
                        const SizedBox(width: 8),
                        Expanded(
                          child: Text(
                            l10n.settingsAppearanceLivePreviewTitle,
                            style: Theme.of(context).textTheme.titleSmall
                                ?.copyWith(fontWeight: FontWeight.w600),
                          ),
                        ),
                        SettingsInfoBadge(
                          label: _mobilePreviewExpanded ? '折叠' : '展开',
                        ),
                        const SizedBox(width: 4),
                        Icon(
                          _mobilePreviewExpanded
                              ? Icons.keyboard_arrow_up
                              : Icons.keyboard_arrow_down,
                          size: 18,
                          color: colorScheme.onSurfaceVariant,
                        ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
            if (_mobilePreviewExpanded) ...<Widget>[
              Padding(
                padding: const EdgeInsets.only(bottom: 12),
                child: widget.previewCard,
              ),
            ],
            ...widget.children,
          ],
        );
      },
    );
  }
}

class _ChatAppearanceLivePreviewCard extends StatelessWidget {
  const _ChatAppearanceLivePreviewCard({
    required this.snapshot,
    required this.showInputPreview,
  });

  final ThemePreferenceSnapshot snapshot;
  final bool showInputPreview;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return _SectionCard(
      title: l10n.settingsAppearanceLivePreviewTitle,
      icon: Icons.preview_outlined,
      action: SettingsInfoBadge(
        label: _messageStyleLabel(l10n, snapshot.chatStyle),
      ),
      children: <Widget>[
        ChatAppearancePreview(showInputPreview: showInputPreview),
      ],
    );
  }
}

/// Returns the localized tab label for one appearance settings section.
String _appearanceSettingsTabLabel(
  AppLocalizations l10n,
  _AppearanceSettingsTab tab,
) {
  return switch (tab) {
    _AppearanceSettingsTab.theme => l10n.settingsAppearanceThemeSection,
    _AppearanceSettingsTab.background =>
      l10n.settingsAppearanceBackgroundSection,
    _AppearanceSettingsTab.bubbles => l10n.settingsAppearanceBubblesTab,
    _AppearanceSettingsTab.interaction => l10n.settingsAppearanceInteractionTab,
  };
}

/// Crops, imports, and saves a selected image as the active background asset.
Future<void> _pickBackgroundImage(
  BuildContext context,
  OperitThemeController themeController,
) async {
  final screenSize = MediaQuery.sizeOf(context);
  final imported = await _pickCroppedThemeImage(
    context,
    aspectRatio: screenSize.width / screenSize.height,
    outputRole: 'background',
  );
  if (imported == null) {
    return;
  }
  await themeController.saveThemeSettings(
    useBackgroundImage: true,
    backgroundImageUri: imported.storagePath,
    backgroundMediaType: UserPreferencesManager.MEDIA_TYPE_IMAGE,
  );
}

/// Imports and saves a selected video as the active background asset.
Future<void> _pickBackgroundVideo(OperitThemeController themeController) async {
  const videoGroup = XTypeGroup(
    label: 'video',
    extensions: <String>['mp4', 'mov', 'm4v', 'webm', 'mkv', 'avi'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[videoGroup]);
  if (file == null) {
    return;
  }
  final imported = await ThemeAssetStore().importFile(file);
  await themeController.saveThemeSettings(
    useBackgroundImage: true,
    backgroundImageUri: imported.storagePath,
    backgroundMediaType: UserPreferencesManager.MEDIA_TYPE_VIDEO,
    videoBackgroundMuted: true,
    videoBackgroundLoop: true,
  );
}

/// Lets the user select and crop one image before importing it.
Future<ThemeAssetImport?> _pickCroppedThemeImage(
  BuildContext context, {
  required double aspectRatio,
  required String outputRole,
}) async {
  const imageGroup = XTypeGroup(
    label: 'image',
    extensions: <String>['jpg', 'jpeg', 'png', 'webp', 'bmp', 'gif'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[imageGroup]);
  if (file == null) {
    return null;
  }
  final bytes = await file.readAsBytes();
  final croppedBytes = await _showThemeImageCropDialog(
    // ignore: use_build_context_synchronously
    context,
    bytes: bytes,
    aspectRatio: aspectRatio,
  );
  if (croppedBytes == null) {
    return null;
  }
  return ThemeAssetStore().importBytes(
    bytes: croppedBytes,
    fileName: _croppedThemeImageFileName(file.name, outputRole),
  );
}

/// Builds a PNG file name for a cropped theme image.
String _croppedThemeImageFileName(String fileName, String outputRole) {
  final normalized = fileName.replaceAll('\\', '/');
  final slashIndex = normalized.lastIndexOf('/');
  final baseName = normalized.substring(slashIndex + 1).trim();
  if (baseName.isEmpty) {
    throw StateError('theme image file name is empty');
  }
  final dotIndex = baseName.lastIndexOf('.');
  final stem = dotIndex <= 0 ? baseName : baseName.substring(0, dotIndex);
  return '${stem}_$outputRole.png';
}

/// Shows an equal-aspect crop dialog and returns encoded PNG bytes.
Future<Uint8List?> _showThemeImageCropDialog(
  BuildContext context, {
  required Uint8List bytes,
  required double aspectRatio,
}) async {
  final sourceImage = await _decodeThemeImage(bytes);
  return showDialog<Uint8List>(
    // ignore: use_build_context_synchronously
    context: context,
    builder: (dialogContext) {
      return _ThemeImageCropDialog(
        sourceImage: sourceImage,
        aspectRatio: aspectRatio,
      );
    },
  );
}

/// Decodes selected image bytes for the crop editor.
Future<ui.Image> _decodeThemeImage(Uint8List bytes) async {
  final codec = await ui.instantiateImageCodec(bytes);
  final frame = await codec.getNextFrame();
  return frame.image;
}

class _ThemeImageCropSettings {
  /// Creates cover crop settings for one source image.
  const _ThemeImageCropSettings({
    required this.zoom,
    required this.offsetX,
    required this.offsetY,
  });

  final double zoom;
  final double offsetX;
  final double offsetY;
}

class _ThemeImageCropDialog extends StatefulWidget {
  /// Creates a crop dialog that preserves the requested aspect ratio.
  const _ThemeImageCropDialog({
    required this.sourceImage,
    required this.aspectRatio,
  });

  final ui.Image sourceImage;
  final double aspectRatio;

  /// Creates the crop dialog state.
  @override
  State<_ThemeImageCropDialog> createState() => _ThemeImageCropDialogState();
}

class _ThemeImageCropDialogState extends State<_ThemeImageCropDialog> {
  double _zoom = 1;
  double _offsetX = 0;
  double _offsetY = 0;

  /// Releases the decoded source image after the dialog is unmounted.
  @override
  void dispose() {
    widget.sourceImage.dispose();
    super.dispose();
  }

  /// Builds the crop editor with cover preview controls.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    final settings = _ThemeImageCropSettings(
      zoom: _zoom,
      offsetX: _offsetX,
      offsetY: _offsetY,
    );
    final maxDialogHeight = (MediaQuery.sizeOf(context).height * 0.9)
        .clamp(0.0, 720.0)
        .toDouble();
    return OperitDialogScaffold(
      title: l10n.settingsAppearanceBubbleImageCrop,
      maxWidth: 460,
      // Keep the action bar on screen on small portrait devices. Without a
      // finite height, the scroll view receives unbounded constraints and
      // lays out the tall background preview together with all controls,
      // leaving the save button outside the viewport (especially on iOS).
      maxHeight: maxDialogHeight,
      showCloseButton: true,
      actions: <Widget>[
        TextButton(
          onPressed: () => Navigator.of(context).pop(),
          child: Text(l10n.cancel),
        ),
        FilledButton(
          onPressed: () {
            unawaited(_saveCroppedImage(context, settings));
          },
          child: Text(l10n.save),
        ),
      ],
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.start,
          children: <Widget>[
            AspectRatio(
              aspectRatio: widget.aspectRatio,
              child: ClipRRect(
                borderRadius: BorderRadius.circular(12),
                child: CustomPaint(
                  painter: _ThemeImageCropPreviewPainter(
                    image: widget.sourceImage,
                    aspectRatio: widget.aspectRatio,
                    settings: settings,
                  ),
                ),
              ),
            ),
            const SizedBox(height: 12),
            SettingsSliderRow(
              icon: Icons.zoom_in_outlined,
              label: l10n.zoom,
              value: _zoom,
              min: 1,
              max: 4,
              divisions: 30,
              valueText: '${_zoom.toStringAsFixed(1)}x',
              onChanged: (value) => setState(() => _zoom = value),
            ),
            SettingsSliderRow(
              icon: Icons.swap_horiz_outlined,
              label: 'X',
              value: _offsetX,
              min: -1,
              max: 1,
              divisions: 40,
              valueText: _offsetX.toStringAsFixed(2),
              onChanged: (value) => setState(() => _offsetX = value),
            ),
            SettingsSliderRow(
              icon: Icons.swap_vert_outlined,
              label: 'Y',
              value: _offsetY,
              min: -1,
              max: 1,
              divisions: 40,
              valueText: _offsetY.toStringAsFixed(2),
              onChanged: (value) => setState(() => _offsetY = value),
            ),
          ],
        ),
      ),
    );
  }

  /// Encodes the current crop and closes the dialog.
  Future<void> _saveCroppedImage(
    BuildContext context,
    _ThemeImageCropSettings settings,
  ) async {
    final bytes = await _encodeCroppedThemeImage(
      widget.sourceImage,
      aspectRatio: widget.aspectRatio,
      settings: settings,
    );
    if (!context.mounted) {
      return;
    }
    Navigator.of(context).pop(bytes);
  }
}

class _ThemeImageCropPreviewPainter extends CustomPainter {
  /// Creates a painter for the exact crop area that will be encoded.
  const _ThemeImageCropPreviewPainter({
    required this.image,
    required this.aspectRatio,
    required this.settings,
  });

  final ui.Image image;
  final double aspectRatio;
  final _ThemeImageCropSettings settings;

  /// Paints the cropped image without stretching its pixels.
  @override
  void paint(Canvas canvas, Size size) {
    final src = _themeImageCropRect(
      image,
      aspectRatio: aspectRatio,
      settings: settings,
    );
    canvas.drawImageRect(image, src, Offset.zero & size, Paint());
  }

  /// Reports whether the crop preview needs repainting.
  @override
  bool shouldRepaint(covariant _ThemeImageCropPreviewPainter oldDelegate) {
    return oldDelegate.image != image ||
        oldDelegate.aspectRatio != aspectRatio ||
        oldDelegate.settings != settings;
  }
}

/// Encodes one cover crop as PNG bytes.
Future<Uint8List> _encodeCroppedThemeImage(
  ui.Image image, {
  required double aspectRatio,
  required _ThemeImageCropSettings settings,
}) async {
  final src = _themeImageCropRect(
    image,
    aspectRatio: aspectRatio,
    settings: settings,
  );
  final outputWidth = src.width.round().clamp(1, image.width).toInt();
  final outputHeight = src.height.round().clamp(1, image.height).toInt();
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawImageRect(
    image,
    src,
    Rect.fromLTWH(0, 0, outputWidth.toDouble(), outputHeight.toDouble()),
    Paint(),
  );
  final picture = recorder.endRecording();
  final croppedImage = await picture.toImage(outputWidth, outputHeight);
  final byteData = await croppedImage.toByteData(
    format: ui.ImageByteFormat.png,
  );
  picture.dispose();
  croppedImage.dispose();
  if (byteData == null) {
    throw StateError('cropped theme image bytes are unavailable');
  }
  return byteData.buffer.asUint8List();
}

/// Calculates a non-stretched source crop rectangle for cover display.
Rect _themeImageCropRect(
  ui.Image image, {
  required double aspectRatio,
  required _ThemeImageCropSettings settings,
}) {
  if (aspectRatio <= 0) {
    throw StateError('theme image crop aspect ratio must be positive');
  }
  final imageWidth = image.width.toDouble();
  final imageHeight = image.height.toDouble();
  final imageAspectRatio = imageWidth / imageHeight;
  final baseWidth = imageAspectRatio > aspectRatio
      ? imageHeight * aspectRatio
      : imageWidth;
  final baseHeight = imageAspectRatio > aspectRatio
      ? imageHeight
      : imageWidth / aspectRatio;
  final zoom = settings.zoom.clamp(1.0, 4.0);
  final cropWidth = baseWidth / zoom;
  final cropHeight = baseHeight / zoom;
  final xRange = (imageWidth - cropWidth) / 2;
  final yRange = (imageHeight - cropHeight) / 2;
  final left = xRange + xRange * settings.offsetX.clamp(-1.0, 1.0);
  final top = yRange + yRange * settings.offsetY.clamp(-1.0, 1.0);
  return Rect.fromLTWH(left, top, cropWidth, cropHeight);
}

/// Imports and saves a selected font as the active custom font asset.
Future<void> _pickCustomFont(OperitThemeController themeController) async {
  const fontGroup = XTypeGroup(
    label: 'font',
    extensions: <String>['ttf', 'otf', 'ttc'],
  );
  final file = await openFile(acceptedTypeGroups: <XTypeGroup>[fontGroup]);
  if (file == null) {
    return;
  }
  final imported = await ThemeAssetStore().importFile(file);
  await themeController.saveThemeSettings(
    useCustomFont: true,
    fontType: UserPreferencesManager.FONT_TYPE_FILE,
    customFontPath: imported.storagePath,
  );
}

/// Shows font controls for the selected message bubble side.
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
            final imported = await ThemeAssetStore().importFile(file);
            setDialogState(() {
              useCustomFont = true;
              fontType = UserPreferencesManager.FONT_TYPE_FILE;
              customFontPath = imported.storagePath;
            });
          }

          return OperitDialogScaffold(
            title: isUser
                ? l10n.settingsAppearanceAdjustUserBubbleFont
                : l10n.settingsAppearanceAdjustAiBubbleFont,
            maxWidth: 460,
            showCloseButton: true,
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
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                SettingsSwitchRow(
                  icon: Icons.font_download_outlined,
                  title: l10n.settingsAppearanceEnableBubbleFont,
                  value: useCustomFont,
                  onChanged: (value) {
                    setDialogState(() {
                      useCustomFont = value;
                    });
                  },
                ),
                const SizedBox(height: 8),
                _AppearanceFieldBlock(
                  label: l10n.settingsAppearanceFontFamily,
                  badge: _fontFamilyPresetLabel(
                    l10n,
                    _fontFamilyPresetFromSystemName(systemFontName),
                  ),
                  child: _FontFamilySelector(
                    value: _fontFamilyPresetFromSystemName(systemFontName),
                    onChanged: (value) {
                      setDialogState(() {
                        useCustomFont = true;
                        fontType = UserPreferencesManager.FONT_TYPE_SYSTEM;
                        systemFontName = _systemFontNameFromPreset(value);
                      });
                    },
                  ),
                ),
                const SizedBox(height: 10),
                _CompactAssetTile(
                  icon: Icons.folder_open_outlined,
                  title: l10n.settingsAppearanceCustomFont,
                  subtitle: _customFontLabel(l10n, customFontPath),
                  actions: <Widget>[
                    FilledButton.tonalIcon(
                      style: SettingsControlStyles.sectionTextButton(),
                      onPressed: () {
                        unawaited(pickFontFile());
                      },
                      icon: const Icon(Icons.file_upload_outlined, size: 16),
                      label: Text(l10n.settingsAppearanceChooseCustomFont),
                    ),
                    if (customFontPath != null && customFontPath!.isNotEmpty)
                      SettingsEntityIconButton(
                        tooltip: l10n.settingsAppearanceClearCustomFont,
                        icon: Icons.close,
                        onPressed: () {
                          setDialogState(() {
                            customFontPath = '';
                            fontType = UserPreferencesManager.FONT_TYPE_SYSTEM;
                          });
                        },
                      ),
                  ],
                ),
              ],
            ),
          );
        },
      );
    },
  );
}

/// Imports and saves a selected image as a bubble surface asset.
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
  final imported = await ThemeAssetStore().importFile(file);
  final useImage = !snapshot.transparentSurfaceEnabled;
  if (_isNinePatchPngPath(imported.fileName)) {
    final autoParams = await _parseNinePatchBubbleParams(imported.bytes);
    await themeController.saveThemeSettings(
      bubbleUserImageRenderMode: isUser
          ? UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
          : null,
      bubbleAiImageRenderMode: isUser
          ? null
          : UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
      bubbleUserUseImage: isUser ? useImage : null,
      bubbleAiUseImage: isUser ? null : useImage,
      bubbleUserImageUri: isUser ? imported.storagePath : null,
      bubbleAiImageUri: isUser ? null : imported.storagePath,
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
    bubbleUserImageUri: isUser ? imported.storagePath : null,
    bubbleAiImageUri: isUser ? null : imported.storagePath,
  );
}

class _NinePatchBubbleAutoParams {
  /// Creates parsed nine-patch ratios for bubble image rendering.
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

/// Returns whether a selected image file name uses Android nine-patch naming.
bool _isNinePatchPngPath(String path) {
  return path.toLowerCase().endsWith('.9.png');
}

/// Parses nine-patch stretch markers from decoded PNG bytes.
Future<_NinePatchBubbleAutoParams> _parseNinePatchBubbleParams(
  Uint8List bytes,
) async {
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

/// Returns whether one pixel is a nine-patch stretch marker.
bool _isNinePatchMarker(ByteData bytes, int width, int x, int y) {
  final offset = ((y * width + x) * 4);
  final red = bytes.getUint8(offset);
  final green = bytes.getUint8(offset + 1);
  final blue = bytes.getUint8(offset + 2);
  final alpha = bytes.getUint8(offset + 3);
  return alpha >= 0x80 && red < 32 && green < 32 && blue < 32;
}

/// Converts marked pixel positions into normalized stretch bounds.
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
  final colorScheme = Theme.of(context).colorScheme;
  final imagePath = isUser
      ? snapshot.bubbleUserImageUri ?? ''
      : snapshot.bubbleAiImageUri ?? '';
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
          void update(VoidCallback callback) {
            setDialogState(callback);
          }

          final previewColor = isUser
              ? snapshot.bubbleUserBubbleColor == null
                    ? colorScheme.primary
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

          return OperitDialogScaffold(
            title: isUser
                ? l10n.settingsAppearanceBubbleImageAdjustUser
                : l10n.settingsAppearanceBubbleImageAdjustAi,
            maxWidth: 520,
            maxHeight: 660,
            showCloseButton: true,
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
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  SizedBox(
                    height: 104,
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
                  const SizedBox(height: 12),
                  _DialogSectionTitle(l10n.settingsAppearanceBubbleImageCrop),
                  _TwoColumnSliderGrid(
                    children: <Widget>[
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageCropLeft,
                        value: cropLeft,
                        min: 0,
                        max: 0.45,
                        divisions: 45,
                        valueText: '${(cropLeft * 100).round()}%',
                        onChanged: (value) => update(() => cropLeft = value),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageCropTop,
                        value: cropTop,
                        min: 0,
                        max: 0.45,
                        divisions: 45,
                        valueText: '${(cropTop * 100).round()}%',
                        onChanged: (value) => update(() => cropTop = value),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageCropRight,
                        value: cropRight,
                        min: 0,
                        max: 0.45,
                        divisions: 45,
                        valueText: '${(cropRight * 100).round()}%',
                        onChanged: (value) => update(() => cropRight = value),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageCropBottom,
                        value: cropBottom,
                        min: 0,
                        max: 0.45,
                        divisions: 45,
                        valueText: '${(cropBottom * 100).round()}%',
                        onChanged: (value) => update(() => cropBottom = value),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  _DialogSectionTitle(l10n.settingsAppearanceBubbleImageRepeat),
                  _TwoColumnSliderGrid(
                    children: <Widget>[
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageRepeatStart,
                        value: repeatStart,
                        min: 0.05,
                        max: 0.9,
                        divisions: 85,
                        valueText: '${(repeatStart * 100).round()}%',
                        onChanged: (value) => update(() {
                          repeatStart = value;
                          if (repeatEnd <= repeatStart + 0.01) {
                            repeatEnd = (repeatStart + 0.01).clamp(0.06, 0.95);
                          }
                        }),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageRepeatEnd,
                        value: repeatEnd,
                        min: repeatStart + 0.01,
                        max: 0.95,
                        divisions: ((0.95 - (repeatStart + 0.01)) * 100)
                            .round()
                            .clamp(1, 100),
                        valueText: '${(repeatEnd * 100).round()}%',
                        onChanged: (value) => update(() => repeatEnd = value),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageRepeatYStart,
                        value: repeatYStart,
                        min: 0.05,
                        max: 0.9,
                        divisions: 85,
                        valueText: '${(repeatYStart * 100).round()}%',
                        onChanged: (value) => update(() {
                          repeatYStart = value;
                          if (repeatYEnd <= repeatYStart + 0.01) {
                            repeatYEnd = (repeatYStart + 0.01).clamp(
                              0.06,
                              0.95,
                            );
                          }
                        }),
                      ),
                      SettingsSliderRow(
                        compact: true,
                        label: l10n.settingsAppearanceBubbleImageRepeatYEnd,
                        value: repeatYEnd,
                        min: repeatYStart + 0.01,
                        max: 0.95,
                        divisions: ((0.95 - (repeatYStart + 0.01)) * 100)
                            .round()
                            .clamp(1, 100),
                        valueText: '${(repeatYEnd * 100).round()}%',
                        onChanged: (value) => update(() => repeatYEnd = value),
                      ),
                    ],
                  ),
                  const SizedBox(height: 10),
                  _DialogSectionTitle(l10n.settingsAppearanceBubbleImageScale),
                  SettingsSliderRow(
                    compact: true,
                    label: l10n.settingsAppearanceBubbleImageScale,
                    value: imageScale,
                    min: 0.2,
                    max: 3,
                    divisions: 28,
                    valueText: '${imageScale.toStringAsFixed(1)}x',
                    onChanged: (value) => update(() => imageScale = value),
                  ),
                ],
              ),
            ),
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

const Color _customPresetPrimaryPreviewColor = Color(0xFF2F80ED);
const Color _customPresetSecondaryPreviewColor = Color(0xFFFFB020);

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
    required this.snapshot,
    required this.onChanged,
    required this.onCustomTap,
  });

  final String selectedId;
  final ThemePreferenceSnapshot snapshot;
  final ValueChanged<_ThemeColorPreset> onChanged;
  final VoidCallback onCustomTap;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 10,
      runSpacing: 10,
      children: <Widget>[
        for (final preset in _themeColorPresets)
          _ColorPresetTile(
            selected: selectedId == preset.id,
            label: Text(_themeColorPresetLabel(context, preset.id)),
            primaryColor: _themePresetPrimaryColor(context, preset),
            secondaryColor: _themePresetSecondaryColor(context, preset),
            onTap: () => onChanged(preset),
          ),
        _ColorPresetTile(
          selected: selectedId == 'custom',
          label: Text(_themeColorPresetLabel(context, 'custom')),
          primaryColor: _customPresetPrimaryPreviewColor,
          secondaryColor: _customPresetSecondaryPreviewColor,
          onTap: onCustomTap,
        ),
      ],
    );
  }
}

Color _themePresetPrimaryColor(BuildContext context, _ThemeColorPreset preset) {
  final colorScheme = Theme.of(context).colorScheme;
  return preset.primaryColor == null
      ? colorScheme.primary
      : Color(preset.primaryColor!);
}

Color _themePresetSecondaryColor(
  BuildContext context,
  _ThemeColorPreset preset,
) {
  final colorScheme = Theme.of(context).colorScheme;
  return preset.secondaryColor == null
      ? colorScheme.secondary
      : Color(preset.secondaryColor!);
}

class _ColorPresetTile extends StatelessWidget {
  const _ColorPresetTile({
    required this.selected,
    required this.label,
    required this.primaryColor,
    required this.secondaryColor,
    required this.onTap,
  });

  final bool selected;
  final Widget label;
  final Color primaryColor;
  final Color secondaryColor;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return SizedBox(
      width: 82,
      child: Material(
        color: Colors.transparent,
        child: InkWell(
          borderRadius: BorderRadius.circular(18),
          onTap: onTap,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 180),
            curve: Curves.easeOutCubic,
            padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 8),
            decoration: BoxDecoration(
              color: selected
                  ? colorScheme.primaryContainer.withValues(alpha: 0.28)
                  : Colors.transparent,
              borderRadius: BorderRadius.circular(18),
            ),
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _SplitColorCircle(
                  selected: selected,
                  primaryColor: primaryColor,
                  secondaryColor: secondaryColor,
                ),
                const SizedBox(height: 6),
                DefaultTextStyle.merge(
                  textAlign: TextAlign.center,
                  maxLines: 1,
                  overflow: TextOverflow.ellipsis,
                  style: Theme.of(context).textTheme.labelSmall?.copyWith(
                    color: selected
                        ? colorScheme.onPrimaryContainer
                        : colorScheme.onSurfaceVariant,
                    fontWeight: selected ? FontWeight.w700 : FontWeight.w500,
                  ),
                  child: label,
                ),
              ],
            ),
          ),
        ),
      ),
    );
  }
}

class _SplitColorCircle extends StatelessWidget {
  const _SplitColorCircle({
    required this.selected,
    required this.primaryColor,
    required this.secondaryColor,
  });

  final bool selected;
  final Color primaryColor;
  final Color secondaryColor;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return AnimatedContainer(
      duration: const Duration(milliseconds: 180),
      curve: Curves.easeOutCubic,
      width: 48,
      height: 48,
      padding: const EdgeInsets.all(3),
      decoration: BoxDecoration(
        shape: BoxShape.circle,
        border: Border.all(
          width: selected ? 2 : 1,
          color: selected
              ? colorScheme.primary
              : colorScheme.outlineVariant.withValues(alpha: 0.68),
        ),
        boxShadow: selected
            ? <BoxShadow>[
                BoxShadow(
                  blurRadius: 14,
                  offset: const Offset(0, 4),
                  color: colorScheme.primary.withValues(alpha: 0.16),
                ),
              ]
            : const <BoxShadow>[],
      ),
      child: Stack(
        fit: StackFit.expand,
        children: <Widget>[
          ClipOval(
            child: DecoratedBox(
              decoration: BoxDecoration(
                gradient: LinearGradient(
                  begin: Alignment.topLeft,
                  end: Alignment.bottomRight,
                  colors: <Color>[
                    primaryColor,
                    primaryColor,
                    secondaryColor,
                    secondaryColor,
                  ],
                  stops: const <double>[0, 0.5, 0.5, 1],
                ),
              ),
              child: const SizedBox.expand(),
            ),
          ),
          if (selected)
            Center(
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: colorScheme.surface.withValues(alpha: 0.84),
                  shape: BoxShape.circle,
                ),
                child: Padding(
                  padding: const EdgeInsets.all(3),
                  child: Icon(
                    Icons.check,
                    size: 16,
                    color: colorScheme.primary,
                  ),
                ),
              ),
            ),
        ],
      ),
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
    'custom' => l10n.settingsAppearanceColorCustom,
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
    return SettingsSegmentedSelector<_AvatarShapePreset>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<_AvatarShapePreset>>[
        ButtonSegment<_AvatarShapePreset>(
          value: _AvatarShapePreset.circle,
          icon: const Icon(Icons.circle_outlined, size: 16),
          label: Text(l10n.settingsAppearanceAvatarShapeCircle),
        ),
        ButtonSegment<_AvatarShapePreset>(
          value: _AvatarShapePreset.square,
          icon: const Icon(Icons.crop_square_outlined, size: 16),
          label: Text(l10n.settingsAppearanceAvatarShapeSquare),
        ),
      ],
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
    return SettingsSegmentedSelector<String>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<String>>[
        ButtonSegment<String>(
          value: UserPreferencesManager.CHAT_STYLE_CURSOR,
          icon: const Icon(Icons.terminal_outlined, size: 16),
          label: Text(l10n.settingsAppearanceMessageStyleClean),
        ),
        ButtonSegment<String>(
          value: UserPreferencesManager.CHAT_STYLE_BUBBLE,
          icon: const Icon(Icons.chat_bubble_outline, size: 16),
          label: Text(l10n.settingsAppearanceMessageStyleCard),
        ),
      ],
    );
  }
}

class _InputStyleSelector extends StatelessWidget {
  const _InputStyleSelector({required this.value, required this.onChanged});

  final String value;
  final ValueChanged<String> onChanged;

  /// Builds the segmented control for input style choices.
  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSegmentedSelector<String>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<String>>[
        ButtonSegment<String>(
          value: UserPreferencesManager.INPUT_STYLE_CLASSIC,
          icon: const Icon(Icons.chat_outlined, size: 16),
          label: Text(l10n.settingsAppearanceInputStyleClassic),
        ),
        ButtonSegment<String>(
          value: UserPreferencesManager.INPUT_STYLE_AGENT,
          icon: const Icon(Icons.hub_outlined, size: 16),
          label: Text(l10n.settingsAppearanceInputStyleAgent),
        ),
      ],
    );
  }
}

/// Validates and returns the stored input style value.
String _inputStyleValue(String value) {
  return switch (value) {
    UserPreferencesManager.INPUT_STYLE_CLASSIC =>
      UserPreferencesManager.INPUT_STYLE_CLASSIC,
    UserPreferencesManager.INPUT_STYLE_AGENT =>
      UserPreferencesManager.INPUT_STYLE_AGENT,
    _ => throw FormatException('invalid input style preference: $value'),
  };
}

/// Formats the input style label for the appearance settings panel.
String _inputStyleLabel(AppLocalizations l10n, String value) {
  return switch (_inputStyleValue(value)) {
    UserPreferencesManager.INPUT_STYLE_CLASSIC =>
      l10n.settingsAppearanceInputStyleClassic,
    UserPreferencesManager.INPUT_STYLE_AGENT =>
      l10n.settingsAppearanceInputStyleAgent,
    _ => throw FormatException('invalid input style preference: $value'),
  };
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
    required this.onCustomTap,
  });

  final _MessageColorPreset value;
  final ValueChanged<_MessageColorPreset> onChanged;
  final VoidCallback onCustomTap;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8),
      child: Wrap(
        spacing: 10,
        runSpacing: 10,
        children: <Widget>[
          for (final preset in _messageColorPresetChoices)
            _ColorPresetTile(
              selected: value == preset,
              label: Text(_messageColorPresetName(l10n, preset)),
              primaryColor: _messagePresetPrimaryColor(context, preset),
              secondaryColor: _messagePresetSecondaryColor(context, preset),
              onTap: () => onChanged(preset),
            ),
          _ColorPresetTile(
            selected: value == _MessageColorPreset.custom,
            label: Text(
              _messageColorPresetName(l10n, _MessageColorPreset.custom),
            ),
            primaryColor: _customPresetPrimaryPreviewColor,
            secondaryColor: _customPresetSecondaryPreviewColor,
            onTap: onCustomTap,
          ),
        ],
      ),
    );
  }
}

Color _messagePresetPrimaryColor(
  BuildContext context,
  _MessageColorPreset preset,
) {
  final colorScheme = Theme.of(context).colorScheme;
  return switch (preset) {
    _MessageColorPreset.theme => colorScheme.primaryContainer,
    _MessageColorPreset.custom => _customPresetPrimaryPreviewColor,
    _ => Color(_messageColorPresetValues[preset]!.bubbleUserBubbleColor),
  };
}

Color _messagePresetSecondaryColor(
  BuildContext context,
  _MessageColorPreset preset,
) {
  final colorScheme = Theme.of(context).colorScheme;
  return switch (preset) {
    _MessageColorPreset.theme => colorScheme.surfaceContainerHighest,
    _MessageColorPreset.custom => _customPresetSecondaryPreviewColor,
    _ => Color(_messageColorPresetValues[preset]!.bubbleAiBubbleColor),
  };
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
  var primaryColor = Color(
    snapshot.customPrimaryColor ?? colorScheme.primary.toARGB32(),
  );
  var secondaryColor = Color(
    snapshot.customSecondaryColor ?? colorScheme.secondary.toARGB32(),
  );
  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          return OperitDialogScaffold(
            title: l10n.settingsAppearanceCustomColorsTitle,
            maxWidth: 420,
            showCloseButton: true,
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  unawaited(
                    themeController.saveThemeSettings(
                      useCustomColors: true,
                      customPrimaryColor: primaryColor.toARGB32(),
                      customSecondaryColor: secondaryColor.toARGB32(),
                    ),
                  );
                  Navigator.of(dialogContext).pop();
                },
                child: Text(l10n.save),
              ),
            ],
            child: Column(
              mainAxisSize: MainAxisSize.min,
              children: <Widget>[
                _EditableColorRow(
                  label: l10n.settingsAppearancePrimaryColor,
                  color: primaryColor,
                  onTap: () async {
                    final picked = await _showSingleColorPickerDialog(
                      context,
                      title: l10n.settingsAppearancePrimaryColor,
                      initialColor: primaryColor,
                    );
                    if (picked == null || !dialogContext.mounted) {
                      return;
                    }
                    setDialogState(() {
                      primaryColor = picked;
                    });
                  },
                ),
                const SizedBox(height: 10),
                _EditableColorRow(
                  label: l10n.settingsAppearanceSecondaryColor,
                  color: secondaryColor,
                  onTap: () async {
                    final picked = await _showSingleColorPickerDialog(
                      context,
                      title: l10n.settingsAppearanceSecondaryColor,
                      initialColor: secondaryColor,
                    );
                    if (picked == null || !dialogContext.mounted) {
                      return;
                    }
                    setDialogState(() {
                      secondaryColor = picked;
                    });
                  },
                ),
              ],
            ),
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
  var cursorUserColor = Color(
    snapshot.cursorUserBubbleColor ?? colorScheme.primaryContainer.toARGB32(),
  );
  var userBubbleColor = Color(
    snapshot.bubbleUserBubbleColor ?? colorScheme.primaryContainer.toARGB32(),
  );
  var aiBubbleColor = Color(
    snapshot.bubbleAiBubbleColor ??
        colorScheme.surfaceContainerHighest.toARGB32(),
  );
  var userTextColor = Color(
    snapshot.bubbleUserTextColor ?? colorScheme.onPrimaryContainer.toARGB32(),
  );
  var aiTextColor = Color(
    snapshot.bubbleAiTextColor ?? colorScheme.onSurface.toARGB32(),
  );
  await showDialog<void>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          return OperitDialogScaffold(
            title: l10n.settingsAppearanceCustomMessageColorsTitle,
            maxWidth: 460,
            showCloseButton: true,
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () {
                  unawaited(
                    themeController.saveThemeSettings(
                      cursorUserBubbleColor: cursorUserColor.toARGB32(),
                      bubbleUserBubbleColor: userBubbleColor.toARGB32(),
                      bubbleAiBubbleColor: aiBubbleColor.toARGB32(),
                      bubbleUserTextColor: userTextColor.toARGB32(),
                      bubbleAiTextColor: aiTextColor.toARGB32(),
                    ),
                  );
                  Navigator.of(dialogContext).pop();
                },
                child: Text(l10n.save),
              ),
            ],
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  _EditableColorRow(
                    label: l10n.settingsAppearanceCursorUserBubbleColor,
                    color: cursorUserColor,
                    onTap: () async {
                      final picked = await _showSingleColorPickerDialog(
                        context,
                        title: l10n.settingsAppearanceCursorUserBubbleColor,
                        initialColor: cursorUserColor,
                      );
                      if (picked == null || !dialogContext.mounted) {
                        return;
                      }
                      setDialogState(() {
                        cursorUserColor = picked;
                      });
                    },
                  ),
                  const SizedBox(height: 10),
                  _EditableColorRow(
                    label: l10n.settingsAppearanceUserBubbleColor,
                    color: userBubbleColor,
                    onTap: () async {
                      final picked = await _showSingleColorPickerDialog(
                        context,
                        title: l10n.settingsAppearanceUserBubbleColor,
                        initialColor: userBubbleColor,
                      );
                      if (picked == null || !dialogContext.mounted) {
                        return;
                      }
                      setDialogState(() {
                        userBubbleColor = picked;
                      });
                    },
                  ),
                  const SizedBox(height: 10),
                  _EditableColorRow(
                    label: l10n.settingsAppearanceAiBubbleColor,
                    color: aiBubbleColor,
                    onTap: () async {
                      final picked = await _showSingleColorPickerDialog(
                        context,
                        title: l10n.settingsAppearanceAiBubbleColor,
                        initialColor: aiBubbleColor,
                      );
                      if (picked == null || !dialogContext.mounted) {
                        return;
                      }
                      setDialogState(() {
                        aiBubbleColor = picked;
                      });
                    },
                  ),
                  const SizedBox(height: 10),
                  _EditableColorRow(
                    label: l10n.settingsAppearanceUserTextColor,
                    color: userTextColor,
                    onTap: () async {
                      final picked = await _showSingleColorPickerDialog(
                        context,
                        title: l10n.settingsAppearanceUserTextColor,
                        initialColor: userTextColor,
                      );
                      if (picked == null || !dialogContext.mounted) {
                        return;
                      }
                      setDialogState(() {
                        userTextColor = picked;
                      });
                    },
                  ),
                  const SizedBox(height: 10),
                  _EditableColorRow(
                    label: l10n.settingsAppearanceAiTextColor,
                    color: aiTextColor,
                    onTap: () async {
                      final picked = await _showSingleColorPickerDialog(
                        context,
                        title: l10n.settingsAppearanceAiTextColor,
                        initialColor: aiTextColor,
                      );
                      if (picked == null || !dialogContext.mounted) {
                        return;
                      }
                      setDialogState(() {
                        aiTextColor = picked;
                      });
                    },
                  ),
                ],
              ),
            ),
          );
        },
      );
    },
  );
}

Future<Color?> _showSingleColorPickerDialog(
  BuildContext context, {
  required String title,
  required Color initialColor,
}) {
  var hsvColor = HSVColor.fromColor(initialColor);
  return showDialog<Color>(
    context: context,
    builder: (dialogContext) {
      return StatefulBuilder(
        builder: (context, setDialogState) {
          final color = hsvColor.toColor();
          final l10n = AppLocalizations.of(context)!;
          return OperitDialogScaffold(
            title: title,
            maxWidth: 420,
            showCloseButton: true,
            actions: <Widget>[
              TextButton(
                onPressed: () => Navigator.of(dialogContext).pop(),
                child: Text(l10n.cancel),
              ),
              FilledButton(
                onPressed: () => Navigator.of(dialogContext).pop(color),
                child: Text(l10n.save),
              ),
            ],
            child: SingleChildScrollView(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  _ColorPickerPreview(color: color),
                  const SizedBox(height: 14),
                  _ColorPickerSlider(
                    label: '色相',
                    value: hsvColor.hue,
                    min: 0,
                    max: 360,
                    divisions: 360,
                    activeColor: color,
                    valueLabel: '${hsvColor.hue.round()}',
                    onChanged: (value) {
                      setDialogState(() {
                        hsvColor = HSVColor.fromAHSV(
                          hsvColor.alpha,
                          value,
                          hsvColor.saturation,
                          hsvColor.value,
                        );
                      });
                    },
                  ),
                  _ColorPickerSlider(
                    label: '饱和度',
                    value: hsvColor.saturation,
                    min: 0,
                    max: 1,
                    divisions: 100,
                    activeColor: color,
                    valueLabel: '${(hsvColor.saturation * 100).round()}%',
                    onChanged: (value) {
                      setDialogState(() {
                        hsvColor = HSVColor.fromAHSV(
                          hsvColor.alpha,
                          hsvColor.hue,
                          value,
                          hsvColor.value,
                        );
                      });
                    },
                  ),
                  _ColorPickerSlider(
                    label: '明度',
                    value: hsvColor.value,
                    min: 0,
                    max: 1,
                    divisions: 100,
                    activeColor: color,
                    valueLabel: '${(hsvColor.value * 100).round()}%',
                    onChanged: (value) {
                      setDialogState(() {
                        hsvColor = HSVColor.fromAHSV(
                          hsvColor.alpha,
                          hsvColor.hue,
                          hsvColor.saturation,
                          value,
                        );
                      });
                    },
                  ),
                  const SizedBox(height: 12),
                  Text('预设', style: Theme.of(context).textTheme.labelLarge),
                  const SizedBox(height: 8),
                  Wrap(
                    spacing: 8,
                    runSpacing: 8,
                    children: <Widget>[
                      for (final preset in _pickerPresetColors)
                        _ColorSwatchButton(
                          color: preset,
                          selected: preset.toARGB32() == color.toARGB32(),
                          onTap: () {
                            setDialogState(() {
                              hsvColor = HSVColor.fromColor(preset);
                            });
                          },
                        ),
                    ],
                  ),
                ],
              ),
            ),
          );
        },
      );
    },
  );
}

const List<Color> _pickerPresetColors = <Color>[
  Color(0xFFE53935),
  Color(0xFFD81B60),
  Color(0xFF8E24AA),
  Color(0xFF5E35B1),
  Color(0xFF3949AB),
  Color(0xFF1E88E5),
  Color(0xFF039BE5),
  Color(0xFF00ACC1),
  Color(0xFF00897B),
  Color(0xFF43A047),
  Color(0xFF7CB342),
  Color(0xFFC0CA33),
  Color(0xFFFDD835),
  Color(0xFFFFB300),
  Color(0xFFFB8C00),
  Color(0xFFF4511E),
  Color(0xFF6D4C41),
  Color(0xFF546E7A),
  Color(0xFF111827),
  Color(0xFFFFFFFF),
];

class _ColorPickerPreview extends StatelessWidget {
  const _ColorPickerPreview({required this.color});

  final Color color;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: color,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.72),
        ),
      ),
      child: SizedBox(
        height: 72,
        child: Center(
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: colorScheme.surface.withValues(alpha: 0.78),
              borderRadius: BorderRadius.circular(999),
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
              child: Text(
                _hexColorText(color),
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: colorScheme.onSurface,
                  fontWeight: FontWeight.w700,
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _ColorPickerSlider extends StatelessWidget {
  const _ColorPickerSlider({
    required this.label,
    required this.value,
    required this.min,
    required this.max,
    required this.divisions,
    required this.activeColor,
    required this.valueLabel,
    required this.onChanged,
  });

  final String label;
  final double value;
  final double min;
  final double max;
  final int divisions;
  final Color activeColor;
  final String valueLabel;
  final ValueChanged<double> onChanged;

  @override
  Widget build(BuildContext context) {
    return SettingsSliderRow(
      compact: true,
      label: label,
      value: value,
      min: min,
      max: max,
      divisions: divisions,
      activeColor: activeColor,
      valueText: valueLabel,
      onChanged: onChanged,
    );
  }
}

class _ColorSwatchButton extends StatelessWidget {
  const _ColorSwatchButton({
    required this.color,
    required this.selected,
    required this.onTap,
  });

  final Color color;
  final bool selected;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return InkWell(
      borderRadius: BorderRadius.circular(999),
      onTap: onTap,
      child: AnimatedContainer(
        duration: const Duration(milliseconds: 160),
        curve: Curves.easeOutCubic,
        width: 34,
        height: 34,
        padding: const EdgeInsets.all(3),
        decoration: BoxDecoration(
          shape: BoxShape.circle,
          border: Border.all(
            width: selected ? 2 : 1,
            color: selected
                ? colorScheme.primary
                : colorScheme.outlineVariant.withValues(alpha: 0.72),
          ),
        ),
        child: DecoratedBox(
          decoration: BoxDecoration(color: color, shape: BoxShape.circle),
          child: selected
              ? Icon(Icons.check, size: 16, color: colorScheme.onPrimary)
              : null,
        ),
      ),
    );
  }
}

class _EditableColorRow extends StatelessWidget {
  const _EditableColorRow({
    required this.label,
    required this.color,
    required this.onTap,
  });

  final String label;
  final Color color;
  final VoidCallback onTap;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(14),
        onTap: onTap,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
          child: Row(
            children: <Widget>[
              DecoratedBox(
                decoration: BoxDecoration(
                  color: color,
                  shape: BoxShape.circle,
                  border: Border.all(
                    color: colorScheme.outlineVariant.withValues(alpha: 0.72),
                  ),
                ),
                child: const SizedBox.square(dimension: 32),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Text(
                  label,
                  style: Theme.of(
                    context,
                  ).textTheme.bodyMedium?.copyWith(fontWeight: FontWeight.w600),
                ),
              ),
              Text(
                _hexColorText(color),
                style: Theme.of(context).textTheme.labelMedium?.copyWith(
                  color: colorScheme.onSurfaceVariant,
                ),
              ),
              const SizedBox(width: 8),
              const Icon(Icons.palette_outlined, size: 18),
            ],
          ),
        ),
      ),
    );
  }
}

String _hexColorText(Color color) {
  final value = color.toARGB32() & 0xFFFFFF;
  return '#${value.toRadixString(16).padLeft(6, '0').toUpperCase()}';
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
    return SettingsSegmentedSelector<_MessageSurface>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<_MessageSurface>>[
        ButtonSegment<_MessageSurface>(
          value: _MessageSurface.normal,
          icon: const Icon(Icons.layers_outlined, size: 16),
          label: Text(l10n.settingsAppearanceMessageSurfaceNormal),
        ),
        ButtonSegment<_MessageSurface>(
          value: _MessageSurface.transparent,
          icon: const Icon(Icons.blur_on_outlined, size: 16),
          label: Text(l10n.settingsAppearanceMessageSurfaceTransparent),
        ),
      ],
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
    return SettingsSegmentedSelector<String>(
      value: _bubbleImageRenderModeValue(value),
      onChanged: onChanged,
      segments: <ButtonSegment<String>>[
        ButtonSegment<String>(
          value:
              UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE,
          icon: const Icon(Icons.grid_view_outlined, size: 16),
          label: Text(l10n.settingsAppearanceBubbleImageTiledNineSlice),
        ),
        ButtonSegment<String>(
          value: UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH,
          icon: const Icon(Icons.crop_free_outlined, size: 16),
          label: Text(l10n.settingsAppearanceBubbleImageNinePatch),
        ),
      ],
    );
  }
}

String _bubbleImageRenderModeValue(String value) {
  return value == UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
      ? UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_NINE_PATCH
      : UserPreferencesManager.BUBBLE_IMAGE_RENDER_MODE_TILED_NINE_SLICE;
}

enum _FontFamilyPreset { defaultFont, serif, monospace }

class _FontFamilySelector extends StatelessWidget {
  const _FontFamilySelector({required this.value, required this.onChanged});

  final _FontFamilyPreset value;
  final ValueChanged<_FontFamilyPreset> onChanged;

  @override
  Widget build(BuildContext context) {
    final l10n = AppLocalizations.of(context)!;
    return SettingsSegmentedSelector<_FontFamilyPreset>(
      value: value,
      onChanged: onChanged,
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
    return SettingsSegmentedSelector<_MessageDensity>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<_MessageDensity>>[
        ButtonSegment<_MessageDensity>(
          value: _MessageDensity.comfortable,
          icon: const Icon(Icons.view_agenda_outlined, size: 16),
          label: Text(l10n.settingsAppearanceMessageDensityComfortable),
        ),
        ButtonSegment<_MessageDensity>(
          value: _MessageDensity.compact,
          icon: const Icon(Icons.density_small_outlined, size: 16),
          label: Text(l10n.settingsAppearanceMessageDensityCompact),
        ),
      ],
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
    return SettingsSegmentedSelector<ThemeMode>(
      value: value,
      onChanged: onChanged,
      segments: <ButtonSegment<ThemeMode>>[
        ButtonSegment<ThemeMode>(
          value: ThemeMode.system,
          icon: const Icon(Icons.brightness_auto_outlined, size: 16),
          label: Text(l10n.settingsAppearanceThemeSystem),
        ),
        ButtonSegment<ThemeMode>(
          value: ThemeMode.light,
          icon: const Icon(Icons.light_mode_outlined, size: 16),
          label: Text(l10n.settingsAppearanceThemeLight),
        ),
        ButtonSegment<ThemeMode>(
          value: ThemeMode.dark,
          icon: const Icon(Icons.dark_mode_outlined, size: 16),
          label: Text(l10n.settingsAppearanceThemeDark),
        ),
      ],
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

class _SectionCard extends StatelessWidget {
  const _SectionCard({
    required this.title,
    this.icon,
    this.action,
    required this.children,
  });

  final String title;
  final IconData? icon;
  final Widget? action;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final radius = BorderRadius.circular(12);
    return Padding(
      padding: const EdgeInsets.only(bottom: 10),
      child: OperitGlassSurface(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.36),
        borderRadius: radius,
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.18),
        ),
        material: true,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(14, 12, 14, 12),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: <Widget>[
              Row(
                children: <Widget>[
                  if (icon != null) ...<Widget>[
                    Icon(icon, size: 18, color: colorScheme.primary),
                    const SizedBox(width: 8),
                  ],
                  Expanded(
                    child: Text(
                      title,
                      style: SettingsControlStyles.sectionTitleTextStyle(
                        context,
                      ),
                    ),
                  ),
                  if (action != null) ...<Widget>[
                    action!,
                    const SizedBox(width: 2),
                  ],
                ],
              ),
              const SizedBox(height: 8),
              ...children,
            ],
          ),
        ),
      ),
    );
  }
}

class _AppearanceFieldBlock extends StatelessWidget {
  const _AppearanceFieldBlock({
    required this.label,
    this.badge,
    required this.child,
  });

  final String label;
  final String? badge;
  final Widget child;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Row(
          children: <Widget>[
            Expanded(
              child: Text(
                label,
                style: textTheme.bodyMedium?.copyWith(
                  fontWeight: FontWeight.w600,
                  color: colorScheme.onSurface,
                ),
              ),
            ),
            if (badge != null && badge!.isNotEmpty)
              SettingsInfoBadge(label: badge!),
          ],
        ),
        const SizedBox(height: 6),
        child,
      ],
    );
  }
}

class _CompactAssetTile extends StatelessWidget {
  const _CompactAssetTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.actions,
    this.bottom,
  });

  final IconData icon;
  final String title;
  final String subtitle;
  final List<Widget> actions;
  final Widget? bottom;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final textTheme = Theme.of(context).textTheme;

    return Container(
      decoration: BoxDecoration(
        color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.25),
        borderRadius: BorderRadius.circular(10),
        border: Border.all(
          color: colorScheme.outlineVariant.withValues(alpha: 0.24),
          width: 0.8,
        ),
      ),
      padding: const EdgeInsets.fromLTRB(10, 8, 10, 8),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: <Widget>[
          Row(
            children: <Widget>[
              Icon(icon, size: 20, color: colorScheme.primary),
              const SizedBox(width: 8),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    Text(
                      title,
                      style: textTheme.bodyMedium?.copyWith(
                        fontWeight: FontWeight.w600,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    const SizedBox(height: 1),
                    Text(
                      subtitle,
                      style: textTheme.bodySmall?.copyWith(
                        color: colorScheme.onSurfaceVariant,
                        fontSize: 11,
                      ),
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                  ],
                ),
              ),
              const SizedBox(width: 8),
              Wrap(
                spacing: 6,
                runSpacing: 4,
                crossAxisAlignment: WrapCrossAlignment.center,
                children: actions,
              ),
            ],
          ),
          if (bottom != null) ...<Widget>[const SizedBox(height: 6), bottom!],
        ],
      ),
    );
  }
}

class _TwoColumnSliderGrid extends StatelessWidget {
  const _TwoColumnSliderGrid({required this.children});

  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        if (constraints.maxWidth >= 380) {
          final rows = <Widget>[];
          for (var i = 0; i < children.length; i += 2) {
            rows.add(
              Row(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: <Widget>[
                  Expanded(child: children[i]),
                  const SizedBox(width: 12),
                  Expanded(
                    child: i + 1 < children.length
                        ? children[i + 1]
                        : const SizedBox.shrink(),
                  ),
                ],
              ),
            );
          }
          return Column(mainAxisSize: MainAxisSize.min, children: rows);
        }
        return Column(mainAxisSize: MainAxisSize.min, children: children);
      },
    );
  }
}

class _BodyText extends StatelessWidget {
  const _BodyText(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: Text(
        text,
        style: Theme.of(context).textTheme.bodySmall?.copyWith(
          color: Theme.of(context).colorScheme.onSurfaceVariant,
        ),
      ),
    );
  }
}

class _DialogSectionTitle extends StatelessWidget {
  const _DialogSectionTitle(this.text);

  final String text;

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.only(top: 8, bottom: 4),
      child: Text(
        text,
        style: Theme.of(context).textTheme.titleSmall?.copyWith(
          fontWeight: FontWeight.w700,
          color: Theme.of(context).colorScheme.primary,
        ),
      ),
    );
  }
}

String _fontFamilyPresetLabel(AppLocalizations l10n, _FontFamilyPreset preset) {
  return switch (preset) {
    _FontFamilyPreset.defaultFont => l10n.settingsAppearanceFontDefault,
    _FontFamilyPreset.serif => l10n.settingsAppearanceFontSerif,
    _FontFamilyPreset.monospace => l10n.settingsAppearanceFontMonospace,
  };
}
