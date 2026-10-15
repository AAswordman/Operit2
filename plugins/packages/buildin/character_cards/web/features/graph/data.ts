import type { Memory, MemoryGraphResult } from "../../../src/model";
import type { GraphDialog } from "../../bridge/contracts";
/** Requires authoritative graph data before enabling node and relationship mutations. */
export function graphData(dialog: GraphDialog): MemoryGraphResult {
  if (dialog.data === null) throw new Error("记忆图谱尚未加载");
  return dialog.data;
}

/** Requires the actual viewport before applying camera toolbar operations. */
export function graphViewport(dialog: GraphDialog): { width: number; height: number } {
  if (dialog.viewport === null) throw new Error("记忆图谱画布尚未就绪");
  return dialog.viewport;
}

/** Returns the memory items matching the explicit user search and folder selection. */
export function graphItems(dialog: GraphDialog): Memory[] {
  const items = [];
  for (const item of graphData(dialog).items) {
    const folder = item.folderPath === null ? "" : item.folderPath;
    if (dialog.folder !== "*" && dialog.folder !== "" && dialog.folder !== folder && !folder.startsWith(dialog.folder + "/")) continue;
    if (dialog.folder === "" && folder !== "") continue;
    if (dialog.searchIds !== null && !dialog.searchIds.has(item.uuid)) continue;
    items.push(item);
  }
  return items;
}

/** Resolves the selected memory by its UUID without matching title substrings. */
export function selectedMemory(dialog: GraphDialog): Memory | null {
  for (const item of graphData(dialog).items) if (item.uuid === dialog.selected) return item;
  return null;
}
