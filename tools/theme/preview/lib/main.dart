import 'dart:async';
import 'dart:convert';
import 'dart:ui' as ui;

import 'package:crypto/crypto.dart';
import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:operit_ui/operit_ui.dart';

import 'protocol/theme_studio_message_port.dart';
import 'protocol/theme_contract_client.dart';

/// Standalone shared renderer. Package bytes never enter this process as a ZIP:
/// Studio sends declarative JSON and binary assets over MessagePort. The
/// preview's OWN Rust/WASM worker validates them before any Flutter decoder.
Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  const fixtureSourceSha256 = String.fromEnvironment(
    'OPERIT_THEME_FIXTURE_SOURCE_SHA256',
  );
  if (!RegExp(r'^[a-f0-9]{64}$').hasMatch(fixtureSourceSha256)) {
    throw StateError('Build requires a pinned fixture source SHA-256');
  }
  final fixture = await rootBundle.loadString(
    'packages/operit_ui/assets/fixtures/theme-v1.json',
  );
  final spec = ThemeVisualSpecV1.fromCheckedContract(
    jsonDecode(fixture) as Map<String, dynamic>,
  );
  final fonts = await BuiltinThemeFonts.load(bundle: rootBundle);
  final artwork = await _fixtureImage();
  runApp(
    ThemePreviewProbe(
      spec: spec,
      builtinFonts: fonts,
      artwork: artwork,
      fixtureSourceSha256: fixtureSourceSha256,
    ),
  );
}

/// Local, deterministic test artwork, not a resource supplied by an author.
Future<Uint8List> _fixtureImage() async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  canvas.drawRect(
    const Rect.fromLTWH(0, 0, 160, 160),
    Paint()..color = const Color(0xFFD1E4FF),
  );
  canvas.drawRRect(
    RRect.fromRectAndRadius(
      const Rect.fromLTWH(8, 8, 144, 144),
      const Radius.circular(24),
    ),
    Paint()..color = const Color(0xFFA0CAFD),
  );
  canvas.drawCircle(
    const Offset(80, 80),
    28,
    Paint()..color = const Color(0xFFD7E3F8),
  );
  final picture = recorder.endRecording();
  final image = await picture.toImage(160, 160);
  try {
    final data = await image.toByteData(format: ui.ImageByteFormat.png);
    if (data == null) throw StateError('Cannot encode the rendering fixture');
    return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
  } finally {
    image.dispose();
    picture.dispose();
  }
}

class ThemePreviewProbe extends StatefulWidget {
  const ThemePreviewProbe({
    super.key,
    required this.spec,
    required this.builtinFonts,
    required this.artwork,
    required this.fixtureSourceSha256,
  });

  final ThemeVisualSpecV1 spec;
  final BuiltinThemeFonts builtinFonts;
  final Uint8List artwork;
  final String fixtureSourceSha256;

  @override
  State<ThemePreviewProbe> createState() => _ThemePreviewProbeState();
}

class _ThemePreviewProbeState extends State<ThemePreviewProbe> {
  final ThemeProbePreparationController _preparation =
      ThemeProbePreparationController();
  final ThemeFontRegistry _fontRegistry = ThemeFontRegistry();
  final GlobalKey _captureKey = GlobalKey();
  final ThemeStudioMessagePort? _studioPort =
      ThemeStudioMessagePort.fromLocation();
  late final Future<PreparedThemeFont> _mono;
  void Function(FlutterErrorDetails details)? _previousErrorHandler;
  int _scheduledRevision = 0;
  int _failedFrameRevision = 0;
  bool _dark = false;
  bool _glass = false;
  bool _background = true;
  bool _blur = false;
  bool _imageBubble = true;
  bool _stretch = false;
  bool _agentInput = true;
  bool _showConversation = false;
  bool _inspectEnabled = false;
  String? _selectedInspectTargetId;
  static const Set<String> _inspectTargetIds = <String>{
    ThemeInspectableTargetId.background,
    ThemeInspectableTargetId.sidebar,
    ThemeInspectableTargetId.topBar,
    ThemeInspectableTargetId.userMessage,
    ThemeInspectableTargetId.aiMessage,
    ThemeInspectableTargetId.composer,
  };
  late ThemeVisualSpecV1 _activeSpec;
  String? _activeThemeSha256;
  bool _externalTheme = false;
  ({int width, int height})? _requestedViewport;

