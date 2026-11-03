// ignore_for_file: file_names

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

/// Whether the input method still owns an uncommitted part of the draft.
bool hasChatInputComposition(TextEditingController controller) {
  final composing = controller.value.composing;
  return composing.isValid && !composing.isCollapsed;
}

/// Lets IME confirmation reach the platform instead of sending or adding a line.
class ChatInputEnterActivator extends SingleActivator {
  const ChatInputEnterActivator(
    super.trigger, {
    required this.controller,
    super.control,
  }) : super(includeRepeats: false);

  final TextEditingController controller;

  @override
  bool accepts(KeyEvent event, HardwareKeyboard state) {
    return !hasChatInputComposition(controller) && super.accepts(event, state);
  }
}
