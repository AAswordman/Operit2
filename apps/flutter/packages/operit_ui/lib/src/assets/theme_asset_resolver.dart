import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import 'theme_image_decoder.dart';

/// Application: Core-owned bytes. Preview: checked immutable bytes in memory.
abstract interface class ThemeAssetResolver {
  Future<Uint8List> readBytes(String reference);
}

/// A prepared immutable image source. The caller owns each returned clone.
abstract interface class ThemeDecodedImageResolver
    implements ThemeAssetResolver {
  ui.Image cloneImage(String reference);
  ui.Image? tryCloneImage(String reference);
}

class MemoryThemeAssetResolver implements ThemeAssetResolver {
  MemoryThemeAssetResolver(Map<String, Uint8List> assets)
    : _assets = Map.unmodifiable(
        assets.map((key, value) => MapEntry(key, Uint8List.fromList(value))),
      );

  final Map<String, Uint8List> _assets;

  @override
  Future<Uint8List> readBytes(String reference) async {
    final bytes = _assets[reference];
    if (bytes == null) {
      throw StateError('Theme asset is missing: $reference');
    }
    return Uint8List.fromList(bytes);
  }
}

class ThemeAssetScope extends InheritedWidget {
  const ThemeAssetScope({
    super.key,
    required this.resolver,
    required super.child,
  });

  final ThemeAssetResolver resolver;

  static ThemeAssetResolver of(BuildContext context) {
    final scope = context.dependOnInheritedWidgetOfExactType<ThemeAssetScope>();
    if (scope == null) {
      throw StateError('ThemeAssetScope is required to render theme assets');
    }
    return scope.resolver;
  }

  @override
  bool updateShouldNotify(ThemeAssetScope oldWidget) =>
      !identical(resolver, oldWidget.resolver);
}

/// An explicit resolver is required; this widget never fetches a URI or a file.
class ResolvedThemeImage extends StatefulWidget {
  const ResolvedThemeImage({
    super.key,
    required this.resolver,
    required this.reference,
    required this.fit,
  });

  final ThemeAssetResolver resolver;
  final String reference;
  final BoxFit fit;

  @override
  State<ResolvedThemeImage> createState() => _ResolvedThemeImageState();
}

class _ResolvedThemeImageState extends State<ResolvedThemeImage> {
  ui.Image? _image;
  _ThemeAssetImageProvider? _provider;

  /// Resolves prepared images immediately and shares runtime images through ImageCache.
  @override
  void initState() {
    super.initState();
    _bind();
  }

  /// Rebinds only when the asset or its owning resolver changes.
  @override
  void didUpdateWidget(covariant ResolvedThemeImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.reference != widget.reference ||
        !identical(oldWidget.resolver, widget.resolver)) {
      _bind();
    }
  }

  /// Keeps ownership of prepared clones separate from cached image streams.
  void _bind() {
    _image?.dispose();
    _image = null;
    _provider = null;
    final resolver = widget.resolver;
    if (resolver is ThemeDecodedImageResolver) {
      _image = resolver.tryCloneImage(widget.reference);
    } else {
      _provider = _ThemeAssetImageProvider(resolver, widget.reference);
    }
  }

  /// Releases only the prepared image clone owned by this widget.
  @override
  void dispose() {
    _image?.dispose();
    super.dispose();
  }

  /// Lets Flutter synchronously reuse decoded runtime images across page mounts.
  @override
  Widget build(BuildContext context) {
    final provider = _provider;
    if (provider != null) {
      return Image(
        image: provider,
        fit: widget.fit,
        width: double.infinity,
        height: double.infinity,
        errorBuilder: (context, error, stack) =>
            Error.throwWithStackTrace(error, stack ?? StackTrace.current),
      );
    }
    final image = _image;
    if (image == null) return const SizedBox.expand();
    return SizedBox.expand(
      child: RawImage(
        image: image,
        fit: widget.fit,
        width: double.infinity,
        height: double.infinity,
      ),
    );
  }
}

/// Shares in-flight reads and decoded images within Flutter's bounded image cache.
class _ThemeAssetImageProvider extends ImageProvider<_ThemeAssetImageProvider> {
  /// Identifies one immutable asset within its resolver's lifetime.
  const _ThemeAssetImageProvider(this.resolver, this.reference);

  final ThemeAssetResolver resolver;
  final String reference;

  /// Makes cache hits available during the first build of a newly mounted preview.
  @override
  Future<_ThemeAssetImageProvider> obtainKey(
    ImageConfiguration configuration,
  ) => SynchronousFuture<_ThemeAssetImageProvider>(this);

  /// Uses the same validated static-image decoder as prepared theme visuals.
  @override
  ImageStreamCompleter loadImage(
    _ThemeAssetImageProvider key,
    ImageDecoderCallback decode,
  ) => OneFrameImageStreamCompleter(key._load());

  /// Reads and decodes once for all listeners of this resolver and asset.
  Future<ImageInfo> _load() async {
    try {
      final bytes = await resolver.readBytes(reference);
      return ImageInfo(image: await decodeThemeImageBytes(reference, bytes));
    } catch (_) {
      PaintingBinding.instance.imageCache.evict(this);
      rethrow;
    }
  }

  /// Keeps identical references from different runtime owners isolated.
  @override
  bool operator ==(Object other) =>
      other is _ThemeAssetImageProvider &&
      identical(resolver, other.resolver) &&
      reference == other.reference;

  /// Matches the resolver identity and asset reference used by equality.
  @override
  int get hashCode => Object.hash(identityHashCode(resolver), reference);
}