  @override
  void initState() {
    super.initState();
    _preparation.addListener(_changed);
    _mono = widget.builtinFonts.prepare(
      kCascadiaMonoFontId,
      registry: _fontRegistry,
      fixtureText: 'Operit2 Theme Preview 0123456789 {}[]()',
    );
    _previousErrorHandler = FlutterError.onError;
    FlutterError.onError = _frameError;
    _activeSpec = widget.spec;
    unawaited(_prepareRevision());
    unawaited(_initializeStudio());
  }

  Future<void> _initializeStudio() async {
    final port = _studioPort;
    if (port == null) return;
    try {
      await initializeThemeContractWorker(port.rendererReleaseId);
      if (!mounted) return;
      port.attach(
        onRequest: _handleStudioRequest,
        nextRevision: () => _preparation.revision + 1,
      );
    } catch (error) {
      if (!mounted) return;
      _preparation.failFrame(revision: _preparation.revision, error: error);
      port.reportStartupError(
        code: 'contract-startup-failed',
        message: error.toString(),
      );
    }
  }

  Future<void> _handleStudioRequest(Map<String, dynamic> request) async {
    switch (request['type']) {
      case 'operit.theme.preview.requestReceipt.v1':
        _sendReceipt(request);
        return;
      case 'operit.theme.preview.load.v1':
        await _loadStudioTheme(request);
        return;
      case 'operit.theme.preview.capture.v1':
        await _captureStudioFrame(request);
        return;
      case 'operit.theme.preview.inspectMode.v1':
        _setInspectMode(request);
        return;
      case 'operit.theme.preview.sceneMode.v1':
        await _setConversationMode(request);
        return;
      default:
        _studioPort?.sendError(
          code: 'unknown-message',
          message: 'preview protocol message is not registered',
          revision: request['revision'] is int
              ? request['revision'] as int
              : null,
        );
    }
  }

  void _setInspectMode(Map<String, dynamic> request) {
    final revision = request['revision'];
    final enabled = request['enabled'];
    final targetId = request['targetId'];
    final receipt = _preparation.receipt;
    if (revision is! int ||
        enabled is! bool ||
        revision != _preparation.revision ||
        _preparation.phase != ThemeProbePhase.rendered ||
        receipt?.revision != revision ||
        (targetId != null &&
            (targetId is! String || !_inspectTargetIds.contains(targetId))) ||
        (targetId is String &&
            (targetId == ThemeInspectableTargetId.userMessage ||
                targetId == ThemeInspectableTargetId.aiMessage) &&
            !_showConversation) ||
        (!enabled && targetId != null)) {
      throw FormatException(
        'Inspector mode must reference a registered target on the current rendered revision',
      );
    }
    setState(() {
      _inspectEnabled = enabled;
      _selectedInspectTargetId = targetId as String?;
    });
    _studioPort?.sendSelection(
      revision: revision,
      enabled: enabled,
      targetId: targetId as String?,
    );
  }

