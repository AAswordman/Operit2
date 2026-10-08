import type { Card, Group, TagValues } from "./model";

/** Initializes an explicit new-character form using the canonical schema's creation values. */
export function createCharacterDraft(): Card {
  return {
    id: "", name: "", description: "", characterSetting: "", openingStatement: "",
    otherContentChat: "", otherContentVoice: "", avatarUri: null, attachedTagIds: [],
    advancedCustomPrompt: "", marks: "", chatModelBindingMode: "FOLLOW_GLOBAL", chatModelId: null,
    ttsConfigId: null, themeConfigId: null, memoryBindingMode: "CHARACTER", sharedMemoryId: null, sharedMemoryMounts: [],
    toolAccessConfig: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] },
    isDefault: false, createdAt: 0, updatedAt: 0,
  };
}

/** Initializes an explicit new-group form before the host assigns its identity and timestamps. */
export function createGroupDraft(): Group {
  return { id: "", name: "", description: "", members: [], themeConfigId: null, createdAt: 0, updatedAt: 0 };
}

/** Initializes editable fields for an intentional newly created prompt tag. */
export function createTagDraft(): TagValues {
  return { name: "", description: "", promptContent: "", tagType: "CUSTOM" };
}
