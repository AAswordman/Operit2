// ignore_for_file: file_names

import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

/// Holds the immutable display identity saved on the selected message revision.
class ChatMessageIdentity {
  /// Creates the exact saved title and avatar without consulting the current chat.
  const ChatMessageIdentity({required this.title, required this.avatarUri});

  final String title;
  final String? avatarUri;

  /// Identifies an explicit provider-logo avatar through its declared URI scheme.
  bool get usesProviderLogo =>
      avatarUri != null && Uri.parse(avatarUri!).scheme == 'provider-logo';
}

/// Reads optional native display metadata and rejects malformed saved identities.
ChatMessageIdentity? readChatMessageIdentity(core_proxy.ChatMessage message) {
  final value = message.pluginExtensions['host.chat.identity'];
  if (value == null) return null;
  if (value is! Map ||
      value.length != 2 ||
      value['title'] is! String ||
      (value['title'] as String).trim().isEmpty ||
      (value['avatarUri'] != null && value['avatarUri'] is! String) ||
      !value.containsKey('avatarUri')) {
    throw const FormatException('Invalid saved message display identity.');
  }
  return ChatMessageIdentity(
    title: value['title'] as String,
    avatarUri: value['avatarUri'] as String?,
  );
}
