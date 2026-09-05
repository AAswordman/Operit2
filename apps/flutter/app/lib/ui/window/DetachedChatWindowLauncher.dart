// ignore_for_file: file_names

import 'package:desktop_multi_window/desktop_multi_window.dart';

import '../../data/preferences/UserPreferencesManager.dart';
import 'OperitWindowArguments.dart';
import 'OperitWindowPlatform.dart';

class DetachedChatWindowLauncher {
  const DetachedChatWindowLauncher._();

  /// Opens a chat in a new desktop window with its visible theme state.
  static Future<void> openChat({
    required String chatId,
    required String title,
    required ThemePreferenceSnapshot themePreferenceSnapshot,
  }) async {
    if (!operitSupportsDesktopMultiWindow) {
      throw UnsupportedError(
        'Detached chat windows require desktop multi-window support',
      );
    }
    final slotId = 'chat_${DateTime.now().microsecondsSinceEpoch}';
    final controller = await WindowController.create(
      WindowConfiguration(
        hiddenAtLaunch: true,
        arguments: DetachedChatWindowArguments(
          slotId: slotId,
          chatId: chatId,
          title: title,
          themePreferenceSnapshot: themePreferenceSnapshot,
        ).encode(),
      ),
    );
    await controller.show();
  }
}
