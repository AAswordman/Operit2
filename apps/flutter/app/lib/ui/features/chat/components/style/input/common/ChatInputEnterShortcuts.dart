// ignore_for_file: file_names

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../../../../../../main/layout/NavigationLayoutMetrics.dart';

/// Shares Enter behavior across composers using the application display layout.
class ChatInputEnterShortcuts extends StatelessWidget {
  /// Creates Enter bindings that preserve input method candidate confirmation.
  const ChatInputEnterShortcuts({
    super.key,
    required this.controller,
    required this.inputExpanded,
    required this.canSendMessage,
    required this.onSendMessage,
    required this.child,
  });

  final TextEditingController controller;
  final bool inputExpanded;
  final bool canSendMessage;
  final VoidCallback onSendMessage;
  final Widget child;

  /// Sends only from a collapsed desktop layout; other Enter presses add lines.
  @override
  Widget build(BuildContext context) {
    final enterSends = !inputExpanded && useTabletLayoutForContext(context);
    return CallbackShortcuts(
      bindings: <ShortcutActivator, VoidCallback>{
        for (final key in <LogicalKeyboardKey>[
          LogicalKeyboardKey.enter,
          LogicalKeyboardKey.numpadEnter,
        ]) ...{
          _ChatInputEnterActivator(key, controller: controller): enterSends
              ? _sendIfReady
              : _insertNewline,
          _ChatInputEnterActivator(key, controller: controller, control: true):
              _insertNewline,
        },
      },
      child: child,
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
