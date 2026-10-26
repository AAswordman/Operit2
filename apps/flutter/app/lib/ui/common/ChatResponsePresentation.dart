// ignore_for_file: file_names

import '../../core/proxy/generated/CoreProxyModels.g.dart' as core_proxy;

const responsePresentationExtension = 'host.response.presentation';

/// Holds the bubbles already delivered by the native response presentation queue.
class ChatResponsePresentation {
  /// Creates an immutable view of native delivery state.
  const ChatResponsePresentation({
    required this.bubbles,
    required this.pending,
  });

  final List<List<core_proxy.MessagePart>> bubbles;
  final bool pending;
}

/// Reads the generic native presentation protocol without consulting plugin-specific settings.
ChatResponsePresentation? readChatResponsePresentation(
  core_proxy.ChatMessage message,
) {
  final value = message.pluginExtensions[responsePresentationExtension];
  if (value == null) return null;
  if (value is! Map || value['bubbles'] is! List || value['pending'] is! bool) {
    throw const FormatException('Invalid saved response presentation.');
  }
  final bubbles = <List<core_proxy.MessagePart>>[];
  for (final bubble in value['bubbles'] as List) {
    if (bubble is! List || bubble.isEmpty) {
      throw const FormatException('Response bubbles require semantic parts.');
    }
    bubbles.add(
      bubble
          .map(
            (part) => core_proxy.MessagePart.fromJson(
              Map<String, Object?>.from(part as Map),
            ),
          )
          .toList(growable: false),
    );
  }
  return ChatResponsePresentation(
    bubbles: bubbles,
    pending: value['pending'] as bool,
  );
}

/// Copies native identity and metrics to one independent display bubble.
core_proxy.ChatMessage responseBubbleMessage(
  core_proxy.ChatMessage original,
  List<core_proxy.MessagePart> parts, {
  required bool includeMetrics,
}) {
  final extensions = Map<String, Object?>.from(original.pluginExtensions)
    ..remove(responsePresentationExtension);
  return core_proxy.ChatMessage(
    sender: original.sender,
    parts: parts,
    timestamp: original.timestamp,
    roleName: original.roleName,
    pluginExtensions: extensions,
    selectedVariantIndex: original.selectedVariantIndex,
    variantCount: original.variantCount,
    provider: original.provider,
    modelName: original.modelName,
    inputTokens: includeMetrics ? original.inputTokens : 0,
    outputTokens: includeMetrics ? original.outputTokens : 0,
    cachedInputTokens: includeMetrics ? original.cachedInputTokens : 0,
    sentAt: original.sentAt,
    outputDurationMs: includeMetrics ? original.outputDurationMs : 0,
    waitDurationMs: includeMetrics ? original.waitDurationMs : 0,
    completedAt: original.completedAt,
    displayMode: original.displayMode,
    isFavorite: original.isFavorite,
    contentStream: null,
  );
}
