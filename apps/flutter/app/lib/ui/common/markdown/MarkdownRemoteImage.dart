// ignore_for_file: file_names

import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';
import 'package:operit2/core/proxy/generated/CoreProxyModels.g.dart';

import '../interactions/MessagePressShield.dart';
import '../../features/packages/screens/ComposeDslImageSource.dart';
import 'MarkdownImagePreview.dart';
import 'MarkdownFileScope.dart';

class MarkdownRemoteImage extends StatelessWidget {
  /// Creates a Markdown image backed by the runtime file or HTTP service.
  const MarkdownRemoteImage({
    super.key,
    required this.url,
    this.alt = '',
    this.fit = BoxFit.contain,
    this.enablePreview = true,
    this.decodeAtFullResolution = false,
    this.clients = _imageClients,
  });

  final GeneratedCoreProxyClients clients;
  final String url;
  final String alt;
  final BoxFit fit;
  final bool enablePreview;
  final bool decodeAtFullResolution;

  /// Builds the image and its link interaction within the layout constraints.
  @override
  Widget build(BuildContext context) {
    final label = alt.trim().isEmpty ? url : alt.trim();
    final error = _MarkdownImageStatus(
      icon: Icons.broken_image_outlined,
      label: label,
    );
    return LayoutBuilder(
      builder: (context, constraints) {
        // Sidebar animations change local constraints every frame. Decode for
        // the window so they only relayout the existing image/cache entry.
        final windowWidth = MediaQuery.sizeOf(context).width;
        final cacheWidth = decodeAtFullResolution
            ? null
            : _markdownImageCacheExtent(
                context,
                windowWidth > 0 ? windowWidth : constraints.maxWidth,
              );
        final cacheHeight = !decodeAtFullResolution && cacheWidth == null
            ? _markdownImageCacheExtent(context, constraints.maxHeight)
            : null;
        final scope = MarkdownFileScope.maybeOf(context);
        final owner = scope?.clients ?? clients;
        final trimmed = scope?.resolve(url) ?? url.trim();
        Future<Uint8List> readBytes(String path) async {
          try {
            return await readComposeDslVfsBytes(owner, path);
          } catch (_) {
            final source = scope?.sourcePath(path);
            if (source == null) rethrow;
            return readComposeDslVfsBytes(owner, source);
          }
        }

        final isDataImage = trimmed.toLowerCase().startsWith('data:image/');
        final dataBytes = markdownDataImageBytes(trimmed);
        final vfsPath = composeDslVfsImagePath(trimmed);
        final Widget image;
        if (isDataImage && dataBytes == null) {
          image = error;
        } else if (vfsPath != null) {
          image = Image(
            key: ValueKey((trimmed, owner.bridge)),
            gaplessPlayback: true,
            image: ResizeImage.resizeIfNeeded(
              cacheWidth,
              cacheHeight,
              ComposeDslVfsImage(
                path: vfsPath,
                owner: owner.bridge,
                clients: owner,
                readBytes: readBytes,
              ),
            ),
            fit: fit,
            semanticLabel: alt.trim().isEmpty ? null : alt.trim(),
            errorBuilder: (context, failure, stackTrace) => error,
          );
        } else {
          image = _RuntimeImage(
            clients: owner,
            url: trimmed,
            alt: alt,
            fit: fit,
            cacheWidth: cacheWidth,
            cacheHeight: cacheHeight,
            error: error,
          );
        }
        if (!enablePreview) {
          return image;
        }
        return MessagePressShieldRegion(
          child: GestureDetector(
            behavior: HitTestBehavior.opaque,
            onTap: () => showDialog<void>(
              context: context,
              useSafeArea: false,
              builder: (_) => scope == null
                  ? MarkdownImagePreview(
                      alt: alt,
                      image: MarkdownRemoteImage(
                        url: trimmed,
                        alt: alt,
                        clients: owner,
                        enablePreview: false,
                        decodeAtFullResolution: true,
                      ),
                    )
                  : MarkdownFileScope(
                      basePath: scope.basePath,
                      nodeId: scope.nodeId,
                      clients: owner,
                      child: MarkdownImagePreview(
                        alt: alt,
                        image: MarkdownRemoteImage(
                          url: trimmed,
                          alt: alt,
                          clients: owner,
                          enablePreview: false,
                          decodeAtFullResolution: true,
                        ),
                      ),
                    ),
            ),
            child: image,
          ),
        );
      },
    );
  }
}

const _imageClients = GeneratedCoreProxyClients(
  ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
);

/// Renders the image source explicitly selected by the runtime host.
class _RuntimeImage extends StatefulWidget {
  /// Creates a renderer with the requested decode dimensions.
  const _RuntimeImage({
    required this.clients,
    required this.url,
    required this.alt,
    required this.fit,
    required this.cacheWidth,
    required this.cacheHeight,
    required this.error,
  });

