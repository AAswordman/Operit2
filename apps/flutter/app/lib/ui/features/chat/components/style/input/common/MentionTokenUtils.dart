// ignore_for_file: file_names

import 'package:flutter/services.dart';

/// Describes one mention token discovered in the chat input.
class MentionTokenRange {
  /// Creates a mention token range with content and token boundaries.
  const MentionTokenRange({
    required this.triggerChar,
    required this.start,
    required this.contentEndExclusive,
    required this.endExclusive,
  });

  final String triggerChar;
  final int start;
  final int contentEndExclusive;
  final int endExclusive;

  /// Reports whether this token already includes closing whitespace.
  bool get hasTrailingWhitespace => endExclusive > contentEndExclusive;
}

/// Describes the active mention trigger at the current caret.
class ActiveMentionTrigger {
  /// Creates an active mention trigger with its query text.
  const ActiveMentionTrigger({
    required this.triggerChar,
    required this.triggerIndex,
    required this.query,
  });

  final String triggerChar;
  final int triggerIndex;
  final String query;
}

/// Reports whether a character can continue a mention token.
bool isMentionContinuation(String char, [String triggerChar = '@']) {
  return switch (triggerChar) {
    '@' => _isBaseMentionContinuation(char) || char == '/' || char == r'\',
    '/' => _isBaseMentionContinuation(char),
    _ => _isBaseMentionContinuation(char),
  };
}

/// Reports whether a single character is whitespace for mention parsing.
bool isMentionWhitespace(String char) {
  return _isWhitespace(char);
}

/// Finds all mention tokens in the supplied text.
List<MentionTokenRange> findMentionTokens(String text) {
  final tokens = <MentionTokenRange>[];
  var index = 0;
  while (index < text.length) {
    final triggerChar = text.substring(index, index + 1);
    if (!_isMentionTriggerChar(triggerChar) ||
        !_isValidMentionTrigger(text, index, triggerChar)) {
      index += 1;
      continue;
    }

    var contentEnd = index + 1;
    while (contentEnd < text.length &&
        isMentionContinuation(
          text.substring(contentEnd, contentEnd + 1),
          triggerChar,
        )) {
      contentEnd += 1;
    }
    if (contentEnd == index + 1) {
      index += 1;
      continue;
    }

    final endExclusive =
        contentEnd < text.length &&
            _isWhitespace(text.substring(contentEnd, contentEnd + 1))
        ? contentEnd + 1
        : contentEnd;
    tokens.add(
      MentionTokenRange(
        triggerChar: triggerChar,
        start: index,
        contentEndExclusive: contentEnd,
        endExclusive: endExclusive,
      ),
    );
    index = contentEnd;
  }
  return tokens;
}

/// Finds mention tokens that have been committed by trailing whitespace.
List<MentionTokenRange> findCommittedMentionTokens(String text) {
  return findMentionTokens(
    text,
  ).where((token) => token.hasTrailingWhitespace).toList(growable: false);
}

/// Finds a mention token whose content or trailing whitespace ends at the caret.
MentionTokenRange? findMentionTokenEndingAtCursor(String text, int cursor) {
  final safeCursor = _clampedOffset(cursor, text.length);
  for (final token in findMentionTokens(text)) {
    if (safeCursor == token.contentEndExclusive ||
        safeCursor == token.endExclusive) {
      return token;
    }
  }
  return null;
}

/// Finds the active @ or / trigger ending at the editing caret.
ActiveMentionTrigger? findActiveMentionTrigger(TextEditingValue value) {
  final text = value.text;
  final cursor = _clampedOffset(value.selection.start, text.length);
  var index = cursor - 1;
  while (index >= 0) {
    final currentChar = text.substring(index, index + 1);
    if (_isWhitespace(currentChar)) {
      return null;
    }
    if (!_isMentionTriggerChar(currentChar)) {
      index -= 1;
      continue;
    }
    if (currentChar == '@' &&
        index > 0 &&
        isMentionContinuation(text.substring(index - 1, index), '@')) {
      index -= 1;
      continue;
    }
    if (currentChar == '/' &&
        index > 0 &&
        !_isWhitespace(text.substring(index - 1, index))) {
      index -= 1;
      continue;
    }

    final query = text.substring(index + 1, cursor);
    if (_hasWhitespace(query)) {
      return null;
    }

    return ActiveMentionTrigger(
      triggerChar: currentChar,
      triggerIndex: index,
      query: query.trim(),
    );
  }
  return null;
}

/// Reports whether a single character is one of the mention triggers.
bool _isMentionTriggerChar(String char) {
  return char == '@' || char == '/';
}

/// Reports whether a single character belongs to the base token set.
bool _isBaseMentionContinuation(String char) {
  if (char.isEmpty) {
    return false;
  }
  final codeUnit = char.codeUnitAt(0);
  return (codeUnit <= 127 &&
          ((codeUnit >= 48 && codeUnit <= 57) ||
              (codeUnit >= 65 && codeUnit <= 90) ||
              (codeUnit >= 97 && codeUnit <= 122))) ||
      char == '.' ||
      char == '_' ||
      char == '%' ||
      char == '+' ||
      char == '-';
}

/// Reports whether a trigger is valid at the supplied index.
bool _isValidMentionTrigger(String text, int index, String triggerChar) {
  if (!_isMentionTriggerChar(triggerChar)) {
    return false;
  }
  if (index == 0) {
    return true;
  }
  final previousChar = text.substring(index - 1, index);
  return switch (triggerChar) {
    '@' => !_isBaseMentionContinuation(previousChar),
    '/' => _isWhitespace(previousChar),
    _ => false,
  };
}

/// Reports whether the text contains any whitespace code unit.
bool _hasWhitespace(String text) {
  for (var index = 0; index < text.length; index += 1) {
    if (_isWhitespace(text.substring(index, index + 1))) {
      return true;
    }
  }
  return false;
}

/// Reports whether a single character is whitespace.
bool _isWhitespace(String char) {
  if (char.isEmpty) {
    return false;
  }
  final codeUnit = char.codeUnitAt(0);
  return codeUnit == 0x20 ||
      codeUnit == 0x85 ||
      codeUnit == 0xA0 ||
      codeUnit == 0x1680 ||
      codeUnit == 0x2028 ||
      codeUnit == 0x2029 ||
      codeUnit == 0x202F ||
      codeUnit == 0x205F ||
      codeUnit == 0x3000 ||
      (codeUnit >= 0x09 && codeUnit <= 0x0D) ||
      (codeUnit >= 0x2000 && codeUnit <= 0x200A);
}

/// Clamps a text offset into the available code-unit range.
int _clampedOffset(int offset, int textLength) {
  if (offset < 0) {
    return 0;
  }
  if (offset > textLength) {
    return textLength;
  }
  return offset;
}
