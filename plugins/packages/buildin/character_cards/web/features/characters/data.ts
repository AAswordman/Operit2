import type { ActivePrompt, Card, Snapshot } from "../../../src/model";
/** Resolves an existing character by its exact identity. */
export function getCard(snapshot: Snapshot, id: string): Card {
  for (const card of snapshot.cards) if (card.id === id) return card;
  throw new Error("角色卡不存在");
}

/** Resolves the binding exactly and rejects incomplete shared-store references. */
export function ownerFor(card: Card): string {
  switch (card.memoryBindingMode) {
    case "CHARACTER": return `character:${card.id}`;
    case "SHARED":
      if (typeof card.sharedMemoryId !== "string" || card.sharedMemoryId === "") throw new Error("角色的共享记忆绑定不完整");
      return `shared:${card.sharedMemoryId}`;
    default: throw new Error("不支持的角色记忆绑定类型");
  }
}

/** Tests the Core enum representation without guessing string fragments. */
export function isActive(active: ActivePrompt, type: "card" | "group", id: string): boolean {
  if (type === "card") return "CharacterCard" in active && active.CharacterCard.id === id;
  if (type === "group") return "CharacterGroup" in active && active.CharacterGroup.id === id;
  throw new Error("Invalid active profile type");
}
