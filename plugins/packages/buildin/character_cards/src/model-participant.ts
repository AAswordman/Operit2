import type { ChatConfigurationRequest, ChatDisplayParticipant, ChatParticipantProfile, SnapshotRecord } from "./api";

/** Resolves the configured provider/model pair by exact IDs for an explicit model-only conversation. */
export function modelIdentity(request: ChatConfigurationRequest, directory: SnapshotRecord): ChatDisplayParticipant {
  const models = directory.models.filter(
    /** Requires one real model on the configured provider rather than guessing from a display name. */
    model => model.providerId === request.defaultModelBinding.providerId && model.modelId === request.defaultModelBinding.modelId,
  );
  if (models.length !== 1 || models[0].providerTypeId.trim() === "") throw new Error("Model-only binding is unavailable or duplicated");
  return { id: "model:chat", name: models[0].modelId, avatarUri: "provider-logo:" + encodeURIComponent(models[0].providerTypeId) };
}

/** Creates a model participant without a role prompt or role memory grants. */
export function modelParticipant(request: ChatConfigurationRequest, directory: SnapshotRecord): ChatParticipantProfile {
  if (request.participantId !== null && request.participantId !== "model:chat") throw new Error("Invalid model-only execution participant");
  if (directory.ttsConfigs.filter(config => config.id === request.defaultTtsConfigId).length !== 1) throw new Error("Model-only speech configuration is unavailable or duplicated");
  return { ...modelIdentity(request, directory), introPrompt: "", userPreferencesText: "", openingStatement: "",
    modelBinding: { ...request.defaultModelBinding }, ttsConfigId: request.defaultTtsConfigId,
    toolAccess: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] }, resources: [] };
}
