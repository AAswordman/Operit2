import type { TagEdit } from "../../../src/model";
import type { CharacterDialog, TagsDialog } from "../../bridge/contracts";
import { button, escapeHtml, iconButton, notice } from "../../shared/ui/html";
/** Resolves an exact prompt tag inside the enclosing character draft. */
export function tagById(parent: CharacterDialog, id: string): TagEdit {
  for (const tag of parent.tags) if (tag.id === id) return tag;
  throw new Error("提示词标签不存在");
}

/** Renders native tag management while deferring writes until character save. */
export function tagManagerBody(dialog: TagsDialog): string {
  let content = `<div class="section-head"><h2>标签</h2>${button("create-tag", "创建标签", "add", "", "small")}</div>`;
  for (const tag of dialog.parent.tags) content += `<article class="entity"><div class="entity-copy"><strong>${escapeHtml(tag.name)}</strong><div class="muted">${escapeHtml(tag.description)}</div><pre class="tag-prompt">${escapeHtml(tag.promptContent)}</pre></div><div class="actions">${iconButton("edit-tag", "编辑 " + tag.name, "edit", `data-id="${escapeHtml(tag.id)}"`)}${iconButton("delete-tag", "删除 " + tag.name, "trash", `data-id="${escapeHtml(tag.id)}"`)}</div></article>`;
  return content + notice("标签修改在保存角色卡时提交；取消角色编辑不会写入这些修改。");
}