  Future<void> _setConversationMode(Map<String, dynamic> request) async {
    final revision = request['revision'];
    final showConversation = request['showConversation'];
    final receipt = _preparation.receipt;
    if (revision is! int ||
        showConversation is! bool ||
        revision != _preparation.revision ||
        _preparation.phase != ThemeProbePhase.rendered ||
        receipt?.revision != revision ||
        !_externalTheme ||
        _inspectEnabled ||
        _requestedViewport == null) {
      _studioPort?.sendError(
        code: 'invalid-scene-mode',
        message:
            'scene mode requires the current external theme frame with Inspector disabled',
        revision: revision is int ? revision : null,
      );
      return;
    }
    setState(() {
      _showConversation = showConversation;
      _selectedInspectTargetId = null;
    });
    await WidgetsBinding.instance.endOfFrame;
    if (!mounted ||
        revision != _preparation.revision ||
        _preparation.phase != ThemeProbePhase.rendered ||
        _preparation.receipt?.revision != revision ||
        _failedFrameRevision == revision ||
        !_viewportMatches(_viewportJson(), _requestedViewport)) {
      _studioPort?.sendError(
        code: 'scene-frame-failed',
        message: 'Flutter did not complete the selected conversation scene',
        revision: revision,
      );
      return;
    }
    _sendCurrentRenderedReceipt(revision);
  }

  void _selectInspectTarget(String targetId) {
    if (!_inspectEnabled || !_inspectTargetIds.contains(targetId)) return;
    setState(() => _selectedInspectTargetId = targetId);
    _studioPort?.sendSelection(
      revision: _preparation.revision,
      enabled: true,
      targetId: targetId,
    );
  }

  void _sendReceipt(Map<String, dynamic> request) {
    final receipt = _preparation.receipt;
    if (receipt == null ||
        _preparation.phase != ThemeProbePhase.rendered ||
        receipt.themeSha256 == null ||
        request['revision'] != receipt.revision) {
      _studioPort?.sendError(
        code: 'preview-not-rendered',
        message: 'the requested revision has no actual rendered frame',
        revision: request['revision'] is int
            ? request['revision'] as int
            : null,
      );
      return;
    }
    _sendCurrentRenderedReceipt(receipt.revision);
  }

  void _sendCurrentRenderedReceipt(int revision) {
    final receipt = _preparation.receipt;
    if (receipt == null ||
        receipt.revision != revision ||
        _preparation.phase != ThemeProbePhase.rendered ||
        receipt.themeSha256 == null) {
      _studioPort?.sendError(
        code: 'preview-not-rendered',
        message: 'the selected scene has no current rendered frame',
        revision: revision,
      );
      return;
    }
    _studioPort?.sendRendered(
      revision: revision,
      fixtureVersion: receipt.fixtureVersion,
      receipt: <String, dynamic>{
        ...receipt.toJson(),
        'brightness': _dark ? 'dark' : 'light',
        'showConversation': _showConversation,
        'viewport': _viewportJson(),
      },
    );
  }

