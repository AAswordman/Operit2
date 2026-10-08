import type { DialogSpec, ExportDialog, ImportDialog } from "../../bridge/contracts";
import { button, field, notice } from "../../shared/ui/html";
/** Renders the import modal using its finite typed draft. */
export function importSpec(dialog: ImportDialog): DialogSpec {
  return { title: dialog.target === "card" ? "导入角色 JSON" : "导入群组 JSON", style: "large", tabs: [], body: field("粘贴 Operit JSON", "content", dialog.content, 18) + notice("接收原软件导出的完整 Operit JSON。Tavern 格式的插件导入流程尚未接入。"), footer: button("close-dialog", "取消", "") + button("read-import", "导入", "", "", "filled") };
}
/** Renders the explicit-path export modal. */
export function exportSpec(dialog: ExportDialog): DialogSpec {
  return { title: "导出 JSON", style: "large", tabs: [], body: field("导出文件 VFS 路径 *", "path", dialog.path) + field("JSON 内容", "content", dialog.content, 16, true) + notice("与原角色卡/群组编辑器相同的 Operit JSON 格式。请输入宿主可写的 VFS 路径。"), footer: button("close-dialog", "取消", "") + button("save-export", "导出", "", "", "filled") };
}
