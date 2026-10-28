// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:operit2/core/bridge/PlatformCoreProxy.dart';
import 'package:operit2/core/bridge/ProxyCoreRuntimeBridge.dart';
import 'package:operit2/core/proxy/generated/CoreProxyClients.g.dart';

import '../style/input/common/ChatAttachmentImagePreview.dart';
import '../workspace/file_preview/WorkspaceMediaPreviewWidgets.dart';

const GeneratedCoreProxyClients _attachmentMediaClients =
    GeneratedCoreProxyClients(
      ProxyCoreRuntimeBridge(coreProxy: platformCoreProxy),
    );

class ChatAttachment {
  const ChatAttachment({
    required this.id,
    required this.filename,
    required this.mimeType,
    this.size = 0,
    this.content = '',
    this.mediaPoolType,
  });

  final String id;
  final String filename;
  final String mimeType;
  final int size;
  final String content;
  final String? mediaPoolType;
}

/// An immersive, modern attachment viewer dialog.
/// - For images: full-screen frosted glass lightbox with pinch/pan zoom, double-tap zoom,
///   floating controls, clean title badge, and click-outside/Esc dismiss.
/// - For other media/documents: sleek centered frosted glass card with preview.
class AttachmentViewerDialog extends StatelessWidget {
  const AttachmentViewerDialog({
    super.key,
    required this.visible,
    required this.attachment,
    required this.onDismiss,
  });

  final bool visible;
  final ChatAttachment? attachment;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final attachment = this.attachment;
    if (!visible || attachment == null) {
      return const SizedBox.shrink();
    }

    final isImage = attachment.mimeType.startsWith('image/') ||
        attachment.mediaPoolType == 'image';
    final isTextLike = isTextLikeMimeType(attachment.mimeType);

    final Widget viewerContent = isImage
        ? _ImageLightboxViewer(
            attachment: attachment,
            onDismiss: onDismiss,
          )
        : _GeneralAttachmentViewer(
            attachment: attachment,
            isTextLike: isTextLike,
            onDismiss: onDismiss,
          );

    return Dialog(
      backgroundColor: Colors.transparent,
      elevation: 0,
      insetPadding: EdgeInsets.zero,
      child: Shortcuts(
        shortcuts: <ShortcutActivator, Intent>{
          const SingleActivator(LogicalKeyboardKey.escape):
              const DismissIntent(),
        },
        child: Actions(
          actions: <Type, Action<Intent>>{
            DismissIntent: CallbackAction<DismissIntent>(
              onInvoke: (_) => onDismiss(),
            ),
          },
          child: Focus(
            autofocus: true,
            child: viewerContent,
          ),
        ),
      ),
    );
  }
}

/// Full-screen image lightbox with zoom, pan, double-tap toggle, and frosted controls.
class _ImageLightboxViewer extends StatefulWidget {
  const _ImageLightboxViewer({
    required this.attachment,
    required this.onDismiss,
  });

  final ChatAttachment attachment;
  final VoidCallback onDismiss;

  @override
  State<_ImageLightboxViewer> createState() => _ImageLightboxViewerState();
}