  Future<void> _captureStudioFrame(Map<String, dynamic> request) async {
    final receipt = _preparation.receipt;
    final revision = request['revision'];
    final viewport = _readViewport(request['viewport']);
    final showConversation = request['showConversation'];
    if (revision is! int ||
        viewport == null ||
        _requestedViewport != viewport ||
        showConversation is! bool ||
        showConversation != _showConversation ||
        receipt == null ||
        _preparation.phase != ThemeProbePhase.rendered ||
        receipt.revision != revision ||
        receipt.themeSha256 == null ||
        !_externalTheme) {
      _studioPort?.sendError(
        code: 'capture-not-rendered',
        message: 'capture requires the current rendered external theme',
        revision: revision is int ? revision : null,
      );
      return;
    }
    if (_inspectEnabled) {
      _studioPort?.sendError(
        code: 'capture-inspection-active',
        message:
            'disable Inspector selection mode before capturing a clean Flutter frame',
        revision: revision,
      );
      return;
    }
    try {
      await WidgetsBinding.instance.endOfFrame;
      final boundary = _captureKey.currentContext?.findRenderObject();
      if (boundary is! RenderRepaintBoundary) {
        throw StateError('Flutter scene capture boundary is missing');
      }
      final image = await boundary.toImage(pixelRatio: 1);
      try {
        if (image.width != viewport.width || image.height != viewport.height) {
          throw StateError(
            'Flutter preview viewport must be ${viewport.width}×${viewport.height} at DPR 1',
          );
        }
        final data = await image.toByteData(format: ui.ImageByteFormat.png);
        if (data == null) {
          throw StateError('Flutter could not encode the preview frame');
        }
        final bytes = data.buffer.asUint8List(
          data.offsetInBytes,
          data.lengthInBytes,
        );
        if (bytes.isEmpty || bytes.length > 4 * 1024 * 1024) {
          throw StateError('Captured PNG exceeds the theme preview budget');
        }
        if (!mounted ||
            receipt.revision != _preparation.revision ||
            _preparation.phase != ThemeProbePhase.rendered ||
            showConversation != _showConversation ||
            !_viewportMatches(_viewportJson(), viewport)) {
          throw StateError('Theme frame changed during capture');
        }
        _studioPort?.sendCaptured(
          revision: revision,
          themeSha256: receipt.themeSha256!,
          fixtureVersion: receipt.fixtureVersion,
          showConversation: _showConversation,
          viewportWidth: viewport.width,
          viewportHeight: viewport.height,
          pngSha256: sha256.convert(bytes).toString(),
          pngBytes: bytes,
        );
      } finally {
        image.dispose();
      }
    } catch (error) {
      _studioPort?.sendError(
        code: 'capture-failed',
        message: error.toString(),
        revision: revision,
      );
    }
  }

  Future<void> _loadStudioTheme(Map<String, dynamic> request) async {
    final requestedRevision = request['revision'];
    final requestedViewport = _readViewport(request['viewport']);
    final showConversation = request['showConversation'];
    if (requestedRevision is! int ||
        requestedRevision <= _preparation.revision ||
        requestedViewport == null ||
        showConversation is! bool) {
      _studioPort?.sendError(
        code: requestedViewport == null
            ? 'invalid-viewport'
            : showConversation is! bool
            ? 'invalid-scene-mode'
            : 'stale-revision',
        message: requestedViewport == null
            ? 'Theme preview viewport is missing or outside the allowed bounds'
            : showConversation is! bool
            ? 'Theme preview conversation mode must be boolean'
            : 'Theme preview revisions must increase',
        revision: requestedRevision is int ? requestedRevision : null,
      );
      return;
    }
    setState(() {
      _inspectEnabled = false;
      _selectedInspectTargetId = null;
      _showConversation = showConversation;
    });
    try {
      _studioPort?.sendProgress(revision: requestedRevision, stage: 'received');
      // prepare() reserves the revision and invalidates the previous receipt
      // synchronously, including when validation of the new draft fails.
      await _preparation.prepare(
        revision: requestedRevision,
        load: () async {
          final source = request['canonicalThemeJson'];
          final expectedDigest = request['themeSha256'];
          final brightness = request['brightness'];
          if (source is! String ||
              utf8.encode(source).length > 256 * 1024 ||
              expectedDigest is! String ||
              !RegExp(r'^[a-f0-9]{64}$').hasMatch(expectedDigest) ||
              (brightness != 'light' && brightness != 'dark')) {
            throw FormatException(
              'Bounded theme JSON and a lowercase SHA-256 are required',
            );
          }
          final decodedAssets = _decodeStudioAssets(request['assets']);
          _studioPort!.sendProgress(
            revision: requestedRevision,
            stage: 'contract-validating',
          );
          // This cheap transport check does not confer media authority. The
          // preview-owned Rust/WASM operation below is still mandatory.
          if (sha256.convert(utf8.encode(source)).toString() !=
              expectedDigest) {
            throw FormatException('Studio theme transport SHA-256 differs');
          }
          final checked = await CheckedThemePreviewInput.validate(
            rendererReleaseId: _studioPort.rendererReleaseId,
            themeJson: source,
            assets: decodedAssets.bytes,
            mediaTypes: decodedAssets.mediaTypes,
          );
          if (checked.themeSha256 != expectedDigest ||
              checked.canonicalThemeJson != source) {
            throw FormatException(
              'Studio theme differs from the preview contract normalization',
            );
          }
          _studioPort.sendProgress(
            revision: requestedRevision,
            stage: 'contract-validated',
          );
          final spec = ThemeVisualSpecV1.fromCheckedContract(
            jsonDecode(checked.canonicalThemeJson) as Map<String, dynamic>,
          );
          _studioPort.sendProgress(
            revision: requestedRevision,
            stage: 'resources-preparing',
          );
          final prepared = await _prepareResources(
            spec: spec,
            assets: checked.assets,
            assetSha256: checked.assetSha256,
            imageReferences: spec.imageAssetReferences,
          );
          if (!mounted || _preparation.revision != requestedRevision) {
            prepared.dispose();
            throw StateError('Theme revision was superseded');
          }
          _activeSpec = spec;
          _activeThemeSha256 = checked.themeSha256;
          _externalTheme = true;
          _requestedViewport = requestedViewport;
          _dark = brightness == 'dark';
          _studioPort.sendProgress(
            revision: requestedRevision,
            stage: 'resources-prepared',
          );
          return prepared;
        },
      );
      if (!mounted || _preparation.revision != requestedRevision) return;
      if (_preparation.phase == ThemeProbePhase.failed) {
        throw StateError(_preparation.error.toString());
      }
      _studioPort?.sendLoaded(
        revision: requestedRevision,
        themeSha256: _activeThemeSha256!,
      );
      _studioPort?.sendProgress(
        revision: requestedRevision,
        stage: 'frame-waiting',
      );
    } catch (error) {
      _studioPort?.sendError(
        code: 'theme-load-invalid',
        message: error.toString(),
        revision: requestedRevision,
      );
    }
  }

