// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

class MessagePressShieldController {
  final Set<int> _shieldedPointers = <int>{};

  void shieldPointer(int pointer) {
    _shieldedPointers.add(pointer);
  }

  void unshieldPointer(int pointer) {
    _shieldedPointers.remove(pointer);
  }

  bool isPointerShielded(int pointer) {
    return _shieldedPointers.contains(pointer);
  }
}

class MessagePressShield extends InheritedWidget {
  const MessagePressShield({
    super.key,
    required this.controller,
    required super.child,
  });

  final MessagePressShieldController controller;

  static MessagePressShieldController? maybeOf(BuildContext context) {
    final scope = context
        .dependOnInheritedWidgetOfExactType<MessagePressShield>();
    return scope?.controller;
  }

  @override
  bool updateShouldNotify(MessagePressShield oldWidget) {
    return controller != oldWidget.controller;
  }
}

class MessagePressShieldRegion extends StatelessWidget {
  const MessagePressShieldRegion({
    super.key,
    required this.child,
    this.behavior = HitTestBehavior.translucent,
  });

  final Widget child;
  final HitTestBehavior behavior;

  @override
  Widget build(BuildContext context) {
    final controller = MessagePressShield.maybeOf(context);
    if (controller == null) {
      return child;
    }
    return Listener(
      behavior: behavior,
      onPointerDown: (event) => controller.shieldPointer(event.pointer),
      onPointerUp: (event) => controller.unshieldPointer(event.pointer),
      onPointerCancel: (event) => controller.unshieldPointer(event.pointer),
      child: child,
    );
  }
}
