// ignore_for_file: file_names

import 'package:desktop_multi_window/desktop_multi_window.dart';

import 'OperitWindowArguments.dart';

class DetachedChatWindowLauncher {
  const DetachedChatWindowLauncher._();

  static Future<void> openChat({
    required String chatId,
    required String title,
  }) async {
    final slotId = 'chat_${DateTime.now().microsecondsSinceEpoch}';
    final controller = await WindowController.create(
      WindowConfiguration(
        hiddenAtLaunch: true,
        arguments: DetachedChatWindowArguments(
          slotId: slotId,
          chatId: chatId,
          title: title,
        ).encode(),
      ),
    );
    await controller.show();
  }
}