  ({Map<String, Uint8List> bytes, Map<String, String> mediaTypes})
  _decodeStudioAssets(Object? rawAssets) {
    if (rawAssets is! List || rawAssets.length > 24) {
      throw FormatException('Studio asset list exceeds the v1 budget');
    }
    final assets = <String, Uint8List>{};
    final mediaTypes = <String, String>{};
    var totalBytes = 0;
    for (final raw in rawAssets) {
      if (raw is! Map) throw FormatException('Studio asset row is invalid');
      final row = Map<String, dynamic>.from(raw);
      final path = row['path'];
      final bytes = row['bytes'];
      final mediaType = row['mediaType'];
      final declaredSha256 = row['sha256'];
      if (path is! String ||
          !RegExp(r'^assets/(images|fonts)/').hasMatch(path) ||
          bytes is! Uint8List ||
          mediaType is! String ||
          declaredSha256 is! String ||
          !RegExp(r'^[a-f0-9]{64}$').hasMatch(declaredSha256) ||
          assets.containsKey(path)) {
        throw FormatException('Studio asset row has invalid identity');
      }
      totalBytes += bytes.length;
      if (bytes.isEmpty ||
          bytes.length > 32 * 1024 * 1024 ||
          totalBytes > 128 * 1024 * 1024 ||
          sha256.convert(bytes).toString() != declaredSha256) {
        throw FormatException(
          'Studio asset bytes or SHA-256 are invalid: $path',
        );
      }
      assets[path] = Uint8List.fromList(bytes);
      mediaTypes[path] = mediaType;
    }
    return (
      bytes: Map.unmodifiable(assets),
      mediaTypes: Map.unmodifiable(mediaTypes),
    );
  }

  void _changed() {
    if (mounted) setState(() {});
  }

