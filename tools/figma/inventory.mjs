/** Source inventory for coverage reporting; discovering a widget is not capture. */
import { readdir, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const root = new URL("../../", import.meta.url);
async function files(directory) {
  const result = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    if (
      ["node_modules", "build", ".dart_tool", ".git", ".fvm"].includes(
        entry.name,
      )
    )
      continue;
    const path = new URL(
      `${entry.name}${entry.isDirectory() ? "/" : ""}`,
      directory,
    );
    if (entry.isDirectory()) result.push(...(await files(path)));
    else result.push(path);
  }
  return result;
}
export async function sourceInventory() {
  const surfaces = [];
  const ui = new URL("apps/flutter/app/lib/ui/", root);
  for (const file of await files(ui)) {
    if (!file.pathname.endsWith(".dart")) continue;
    const text = await readFile(file, "utf8");
    for (const match of text.matchAll(
      /class\s+(\w+)\s+extends\s+(?:StatefulWidget|StatelessWidget|ConsumerWidget|ConsumerStatefulWidget)\b/g,
    )) {
      const widget = match[1];
      if (
        !/(Screen|Dialog|Sheet|Panel)$/.test(widget) &&
        !/(ToolDisplayComponents|compose_dsl\/)/.test(file.pathname)
      )
        continue;
      const path = fileURLToPath(file).slice(fileURLToPath(root).length);
      surfaces.push({
        id: `${path}#${widget}`,
        kind: /Dialog|Sheet/.test(widget)
          ? "overlay"
          : /ToolDisplayComponents|compose_dsl\//.test(path)
            ? "renderer"
            : "screen-or-panel",
        widget,
        file: path,
        line: text.slice(0, match.index).split("\n").length,
      });
    }
  }
  const pluginFiles = await files(new URL("plugins/packages/", root));
  for (const file of pluginFiles) {
    if (
      !/\.(html|tsx|jsx)$/.test(file.pathname) ||
      /node_modules|\/dist\//.test(file.pathname)
    )
      continue;
    const path = fileURLToPath(file).slice(fileURLToPath(root).length);
    surfaces.push({ id: path, kind: "plugin-resource", file: path });
  }
  return surfaces.sort((a, b) => a.id.localeCompare(b.id));
}
export function observedWidgets(document) {
  const names = new Set();
  function visit(node) {
    if (node.source?.widget) names.add(node.source.widget);
    for (const name of node.source?.ancestors ?? []) names.add(name);
    for (const name of node.source?.surfaces ?? []) names.add(name);
    for (const child of node.children ?? []) visit(child);
  }
  document.scenes.forEach(visit);
  return names;
}
export function coverage(inventory, captures) {
  const observed = new Map();
  for (const capture of captures) {
    for (const widget of observedWidgets(capture.document)) {
      const ids = observed.get(widget) ?? [];
      ids.push(capture.id);
      observed.set(widget, ids);
    }
  }
  return inventory.map((surface) => ({
    ...surface,
    status:
      surface.widget && observed.has(surface.widget)
        ? "observed"
        : "unrecorded",
    captures: observed.get(surface.widget) ?? [],
  }));
}
