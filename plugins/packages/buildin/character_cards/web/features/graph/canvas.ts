import type { GraphDialog } from "../../bridge/contracts";
import { escapeHtml } from "../../shared/ui/html";
import { graphData, graphItems, graphViewport } from "./data";
import { graphInspector } from "./inspector";
import { fitMemoryGraph, graphEntry, layoutMemoryGraph, measureMemoryNode, memoryGraphBounds } from "./layout";
/** Recreates the native component scene only when its node/edge data changes. */
export function initializePositions(dialog: GraphDialog): void {
  const visible = new Set();
  for (const item of graphItems(dialog)) visible.add(item.uuid);
  if (dialog.appliedQuery !== "") {
    const matched = new Set(visible);
    for (const edge of graphData(dialog).graph.edges) {
      if (matched.has(edge.sourceId)) visible.add(edge.targetId);
      if (matched.has(edge.targetId)) visible.add(edge.sourceId);
    }
  }
  const nodes = graphData(dialog).graph.nodes.filter(
    /** Retains exactly the original native graph's filtered node scope. */
    node => visible.has(node.id),
  );
  const edges = graphData(dialog).graph.edges.filter(
    /** Retains relationships whose endpoints survive the current filter. */
    edge => visible.has(edge.sourceId) && visible.has(edge.targetId),
  );
  const signature = JSON.stringify({ nodes, edges });
  if (dialog.layoutSignature === signature) return;
  const scene = layoutMemoryGraph(nodes, edges, measureMemoryNode);
  dialog.positions = scene.positions; dialog.sizes = scene.sizes; dialog.layoutSignature = signature;
  dialog.viewport = null;
}

/** Fits the real scene and retains camera center when the embedded viewport changes size. */
export function setGraphViewport(dialog: GraphDialog, surface: SVGSVGElement): void {
  const bounds = surface.getBoundingClientRect(), width = bounds.width, height = bounds.height;
  surface.setAttribute("viewBox", `0 0 ${width} ${height}`);
  if (dialog.viewport !== null && dialog.viewport.width === width && dialog.viewport.height === height) return;
  if (dialog.viewport === null) dialog.camera = fitMemoryGraph(memoryGraphBounds(dialog.positions, dialog.sizes), width, height);
  else { dialog.camera.x += (width - graphViewport(dialog).width) / 2; dialog.camera.y += (height - graphViewport(dialog).height) / 2; }
  dialog.viewport = { width, height }; surface.innerHTML = graphSvg(dialog);
}

/** Renders the real graph with filtering, pan, zoom and owner-local selection. */
export function graphSvg(dialog: GraphDialog): string {
  initializePositions(dialog);
  const visible = new Set();
  for (const item of graphItems(dialog)) visible.add(item.uuid);
  if (dialog.appliedQuery !== "") {
    const matched = new Set(visible);
    for (const edge of graphData(dialog).graph.edges) {
      if (matched.has(edge.sourceId)) visible.add(edge.targetId);
      if (matched.has(edge.targetId)) visible.add(edge.sourceId);
    }
  }
  let edges = "", nodes = "";
  for (const edge of graphData(dialog).graph.edges) {
    if (!visible.has(edge.sourceId) || !visible.has(edge.targetId)) continue;
    if (!dialog.positions.has(edge.sourceId) || !dialog.positions.has(edge.targetId)) throw new Error("记忆图谱存在无效的关系端点");
    const source = graphEntry(dialog.positions, edge.sourceId), target = graphEntry(dialog.positions, edge.targetId);
    const width = Math.max(0.7, Math.min(4, edge.weight * (edge.isCrossFolderLink ? 1.6 : 2.4)));
    edges += `<g class="graph-edge-action" data-action="select-edge" data-link="${escapeHtml(edge.id)}" role="button" tabindex="0" aria-label="关系 ${escapeHtml(edge.label === null ? "" : edge.label)}"><line class="graph-edge-hit" x1="${source.x}" y1="${source.y}" x2="${target.x}" y2="${target.y}"/><line class="graph-edge ${dialog.selectedEdge === edge.id ? "selected" : ""}" style="stroke-width:${width};${edge.isCrossFolderLink ? "stroke-dasharray:8 6" : ""}" x1="${source.x}" y1="${source.y}" x2="${target.x}" y2="${target.y}"/><title>${escapeHtml(edge.label === null ? "" : edge.label)}</title>${edge.label !== null && dialog.camera.scale >= 0.55 ? `<text class="edge-label" x="${(source.x + target.x) / 2}" y="${(source.y + target.y) / 2 - 8}">${escapeHtml(edge.label)}</text>` : ""}</g>`;
  }
  for (const node of graphData(dialog).graph.nodes) {
    if (!visible.has(node.id)) continue;
    const position = graphEntry(dialog.positions, node.id);
    const size = graphEntry(dialog.sizes, node.id), color = `#${(node.color >>> 0).toString(16).padStart(8, "0").slice(2)}`;
    let lines = "";
    for (let index = 0; index < size.lines.length; index++) lines += `<tspan x="0" y="${(index - (size.lines.length - 1) / 2) * 14.4 + 4}">${escapeHtml(size.lines[index])}</tspan>`;
    nodes += `<g class="graph-node ${dialog.selected === node.id ? "selected" : ""} ${dialog.linkSource === node.id ? "link-source" : ""}" transform="translate(${position.x},${position.y})" data-action="select-memory" data-id="${escapeHtml(node.id)}" role="button" tabindex="0" aria-label="${escapeHtml(node.label)}"><rect x="${-size.width / 2}" y="${-size.height / 2}" width="${size.width}" height="${size.height}" rx="${Math.min(14, size.height / 2)}" style="fill:color-mix(in srgb,${color} 18%,var(--surfaceContainerHigh))"/><text text-anchor="middle">${lines}</text><title>${escapeHtml(node.label)}</title></g>`;
  }
  return `<g transform="translate(${dialog.camera.x},${dialog.camera.y}) scale(${dialog.camera.scale})">${edges}${nodes}</g>`;
}

/** Refreshes graph markup without replacing the active pointer-capture surface. */
export function updateGraphSurface(dialog: GraphDialog, dialogsRoot: HTMLElement): void {
  const surface = dialogsRoot.querySelector<SVGSVGElement>("dialog:last-child [data-graph-surface]");
  const inspector = dialogsRoot.querySelector("dialog:last-child .graph-sidebar");
  if (surface !== null) { initializePositions(dialog); setGraphViewport(dialog, surface); surface.innerHTML = graphSvg(dialog); }
  if (inspector !== null) inspector.innerHTML = graphInspector(dialog);
}