  void _frameError(FlutterErrorDetails details) {
    final phase = _preparation.phase;
    if (phase == ThemeProbePhase.prepared ||
        phase == ThemeProbePhase.rendered) {
      final revision = _preparation.revision;
      _failedFrameRevision = revision;
      scheduleMicrotask(() {
        if (!mounted || _preparation.revision != revision) return;
        _preparation.failFrame(revision: revision, error: details.exception);
        if (_externalTheme) {
          _studioPort?.sendError(
            code: 'preview-frame-failed',
            message: details.exception.toString(),
            revision: revision,
          );
        }
      });
    }
    _previousErrorHandler?.call(details);
  }

  Future<void> _prepareRevision() => _preparation.prepare(
    revision: _preparation.revision + 1,
    load: () => _prepareResources(
      spec: widget.spec,
      assets: <String, Uint8List>{
        'fixture/background.png': widget.artwork,
        'fixture/bubble.png': widget.artwork,
      },
      assetSha256: const <String, String>{},
      imageReferences: <String>{'fixture/background.png', 'fixture/bubble.png'},
    ),
  );

  Future<ThemeProbeResources> _prepareResources({
    required ThemeVisualSpecV1 spec,
    required Map<String, Uint8List> assets,
    required Map<String, String> assetSha256,
    required Set<String> imageReferences,
  }) {
    final references = spec.fontReferences;
    final globalReference = references['global'];
    if (globalReference == null || globalReference.isEmpty) {
      throw FormatException('Theme typography.global is required');
    }
    final resolver = MemoryThemeAssetResolver(assets);
    final ascii = String.fromCharCodes(
      List<int>.generate(95, (index) => index + 32),
    );
    final fontLoads = <String, Future<PreparedThemeFont>>{};
    Future<PreparedThemeFont> prepare(String reference) =>
        fontLoads.putIfAbsent(
          reference,
          () => _prepareFont(
            reference: reference,
            fixtureText: ThemeSurfaceProbe.glyphCorpus + ascii,
            resolver: resolver,
            assetSha256: assetSha256,
          ),
        );
    final global = prepare(globalReference);
    final userReference = references['user'];
    final aiReference = references['ai'];
    return ThemeProbeResources.prepare(
      sans: global,
      mono: _mono,
      // null is the theme contract's explicit global-font inheritance.
      user: userReference == null ? global : prepare(userReference),
      ai: aiReference == null ? global : prepare(aiReference),
      assets: assets,
      imageReferences: imageReferences,
    );
  }

  Future<PreparedThemeFont> _prepareFont({
    required String reference,
    required String fixtureText,
    required ThemeAssetResolver resolver,
    required Map<String, String> assetSha256,
  }) async {
    validateThemeFontReference(reference);
    if (reference == kNotoSansScFontId || reference == kCascadiaMonoFontId) {
      return widget.builtinFonts.prepare(
        reference,
        registry: _fontRegistry,
        fixtureText: fixtureText,
      );
    }
    final expectedSha256 = assetSha256[reference];
    if (expectedSha256 == null) {
      throw StateError('Theme font bytes are missing: $reference');
    }
    final bytes = await resolver.readBytes(reference);
    ThemeFontGlyphs(bytes).requireText(fixtureText, reference: reference);
    return _fontRegistry.registerBytes(
      reference: reference,
      bytes: bytes,
      expectedSha256: expectedSha256,
    );
  }

  @override
  void dispose() {
    FlutterError.onError = _previousErrorHandler;
    _preparation.removeListener(_changed);
    _preparation.dispose();
    _studioPort?.dispose();
    if (_studioPort != null) closeThemeContractWorker();
    super.dispose();
  }

