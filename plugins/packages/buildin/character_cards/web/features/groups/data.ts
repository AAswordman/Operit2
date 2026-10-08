import type { Group, GroupValues, Snapshot } from "../../../src/model";
/** Renumbers explicitly selected group members after an order change. */
export function renumberMembers(group: GroupValues): void {
  for (let index = 0; index < group.members.length; index++) group.members[index].orderIndex = index;
}

/** Resolves a canonical group by exact identity without replacing absent members. */
export function getGroup(snapshot: Snapshot, id: string): Group {
  for (const group of snapshot.groups) if (group.id === id) return group;
  throw new Error("群组不存在");
}
