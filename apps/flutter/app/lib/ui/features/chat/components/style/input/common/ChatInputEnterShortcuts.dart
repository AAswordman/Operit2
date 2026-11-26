// ignore_for_file: file_names

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../../../../../../../data/preferences/UserPreferencesManager.dart';
import '../../../../../../main/layout/NavigationLayoutMetrics.dart';

/// Shares the configurable Enter behavior across composers.
class ChatInputEnterShortcuts extends StatelessWidget {
  /// Creates Enter bindings that preserve input method candidate confirmation.
  const ChatInputEnterShortcuts({
    super.key,
    required this.controller,
    required this.inputExpanded,
    required this.canSendMessage,
    required this.onSendMessage,
    this.onMentionEnter,
    this.onMentionKeyEvent,
    required this.child,
  });

  final TextEditingController controller;
  final bool inputExpanded;
  final bool canSendMessage;
  final VoidCallback onSendMessage;
  final bool Function()? onMentionEnter;
  final KeyEventResult Function(KeyEvent event)? onMentionKeyEvent;
  final Widget child;

  /// Applies the saved switch while keeping expanded editors in multiline mode.
  @override
  Widget build(BuildContext context) {
    return ValueListenableBuilder<bool?>(
      valueListenable: UserPreferencesManager.enterSendsMessage,
      builder: (context, enabled, child) => _buildShortcuts(
        !inputExpanded && (enabled ?? useTabletLayoutForContext(context)),
        child!,
      ),
      child: child,
    );
  }

  /// Binds both Enter keys without replacing the active text editor.
  Widget _buildShortcuts(bool enterSends, Widget child) {
    final innerChild = onMentionKeyEvent == null
        ? child
        : Focus(
            canRequestFocus: false,
            skipTraversal: true,
            onKeyEvent: (_, event) {
              if (_hasChatInputComposition(controller)) {
                return KeyEventResult.ignored;
              }
              return onMentionKeyEvent!(event);
            },
            child: child,
          );
    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        for (final key in <LogicalKeyboardKey>[
          LogicalKeyboardKey.enter,
          LogicalKeyboardKey.numpadEnter,
        ]) ...{
          _ChatInputEnterActivator(key, controller: controller): () {
            if (onMentionEnter?.call() == true) {
              return;
            }
            if (enterSends) {
              _sendIfReady();
            } else {
              _insertNewline();
            }
          },
          _ChatInputEnterActivator(key, controller: controller, control: true):
              _insertNewline,
        },
      },
      child: innerChild,
    );
  }

  /// Sends the draft when there is content.
  void _sendIfReady() {
    if (canSendMessage) {
      onSendMessage();
    }
  }

  /// Replaces the current selection with a newline.
  void _insertNewline() {
    final value = controller.value;
    final text = value.text;
    final selection = value.selection;
    final range = selection.isValid
        ? selection
        : TextSelection.collapsed(offset: text.length);
    controller.value = TextEditingValue(
      text: text.replaceRange(range.start, range.end, '\n'),
      selection: TextSelection.collapsed(offset: range.start + 1),
      composing: TextRange.empty,
    );
  }
}

/// Whether the input method still owns an uncommitted part of the draft.
bool _hasChatInputComposition(TextEditingController controller) {
  final composing = controller.value.composing;
  return composing.isValid && !composing.isCollapsed;
}

/// Lets IME confirmation reach the platform instead of sending or adding a line.
class _ChatInputEnterActivator extends SingleActivator {
  /// Creates a non-repeating shortcut that yields to active composition.
  const _ChatInputEnterActivator(
    super.trigger, {
    required this.controller,
    super.control,
  }) : super(includeRepeats: false);

  final TextEditingController controller;

  /// Leaves composing text and repeated key events to the text input system.
  @override
  bool accepts(KeyEvent event, HardwareKeyboard state) {
    return !_hasChatInputComposition(controller) && super.accepts(event, state);
  }
}
