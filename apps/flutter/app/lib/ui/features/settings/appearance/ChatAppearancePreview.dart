// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;
import '../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../l10n/generated/app_localizations.dart';
import '../../../theme/OperitTheme.dart';
import '../../chat/components/style/ThemedChatMessage.dart';
import '../../chat/components/style/input/agent/AgentChatInputSection.dart';
import '../../chat/components/style/input/classic/ClassicChatInputSection.dart';
import '../../chat/viewmodel/ChatViewModel.dart';

/// Previews sample content using the production chat widgets, not imitations.
class ChatAppearancePreview extends StatefulWidget {
  const ChatAppearancePreview({
    super.key,
    required this.showInputPreview,
    this.viewModel,
  });

  final bool showInputPreview;
  final ChatViewModel? viewModel;

  @override
  State<ChatAppearancePreview> createState() => _ChatAppearancePreviewState();
}

class _ChatAppearancePreviewState extends State<ChatAppearancePreview> {
  final _inputController = TextEditingController();
  final _inputFocusNode = FocusNode(canRequestFocus: false);
  final _defaultViewModel = ChatViewModel();
  final _sampleDay = DateTime.now();

  /// Releases preview-only text input resources.
  @override
  void dispose() {
    _inputController.dispose();
    _inputFocusNode.dispose();
    super.dispose();
  }

  /// Renders ordinary appearance samples without loading any current actor or binding.
  @override
  Widget build(BuildContext context) {
    final controller = OperitTheme.of(context);
    final snapshot = controller.themePreferenceSnapshot;
    const roleName = 'Operit';
    final l10n = AppLocalizations.of(context)!;
    return ClipRRect(
      borderRadius: BorderRadius.circular(14),
      child: OperitThemeBackground(
        themePreferenceSnapshot: snapshot,
        fit: StackFit.loose,
        muteVideo: true,
        // Preview gestures must not focus the editor, send messages, open
        // menus, or activate links. The widgets still render normally.
        child: IgnorePointer(
          child: ExcludeFocus(
            child: Column(
              mainAxisSize: MainAxisSize.min,
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: <Widget>[
                      buildThemedChatMessage(
                        snapshot: snapshot,
                        colorScheme: Theme.of(context).colorScheme,
                        message: _sampleMessage(
                          isUser: true,
                          text: l10n.settingsAppearanceLivePreviewUserSample,
                          roleName: '',
                        ),
                        splitMarkdownContent:
                            (widget.viewModel ?? _defaultViewModel)
                                .splitMarkdownContent,
                        identityAvatarUri: null,
                        enableDialogs: false,
                      ),
                      const SizedBox(height: 8),
                      buildThemedChatMessage(
                        snapshot: snapshot,
                        colorScheme: Theme.of(context).colorScheme,
                        message: _sampleMessage(
                          isUser: false,
                          text: l10n.settingsAppearanceLivePreviewAiSample,
                          roleName: roleName,
                        ),
                        splitMarkdownContent:
                            (widget.viewModel ?? _defaultViewModel)
                                .splitMarkdownContent,
                        identityAvatarUri: null,
                        enableDialogs: false,
                      ),
                    ],
                  ),
                ),
                if (widget.showInputPreview) _buildInput(snapshot),
              ],
            ),
          ),
        ),
      ),
    );
  }

  /// Supplies completed sample messages so metadata uses the real formatters.
  ChatUiMessage _sampleMessage({
    required bool isUser,
    required String text,
    required String roleName,
  }) {
    final timestamp = DateTime(
      _sampleDay.year,
      _sampleDay.month,
      _sampleDay.day,
      14,
      isUser ? 19 : 20,
    ).millisecondsSinceEpoch;
    return ChatUiMessage(
      sender: isUser ? 'user' : 'ai',
      parts: <core_proxy.MessagePart>[
        core_proxy.MessagePart(
          partId: isUser ? 'preview-user' : 'preview-ai',
          sequence: 0,
          kind: core_proxy.MessagePartKind.markdown,
          content: text,
          toolCallId: null,
          toolName: null,
          attributes: const <String, String>{},
        ),
      ],
      timestamp: timestamp,
      roleName: roleName,
      selectedVariantIndex: 0,
      variantCount: 1,
      provider: isUser ? '' : 'OpenAI',
      modelName: isUser ? '' : 'GPT-5',
      inputTokens: isUser ? 0 : 2500,
      outputTokens: isUser ? 0 : 110,
      cachedInputTokens: 0,
      sentAt: timestamp,
      outputDurationMs: isUser ? 0 : 1200,
      waitDurationMs: 0,
      completedAt: isUser ? 0 : timestamp + 1200,
      displayMode: core_proxy.ChatMessageDisplayMode.normal,
      isFavorite: false,
      contentStream: null,
    );
  }

  /// Builds an inert production composer using the explicit preview appearance.
  Widget _buildInput(ThemePreferenceSnapshot snapshot) {
    final viewModel = widget.viewModel ?? _defaultViewModel;
    final inputState = core_proxy.InputProcessingState.idle();
    switch (snapshot.inputStyle) {
      case UserPreferencesManager.INPUT_STYLE_AGENT:
        return AgentChatInputSection(
          controller: _inputController,
          focusNode: _inputFocusNode,
          isLoading: false,
          inputState: inputState,
          viewModel: viewModel,
          currentChatId: null,
          onSendMessage: _noOp,
          onQueueMessage: _noOp,
          onCancelMessage: _noOp,
          isSpeechRecording: false,
          isSpeechTranscribing: false,
          onSpeechInput: _noOp,
        );
      case UserPreferencesManager.INPUT_STYLE_CLASSIC:
        return ClassicChatInputSection(
          controller: _inputController,
          focusNode: _inputFocusNode,
          isLoading: false,
          inputState: inputState,
          viewModel: viewModel,
          currentChatId: null,
          onSendMessage: _noOp,
          onQueueMessage: _noOp,
          onCancelMessage: _noOp,
          isSpeechRecording: false,
          isSpeechTranscribing: false,
          onSpeechInput: _noOp,
        );
      default:
        throw FormatException(
          'Unknown chat input style: ${snapshot.inputStyle}',
        );
    }
  }
}

/// Keeps appearance previews inert without performing chat actions.
void _noOp() {}
