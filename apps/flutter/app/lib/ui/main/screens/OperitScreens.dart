// ignore_for_file: file_names

import 'package:flutter/widgets.dart';

import '../../features/chat/screens/AIChatScreen.dart';

abstract class OperitScreen {
  const OperitScreen({
    required this.routeTypeName,
    this.title,
    this.participatesInCrossfadeTransition = true,
    this.keepAlive = false,
  });

  final String routeTypeName;
  final String? title;
  final bool participatesInCrossfadeTransition;
  final bool keepAlive;

  String? stableScreenKey() {
    return null;
  }

  Widget build(BuildContext context);
}

class AiChatScreenRoute extends OperitScreen {
  const AiChatScreenRoute() : super(routeTypeName: 'AiChat', title: 'AI Chat');

  @override
  Widget build(BuildContext context) {
    return const AIChatScreen();
  }
}
