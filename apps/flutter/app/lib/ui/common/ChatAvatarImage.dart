// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../theme/OperitThemeAssets.dart';

class ChatAvatarImage extends StatelessWidget {
  /// Creates a chat avatar from an identity asset or the default app avatar.
  const ChatAvatarImage({
    super.key,
    required this.avatarUri,
    required this.fit,
  });

  final String? avatarUri;
  final BoxFit fit;

  /// Displays the default avatar for an unset identity or loads its runtime asset.
  @override
  Widget build(BuildContext context) {
    final path = avatarUri?.trim();
    if (path == null || path.isEmpty) {
      return SizedBox.expand(
        child: Image.asset('assets/images/operit_avatar.png', fit: fit),
      );
    }
    return ThemeAssetImage(storagePath: path, fit: fit);
  }
}
