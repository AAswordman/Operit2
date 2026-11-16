import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';

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

enum ThemeAssetPurpose { background, bubble, other }

enum ThemeAssetStage {
  read,
  decode,
  videoOpen,
  playback,
  importAsset,
  dispose;

  String get label => this == importAsset ? 'import' : name;
}

enum ThemeAssetFailureKind { missing, empty, read, decode, media, timeout, unknown }

/// Keeps the resource and request site when a backend supplies no Dart stack.
class ThemeAssetFailure implements Exception {
  const ThemeAssetFailure({
    required this.reference,
    required this.stage,
    required this.kind,
    required this.cause,
    required this.originalStackTrace,
    required this.requestStackTrace,
    this.purpose = ThemeAssetPurpose.other,
    this.themeRevision = 0,
    this.themeId,
    this.byteLength,
    this.retainedPrevious = false,
  });

  /// Adds UI context without replacing the original storage or decoder failure.
  factory ThemeAssetFailure.capture({
    required String reference,
    required ThemeAssetStage stage,
    required ThemeAssetFailureKind kind,
    required Object cause,
    required StackTrace originalStackTrace,
    required StackTrace requestStackTrace,
    ThemeAssetPurpose purpose = ThemeAssetPurpose.other,
    int? byteLength,
    bool retainedPrevious = false,
  }) {
    if (cause is ThemeAssetFailure) {
      return cause.withContext(
        purpose: purpose,
        retainedPrevious: retainedPrevious,
      );
    }
    return ThemeAssetFailure(
      reference: reference,
      stage: stage,
      kind: kind,
      cause: cause,
      originalStackTrace: originalStackTrace,
      requestStackTrace: requestStackTrace,
      purpose: purpose,
      byteLength: byteLength,
      retainedPrevious: retainedPrevious,
    );
  }

  final String reference;
  final ThemeAssetPurpose purpose;
  final int themeRevision;
  final String? themeId;
  final ThemeAssetStage stage;
  final ThemeAssetFailureKind kind;
  final int? byteLength;
  final Object cause;
  final StackTrace originalStackTrace;
  final StackTrace requestStackTrace;
  final bool retainedPrevious;

  bool get hasOriginalStack => originalStackTrace.toString().trim().isNotEmpty;

  ThemeAssetFailure withContext({
    ThemeAssetPurpose? purpose,
    int? themeRevision,
    String? themeId,
    bool? retainedPrevious,
  }) => ThemeAssetFailure(
    reference: reference,
    stage: stage,
    kind: kind,
    cause: cause,
    originalStackTrace: originalStackTrace,
    requestStackTrace: requestStackTrace,
    purpose: purpose ?? this.purpose,
    themeRevision: themeRevision ?? this.themeRevision,
    themeId: themeId ?? this.themeId,
    byteLength: byteLength,
    retainedPrevious: retainedPrevious ?? this.retainedPrevious,
  );

  String get key => '$themeRevision|${purpose.name}|${stage.label}|$reference';

  /// Data URIs can contain the entire video; omit the payload from diagnostics.
  static String safeReference(String value) => value.replaceAll(
    RegExp(r'data:[^\s\)\]\}\x22\x27]+', caseSensitive: false),
    '<data URI>',
  );

  String get details => '$this\n'
      'Original Dart stack: ${hasOriginalStack ? safeReference(originalStackTrace.toString()) : "<empty>"}\n'
      'Request Dart stack (not a native stack): ${safeReference(requestStackTrace.toString())}';

  @override
  String toString() => 'Theme asset failure '
      'reference=${safeReference(reference)} purpose=${purpose.name} '
      'revision=$themeRevision theme=${safeReference(themeId ?? "unknown")} '
      'stage=${stage.label} kind=${kind.name} bytes=${byteLength ?? "unknown"} '
      'retainedPrevious=$retainedPrevious cause=${safeReference(cause.toString())}';
}

