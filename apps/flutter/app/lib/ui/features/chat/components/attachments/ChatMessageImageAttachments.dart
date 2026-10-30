// ignore_for_file: file_names

import 'dart:math' as math;
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../../../../common/components/OperitDialog.dart';

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

  String get _mime => type.toLowerCase().split(';').first.trim();

  String? get _mediaKind {
    for (final kind in const ['image', 'audio', 'video']) {
      if (mediaPoolType == kind) return kind;
    }
    for (final kind in const ['image', 'audio', 'video']) {
      if (_mime.startsWith('$kind/')) return kind;
    }
    final extension = filename.split('.').last.toLowerCase();
    if (const {
      'png',
      'jpg',
      'jpeg',
      'webp',
      'gif',
      'bmp',
      'heic',
    }.contains(extension)) {
      return 'image';
    }
    if (const {
      'mp3',
      'wav',
      'm4a',
      'aac',
      'ogg',
      'flac',
      'opus',
    }.contains(extension)) {
      return 'audio';
    }
    if (const {'mp4', 'mov', 'webm', 'mkv', 'avi', 'm4v'}.contains(extension)) {
      return 'video';
    }
    return null;
  }

  bool get isImage => _mediaKind == 'image';
  bool get isAudio => _mediaKind == 'audio';
  bool get isVideo => _mediaKind == 'video';

  /// Fallback media extensions must open the same preview as their thumbnails.
  String get previewMimeType =>
      _mediaKind == null || _mime.startsWith('$_mediaKind/')
      ? (_mediaKind == null ? type : _mime)
      : '$_mediaKind/*';

  /// Inline text with a synthetic identity is context, even when named *.txt.
  /// File paths and media pool references retain their file/media presentation.
  bool get isInlineContext {
    if (isImage || isAudio || isVideo || mediaPoolType != null) return false;
    final inlineType =
        _mime.startsWith('text/') ||
        const {'application/json', 'application/xml'}.contains(_mime) ||
        _mime.endsWith('+json') ||
        _mime.endsWith('+xml') ||
        _mime.endsWith('+plain');
    if (!inlineType || content.trim().isEmpty) return false;
    if (id.startsWith('workspace_mention:') ||
        id.startsWith('package_attach:')) {
      return true;
    }
    // Also handles Windows paths, relative paths, URLs and content/file URIs.
    return !id.contains('/') &&
        !id.contains(r'\') &&
        id.toLowerCase() != filename.toLowerCase();
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
  const ChatMessageImageAttachments({
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

  List<AttachmentData> get _visualAttachments => widget.attachments
      .where((attachment) => !attachment.isInlineContext)
      .toList();

  void _openViewer(AttachmentData attachment) {
    if (!widget.enableDialogs) return;
    showDialog<void>(
      context: context,
      barrierColor: Colors.transparent,
      builder: (dialogContext) => AttachmentViewerDialog(
        visible: true,
        attachment: ChatAttachment(
          id: attachment.id,
          filename: attachment.filename,
          mimeType: attachment.previewMimeType,
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
    if (_visualAttachments.isEmpty) return const SizedBox.shrink();
    return widget.isBubbleMode
        ? _buildBubbleLayout()
        : _buildCommandLineLayout();
  }

  /// Command line mode: normal left-to-right flow.
  Widget _buildCommandLineLayout() {
    return Wrap(
      spacing: _cardSpacing,
      runSpacing: _cardSpacing,
      children: <Widget>[
        for (final attachment in _visualAttachments)
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
    final count = _visualAttachments.length;
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
          for (final attachment in _visualAttachments)
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
    final count = _visualAttachments.length;
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
                  attachment: _visualAttachments[i],
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
    final count = _visualAttachments.length;

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
            for (final attachment in _visualAttachments)
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

/// Supplementary prompt context is message metadata, separate from attachments.
/// Its single quiet entry stays the same size regardless of the plugin count.
class ChatMessageContextMetadata extends StatelessWidget {
  const ChatMessageContextMetadata({
    super.key,
    required this.attachments,
    required this.enableDialogs,
  });

  final List<AttachmentData> attachments;
  final bool enableDialogs;

  @override
  Widget build(BuildContext context) {
    final contexts = attachments
        .where((attachment) => attachment.isInlineContext)
        .toList();
    if (contexts.isEmpty) return const SizedBox.shrink();
    final theme = Theme.of(context);
    return Material(
      color: Colors.transparent,
      child: InkWell(
        borderRadius: BorderRadius.circular(3),
        onTap: enableDialogs ? () => _showDetails(context, contexts) : null,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 2, vertical: 1),
          child: Text(
            contexts.length == 1 ? '补充信息' : '补充信息 · ${contexts.length}',
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            style: theme.textTheme.labelSmall?.copyWith(
              fontSize: 10,
              height: 1.2,
              fontWeight: FontWeight.normal,
              color: theme.colorScheme.onSurfaceVariant.withValues(alpha: 0.5),
            ),
          ),
        ),
      ),
    );
  }

  void _showDetails(BuildContext context, List<AttachmentData> contexts) {
    final content = contexts.length == 1
        ? contexts.single.content
        : contexts
              .map((entry) => '${_contextLabel(entry)}\n${entry.content}')
              .join('\n\n');
    showDialog<void>(
      context: context,
      builder: (dialogContext) => OperitDialogScaffold(
        title: '补充信息',
        maxWidth: 520,
        maxHeight: 360,
        expandContent: false,
        showCloseButton: true,
        actions: <Widget>[
          TextButton.icon(
            onPressed: () => Clipboard.setData(ClipboardData(text: content)),
            icon: const Icon(Icons.copy_outlined, size: 16),
            label: Text(
              MaterialLocalizations.of(dialogContext).copyButtonLabel,
            ),
          ),
        ],
        child: SingleChildScrollView(
          child: contexts.length == 1
              ? SelectableText(
                  content,
                  style: Theme.of(
                    dialogContext,
                  ).textTheme.bodySmall?.copyWith(height: 1.5),
                )
              : Column(
                  mainAxisSize: MainAxisSize.min,
                  children: <Widget>[
                    for (final entry in contexts)
                      ExpansionTile(
                        tilePadding: EdgeInsets.zero,
                        title: Text(
                          _contextLabel(entry),
                          style: Theme.of(dialogContext).textTheme.bodySmall,
                        ),
                        children: <Widget>[
                          Align(
                            alignment: Alignment.centerLeft,
                            child: SelectableText(
                              entry.content,
                              style: Theme.of(
                                dialogContext,
                              ).textTheme.bodySmall?.copyWith(height: 1.5),
                            ),
                          ),
                        ],
                      ),
                  ],
                ),
        ),
      ),
    );
  }
}

String _contextLabel(AttachmentData attachment) {
  if (attachment.id.startsWith('message_insert_extra_bundle_')) return '附加信息';
  if (attachment.id.startsWith('workspace_mention:')) {
    return '@${attachment.filename}';
  }
  if (attachment.id.startsWith('pasted_text')) return '粘贴文本';
  if (attachment.id.startsWith('screen_ocr_') ||
      attachment.filename == 'screen_content.json') {
    return '屏幕内容';
  }
  if (attachment.id.startsWith('notifications_')) return '当前通知';
  if (attachment.id.startsWith('location_')) return '当前位置';
  return attachment.filename;
}
