import type { Card, Group, TagValues } from "./model";

/** Initializes an explicit new-character form using the canonical schema's creation values. */
export function createCharacterDraft(): Card {
  return {
    id: "", name: "", description: "", characterSetting: "", openingStatement: "",
    otherContentChat: "", otherContentVoice: "", avatarUri: null, attachedTagIds: [],
    advancedCustomPrompt: "", marks: "", chatModelBindingMode: "FOLLOW_GLOBAL", chatModelId: null, chatProviderId: null,
    ttsConfigId: null, themeConfigId: null, memoryBindingMode: "CHARACTER", sharedMemoryId: null, sharedMemoryMounts: [],
    toolAccessConfig: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] },
    createdAt: 0, updatedAt: 0,
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

/** Reads old or imported cards once, using the same defaults as a new card. */
export function readCharacter(value: unknown, id = ""): Card {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new Error("角色卡必须是对象");
  const source = value as Record<string, unknown>, defaults = createCharacterDraft();
  const card = { ...source } as unknown as Card;
  for (const [field, fallback] of Object.entries(defaults)) {
    const supplied = source[field];
    const valid = fallback === null ? typeof supplied === "string"
      : Array.isArray(fallback) ? Array.isArray(supplied)
      : supplied !== null && typeof supplied === typeof fallback;
    Object.assign(card, { [field]: valid ? supplied : fallback });
  }
  card.id = typeof source.id === "string" ? source.id : id;
  if (card.themeConfigId?.trim() === "") card.themeConfigId = null;
  card.attachedTagIds = card.attachedTagIds.filter((tag): tag is string => typeof tag === "string");
  if (card.memoryBindingMode !== "SHARED") card.memoryBindingMode = "CHARACTER";
  for (const field of ["createdAt", "updatedAt"] as const) {
    const timestamp = typeof source[field] === "string" ? Number(source[field]) : source[field];
    card[field] = typeof timestamp === "number" && Number.isSafeInteger(timestamp) && timestamp >= 0 ? timestamp : 0;
  }
  const policy = card.toolAccessConfig;
  card.toolAccessConfig = { ...defaults.toolAccessConfig };
  card.toolAccessConfig.enabled = policy.enabled === true;
  for (const field of ["allowedBuiltinTools", "allowedPackages", "allowedSkills", "allowedMcpServers"] as const)
    card.toolAccessConfig[field] = Array.isArray(policy[field]) ? policy[field].filter(item => typeof item === "string") : [];
  card.sharedMemoryMounts = card.sharedMemoryMounts
    .filter(mount => mount !== null && typeof mount === "object" && typeof mount.sharedMemoryId === "string" && mount.sharedMemoryId.trim() !== "")
    .map(mount => ({ sharedMemoryId: mount.sharedMemoryId, readable: mount.readable === true, writable: mount.writable === true }));
  return card;
}
