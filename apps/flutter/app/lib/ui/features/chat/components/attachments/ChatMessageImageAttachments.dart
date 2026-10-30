// ignore_for_file: file_names

import 'dart:math' as math;
import 'package:flutter/material.dart';

import '../style/input/common/ChatAttachmentImagePreview.dart';
import 'AttachmentViewerDialog.dart';

/// Models an attachment parsed from message markup.
class AttachmentData {
  const AttachmentData({
    required this.id,
    required this.filename,
    required this.type,
    this.size = 0,
    this.content = '',
    this.mediaPoolType,
  });

  final String id;
  final String filename;
  final String type;
  final int size;
  final String content;
  final String? mediaPoolType;

  /// Reports whether this attachment is an image.
  bool get isImage {
    if (mediaPoolType == 'image') {
      return true;
    }
    final mime = type.toLowerCase();
    if (mime.startsWith('image/')) {
      return true;
    }
    final ext = filename.toLowerCase();
    return ext.endsWith('.png') ||
        ext.endsWith('.jpg') ||
        ext.endsWith('.jpeg') ||
        ext.endsWith('.webp') ||
        ext.endsWith('.gif') ||
        ext.endsWith('.bmp') ||
        ext.endsWith('.heic');
  }
}

/// Displays attachments (images, audio, video, documents, files) as thumbnails / cards:
/// - In command line mode (Cursor style): renders normal left-to-right flow.
/// - In bubble mode: measures against bubble width; when total unstacked width
///   exceeds bubble width, collapses into a poker-card stack with an expand/collapse toggle.
class ChatMessageAttachments extends StatefulWidget {
  const ChatMessageAttachments({
    super.key,
    required this.attachments,
    required this.enableDialogs,
    this.bubbleWidth,
    this.isBubbleMode = false,
  });

  final List<AttachmentData> attachments;
  final bool enableDialogs;
  final double? bubbleWidth;
  final bool isBubbleMode;

  @override
  State<ChatMessageAttachments> createState() => _ChatMessageAttachmentsState();
}

/// Backwards compatibility subclass for image-specific call sites.
class ChatMessageImageAttachments extends ChatMessageAttachments {
  ChatMessageImageAttachments({
    super.key,
    required List<AttachmentData> images,
    required super.enableDialogs,
    super.bubbleWidth,
    super.isBubbleMode,
  }) : super(attachments: images);
}

class _ChatMessageAttachmentsState extends State<ChatMessageAttachments> {
  static const double _cardSize = 72.0;
  static const double _cardSpacing = 8.0;

  bool _isExpanded = false;

