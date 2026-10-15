// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../theme/OperitThemeAssets.dart';
import '../features/settings/model/ProviderLogo.dart';

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
    final uri = Uri.parse(path);
    if (uri.scheme == 'provider-logo') {
      if (uri.hasAuthority ||
          uri.hasQuery ||
          uri.hasFragment ||
          uri.path.isEmpty) {
        throw const FormatException('Invalid provider logo avatar URI.');
      }
      return LayoutBuilder(
        builder: (context, constraints) => ProviderIdentityLogo(
          providerTypeId: Uri.decodeComponent(uri.path),
          size: constraints.biggest.shortestSide,
        ),
      );
    }
    return ThemeAssetImage(storagePath: path, fit: fit);
  }
}
