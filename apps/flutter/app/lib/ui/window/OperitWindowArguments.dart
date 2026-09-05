// ignore_for_file: file_names

import 'dart:convert';

import '../../data/preferences/UserPreferencesManager.dart';

sealed class OperitWindowArguments {
  const OperitWindowArguments();

  static const String mainKind = 'main';
  static const String detachedChatKind = 'detached_chat';

  factory OperitWindowArguments.parse(String value) {
    if (value.isEmpty) {
      return const MainWindowArguments();
    }
    final json = jsonDecode(value) as Map<String, Object?>;
    final kind = json['kind'] as String;
    return switch (kind) {
      mainKind => const MainWindowArguments(),
      detachedChatKind => DetachedChatWindowArguments.fromJson(json),
      _ => throw StateError('Unknown window kind: $kind'),
    };
  }

  String get kind;

  Map<String, Object?> toJson();

  String encode() => jsonEncode(<String, Object?>{'kind': kind, ...toJson()});
}

class MainWindowArguments extends OperitWindowArguments {
  const MainWindowArguments();

  @override
  String get kind => OperitWindowArguments.mainKind;

  @override
  Map<String, Object?> toJson() => const <String, Object?>{};
}

class DetachedChatWindowArguments extends OperitWindowArguments {
  /// Creates the launch contract for a detached chat window.
  const DetachedChatWindowArguments({
    required this.slotId,
    required this.chatId,
    required this.title,
    required this.themePreferenceSnapshot,
  });

  /// Decodes the launch contract supplied to a detached Flutter engine.
  factory DetachedChatWindowArguments.fromJson(Map<String, Object?> json) {
    return DetachedChatWindowArguments(
      slotId: json['slotId'] as String,
      chatId: json['chatId'] as String,
      title: json['title'] as String,
      themePreferenceSnapshot: ThemePreferenceSnapshot.fromJson(
        (json['themePreferenceSnapshot'] as Map).cast<String, Object?>(),
      ),
    );
  }

  final String slotId;
  final String chatId;
  final String title;
  final ThemePreferenceSnapshot themePreferenceSnapshot;

  @override
  String get kind => OperitWindowArguments.detachedChatKind;

  @override
  Map<String, Object?> toJson() {
    return <String, Object?>{
      'slotId': slotId,
      'chatId': chatId,
      'title': title,
      'themePreferenceSnapshot': themePreferenceSnapshot.toJson(),
    };
  }
}