  void _openViewer(AttachmentData attachment) {
    if (!widget.enableDialogs) {
      return;
    }
    showDialog<void>(
      context: context,
      barrierColor: Colors.transparent,
      builder: (dialogContext) => AttachmentViewerDialog(
        visible: true,
        attachment: ChatAttachment(
          id: attachment.id,
          filename: attachment.filename,
          mimeType: attachment.type,
          size: attachment.size,
          content: attachment.content,
          mediaPoolType: attachment.mediaPoolType,
        ),
        onDismiss: () => Navigator.of(dialogContext).pop(),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (widget.attachments.isEmpty) {
      return const SizedBox.shrink();
    }

    if (!widget.isBubbleMode) {
      return _buildCommandLineLayout();
    }

    return _buildBubbleLayout();
  }

  /// Command line mode: normal left-to-right flow.
  Widget _buildCommandLineLayout() {
    return Wrap(
      spacing: _cardSpacing,
      runSpacing: _cardSpacing,
      children: <Widget>[
        for (final attachment in widget.attachments)
          _AttachmentThumbnailCard(
            attachment: attachment,
            size: _cardSize,
            onTap: () => _openViewer(attachment),
          ),
      ],
    );
  }

  /// Bubble mode: poker stack when exceeding bubble width, otherwise flat unstacked.
  Widget _buildBubbleLayout() {
    final count = widget.attachments.length;
    final unstackedWidth = count * _cardSize + (count - 1) * _cardSpacing;
    final measuredWidth = widget.bubbleWidth;
    final exceedsBubble =
        count > 1 && measuredWidth != null && unstackedWidth > measuredWidth;

    if (!exceedsBubble) {
      return Wrap(
        alignment: WrapAlignment.end,
        spacing: _cardSpacing,
        runSpacing: _cardSpacing,
        children: <Widget>[
          for (final attachment in widget.attachments)
            _AttachmentThumbnailCard(
              attachment: attachment,
              size: _cardSize,
              onTap: () => _openViewer(attachment),
            ),
        ],
      );
    }

    return AnimatedSize(
      duration: const Duration(milliseconds: 240),
      curve: Curves.easeOutCubic,
      alignment: Alignment.centerRight,
      child: _isExpanded ? _buildExpandedGrid() : _buildPokerCardStack(),
    );
  }

  /// Poker card stack: layers cards with rotation and offset.
  Widget _buildPokerCardStack() {
    final count = widget.attachments.length;
    final visibleCount = math.min(count, 3);
    const stackWidth = _cardSize + 16.0;
    const stackHeight = _cardSize + 8.0;

    return Tooltip(
      message: '点击展开共 $count 个附件',
      child: GestureDetector(
        behavior: HitTestBehavior.opaque,
        onTap: () => setState(() => _isExpanded = true),
        child: SizedBox(
          width: stackWidth,
          height: stackHeight,
          child: Stack(
            clipBehavior: Clip.none,
            alignment: Alignment.bottomRight,
            children: <Widget>[
              // Render from bottom/back layer to top/front layer
              for (var i = visibleCount - 1; i >= 0; i--)
                _buildStackedCard(
                  layerIndex: i,
                  totalVisible: visibleCount,
                  attachment: widget.attachments[i],
                ),
              // Count badge on top right of the deck
              Positioned(
                top: 0,
                right: 0,
                child: Container(
                  padding:
                      const EdgeInsets.symmetric(horizontal: 6, vertical: 2),
                  decoration: BoxDecoration(
                    color: Colors.black.withValues(alpha: 0.72),
                    borderRadius: BorderRadius.circular(10),
                    boxShadow: const <BoxShadow>[
                      BoxShadow(
                        color: Colors.black26,
                        blurRadius: 4,
                        offset: Offset(0, 1),
                      ),
                    ],
                  ),
                  child: Row(
                    mainAxisSize: MainAxisSize.min,
                    children: <Widget>[
                      const Icon(
                        Icons.style_outlined,
                        size: 11,
                        color: Colors.white,
                      ),
                      const SizedBox(width: 3),
                      Text(
                        '$count',
                        style: const TextStyle(
                          color: Colors.white,
                          fontSize: 10,
                          fontWeight: FontWeight.bold,
                        ),
                      ),
                    ],
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }

  Widget _buildStackedCard({
    required int layerIndex,
    required int totalVisible,
    required AttachmentData attachment,
  }) {
    // layerIndex: 0 is front card, 1 is middle, 2 is back card
    final double dx;
    final double dy;
    final double angle;
    final double scale;

    switch (layerIndex) {
      case 2:
        dx = -14.0;
        dy = -4.0;
        angle = -0.07;
        scale = 0.92;
        break;
      case 1:
        dx = -7.0;
        dy = -2.0;
        angle = -0.035;
        scale = 0.96;
        break;
      default:
        dx = 0.0;
        dy = 0.0;
        angle = 0.0;
        scale = 1.0;
        break;
    }

    return Transform.translate(
      offset: Offset(dx, dy),
      child: Transform.rotate(
        angle: angle,
        child: Transform.scale(
          scale: scale,
          child: _AttachmentThumbnailCard(
            attachment: attachment,
            size: _cardSize,
            interactive: false,
            elevation: layerIndex == 0 ? 4.0 : 2.0,
          ),
        ),
      ),
    );
  }

  /// Expanded grid: displays all thumbnails with a collapse toggle.
  Widget _buildExpandedGrid() {
    final theme = Theme.of(context);
    final count = widget.attachments.length;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      mainAxisSize: MainAxisSize.min,
      children: <Widget>[
        Padding(
          padding: const EdgeInsets.only(bottom: 6),
          child: InkWell(
            borderRadius: BorderRadius.circular(12),
            onTap: () => setState(() => _isExpanded = false),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 3),
              decoration: BoxDecoration(
                color: theme.colorScheme.surfaceContainerHighest
                    .withValues(alpha: 0.7),
                borderRadius: BorderRadius.circular(12),
                border: Border.all(
                  color: theme.colorScheme.outlineVariant.withValues(alpha: 0.5),
                ),
              ),
              child: Row(
                mainAxisSize: MainAxisSize.min,
                children: <Widget>[
                  Icon(
                    Icons.unfold_less_rounded,
                    size: 13,
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                  const SizedBox(width: 4),
                  Text(
                    '收起 ($count)',
                    style: theme.textTheme.labelSmall?.copyWith(
                      color: theme.colorScheme.onSurfaceVariant,
                      fontWeight: FontWeight.w500,
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
        Wrap(
          alignment: WrapAlignment.end,
          spacing: _cardSpacing,
          runSpacing: _cardSpacing,
          children: <Widget>[
            for (final attachment in widget.attachments)
              _AttachmentThumbnailCard(
                attachment: attachment,
                size: _cardSize,
                onTap: () => _openViewer(attachment),
              ),
          ],
        ),
      ],
    );
  }
}

/// Renders a single attachment thumbnail card with rounded corners, subtle shadow, and preview.
class _AttachmentThumbnailCard extends StatelessWidget {
  const _AttachmentThumbnailCard({
    required this.attachment,
    required this.size,
    this.onTap,
    this.interactive = true,
    this.elevation = 2.0,
  });

  final AttachmentData attachment;
  final double size;
  final VoidCallback? onTap;
  final bool interactive;
  final double elevation;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final colorScheme = theme.colorScheme;
    final borderRadius = BorderRadius.circular(10);

    final Widget cardBody;
    if (attachment.isImage) {
      cardBody = ChatAttachmentImagePreview(
        attachmentPath: attachment.id,
        fileName: attachment.filename,
        mediaPoolType: attachment.mediaPoolType,
        fit: BoxFit.cover,
      );
    } else {
      final icon = _getAttachmentIcon(attachment);
      final label = _getAttachmentDisplayLabel(attachment);
      cardBody = Container(
        color: colorScheme.surfaceContainerHighest,
        padding: const EdgeInsets.symmetric(horizontal: 4, vertical: 6),
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          crossAxisAlignment: CrossAxisAlignment.center,
          children: <Widget>[
            Icon(
              icon,
              size: 26,
              color: colorScheme.primary,
            ),
            const SizedBox(height: 4),
            Text(
              label,
              maxLines: 2,
              overflow: TextOverflow.ellipsis,
              textAlign: TextAlign.center,
              style: theme.textTheme.labelSmall?.copyWith(
                fontSize: 9.5,
                height: 1.15,
                fontWeight: FontWeight.w500,
                color: colorScheme.onSurface,
              ),
            ),
          ],
        ),
      );
    }

    return Tooltip(
      message: attachment.filename,
      child: Container(
        width: size,
        height: size,
        decoration: BoxDecoration(
          color: colorScheme.surfaceContainerHighest,
          borderRadius: borderRadius,
          boxShadow: <BoxShadow>[
            BoxShadow(
              color:
                  Colors.black.withValues(alpha: elevation > 2 ? 0.22 : 0.12),
              blurRadius: elevation * 2,
              offset: Offset(0, elevation / 2),
            ),
          ],
          border: Border.all(
            color: colorScheme.outlineVariant.withValues(alpha: 0.6),
            width: 1,
          ),
        ),
        child: ClipRRect(
          borderRadius: borderRadius,
          child: Stack(
            fit: StackFit.expand,
            children: <Widget>[
              cardBody,
              if (interactive && onTap != null)
                Material(
                  color: Colors.transparent,
                  child: InkWell(
                    borderRadius: borderRadius,
                    onTap: onTap,
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

IconData _getAttachmentIcon(AttachmentData attachment) {
  final mime = attachment.type.toLowerCase();
  final ext = attachment.filename.toLowerCase();
  if (mime.startsWith('image/')) {
    return Icons.image_outlined;
  }
  if (mime.startsWith('audio/')) {
    return Icons.audio_file_outlined;
  }
  if (mime.startsWith('video/')) {
    return Icons.video_file_outlined;
  }
  if (attachment.type == 'text/json' &&
      attachment.filename == 'screen_content.json') {
    return Icons.screenshot_monitor_outlined;
  }
  if (ext.endsWith('.pdf')) {
    return Icons.picture_as_pdf_outlined;
  }
  if (ext.endsWith('.dart') ||
      ext.endsWith('.rs') ||
      ext.endsWith('.py') ||
      ext.endsWith('.js') ||
      ext.endsWith('.ts') ||
      ext.endsWith('.json') ||
      ext.endsWith('.html') ||
      ext.endsWith('.css') ||
      ext.endsWith('.cpp') ||
      ext.endsWith('.c') ||
      ext.endsWith('.h') ||
      ext.endsWith('.java') ||
      ext.endsWith('.kt')) {
    return Icons.code_outlined;
  }
  if (ext.endsWith('.zip') ||
      ext.endsWith('.tar') ||
      ext.endsWith('.gz') ||
      ext.endsWith('.7z') ||
      ext.endsWith('.rar')) {
    return Icons.folder_zip_outlined;
  }
  if (ext.endsWith('.txt') || ext.endsWith('.md')) {
    return Icons.article_outlined;
  }
  return Icons.description_outlined;
}

String _getAttachmentDisplayLabel(AttachmentData attachment) {
  if (attachment.type == 'text/json' &&
      attachment.filename == 'screen_content.json') {
    return 'Screen';
  }
  return attachment.filename;
}