  void _scheduleReceipt(int revision) {
    if (_scheduledRevision == revision) return;
    _scheduledRevision = revision;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (!mounted ||
          revision != _preparation.revision ||
          _preparation.phase != ThemeProbePhase.prepared ||
          _failedFrameRevision == revision) {
        return;
      }
      final actualViewport = _viewportJson();
      if (_externalTheme &&
          !_viewportMatches(actualViewport, _requestedViewport)) {
        final error = StateError(
          'Flutter frame viewport ${actualViewport['width']}×${actualViewport['height']} '
          'does not match the requested Studio viewport',
        );
        _preparation.failFrame(revision: revision, error: error);
        _studioPort?.sendError(
          code: 'viewport-mismatch',
          message: error.toString(),
          revision: revision,
        );
        return;
      }
      _preparation.markRendered(
        revision: revision,
        fixtureVersion: ThemeClientScene.fixtureVersion,
        fixtureSourceSha256: widget.fixtureSourceSha256,
        themeSha256: _activeThemeSha256,
      );
      if (_externalTheme) {
        _sendCurrentRenderedReceipt(revision);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final resources = _preparation.resources;
    if (_preparation.phase == ThemeProbePhase.failed || resources == null) {
      return MaterialApp(
        debugShowCheckedModeBanner: false,
        home: Scaffold(
          body: Center(
            child: Padding(
              padding: const EdgeInsets.all(24),
              child: _preparation.error == null
                  ? const Text(
                      'Preparing pinned fonts, glyph coverage and images…',
                    )
                  : SelectableText(
                      'Probe preparation failed: ${_preparation.error}',
                    ),
            ),
          ),
        ),
      );
    }
    final revision = _preparation.revision;
    if (_preparation.phase == ThemeProbePhase.prepared) {
      _scheduleReceipt(revision);
    }
    final base = _activeSpec.toSnapshot(ThemePreferenceDefaults.snapshot);
    final snapshot = _externalTheme
        ? base
        : ThemePreferenceSnapshot.fromJson(<String, Object?>{
            ...base.toJson(),
            'transparentSurfaceEnabled': _glass,
            'useBackgroundImage': _background,
            'backgroundImageUri': 'fixture/background.png',
            'useBackgroundBlur': _blur,
            'bubbleUserUseImage': _imageBubble,
            'bubbleUserImageUri': 'fixture/bubble.png',
            'bubbleUserImageRenderMode': _stretch
                ? 'nine_patch'
                : 'tiled_nine_slice',
            'inputStyle': _agentInput ? 'agent' : 'classic',
          });
    final backgroundReference = snapshot.backgroundImageUri;
    return ThemeAssetScope(
      resolver: resources,
      child: ThemeRenderRoot(
        title: 'Operit2 Theme Client Preview',
        themePreferenceSnapshot: snapshot,
        lightColorScheme: _activeSpec.colorScheme(Brightness.light),
        darkColorScheme: _activeSpec.colorScheme(Brightness.dark),
        fontFamily: resources.sans.family,
        fontFamilyFallback: const <String>[],
        themeMode: _dark ? ThemeMode.dark : ThemeMode.light,
        themeAnimationDuration: Duration.zero,
        builder: (context, child) => RepaintBoundary(
          key: _captureKey,
          child: ThemeInspectTarget(
            targetId: ThemeInspectableTargetId.background,
            label: '聊天背景',
            enabled: _studioPort != null && _inspectEnabled,
            selected:
                _selectedInspectTargetId == ThemeInspectableTargetId.background,
            onSelected: _selectInspectTarget,
            fillHighlight: false,
            child: ThemeBackground(
              themePreferenceSnapshot: snapshot,
              media: snapshot.useBackgroundImage && backgroundReference != null
                  ? ThemeBackgroundMedia(
                      key: const ValueKey<String>('probe-background'),
                      opacity: snapshot.backgroundImageOpacity,
                      blurEnabled: snapshot.useBackgroundBlur,
                      blurRadius: snapshot.backgroundBlurRadius,
                      child: ResolvedThemeImage(
                        resolver: resources,
                        reference: backgroundReference,
                        fit: BoxFit.cover,
                      ),
                    )
                  : null,
              child: child!,
            ),
          ),
        ),
        home: _studioPort != null
            ? ThemeClientScene(
                snapshot: snapshot,
                mode: _showConversation
                    ? ThemeClientSceneMode.conversation
                    : ThemeClientSceneMode.empty,
                userFontFamily: resources.user.family,
                aiFontFamily: resources.ai.family,
                inspectEnabled: _inspectEnabled,
                selectedTargetId: _selectedInspectTargetId,
                onInspectTargetSelected: _selectInspectTarget,
              )
            : Scaffold(
                appBar: AppBar(title: const Text('P0 · Shared renderer probe')),
                body: SingleChildScrollView(
                  padding: const EdgeInsets.all(24),
                  child: Center(
                    child: ConstrainedBox(
                      constraints: const BoxConstraints(maxWidth: 880),
                      child: Column(
                        crossAxisAlignment: CrossAxisAlignment.stretch,
                        children: <Widget>[
                          if (_studioPort == null) ...<Widget>[
                            Text(
                              _preparation.receipt == null
                                  ? 'Resources prepared; awaiting this revision frame.'
                                  : 'Surface probe rendered, revision $revision'
                                        '${_externalTheme ? '. External normalized theme rendered.' : '. Fixture theme rendered.'}',
                            ),
                            const SizedBox(height: 12),
                          ],
                          if (_studioPort == null)
                            Wrap(
                              spacing: 8,
                              runSpacing: 8,
                              children: _externalTheme
                                  ? const <Widget>[]
                                  : <Widget>[
                                      _toggle('Dark', _dark, (v) => _dark = v),
                                      _toggle(
                                        'Glass',
                                        _glass,
                                        (v) => _glass = v,
                                      ),
                                      _toggle(
                                        'Background',
                                        _background,
                                        (v) => _background = v,
                                      ),
                                      _toggle('Blur', _blur, (v) => _blur = v),
                                      _toggle(
                                        'Bubble image',
                                        _imageBubble,
                                        (v) => _imageBubble = v,
                                      ),
                                      _toggle(
                                        'Stretch nine-patch',
                                        _stretch,
                                        (v) => _stretch = v,
                                      ),
                                      _toggle(
                                        'Agent input',
                                        _agentInput,
                                        (v) => _agentInput = v,
                                      ),
                                    ],
                            ),
                          const SizedBox(height: 24),
                          ThemeSurfaceProbe(
                            snapshot: snapshot,
                            userFontFamily: resources.user.family,
                            aiFontFamily: resources.ai.family,
                          ),
                          const SizedBox(height: 24),
                          Text(
                            'Operit2 Theme Preview 0123456789 {}[]()',
                            style: TextStyle(fontFamily: resources.mono.family),
                          ),
                          const SizedBox(height: 12),
                          if (_studioPort == null &&
                              _preparation.receipt != null)
                            SelectableText(
                              jsonEncode(_preparation.receipt!.toJson()),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
      ),
    );
  }

  Widget _toggle(String label, bool selected, ValueChanged<bool> update) =>
      FilterChip(
        label: Text(label),
        selected: selected,
        onSelected: (value) {
          update(value);
          unawaited(_prepareRevision());
        },
      );

  Map<String, int> _viewportJson() {
    final captureContext = _captureKey.currentContext;
    if (captureContext == null) {
      throw StateError('Flutter frame boundary is not mounted');
    }
    final size = MediaQuery.sizeOf(captureContext);
    return <String, int>{
      'width': size.width.round(),
      'height': size.height.round(),
    };
  }
}

({int width, int height})? _readViewport(Object? raw) {
  if (raw is! Map) return null;
  final width = raw['width'];
  final height = raw['height'];
  if (width is! int ||
      height is! int ||
      width < 240 ||
      width > 2400 ||
      height < 360 ||
      height > 2400) {
    return null;
  }
  return (width: width, height: height);
}

bool _viewportMatches(
  Map<String, int> actual,
  ({int width, int height})? expected,
) =>
    expected != null &&
    actual['width'] == expected.width &&
    actual['height'] == expected.height;
