// ignore_for_file: file_names

import 'package:flutter/material.dart';

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../features/packages/screens/ComposeDslImageSource.dart';
import '../features/settings/model/ProviderLogo.dart';

class ChatAvatarImage extends StatefulWidget {
  /// Creates a chat avatar backed by the same file service as plugin images.
  const ChatAvatarImage({
    super.key,
    required this.avatarUri,
    required this.fit,
    this.clients = const GeneratedCoreProxyClients(ProxyCoreRuntimeBridge()),
  });

  final String? avatarUri;
  final BoxFit fit;
  final GeneratedCoreProxyClients clients;

  /// Retains the resolved image while the surrounding message is rebuilt.
  @override
  State<ChatAvatarImage> createState() => _ChatAvatarImageState();
}

class _ChatAvatarImageState extends State<ChatAvatarImage> {
  Future<ImageProvider>? _provider;

  /// Resolves the initial identity image through the shared image loader.
  @override
  void initState() {
    super.initState();
    _resolveImage();
  }

  /// Resolves a new image when its identity or runtime bridge changes.
  @override
  void didUpdateWidget(covariant ChatAvatarImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.avatarUri != widget.avatarUri ||
        oldWidget.clients.bridge != widget.clients.bridge) {
      _resolveImage();
    }
  }

  /// Uses the Core file service for VFS paths and shares Flutter image caching.
  void _resolveImage() {
    final path = widget.avatarUri?.trim();
    if (path == null ||
        path.isEmpty ||
        Uri.parse(path).scheme == 'provider-logo') {
      _provider = null;
      return;
    }
    final clients = widget.clients;
    _provider = resolveComposeDslImage(
      path,
      readVfsBytes: (path) => readComposeDslVfsBytes(clients, path),
      vfsOwner: clients.bridge,
    );
  }

  /// Displays the identity image, provider logo, or configured default avatar.
  @override
  Widget build(BuildContext context) {
    final path = widget.avatarUri?.trim();
    if (path == null || path.isEmpty) {
      return SizedBox.expand(
        child: Image.asset('assets/images/operit_avatar.png', fit: widget.fit),
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
    return SizedBox.expand(
      child: FutureBuilder<ImageProvider>(
        future: _provider,
        builder: (context, snapshot) {
          if (snapshot.hasError) {
            Error.throwWithStackTrace(snapshot.error!, snapshot.stackTrace!);
          }
          if (!snapshot.hasData) return const SizedBox.expand();
          return Image(image: snapshot.requireData, fit: widget.fit);
        },
      ),
    );
  }
}
