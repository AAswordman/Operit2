import type { Definition } from "./model";
import type { ChatUiRoutes } from "./ui-contributions";

/** Single package-owned definition used by route registration and freshly loaded API modules. */
export const definition: Definition = { id: "com.operit.character_cards", title: "角色卡", icon: "Badge", order: 150 };

/** Stable registered route IDs, independent of any registration runtime's module state. */
export const chatUiRoutes: ChatUiRoutes = {
  editor: `toolpkg:${definition.id}:ui:main`,
  selection: `toolpkg:${definition.id}:ui:selection`,
  execution: `toolpkg:${definition.id}:ui:group-execution`,
};
