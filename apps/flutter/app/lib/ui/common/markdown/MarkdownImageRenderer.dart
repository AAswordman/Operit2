// ignore_for_file: file_names

import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';

import 'MarkdownAudioRenderer.dart';
import 'MarkdownVideoRenderer.dart';

class MarkdownImageRenderer extends StatelessWidget {
  const MarkdownImageRenderer({
    super.key,
    required this.imageMarkdown,
    required this.textColor,
    this.maxImageHeight = 140,
  });

  final String imageMarkdown;
  final Color textColor;
  final double maxImageHeight;

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
      padding: const EdgeInsets.symmetric(vertical: 1),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(8),
        child: ConstrainedBox(
          constraints: BoxConstraints(maxHeight: maxImageHeight),
          child: _MarkdownImageBody(imageUrl: imageUrl, imageAlt: imageAlt),
        ),
      ),
    );
  }
}

class _MarkdownImageBody extends StatelessWidget {
  const _MarkdownImageBody({required this.imageUrl, required this.imageAlt});

  final String imageUrl;
  final String imageAlt;

  @override
  Widget build(BuildContext context) {
    final dataBytes = _dataUriBytes(imageUrl);
    if (dataBytes != null) {
      return Image.memory(dataBytes, fit: BoxFit.contain);
    }
    return Image.network(
      imageUrl,
      fit: BoxFit.contain,
      semanticLabel: imageAlt.isEmpty ? null : imageAlt,
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
  return RegExp(r'\]\(([^)]+)\)$').firstMatch(imageContent.trim())?.group(1) ??
      '';
}

Uint8List? _dataUriBytes(String imageUrl) {
  final match = RegExp(
    r'^data:image/[^;]+;base64,(.+)$',
    caseSensitive: false,
  ).firstMatch(imageUrl);
  if (match == null) {
    return null;
  }
  return base64Decode(match.group(1)!);
}
