import 'dart:async';
import 'dart:typed_data';
import 'dart:ui' as ui;

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
  Object? _loadError;
  StackTrace? _loadStack;
  int _loadToken = 0;

  @override
  void initState() {
    super.initState();
    _bind();
  }

  @override
  void didUpdateWidget(covariant ResolvedThemeImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.reference != widget.reference ||
        !identical(oldWidget.resolver, widget.resolver)) {
      _bind();
    }
  }

  void _bind() {
    final token = ++_loadToken;
    _image?.dispose();
    _image = null;
    _loadError = null;
    _loadStack = null;
    final resolver = widget.resolver;
    if (resolver is ThemeDecodedImageResolver) {
      _image = resolver.tryCloneImage(widget.reference);
    } else {
      unawaited(_loadBytes(resolver, token));
    }
  }

  Future<void> _loadBytes(ThemeAssetResolver resolver, int token) async {
    try {
      final bytes = await resolver.readBytes(widget.reference);
      final image = await decodeThemeImageBytes(widget.reference, bytes);
      if (!mounted || token != _loadToken) {
        image.dispose();
        return;
      }
      setState(() => _image = image);
    } catch (error, stack) {
      if (!mounted || token != _loadToken) return;
      setState(() {
        _loadError = error;
        _loadStack = stack;
      });
    }
  }

  @override
  void dispose() {
    _loadToken++;
    _image?.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    if (_loadError != null) {
      Error.throwWithStackTrace(_loadError!, _loadStack!);
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