class _ImageLightboxViewerState extends State<_ImageLightboxViewer>
    with SingleTickerProviderStateMixin {
  final TransformationController _transformController =
      TransformationController();
  late AnimationController _animController;
  Animation<Matrix4>? _zoomAnimation;
  TapDownDetails? _doubleTapDetails;
  bool _isZoomed = false;

  @override
  void initState() {
    super.initState();
    _animController = AnimationController(
      vsync: this,
      duration: const Duration(milliseconds: 240),
    )..addListener(() {
        if (_zoomAnimation != null) {
          _transformController.value = _zoomAnimation!.value;
        }
      });
    _transformController.addListener(_onTransformChanged);
  }

  void _onTransformChanged() {
    final scale = _transformController.value.getMaxScaleOnAxis();
    final zoomed = scale > 1.05;
    if (zoomed != _isZoomed) {
      setState(() => _isZoomed = zoomed);
    }
  }

  @override
  void dispose() {
    _transformController.removeListener(_onTransformChanged);
    _transformController.dispose();
    _animController.dispose();
    super.dispose();
  }

  void _handleDoubleTapDown(TapDownDetails details) {
    _doubleTapDetails = details;
  }

  void _handleDoubleTap() {
    final currentScale = _transformController.value.getMaxScaleOnAxis();
    final Matrix4 endMatrix;
    if (currentScale > 1.15) {
      endMatrix = Matrix4.identity();
    } else {
      final position = _doubleTapDetails?.localPosition ?? Offset.zero;
      endMatrix = Matrix4.identity()
        ..translate(-position.dx * 1.5, -position.dy * 1.5)
        ..scale(2.5);
    }

    _zoomAnimation = Matrix4Tween(
      begin: _transformController.value,
      end: endMatrix,
    ).animate(CurvedAnimation(
      parent: _animController,
      curve: Curves.easeOutCubic,
    ));

    _animController.forward(from: 0);
  }

  void _resetZoom() {
    _zoomAnimation = Matrix4Tween(
      begin: _transformController.value,
      end: Matrix4.identity(),
    ).animate(CurvedAnimation(
      parent: _animController,
      curve: Curves.easeOutCubic,
    ));
    _animController.forward(from: 0);
  }

  @override
  Widget build(BuildContext context) {
    final size = MediaQuery.of(context).size;
    final topPadding = MediaQuery.of(context).padding.top;
    final title = _cleanImageTitle(widget.attachment.filename);

    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        // Backdrop with frosted blur - tap to dismiss
        Positioned.fill(
          child: GestureDetector(
            onTap: widget.onDismiss,
            behavior: HitTestBehavior.opaque,
            child: BackdropFilter(
              filter: ImageFilter.blur(sigmaX: 18, sigmaY: 18),
              child: Container(
                color: Colors.black.withValues(alpha: 0.84),
              ),
            ),
          ),
        ),

        // Main image in center with interactive zoom and pan
        Positioned.fill(
          child: Center(
            child: Padding(
              padding: EdgeInsets.fromLTRB(
                20,
                topPadding + 64,
                20,
                28,
              ),
              child: GestureDetector(
                onTap: () {}, // Prevent tap on image from closing lightbox
                onDoubleTapDown: _handleDoubleTapDown,
                onDoubleTap: _handleDoubleTap,
                child: InteractiveViewer(
                  transformationController: _transformController,
                  minScale: 0.5,
                  maxScale: 6.0,
                  clipBehavior: Clip.none,
                  child: Container(
                    constraints: BoxConstraints(
                      maxWidth: size.width * 0.92,
                      maxHeight: size.height * 0.85,
                    ),
                    decoration: BoxDecoration(
                      borderRadius: BorderRadius.circular(10),
                      boxShadow: <BoxShadow>[
                        BoxShadow(
                          color: Colors.black.withValues(alpha: 0.55),
                          blurRadius: 36,
                          spreadRadius: 2,
                          offset: const Offset(0, 10),
                        ),
                      ],
                    ),
                    child: ClipRRect(
                      borderRadius: BorderRadius.circular(10),
                      child: ChatAttachmentImagePreview(
                        attachmentPath: widget.attachment.id,
                        fileName: widget.attachment.filename,
                        mediaPoolType: widget.attachment.mediaPoolType,
                        fit: BoxFit.contain,
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),

        // Floating top control bar
        Positioned(
          top: topPadding + 14,
          left: 18,
          right: 18,
          child: Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: <Widget>[
              // Title badge
              Tooltip(
                message: widget.attachment.filename,
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 14, vertical: 7),
                  decoration: BoxDecoration(
                    color: Colors.white.withValues(alpha: 0.12),
                    borderRadius: BorderRadius.circular(20),
                    border: Border.all(
                      color: Colors.white.withValues(alpha: 0.18),
                      width: 0.8,
                    ),
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      const Icon(
                        Icons.image_outlined,
                        size: 15,
                        color: Colors.white70,
                      ),
                      const SizedBox(width: 8),
                      ConstrainedBox(
                        constraints: const BoxConstraints(maxWidth: 240),
                        child: Text(
                          title,
                          maxLines: 1,
                          overflow: TextOverflow.ellipsis,
                          style: const TextStyle(
                            color: Colors.white,
                            fontSize: 12.5,
                            fontWeight: FontWeight.w500,
                            letterSpacing: 0.2,
                          ),
                        ),
                      ),
                      if (widget.attachment.size > 0) ...<Widget>[
                        const SizedBox(width: 8),
                        Text(
                          _formatFileSize(widget.attachment.size),
                          style: TextStyle(
                            color: Colors.white.withValues(alpha: 0.6),
                            fontSize: 11,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
              ),

              // Action buttons
              Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  if (_isZoomed)
                    Padding(
                      padding: const EdgeInsets.only(right: 10),
                      child: _FrostedCircleButton(
                        icon: Icons.zoom_out_map_rounded,
                        tooltip: '重置缩放',
                        onTap: _resetZoom,
                      ),
                    ),
                  _FrostedCircleButton(
                    icon: Icons.close_rounded,
                    tooltip: '关闭 (Esc)',
                    onTap: widget.onDismiss,
                  ),
                ],
              ),
            ],
          ),
        ),
      ],
    );
  }
}

/// Frosted translucent circular button for lightbox actions.
class _FrostedCircleButton extends StatefulWidget {
  const _FrostedCircleButton({
    required this.icon,
    required this.tooltip,
    required this.onTap,
  });

  final IconData icon;
  final String tooltip;
  final VoidCallback onTap;

  @override
  State<_FrostedCircleButton> createState() => _FrostedCircleButtonState();
}

class _FrostedCircleButtonState extends State<_FrostedCircleButton> {
  bool _hovered = false;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: widget.tooltip,
      child: MouseRegion(
        onEnter: (_) => setState(() => _hovered = true),
        onExit: (_) => setState(() => _hovered = false),
        cursor: SystemMouseCursors.click,
        child: GestureDetector(
          onTap: widget.onTap,
          child: AnimatedContainer(
            duration: const Duration(milliseconds: 160),
            width: 36,
            height: 36,
            decoration: BoxDecoration(
              shape: BoxShape.circle,
              color: _hovered
                  ? Colors.white.withValues(alpha: 0.25)
                  : Colors.white.withValues(alpha: 0.14),
              border: Border.all(
                color: _hovered
                    ? Colors.white.withValues(alpha: 0.35)
                    : Colors.white.withValues(alpha: 0.18),
                width: 0.8,
              ),
              boxShadow: const <BoxShadow>[
                BoxShadow(
                  color: Colors.black26,
                  blurRadius: 8,
                  offset: Offset(0, 2),
                ),
              ],
            ),
            child: Icon(
              widget.icon,
              color: Colors.white,
              size: 19,
            ),
          ),
        ),
      ),
    );
  }
}

/// Modern preview card for non-image attachments (code, text, audio, video).
class _GeneralAttachmentViewer extends StatelessWidget {
  const _GeneralAttachmentViewer({
    required this.attachment,
    required this.isTextLike,
    required this.onDismiss,
  });

  final ChatAttachment attachment;
  final bool isTextLike;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final isAudio = attachment.mimeType.startsWith('audio/');
    final isVideo = attachment.mimeType.startsWith('video/');

    return Stack(
      fit: StackFit.expand,
      children: <Widget>[
        // Backdrop blur
        Positioned.fill(
          child: GestureDetector(
            onTap: onDismiss,
            behavior: HitTestBehavior.opaque,
            child: BackdropFilter(
              filter: ImageFilter.blur(sigmaX: 18, sigmaY: 18),
              child: Container(
                color: Colors.black.withValues(alpha: 0.72),
              ),
            ),
          ),
        ),

        // Centered modal card
        Center(
          child: Container(
            constraints: const BoxConstraints(
              maxWidth: 680,
              maxHeight: 540,
            ),
            margin: const EdgeInsets.symmetric(horizontal: 24, vertical: 24),
            decoration: BoxDecoration(
              color: colorScheme.surface,
              borderRadius: BorderRadius.circular(18),
              border: Border.all(
                color: colorScheme.outlineVariant.withValues(alpha: 0.4),
                width: 1,
              ),
              boxShadow: <BoxShadow>[
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.38),
                  blurRadius: 32,
                  spreadRadius: 2,
                  offset: const Offset(0, 8),
                ),
              ],
            ),
            child: ClipRRect(
              borderRadius: BorderRadius.circular(18),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: <Widget>[
                  // Modern Header
                  Padding(
                    padding: const EdgeInsets.fromLTRB(18, 14, 12, 12),
                    child: Row(
                      children: <Widget>[
                        Container(
                          padding: const EdgeInsets.all(7),
                          decoration: BoxDecoration(
                            color: colorScheme.primaryContainer
                                .withValues(alpha: 0.7),
                            borderRadius: BorderRadius.circular(10),
                          ),
                          child: Icon(
                            _attachmentIcon(
                              isImage: false,
                              isAudio: isAudio,
                              isVideo: isVideo,
                            ),
                            size: 18,
                            color: colorScheme.primary,
                          ),
                        ),
                        const SizedBox(width: 12),
                        Expanded(
                          child: Column(
                            crossAxisAlignment: CrossAxisAlignment.start,
                            mainAxisSize: MainAxisSize.min,
                            children: <Widget>[
                              Text(
                                attachment.filename,
                                maxLines: 1,
                                overflow: TextOverflow.ellipsis,
                                style: theme.textTheme.titleMedium?.copyWith(
                                  fontWeight: FontWeight.w600,
                                ),
                              ),
                              if (attachment.size > 0 ||
                                  attachment.mimeType.isNotEmpty)
                                Text(
                                  [
                                    if (attachment.size > 0)
                                      _formatFileSize(attachment.size),
                                    attachment.mimeType,
                                  ].join(' • '),
                                  style: theme.textTheme.bodySmall?.copyWith(
                                    color: colorScheme.onSurfaceVariant,
                                    fontSize: 11,
                                  ),
                                ),
                            ],
                          ),
                        ),
                        if (isTextLike && attachment.content.isNotEmpty)
                          IconButton(
                            icon: const Icon(Icons.copy_rounded, size: 18),
                            tooltip: '复制内容',
                            onPressed: () {
                              Clipboard.setData(
                                ClipboardData(text: attachment.content),
                              );
                              ScaffoldMessenger.of(context).showSnackBar(
                                const SnackBar(
                                  content: Text('已复制到剪贴板'),
                                  duration: Duration(seconds: 1),
                                ),
                              );
                            },
                          ),
                        IconButton(
                          icon: const Icon(Icons.close_rounded, size: 20),
                          tooltip: '关闭',
                          onPressed: onDismiss,
                        ),
                      ],
                    ),
                  ),
                  Divider(
                    height: 1,
                    thickness: 1,
                    color: colorScheme.outlineVariant.withValues(alpha: 0.35),
                  ),
                  // Body content
                  Flexible(
                    child: SingleChildScrollView(
                      padding: const EdgeInsets.all(18),
                      child: _buildGeneralContent(theme, colorScheme),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ],
    );
  }

  Widget _buildGeneralContent(ThemeData theme, ColorScheme colorScheme) {
    if (attachment.mimeType.startsWith('audio/') ||
        attachment.mimeType.startsWith('video/')) {
      return _MediaPoolAttachmentPreview(attachment: attachment);
    }

    if (isTextLike || attachment.content.isNotEmpty) {
      return Container(
        width: double.infinity,
        padding: const EdgeInsets.all(14),
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest.withValues(alpha: 0.5),
          borderRadius: BorderRadius.circular(10),
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.3),
          ),
        ),
        child: SelectableText(
          attachment.content,
          style: theme.textTheme.bodySmall?.copyWith(
            color: colorScheme.onSurface,
            fontFamily: 'monospace',
            height: 1.5,
          ),
        ),
      );
    }

    return Center(
      child: Padding(
        padding: const EdgeInsets.symmetric(vertical: 24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: <Widget>[
            Icon(
              Icons.insert_drive_file_outlined,
              size: 48,
              color: colorScheme.primary.withValues(alpha: 0.7),
            ),
            const SizedBox(height: 12),
            Text(
              attachment.filename,
              style: theme.textTheme.titleSmall?.copyWith(
                fontWeight: FontWeight.w600,
              ),
            ),
            const SizedBox(height: 4),
            Text(
              '${attachment.mimeType} (${_formatFileSize(attachment.size)})',
              style: theme.textTheme.bodySmall?.copyWith(
                color: colorScheme.onSurfaceVariant,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

/// Loads audio or video bytes from a media pool id or an ordinary attachment path.
class _MediaPoolAttachmentPreview extends StatelessWidget {
  const _MediaPoolAttachmentPreview({required this.attachment});

  final ChatAttachment attachment;

  @override
  Widget build(BuildContext context) {
    return FutureBuilder<Uint8List>(
      future: _readAttachmentMediaBytes(attachment),
      builder: (context, snapshot) {
        if (snapshot.hasError) {
          return Text(
            snapshot.error.toString(),
            style: Theme.of(context).textTheme.bodySmall,
          );
        }
        if (!snapshot.hasData) {
          return const Center(child: CircularProgressIndicator());
        }
        final bytes = snapshot.requireData;
        if (attachment.mimeType.startsWith('audio/')) {
          return WorkspaceAudioPreview(
            bytes: bytes,
            title: attachment.filename,
          );
        }
        return WorkspaceVideoPreview(
          bytes: bytes,
          fileName: attachment.filename,
        );
      },
    );
  }
}

/// Reads attachment media bytes through the runtime pool bridge or file host.
Future<Uint8List> _readAttachmentMediaBytes(ChatAttachment attachment) async {
  if (attachment.mediaPoolType != null) {
    final data = await _attachmentMediaClients
        .servicesRuntimeHostInteractionService
        .getMediaPoolData(
          mediaType: attachment.mediaPoolType!,
          id: attachment.id,
        );
    return base64Decode(data.base64);
  }
  return readChatAttachmentFileBytes(attachment.id);
}

bool isTextLikeMimeType(String mimeType) {
  return mimeType.startsWith('text/') ||
      mimeType == 'application/json' ||
      mimeType == 'application/xml';
}

IconData _attachmentIcon({
  required bool isImage,
  required bool isAudio,
  required bool isVideo,
}) {
  if (isImage) {
    return Icons.image_outlined;
  }
  if (isAudio) {
    return Icons.volume_up_outlined;
  }
  if (isVideo) {
    return Icons.play_arrow_outlined;
  }
  return Icons.description_outlined;
}

String _cleanImageTitle(String filename) {
  if (filename.startsWith('pasted_image_')) {
    return '剪贴板图片';
  }
  return filename;
}

String _formatFileSize(int bytes) {
  if (bytes <= 0) {
    return '';
  }
  if (bytes < 1024) {
    return '$bytes B';
  }
  if (bytes < 1024 * 1024) {
    return '${(bytes / 1024).toStringAsFixed(1)} KB';
  }
  return '${(bytes / (1024 * 1024)).toStringAsFixed(1)} MB';
}