/// Owns transient failures and retry epochs, never appearance preferences.
class ThemeAssetDiagnostics extends ChangeNotifier {
  final Map<String, ThemeAssetFailure> _failures = {};
  final Map<Object?, Map<String, ThemeAssetFailure>> _ownedFailures = {};
  final Map<String, int> _retryEpochs = {};
  int _themeRevision = 0;
  String? _themeId;
  Set<String>? _activeReferences;
  bool _notificationScheduled = false;
  bool _disposed = false;

  int get themeRevision => _themeRevision;
  String? get themeId => _themeId;
  List<ThemeAssetFailure> get failures => List.unmodifiable(_failures.values);
  int retryEpoch(String reference) => _retryEpochs[reference] ?? 0;

  /// Clears only failures for resources no longer used by this appearance.
  void updateContext({
    required int themeRevision,
    String? themeId,
    Set<String>? activeReferences,
  }) {
    if (_disposed) return;
    _themeRevision = themeRevision;
    _themeId = themeId;
    _activeReferences = activeReferences == null ? null : Set.unmodifiable(activeReferences);
    if (activeReferences != null) {
      var changed = false;
      _ownedFailures.removeWhere((_, failures) {
        final before = failures.length;
        failures.removeWhere((_, f) => !activeReferences.contains(f.reference));
        changed = changed || failures.length != before;
        return failures.isEmpty;
      });
      if (changed) _rebuildFailures();
    }
  }

  /// Consumers share one notice but retain independent retry eligibility.
  void report(ThemeAssetFailure failure, {Object? owner}) {
    if (_disposed) return;
    if (_activeReferences != null && !_activeReferences!.contains(failure.reference)) return;
    final contextual = failure.withContext(
      themeRevision: _themeRevision,
      themeId: _themeId,
    );
    final failures = _ownedFailures.putIfAbsent(owner, () => {});
    if (failures.containsKey(contextual.key)) return;
    failures[contextual.key] = contextual;
    _rebuildFailures();
  }

  void resolve(
    String reference, {
    ThemeAssetPurpose purpose = ThemeAssetPurpose.other,
    Object? owner,
  }) {
    if (_disposed) return;
    final failures = _ownedFailures[owner];
    if (failures == null) return;
    final before = failures.length;
    failures.removeWhere(
      (_, f) => f.reference == reference && f.purpose == purpose,
    );
    if (failures.isEmpty) _ownedFailures.remove(owner);
    if (failures.length != before) _rebuildFailures();
  }

  /// Unmounting one consumer cannot clear a different consumer's failure.
  void release(Object owner) {
    if (!_disposed && _ownedFailures.remove(owner) != null) _rebuildFailures();
  }

  void _rebuildFailures() {
    _failures.clear();
    for (final failures in _ownedFailures.values) {
      for (final entry in failures.entries) {
        final previous = _failures[entry.key];
        if (previous == null) {
          _failures[entry.key] = entry.value;
        } else if (previous.retainedPrevious && !entry.value.retainedPrevious) {
          _failures[entry.key] = previous.withContext(retainedPrevious: false);
        }
      }
    }
    _notify();
  }

  /// A new epoch bypasses a failed image-cache entry for the same logical path.
  void retry(String reference) {
    if (_disposed || !_failures.values.any((f) => f.reference == reference)) {
      return;
    }
    _retryEpochs[reference] = retryEpoch(reference) + 1;
    _ownedFailures.removeWhere((_, failures) {
      failures.removeWhere((_, f) => f.reference == reference);
      return failures.isEmpty;
    });
    _rebuildFailures();
  }

  void _notify() {
    if (_disposed) return;
    if (SchedulerBinding.instance.schedulerPhase == SchedulerPhase.persistentCallbacks) {
      if (_notificationScheduled) return;
      _notificationScheduled = true;
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _notificationScheduled = false;
        if (!_disposed) notifyListeners();
      });
      WidgetsBinding.instance.scheduleFrame();
    } else {
      notifyListeners();
    }
  }

  @override
  void dispose() {
    _disposed = true;
    _ownedFailures.clear();
    _failures.clear();
    _retryEpochs.clear();
    super.dispose();
  }
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

