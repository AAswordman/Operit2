import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Card, Group } from "./model";

/** Carries only the ordered portrait sources needed by the plugin's group views. */
export interface GroupPortrait {
  uri: string | null;
  members: { id: string; name: string; avatarUri: string | null; updatedAt: number }[];
}

/** Projects a custom image or the first nine ordered member portraits without changing saved group data. */
export function groupPortrait(group: Group, cards: readonly Card[]): GroupPortrait {
  return { uri: group.avatarUri ?? null, members: [...group.members].sort((a, b) => a.orderIndex - b.orderIndex).slice(0, 9).map(member => {
    const card = cards.find(card => card.id === member.characterCardId);
    if (card === undefined) throw new Error("群组头像引用的角色不存在：" + member.characterCardId);
    return { id: card.id, name: card.name, avatarUri: card.avatarUri, updatedAt: card.updatedAt };
  }) };
}

/** Centers a partial first row and keeps the same square crop geometry in DSL and PNG portraits. */
export function groupAvatarLayout(count: number, size: number): { tile: number; gap: number; rows: { x: number; y: number; count: number }[] } {
  if (!Number.isInteger(count) || count < 1 || count > 9) throw new Error("Group portrait requires one to nine members");
  const columns = count === 1 ? 1 : count <= 4 ? 2 : 3, rowCount = Math.ceil(count / columns);
  const gap = count === 1 ? 0 : size / 50, padding = count === 1 ? 0 : size / 25;
  const tile = (size - padding * 2 - gap * (columns - 1)) / columns;
  const top = (size - rowCount * tile - (rowCount - 1) * gap) / 2;
  const first = count - columns * (rowCount - 1);
  return { tile, gap, rows: Array.from({ length: rowCount }, (_, index) => {
    const count = index === 0 ? first : columns;
    return { x: (size - count * tile - (count - 1) * gap) / 2, y: top + index * (tile + gap), count };
  }) };
}

/** Renders live member composition at any native portrait size, including unsaved group edits. */
export function renderGroupAvatar(ctx: ComposeDslContext, portrait: GroupPortrait, defaultAvatar: string | null, size: number, key = "group-avatar"): ComposeNode {
  const props = { key, width: size, height: size, modifier: ctx.Modifier.clip({ cornerRadius: size / 5 }) };
  if (portrait.uri !== null) return ctx.UI.Image({ ...props, uri: portrait.uri, contentScale: "crop", contentDescription: "群组头像" });
  if (portrait.members.length === 0) return ctx.UI.Box({ ...props, contentAlignment: "center" }, ctx.UI.Icon({ name: "Groups", size: size * 0.65, tint: ctx.MaterialTheme.colorScheme.onSurfaceVariant }));
  const layout = groupAvatarLayout(portrait.members.length, size);
  let offset = 0;
  return ctx.UI.Column({ ...props, verticalArrangement: "center", horizontalAlignment: "center", spacing: layout.gap }, layout.rows.map(row => {
    const members = portrait.members.slice(offset, offset += row.count);
    return ctx.UI.Row({ horizontalArrangement: "center", spacing: layout.gap }, members.map(member => {
      const props = { key: key + ":" + member.id, width: layout.tile, height: layout.tile, contentDescription: member.name };
      if (member.avatarUri !== null) return ctx.UI.Image({ ...props, uri: member.avatarUri, contentScale: "crop" });
      if (defaultAvatar !== null) return ctx.UI.Image({ ...props, path: defaultAvatar, contentScale: "crop" });
      return ctx.UI.Box({ ...props, contentAlignment: "center" }, ctx.UI.Icon({ name: "Person", size: layout.tile * 0.8, tint: ctx.MaterialTheme.colorScheme.onSurfaceVariant }));
    }));
  }));
}
