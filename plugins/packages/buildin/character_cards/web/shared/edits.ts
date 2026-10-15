import type { Dialog, EditableDialog } from "../bridge/contracts";
import { isKey } from "./dom";
/** Requires a dialog whose current draft can accept text-field edits. */
export function editableDialog(dialog: Dialog): EditableDialog {
  switch (dialog.type) {
    case "store": case "memory-edit": case "link": case "link-edit":
    case "user": case "import": case "export": case "owner-settings": return dialog;
    default: throw new Error("当前页面不能编辑文字字段");
  }
}

/** Updates one explicitly declared editable field while retaining non-text canonical data. */
export function setEditableValue(dialog: EditableDialog, key: string, value: string): void {
  switch (dialog.type) {
    case "store": if (key === "name") { dialog.store.name = value; return; } break;
    case "memory-edit":
      if (isKey(key, ["credibility", "importance"])) { dialog.draft[key] = Number(value); return; }
      if (isKey(key, ["title", "content", "contentType", "source", "folderPath", "tags"])) { dialog.draft[key] = value; return; }
      break;
    case "link": case "link-edit":
      if (isKey(key, ["sourceTitle", "targetTitle", "linkType", "description", "weight"])) { dialog.draft[key] = value; return; }
      break;
    case "owner-settings": {
      if (key === "searchQuery") { dialog.searchQuery = value; dialog.searchResult = null; return; }
      if (key === "searchThreshold") { dialog.searchThreshold = Number(value); dialog.searchResult = null; return; }
      if (key === "from" || key === "to") { dialog[key] = value; return; }
      if (key === "windowMessageCount") { dialog.windowMessageCount = Number(value); return; }
      if (dialog.settings === null || dialog.config === null) throw new Error("记忆设置尚未读取成功");
      if (key === "interval") { dialog.settings.autoSaveIntervalMinutes = Number(value); return; }
      if (isKey(key, ["keywordWeight", "tagWeight", "vectorWeight", "edgeWeight"])) { dialog.config[key] = Number(value); dialog.searchResult = null; return; }
      const fields = { extractionRules: "memoryExtractionCustomRules", endpoint: "cloudEmbeddingEndpoint", embeddingModel: "cloudEmbeddingModel", apiKey: "cloudEmbeddingApiKey" } as const;
      if (isKey(key, ["extractionRules", "endpoint", "embeddingModel", "apiKey"])) { dialog.settings[fields[key]] = value; return; }
      break;
    }
    case "import":
      if (key === "format" && (value === "operit" || value === "tavern")) { dialog.format = value; return; }
      if (key === "strategy" && (value === "SKIP" || value === "UPDATE" || value === "CREATE_NEW")) { dialog.strategy = value; return; }
      if (key === "content") { dialog.content = value; return; }
      break;
    case "user": if (key === "content") { dialog.content = value; return; } break;
  }
  throw new Error(`此页面不能编辑字段：${key}`);
}

/** Reads only an existing textual field when opening the expanded editor. */
export function editableValue(dialog: EditableDialog, key: string): string {
  let record: object;
  switch (dialog.type) {
    case "store": record = dialog.store; break;
    case "memory-edit": case "link": case "link-edit": record = dialog.draft; break;
    case "user": case "import": case "export": record = dialog; break;
    case "owner-settings":
      if (dialog.settings === null || key !== "extractionRules") throw new Error("展开编辑字段不是文字");
      return dialog.settings.memoryExtractionCustomRules;
  }
  const value: unknown = Reflect.get(record, key);
  if (typeof value !== "string") throw new Error("展开编辑字段不是文字");
  return value;
}