  final GeneratedCoreProxyClients clients;
  final String url;
  final String alt;
  final BoxFit fit;
  final int? cacheWidth;
  final int? cacheHeight;
  final Widget error;

  /// Creates state that retains the request across layout changes.
  @override
  State<_RuntimeImage> createState() => _RuntimeImageState();
}

class _RuntimeImageState extends State<_RuntimeImage> {
  late Stream<RuntimeImageData> _request;

  /// Starts the runtime request when the image enters the widget tree.
  @override
  void initState() {
    super.initState();
    _load();
  }

  /// Replaces the request when the source changes.
  @override
  void didUpdateWidget(covariant _RuntimeImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.url != widget.url ||
        oldWidget.clients.bridge != widget.clients.bridge) {
      _load();
    }
  }

  /// Creates the cold proxy stream owned by the image subscription.
  void _load() {
    final bytes = markdownDataImageBytes(widget.url);
    if (bytes != null) {
      _request =
          Stream.fromFuture(
            readImageDisplayBytes(bytes, clients: widget.clients),
          ).map(
            (bytes) =>
                RuntimeImageData(bytes: bytes, displayUrl: null, error: null),
          );
      return;
    }
    _request = widget.clients.servicesRuntimeHostInteractionService.imageBytes(
      url: widget.url,
    );
  }

  /// Displays the terminal result and cancels collection on disposal.
  @override
  Widget build(BuildContext context) {
    return StreamBuilder<RuntimeImageData>(
      key: ValueKey(widget.url),
      stream: _request,
      builder: (context, snapshot) {
        if (snapshot.hasError || snapshot.data?.error != null) {
          return widget.error;
        }
        final data = snapshot.data;
        if (data == null) {
          if (snapshot.connectionState == ConnectionState.done) {
            return widget.error;
          }
          return const _MarkdownImageStatus(icon: null, label: '');
        }
        final displayUrl = data.displayUrl;
        if (displayUrl != null) {
          // The host selected direct display; no byte request is started here.
          return Image.network(
            displayUrl,
            key: ValueKey((widget.url, widget.clients.bridge)),
            gaplessPlayback: true,
            fit: widget.fit,
            webHtmlElementStrategy: WebHtmlElementStrategy.prefer,
            semanticLabel: widget.alt.trim().isEmpty ? null : widget.alt.trim(),
            loadingBuilder: (context, child, progress) => progress == null
                ? child
                : const _MarkdownImageStatus(icon: null, label: ''),
            errorBuilder: (context, failure, stackTrace) => widget.error,
          );
        }
        return Image.memory(
          data.bytes,
          key: ValueKey((widget.url, widget.clients.bridge)),
          gaplessPlayback: true,
          fit: widget.fit,
          cacheWidth: widget.cacheWidth,
          cacheHeight: widget.cacheHeight,
          semanticLabel: widget.alt.trim().isEmpty ? null : widget.alt.trim(),
          errorBuilder: (context, failure, stackTrace) => widget.error,
        );
      },
    );
  }
}

/// Limits decoded image dimensions to the available physical pixel extent.
int? _markdownImageCacheExtent(BuildContext context, double extent) {
  if (!extent.isFinite || extent <= 0) {
    return null;
  }
  final pixels = (extent * MediaQuery.devicePixelRatioOf(context)).ceil();
  if (pixels <= 0) {
    return null;
  }
  return pixels > 4096 ? 4096 : pixels;
}

class _MarkdownImageStatus extends StatelessWidget {
  /// Creates a loading or failure indicator for the image request.
  const _MarkdownImageStatus({required this.icon, required this.label});

  final IconData? icon;
  final String label;

  /// Builds the current image loading or failure status.
  @override
  Widget build(BuildContext context) {
    final color = Theme.of(context).colorScheme.onSurfaceVariant;
    if (icon == null) {
      return const SizedBox(
        height: 72,
        width: double.infinity,
        child: Center(
          child: SizedBox.square(
            dimension: 18,
            child: CircularProgressIndicator(strokeWidth: 2),
          ),
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 4),
      child: Row(
        children: <Widget>[
          Icon(icon, size: 18, color: color),
          const SizedBox(width: 8),
          Expanded(
            child: Text(
              label,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.bodySmall?.copyWith(
                color: color,
                decoration: TextDecoration.underline,
              ),
            ),
          ),
        ],
      ),
    );
  }
}

final _markdownDataImages = ComposeDslDataImageCache();

/// Reuses immutable data-URI bytes across rebuilds within a bounded cache.
/// Malformed input is reported as invalid data.
Uint8List? markdownDataImageBytes(String imageUrl) {
  final trimmed = imageUrl.trim();
  if (!trimmed.toLowerCase().startsWith('data:image/')) {
    return null;
  }
  try {
    return _markdownDataImages.resolve(trimmed).bytes;
  } on FormatException {
    return null;
  }
}
