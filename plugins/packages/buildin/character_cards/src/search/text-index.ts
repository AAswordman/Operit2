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
