import type { DialogSpec, LinkDialog, LinkEditDialog } from "../../bridge/contracts";
import { button, field, selectField } from "../../shared/ui/html";
/** Renders the link modal using its finite typed draft. */
export function linkSpec(dialog: LinkDialog): DialogSpec {
  { const options = []; for (const item of dialog.items) options.push({ id: item.title, label: item.title }); return { title: "创建记忆链接", style: "compact", tabs: [], body: selectField("源记忆", "sourceTitle", dialog.draft.sourceTitle, options, false) + selectField("目标记忆", "targetTitle", dialog.draft.targetTitle, options, false) + field("关系类型", "linkType", dialog.draft.linkType) + field("强度（0–1）", "weight", dialog.draft.weight) + field("描述", "description", dialog.draft.description, 3), footer: button("close-dialog", "取消", "") + button("save-link", "创建", "", "", "filled") }; }
}
/** Renders a persisted relationship editor without substituting its missing record fields. */
export function linkEditSpec(dialog: LinkEditDialog): DialogSpec {
  return { title: "编辑记忆关系", style: "compact", tabs: [], body: field("源记忆", "sourceTitle", dialog.draft.sourceTitle, 0, true) + field("目标记忆", "targetTitle", dialog.draft.targetTitle, 0, true) + field("关系类型", "linkType", dialog.draft.linkType) + field("强度（0–1）", "weight", dialog.draft.weight) + field("描述", "description", dialog.draft.description, 3), footer: button("close-dialog", "取消", "") + button("save-link-edit", "保存", "", "", "filled") };
}
