// ignore_for_file: file_names

import '../../../core/proxy/generated/CoreProxyModels.g.dart' as core;

/// Presents one possible sender's exact provider-scoped model and selection rule.
class ChatParticipantModel {
  /// Creates a model projection supplied by the authenticated configuration owner.
  const ChatParticipantModel({
    required this.id,
    required this.name,
    required this.binding,
    required this.followsGlobal,
  });

  final String id;
  final String name;
  final core.FunctionModelBinding binding;
  final bool followsGlobal;
}

/// Keeps group models explicit without selecting an executor for presentation.
class ChatModelSelection {
  /// Creates the native global selection or a plugin-owned participant selection.
  const ChatModelSelection({
    required this.globalBinding,
    required this.participants,
  });

  final core.FunctionModelBinding globalBinding;
  final List<ChatParticipantModel> participants;

  /// Allows the global picker only when it affects every possible sender.
  bool get canSelectGlobal =>
      participants.every((participant) => participant.followsGlobal);

  /// Returns a single actual model only when every sender uses the same pair.
  core.FunctionModelBinding? get singleBinding {
    if (participants.isEmpty) return globalBinding;
    final binding = participants.first.binding;
    return participants.every(
          (participant) =>
              participant.binding.providerId == binding.providerId &&
              participant.binding.modelId == binding.modelId,
        )
        ? binding
        : null;
  }

  /// Labels fixed selections and multiple senders without implying a global switch succeeded.
  String get label {
    final binding = singleBinding;
    if (binding == null) return '多模型 · ${participants.length} 个角色';
    return canSelectGlobal ? binding.modelId : '固定 · ${binding.modelId}';
  }
}