class ThemeAssetScope extends InheritedNotifier<ThemeAssetDiagnostics> {
  const ThemeAssetScope({
    super.key,
    required this.resolver,
    ThemeAssetDiagnostics? diagnostics,
    required super.child,
  }) : super(notifier: diagnostics);

  final ThemeAssetResolver resolver;
  ThemeAssetDiagnostics? get diagnostics => notifier;

  static ThemeAssetScope? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<ThemeAssetScope>();

  static ThemeAssetResolver of(BuildContext context) {
    final scope = maybeOf(context);
    if (scope == null) {
      throw StateError('ThemeAssetScope is required to render theme assets');
    }
    return scope.resolver;
  }

  static ThemeAssetDiagnostics? diagnosticsOf(BuildContext context) =>
      maybeOf(context)?.diagnostics;

  @override
  bool updateShouldNotify(ThemeAssetScope oldWidget) =>
      !identical(resolver, oldWidget.resolver) || super.updateShouldNotify(oldWidget);
}

/// An explicit resolver is required; this widget never fetches a URI or a file.
class ResolvedThemeImage extends StatefulWidget {
  const ResolvedThemeImage({
    super.key,
    required this.resolver,
    required this.reference,
    required this.fit,
    this.purpose = ThemeAssetPurpose.background,
    this.diagnostics,
  });

  final ThemeAssetResolver resolver;
  final String reference;
  final BoxFit fit;
  final ThemeAssetPurpose purpose;
  final ThemeAssetDiagnostics? diagnostics;

  @override
  State<ResolvedThemeImage> createState() => _ResolvedThemeImageState();
}

