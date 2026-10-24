import type { SidebarChatSummary } from './ui-sidebar';

/** Projects canonical parent metadata into ordered branches, including surviving orphan roots. */
export class SidebarChatTree {
  readonly roots = new Set<string>();
  readonly children = new Map<string, SidebarChatSummary[]>();

  /** Rejects duplicate identities and cycles before any recursive rows are rendered. */
  constructor(chats: readonly SidebarChatSummary[]) {
    const ids = new Set(chats.map(chat => chat.id));
    if (ids.size !== chats.length) throw new Error("Duplicate conversation id");
    for (const chat of chats) this.children.set(chat.id, []);
    for (const chat of chats) {
      if (chat.parentChatId === null || !ids.has(chat.parentChatId)) this.roots.add(chat.id);
      else this.children.get(chat.parentChatId)!.push(chat);
    }
    const visiting = new Set<string>(), visited = new Set<string>();
    /** Validates parent relationships independently of visibility or expansion. */
    const visit = (chat: SidebarChatSummary): void => {
      if (visited.has(chat.id)) return;
      if (visiting.has(chat.id)) throw new Error("Cyclic conversation parent: " + chat.id);
      visiting.add(chat.id);
      for (const child of this.children.get(chat.id)!) visit(child);
      visiting.delete(chat.id); visited.add(chat.id);
    };
    for (const chat of chats) visit(chat);
  }

  /** Retains the complete ancestor path of every matching descendant. */
  matching(chats: readonly SidebarChatSummary[], predicate: (chat: SidebarChatSummary) => boolean): Set<string> {
    const result = new Set<string>();
    /** Visits every child even when its ancestor already matches. */
    const visit = (chat: SidebarChatSummary): boolean => {
      let matches = predicate(chat);
      for (const child of this.children.get(chat.id)!) matches = visit(child) || matches;
      if (matches) result.add(chat.id);
      return matches;
    };
    for (const chat of chats) if (this.roots.has(chat.id)) visit(chat);
    return result;
  }

}
