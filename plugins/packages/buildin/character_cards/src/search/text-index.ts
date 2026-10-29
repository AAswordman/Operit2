/** Literal substring index. Posting intersection narrows candidates before exact verification. */
export class TextIndex {
  private readonly text = new Map<string, string>();
  private readonly postings = new Map<string, Set<string>>();
  private grams(value: string, width: number): Set<string> {
    const result = new Set<string>();
    for (let offset = 0; offset <= value.length - width; offset++) result.add(value.slice(offset, offset + width));
    return result;
  }
  remove(id: string): void {
    const old = this.text.get(id);
    if (old === undefined) return;
    for (let width = 1; width <= 3; width++) for (const gram of this.grams(old, width)) {
      const posting = this.postings.get(gram)!; posting.delete(id); if (posting.size === 0) this.postings.delete(gram);
    }
    this.text.delete(id);
  }
  put(id: string, value: string): void {
    this.remove(id); value = value.toLowerCase(); this.text.set(id, value);
    for (let width = 1; width <= 3; width++) for (const gram of this.grams(value, width)) {
      let posting = this.postings.get(gram);
      if (posting === undefined) { posting = new Set(); this.postings.set(gram, posting); }
      posting.add(id);
    }
  }
  search(term: string): string[] {
    if (term === "") return [];
    if (term.includes("*")) {
      const parts = term.split("*").filter(Boolean);
      const candidates = parts.length ? parts.map(part => this.search(part)).sort((a, b) => a.length - b.length)[0] : [...this.text.keys()];
      const pattern = new RegExp(parts.length ? term.split("*").map(part => part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")).join("[^\\0]*") : "", "u");
      return candidates.filter(id => pattern.test(this.text.get(id)!));
    }
    const postings = [...this.grams(term, Math.min(3, term.length))].map(gram => this.postings.get(gram));
    if (postings.some(posting => posting === undefined)) return [];
    const ordered = (postings as Set<string>[]).sort((a, b) => a.size - b.size);
    return [...ordered[0]].filter(id => ordered.every(posting => posting.has(id)) && this.text.get(id)!.includes(term));
  }
}
interface TitleNode { children: Map<string, TitleNode>; ids: Set<string> }
/** Finds titles contained in a query without scanning every stored title. */
export class TitleIndex {
  private readonly root: TitleNode = { children: new Map(), ids: new Set() };
  put(id: string, title: string): void {
    let node = this.root;
    for (const character of title.toLowerCase()) {
      let child = node.children.get(character);
      if (!child) { child = { children: new Map(), ids: new Set() }; node.children.set(character, child); }
      node = child;
    }
    node.ids.add(id);
  }
  remove(id: string, title: string): void {
    let node = this.root; const path: [TitleNode, string][] = [];
    for (const character of title.toLowerCase()) { path.push([node, character]); const child = node.children.get(character); if (!child) return; node = child; }
    node.ids.delete(id);
    for (const [parent, character] of path.reverse()) {
      const child = parent.children.get(character)!;
      if (child.ids.size || child.children.size) break;
      parent.children.delete(character);
    }
  }
  search(query: string): Set<string> {
    const characters = [...query.toLowerCase()], result = new Set<string>();
    for (let start = 0; start < characters.length; start++) {
      let node = this.root;
      for (let index = start; index < characters.length; index++) {
        const child = node.children.get(characters[index]); if (!child) break;
        node = child; for (const id of node.ids) result.add(id);
      }
    }
    return result;
  }
}
