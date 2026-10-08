import type { Dialog, EditableDialog } from "../bridge/contracts";
import { isKey } from "./dom";
/** Requires a dialog whose current draft can accept text-field edits. */
export function editableDialog(dialog: Dialog): EditableDialog {
  switch (dialog.type) {
    case "character": case "group": case "store": case "memory-edit": case "link": case "link-edit":
    case "tag-edit": case "user": case "import": case "export": return dialog;
    default: throw new Error("当前页面不能编辑文字字段");
  }
}

/** Updates one explicitly declared editable field while retaining non-text canonical data. */
export function setEditableValue(dialog: EditableDialog, key: string, value: string): void {
  switch (dialog.type) {
    case "character":
      if (isKey(key, ["name", "description", "characterSetting", "openingStatement", "otherContentChat", "otherContentVoice", "advancedCustomPrompt", "marks"])) { dialog.card[key] = value; return; }
      if (isKey(key, ["avatarUri", "chatModelId", "ttsConfigId", "sharedMemoryId"])) { dialog.card[key] = value === "" ? null : value; return; }
      break;
    case "group":
      if (isKey(key, ["name", "description"])) { dialog.group[key] = value; return; }
      break;
    case "store": if (key === "name") { dialog.store.name = value; return; } break;
    case "memory-edit":
      if (isKey(key, ["credibility", "importance"])) { dialog.draft[key] = Number(value); return; }
      if (isKey(key, ["title", "content", "contentType", "source", "folderPath", "tags"])) { dialog.draft[key] = value; return; }
      break;
    case "link": case "link-edit":
      if (isKey(key, ["sourceTitle", "targetTitle", "linkType", "description", "weight"])) { dialog.draft[key] = value; return; }
      break;
    case "tag-edit":
      if (isKey(key, ["name", "description", "promptContent"])) { dialog.draft[key] = value; return; }
      break;
    case "user": case "import": if (key === "content") { dialog.content = value; return; } break;
    case "export": if (key === "path") { dialog.path = value; return; } break;
  }
  throw new Error(`此页面不能编辑字段：${key}`);
}

/** Reads only an existing textual field when opening the expanded editor. */
export function editableValue(dialog: EditableDialog, key: string): string {
  let record: object;
  switch (dialog.type) {
    case "character": record = dialog.card; break;
    case "group": record = dialog.group; break;
    case "store": record = dialog.store; break;
    case "memory-edit": case "link": case "link-edit": case "tag-edit": record = dialog.draft; break;
    case "user": case "import": case "export": record = dialog; break;
  }
  const value: unknown = Reflect.get(record, key);
  if (typeof value !== "string") throw new Error("展开编辑字段不是文字");
  return value;
}
