// ignore_for_file: file_names

import 'dart:math' as math;

import 'package:flutter/material.dart';

import '../../../../l10n/generated/app_localizations.dart';
import '../viewmodel/ChatViewModel.dart';

String senderLabel(BuildContext context, String sender) {
  final l10n = AppLocalizations.of(context)!;
  switch (sender) {
    case 'user':
      return l10n.messageSenderUser;
    case 'ai':
      return 'AI';
    case 'summary':
      return l10n.messageSenderSummary;
    case 'system':
      return l10n.messageSenderSystem;
    case 'think':
      return l10n.messageSenderThinking;
    default:
      return l10n.messageSenderOther;
  }
}

String visibleLocatorContent(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.sender == 'user' && preview.displayMode == 'HIDDEN_PLACEHOLDER') {
    return l10n.hiddenUserMessage;
  }
  return preview.previewContent;
}

int messageContentLength(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
) {
  if (preview.contentLength > 0) {
    return preview.contentLength;
  }
  return math.max(visibleLocatorContent(l10n, preview).length, 1);
}

String buildMessagePreview(
  AppLocalizations l10n,
  ChatMessageLocatorPreview preview,
  String searchQuery,
) {
  final content = normalizeMessageSearchText(
    visibleLocatorContent(l10n, preview),
  );
  if (content.isEmpty) {
    return preview.sender;
  }
  final normalizedSearchQuery = normalizeMessageSearchText(searchQuery);
  if (normalizedSearchQuery.isNotEmpty) {
    final matchIndex = content.toLowerCase().indexOf(
      normalizedSearchQuery.toLowerCase(),
    );
    if (matchIndex >= 0) {
      const previewLength = 72;
      final preferredStart = math.max(matchIndex - 18, 0);
      final start = math.min(
        preferredStart,
        math.max(content.length - previewLength, 0),
      );
      final end = math.min(start + previewLength, content.length);
      final prefix = start > 0 ? '...' : '';
      final suffix = end < content.length ? '...' : '';
      final snippet = content.substring(start, end).trim();
      if (snippet.isNotEmpty) {
        return '$prefix$snippet$suffix';
      }
    }
  }
  final previewText = content.length > 72 ? content.substring(0, 72) : content;
  return previewText.length < content.length
      ? '${previewText.trimRight()}...'
      : previewText;
}

String normalizeMessageSearchText(String text) {
  if (text.isEmpty) {
    return '';
  }
  final buffer = StringBuffer();
  var pendingWhitespace = false;
  for (final codePoint in text.runes) {
    final char = String.fromCharCode(codePoint);
    final normalizedChar = char == '\n' || char == '\r' || char == '\t'
        ? ' '
        : char;
    if (normalizedChar.trim().isEmpty) {
      pendingWhitespace = buffer.isNotEmpty;
      continue;
    }
    if (pendingWhitespace) {
      buffer.write(' ');
      pendingWhitespace = false;
    }
    buffer.write(normalizedChar);
  }
  return buffer.toString().trim();
}
