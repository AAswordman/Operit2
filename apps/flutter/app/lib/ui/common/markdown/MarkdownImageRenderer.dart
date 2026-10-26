// ignore_for_file: file_names

import 'package:flutter/material.dart';

import 'MarkdownAudioRenderer.dart';
import 'MarkdownRemoteImage.dart';
import 'MarkdownVideoRenderer.dart';

class MarkdownImageRenderer extends StatelessWidget {
  const MarkdownImageRenderer({
    super.key,
    required this.imageMarkdown,
    required this.textColor,
    this.maxImageHeight = 140,
    this.onOpenLink,
    this.enablePreview = true,
  });

  final String imageMarkdown;
  final Color textColor;
  final double maxImageHeight;
  final bool enablePreview;
  final void Function(String url)? onOpenLink;

  @override
  Widget build(BuildContext context) {
    if (!isCompleteImageMarkdown(imageMarkdown)) {
      return SelectableText(
        imageMarkdown,
        style: Theme.of(
          context,
        ).textTheme.bodyMedium?.copyWith(color: textColor, height: 1.3),
      );
    }
    final imageAlt = extractMarkdownImageAlt(imageMarkdown);
    final imageUrl = extractMarkdownImageUrl(imageMarkdown);
    if (isLikelyVideoUrl(imageUrl)) {
      return MarkdownVideoRenderer(
        videoMarkdown: imageMarkdown,
        textColor: textColor,
        maxVideoHeight: maxImageHeight,
      );
    }
    if (isLikelyAudioUrl(imageUrl)) {
      return MarkdownAudioRenderer(
        audioMarkdown: imageMarkdown,
        textColor: textColor,
      );
    }
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          Center(
            child: ClipRRect(
              borderRadius: BorderRadius.circular(8),
              child: ConstrainedBox(
                constraints: BoxConstraints(maxHeight: maxImageHeight),
                child: MarkdownRemoteImage(
                  url: imageUrl,
                  alt: imageAlt,
                  enablePreview: enablePreview,
                ),
              ),
            ),
          ),
          if (imageAlt.trim().isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(top: 4),
              child: Text(
                imageAlt.trim(),
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  color: textColor.withValues(alpha: 0.7),
                ),
              ),
            ),
        ],
      ),
    );
  }
}

bool isCompleteImageMarkdown(String content) {
  return RegExp(r'^!\[[^\]]*\]\([^)]+\)$').hasMatch(content.trim());
}

String extractMarkdownImageAlt(String imageContent) {
  return RegExp(r'^!\[([^\]]*)\]').firstMatch(imageContent.trim())?.group(1) ??
      '';
}

String extractMarkdownImageUrl(String imageContent) {
  final destination =
      RegExp(r'\]\(([^)]+)\)$').firstMatch(imageContent.trim())?.group(1) ?? '';
  return destination
      .replaceFirst(RegExp(r"""\s+(?:"[^"]*"|'[^']*'|\([^)]*\))\s*$"""), '')
      .trim();
}

/// Whether the entire message is one still/animated image, not audio or video.
bool isStandaloneMarkdownImage(String content) {
  if (!isCompleteImageMarkdown(content)) return false;
  final url = extractMarkdownImageUrl(content);
  return url.isNotEmpty && !isLikelyVideoUrl(url) && !isLikelyAudioUrl(url);
}
