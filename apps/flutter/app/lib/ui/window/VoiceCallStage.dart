// ignore_for_file: file_names
import 'dart:math' as math;
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:operit_ui/operit_ui.dart' show ResolvedThemeImage;

import '../../core/bridge/ProxyCoreRuntimeBridge.dart';
import '../../core/host/VoiceCallCamera.dart';
import '../../core/proxy/generated/CoreProxyClients.g.dart';
import '../../data/preferences/UserPreferencesManager.dart';
import '../features/packages/screens/ComposeDslImageSource.dart';
import '../theme/OperitThemeAssets.dart';

class VoiceCallStage extends StatefulWidget {
  const VoiceCallStage({
    super.key,
    this.camera,
    this.avatarUri,
    this.avatarTitle,
    this.appearance,
    required this.controls,
  });

  final VoiceCallCamera? camera;
  final String? avatarUri;
  final String? avatarTitle;
  final ThemePreferenceSnapshot? appearance;
  final Widget controls;

  @override
  State<VoiceCallStage> createState() => _VoiceCallStageState();
}

class _VoiceCallStageState extends State<VoiceCallStage> {
  static const _clients = GeneratedCoreProxyClients(ProxyCoreRuntimeBridge());
  final _themeAssets = ThemeAssetStore();
  Future<ImageProvider>? _avatar;
  Widget? _cameraTexture;
  bool _avatarIsMain = false;

  @override
  void initState() {
    super.initState();
    widget.camera?.addListener(_cameraChanged);
    _resolveAvatar();
  }