class _ResolvedThemeImageState extends State<ResolvedThemeImage> {
  ui.Image? _image;
  ImageStream? _stream;
  ImageStreamListener? _listener;
  _ThemeAssetImageProvider? _provider;
  ThemeAssetDiagnostics? _diagnostics;
  ThemeAssetResolver? _boundResolver;
  String? _boundReference;
  int _epoch = -1;
  int _loadToken = 0;
  bool _streamComplete = true;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    _setDiagnostics(widget.diagnostics ?? ThemeAssetScope.diagnosticsOf(context));
    _syncBinding();
  }

  @override
  void didUpdateWidget(covariant ResolvedThemeImage oldWidget) {
    super.didUpdateWidget(oldWidget);
    _setDiagnostics(widget.diagnostics ?? ThemeAssetScope.diagnosticsOf(context));
    _syncBinding();
  }

  void _setDiagnostics(ThemeAssetDiagnostics? diagnostics) {
    if (identical(_diagnostics, diagnostics)) return;
    _diagnostics?.removeListener(_handleRetry);
    _diagnostics?.release(this);
    _diagnostics = diagnostics;
    _diagnostics?.addListener(_handleRetry);
  }

  void _handleRetry() {
    if (mounted && (_diagnostics?.retryEpoch(widget.reference) ?? 0) != _epoch) {
      setState(_syncBinding);
    }
  }

  /// A repaint or another failed resource never reloads this image.
  void _syncBinding() {
    final epoch = _diagnostics?.retryEpoch(widget.reference) ?? 0;
    if (identical(_boundResolver, widget.resolver) &&
        _boundReference == widget.reference &&
        _epoch == epoch) return;
    final token = ++_loadToken;
    _detach();
    if (_boundReference == widget.reference && _epoch != epoch && _provider != null) {
      PaintingBinding.instance.imageCache.evict(_provider!);
    }
    _boundResolver = widget.resolver;
    _boundReference = widget.reference;
    _epoch = epoch;
    _diagnostics?.release(this);
    _provider = null;
    final resolver = widget.resolver;
    final reference = widget.reference;
    final requestStack = StackTrace.current;
    if (resolver is ThemeDecodedImageResolver) {
      try {
        final image = resolver.tryCloneImage(reference);
        if (image == null) throw StateError('Decoded theme asset is missing: $reference');
        _replace(image, synchronous: true);
      } catch (error, stack) {
        _report(error, stack, requestStack, reference);
      }
      return;
    }
    _provider = _ThemeAssetImageProvider(resolver, reference, epoch);
    _streamComplete = false;
    _stream = _provider!.resolve(createLocalImageConfiguration(context));
    _listener = ImageStreamListener(
      (info, synchronous) {
        if (!mounted || token != _loadToken) {
          info.dispose();
          return;
        }
        _streamComplete = true;
        final image = info.image.clone();
        info.dispose();
        _replace(image, synchronous: synchronous);
      },
      onError: (Object error, StackTrace? stack) {
        if (mounted && token == _loadToken) {
          _streamComplete = true;
          _report(error, stack ?? StackTrace.empty, requestStack, reference);
        }
      },
    );
    _stream!.addListener(_listener!);
  }

  void _replace(ui.Image image, {required bool synchronous}) {
    final previous = _image;
    if (synchronous) {
      _image = image;
    } else {
      setState(() => _image = image);
    }
    previous?.dispose();
    _diagnostics?.resolve(widget.reference, purpose: widget.purpose, owner: this);
  }

  void _report(Object error, StackTrace stack, StackTrace requestStack, String reference) {
    _diagnostics?.report(ThemeAssetFailure.capture(
      reference: reference,
      stage: ThemeAssetStage.decode,
      kind: ThemeAssetFailureKind.decode,
      cause: error,
      originalStackTrace: stack,
      requestStackTrace: requestStack,
      purpose: widget.purpose,
      retainedPrevious: _image != null,
    ), owner: this);
  }

  void _detach() {
    final stream = _stream;
    final listener = _listener;
    if (stream != null && listener != null) {
      // A pending completer can report a late error even after the widget leaves.
      // Drain it without retaining this State or publishing an obsolete failure.
      if (!_streamComplete) {
        late final ImageStreamListener drain;
        drain = ImageStreamListener(
          (info, _) {
            info.dispose();
            stream.removeListener(drain);
          },
          onError: (Object error, StackTrace? stack) => stream.removeListener(drain),
        );
        stream.addListener(drain);
      }
      stream.removeListener(listener);
    }
    _stream = null;
    _listener = null;
    _streamComplete = true;
  }

  /// Releases only the prepared image clone owned by this widget.
  @override
  void dispose() {
    _loadToken++;
    _diagnostics?.removeListener(_handleRetry);
    _diagnostics?.release(this);
    _detach();
    _image?.dispose();
    super.dispose();
  }

  /// Lets Flutter synchronously reuse decoded runtime images across page mounts.
  @override
  Widget build(BuildContext context) {
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
  const _ThemeAssetImageProvider(this.resolver, this.reference, this.epoch);

  final ThemeAssetResolver resolver;
  final String reference;
  final int epoch;

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
    final requestStack = StackTrace.current;
    var stage = ThemeAssetStage.read;
    var kind = ThemeAssetFailureKind.read;
    int? byteLength;
    try {
      final bytes = await resolver.readBytes(reference);
      byteLength = bytes.length;
      if (bytes.isEmpty) {
        kind = ThemeAssetFailureKind.empty;
        throw StateError('Theme asset is empty: $reference');
      }
      stage = ThemeAssetStage.decode;
      kind = ThemeAssetFailureKind.decode;
      return ImageInfo(image: await decodeThemeImageBytes(reference, bytes));
    } catch (error, stack) {
      PaintingBinding.instance.imageCache.evict(this);
      throw ThemeAssetFailure.capture(
        reference: reference,
        stage: stage,
        kind: kind,
        cause: error,
        originalStackTrace: stack,
        requestStackTrace: requestStack,
        byteLength: byteLength,
      );
    }
  }

  /// Keeps identical references from different runtime owners isolated.
  @override
  bool operator ==(Object other) =>
      other is _ThemeAssetImageProvider &&
      identical(resolver, other.resolver) &&
      reference == other.reference && epoch == other.epoch;

  /// Matches the resolver identity and asset reference used by equality.
  @override
  int get hashCode => Object.hash(identityHashCode(resolver), reference, epoch);
}