  @override
  void didUpdateWidget(covariant VoiceCallStage oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.camera != widget.camera) {
      oldWidget.camera?.removeListener(_cameraChanged);
      widget.camera?.addListener(_cameraChanged);
      _cameraTexture = null;
      _avatarIsMain = false;
    }
    if (oldWidget.avatarUri != widget.avatarUri) _resolveAvatar();
  }

  void _resolveAvatar() {
    final uri = widget.avatarUri;
    _avatar = uri == null || uri.isEmpty
        ? null
        : resolveComposeDslImage(
            uri,
            vfsOwner: this,
            readVfsBytes: (path) => readComposeDslVfsBytes(_clients, path),
          );
  }

  void _cameraChanged() {
    setState(() {
      if (widget.camera?.preview == null) {
        _cameraTexture = null;
        _avatarIsMain = false;
      }
    });
  }

  @override
  void dispose() {
    widget.camera?.removeListener(_cameraChanged);
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colors = Theme.of(context).colorScheme;
    final zh = Localizations.localeOf(context).languageCode == 'zh';
    final camera = widget.camera;
    final preview = camera?.preview;
    final texture = preview == null
        ? null
        : _cameraTexture ??= camera!.platform.buildPreview(preview.cameraId);
    final cameraIsMain = preview != null && !_avatarIsMain;
    final swapLabel = zh ? '交换大小窗' : 'Swap main view and inset';
    return ClipRRect(
      borderRadius: BorderRadius.circular(28),
      child: ColoredBox(
        color: colors.surface,
        child: FutureBuilder<ImageProvider>(
          future: _avatar,
          builder: (context, snapshot) {
            final avatar = snapshot.connectionState == ConnectionState.done
                ? snapshot.data
                : null;
            return LayoutBuilder(
              builder: (context, constraints) {
                final size = constraints.biggest;
                final shortSide = size.shortestSide;
                final portraitSize = math.min(shortSide * 0.40, 152.0);
                final insetWidth = math.min(shortSide * 0.29, 140.0);
                final margin = math.min(shortSide * 0.05, 16.0);
                final full = Offset.zero & size;
                final inset = Rect.fromLTWH(
                  size.width - insetWidth - margin,
                  margin,
                  insetWidth,
                  math.min(insetWidth * 1.25, size.height * 0.38),
                );
                final portraitCamera = _portraitCamera(context);
                final cameraButtonHeight = camera == null ? 0.0 : 48.0;
                final controlsHeight =
                    (size.height / 2 -
                            portraitSize / 2 -
                            cameraButtonHeight -
                            margin * 2)
                        .clamp(math.min(80.0, size.height * 0.28), 128.0)
                        .toDouble();
                return Stack(
                  fit: StackFit.expand,
                  children: [
                    if (!cameraIsMain) _buildBackdrop(colors, avatar),
                    if (preview != null)
                      Positioned.fromRect(
                        key: const ValueKey('call-camera-view'),
                        rect: cameraIsMain ? full : inset,
                        child: ClipRRect(
                          borderRadius: BorderRadius.circular(
                            cameraIsMain ? 0 : 18,
                          ),
                          child: FittedBox(
                            fit: BoxFit.cover,
                            child: SizedBox(
                              width: portraitCamera
                                  ? preview.previewHeight
                                  : preview.previewWidth,
                              height: portraitCamera
                                  ? preview.previewWidth
                                  : preview.previewHeight,
                              child: texture,
                            ),
                          ),
                        ),
                      ),
                    Positioned.fromRect(
                      key: const ValueKey('call-avatar-view'),
                      rect: cameraIsMain ? inset : full,
                      child: IgnorePointer(
                        child: _buildAvatar(
                          colors,
                          avatar,
                          cameraIsMain
                              ? inset.size.shortestSide * 0.66
                              : portraitSize,
                          small: cameraIsMain,
                        ),
                      ),
                    ),
                    if (preview != null)
                      Positioned.fromRect(
                        rect: inset,
                        child: Tooltip(
                          message: swapLabel,
                          excludeFromSemantics: true,
                          child: Semantics(
                            label: swapLabel,
                            button: true,
                            child: Material(
                              shape: RoundedRectangleBorder(
                                borderRadius: BorderRadius.circular(18),
                                side: const BorderSide(color: Colors.white54),
                              ),
                              clipBehavior: Clip.antiAlias,
                              color: Colors.transparent,
                              child: InkWell(
                                key: const ValueKey('call-swap-views'),
                                onTap: () => setState(
                                  () => _avatarIsMain = !_avatarIsMain,
                                ),
                              ),
                            ),
                          ),
                        ),
                      ),
                    Positioned(
                      left: margin,
                      right: margin,
                      bottom: margin,
                      child: Center(
                        child: ConstrainedBox(
                          constraints: const BoxConstraints(maxWidth: 440),
                          child: Material(
                            color: colors.surface.withValues(alpha: 0.94),
                            borderRadius: BorderRadius.circular(22),
                            child: Padding(
                              padding: const EdgeInsets.symmetric(
                                horizontal: 10,
                                vertical: 4,
                              ),
                              child: Column(
                                mainAxisSize: MainAxisSize.min,
                                children: [
                                  SizedBox(
                                    height: controlsHeight,
                                    child: widget.controls,
                                  ),
                                  if (camera != null)
                                    SizedBox(
                                      height: cameraButtonHeight,
                                      child: _buildCameraControl(camera, zh),
                                    ),
                                ],
                              ),
                            ),
                          ),
                        ),
                      ),
                    ),
                  ],
                );
              },
            );
          },
        ),
      ),
    );
  }

  bool _portraitCamera(BuildContext context) =>
      voiceCallCameraNeedsActivity &&
      MediaQuery.orientationOf(context) == Orientation.portrait;

  Widget _buildBackdrop(ColorScheme colors, ImageProvider? avatar) {
    final appearance = widget.appearance;
    final background =
        appearance?.useBackgroundImage == true &&
            appearance?.backgroundMediaType ==
                UserPreferencesManager.MEDIA_TYPE_IMAGE
        ? appearance?.backgroundImageUri
        : null;
    return Stack(
      fit: StackFit.expand,
      children: [
        DecoratedBox(
          decoration: BoxDecoration(
            gradient: LinearGradient(
              begin: Alignment.topLeft,
              end: Alignment.bottomRight,
              colors: [colors.primaryContainer, colors.surface],
            ),
          ),
        ),
        if (avatar != null)
          ImageFiltered(
            imageFilter: ui.ImageFilter.blur(
              sigmaX: 28,
              sigmaY: 28,
              tileMode: TileMode.clamp,
            ),
            child: Image(
              image: avatar,
              fit: BoxFit.cover,
              errorBuilder: _emptyImage,
            ),
          ),
        if (background != null && background.isNotEmpty)
          ResolvedThemeImage(
            resolver: _themeAssets,
            reference: background,
            fit: BoxFit.cover,
          ),
        if (avatar != null || background != null)
          const ColoredBox(color: Color(0x66000000)),
      ],
    );
  }

  Widget _buildAvatar(
    ColorScheme colors,
    ImageProvider? avatar,
    double size, {
    required bool small,
  }) {
    final title = widget.avatarTitle;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: small ? colors.surfaceContainerHigh : null,
        borderRadius: small ? BorderRadius.circular(18) : null,
      ),
      child: Center(
        child: Stack(
          alignment: Alignment.center,
          clipBehavior: Clip.none,
          children: [
            Container(
              width: size,
              height: size,
              decoration: BoxDecoration(
                shape: BoxShape.circle,
                color: colors.primaryContainer,
                border: Border.all(color: Colors.white54, width: 2),
                boxShadow: const [
                  BoxShadow(color: Colors.black38, blurRadius: 20),
                ],
              ),
              child: ClipOval(
                child: Stack(
                  fit: StackFit.expand,
                  children: [
                    Icon(
                      Icons.smart_toy_outlined,
                      size: size * 0.45,
                      color: colors.onPrimaryContainer,
                    ),
                    if (avatar != null)
                      Image(
                        image: avatar,
                        fit: BoxFit.cover,
                        semanticLabel: title,
                        errorBuilder: _emptyImage,
                      ),
                  ],
                ),
              ),
            ),
            if (!small && title != null && title.isNotEmpty)
              Positioned(
                top: -32,
                child: ConstrainedBox(
                  constraints: const BoxConstraints(maxWidth: 200),
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: const Color(0x88000000),
                      borderRadius: BorderRadius.circular(14),
                    ),
                    child: Padding(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 10,
                        vertical: 3,
                      ),
                      child: Text(
                        title,
                        maxLines: 1,
                        overflow: TextOverflow.ellipsis,
                        style: const TextStyle(color: Colors.white),
                      ),
                    ),
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildCameraControl(VoiceCallCamera camera, bool zh) {
    final label = camera.enabled
        ? (zh ? '关闭摄像头' : 'Turn camera off')
        : (zh ? '开启摄像头' : 'Turn camera on');
    final message = camera.error != null
        ? (zh
              ? '摄像头不可用，请检查设备和权限'
              : 'Camera unavailable. Check the device and permissions.')
        : camera.preview != null
        ? (zh ? '每轮发言共享一张画面' : 'Shares one frame per spoken turn')
        : camera.enabled
        ? (zh ? '正在打开摄像头…' : 'Opening camera…')
        : null;
    return Row(
      mainAxisAlignment: MainAxisAlignment.center,
      children: [
        IconButton.filledTonal(
          tooltip: label,
          isSelected: camera.enabled,
          icon: const Icon(Icons.videocam_off_outlined),
          selectedIcon: const Icon(Icons.videocam_outlined),
          onPressed: camera.enabled ? camera.stop : camera.start,
        ),
        if (message != null)
          Flexible(
            child: Padding(
              padding: const EdgeInsets.only(left: 8),
              child: Text(
                message,
                maxLines: 2,
                overflow: TextOverflow.ellipsis,
                style: Theme.of(context).textTheme.bodySmall,
              ),
            ),
          ),
      ],
    );
  }

  static Widget _emptyImage(
    BuildContext context,
    Object error,
    StackTrace? stack,
  ) => const SizedBox.shrink();
}
