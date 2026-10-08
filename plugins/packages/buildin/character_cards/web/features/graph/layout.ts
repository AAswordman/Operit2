import type { MemoryGraph } from "../../../src/model";

export interface Point { x: number; y: number }
export interface NodeSize { width: number; height: number; lines: string[] }
export interface GraphBounds { left: number; top: number; width: number; height: number }
export interface GraphCamera extends Point { scale: number }
export interface GraphScene { positions: Map<string, Point>; sizes: Map<string, NodeSize>; bounds: GraphBounds }

/** Requires an existing graph entry rather than inventing coordinates or adjacency. */
export function graphEntry<T>(map: ReadonlyMap<string, T>, id: string): T {
  const value = map.get(id);
  if (value === undefined) throw new Error(`Missing graph entry: ${id}`);
  return value;
}

"use strict";

/** Measures native-sized graph node labels with the document's actual typeface. */
export function measureMemoryNode(label: string): NodeSize {
  const canvas = document.createElement("canvas");
  const context = canvas.getContext("2d");
  if (context === null) throw new Error("Graph text measurement is unavailable");
  context.font = `12px ${getComputedStyle(document.body).fontFamily}`;
  const lines = [""];
  for (const character of label) {
    const index = lines.length - 1;
    if (context.measureText(lines[index] + character).width > 240 && lines[index] !== "") {
      if (lines.length === 2) {
        while (context.measureText(lines[index] + "…").width > 240) lines[index] = lines[index].slice(0, -1);
        lines[index] += "…"; break;
      }
      lines.push(character);
    } else lines[index] += character;
  }
  let width = 0;
  for (const line of lines) width = Math.max(width, context.measureText(line).width);
  return { width: Math.max(56, width + 28), height: Math.max(28, lines.length * 14.4 + 12), lines };
}

/** Packs graph components using the original Flutter MemoryGraphCanvas algorithm. */
export function layoutMemoryGraph(nodes: MemoryGraph["nodes"], edges: MemoryGraph["edges"], measure: (label: string) => NodeSize): GraphScene {
  const adjacency = new Map<string, string[]>(), sizes = new Map<string, NodeSize>();
  for (const node of nodes) { adjacency.set(node.id, []); sizes.set(node.id, measure(node.label)); }
  for (const edge of edges) {
    if (edge.isCrossFolderLink) continue;
    if (!adjacency.has(edge.sourceId) || !adjacency.has(edge.targetId)) throw new Error("Invalid graph relationship endpoint");
    graphEntry(adjacency, edge.sourceId).push(edge.targetId); graphEntry(adjacency, edge.targetId).push(edge.sourceId);
  }
  const visited = new Set<string>(), clusters: string[][] = [];
  for (const node of nodes) {
    if (visited.has(node.id)) continue;
    visited.add(node.id); const cluster = [node.id];
    for (let index = 0; index < cluster.length; index++) {
      for (const neighbor of graphEntry(adjacency, cluster[index])) {
        if (!visited.has(neighbor)) { visited.add(neighbor); cluster.push(neighbor); }
      }
    }
    /** Orders each component by degree and then stable identifier, as in Flutter. */
    cluster.sort((a, b) => graphEntry(adjacency, b).length - graphEntry(adjacency, a).length || (a < b ? -1 : a > b ? 1 : 0));
    clusters.push(cluster);
  }
  /** Packs the largest connected components first. */
  clusters.sort((a, b) => b.length - a.length);
  const columns = Math.max(1, Math.ceil(Math.sqrt(clusters.length))), positions = new Map<string, Point>();
  let x = 0, y = 0, rowHeight = 0;
  for (let index = 0; index < clusters.length; index++) {
    const ids = clusters[index], local = new Map([[ids[0], { x: 0, y: 0 }]]);
    let diameter = 0;
    for (const id of ids) { const size = graphEntry(sizes, id); diameter = Math.max(diameter, Math.hypot(size.width, size.height)); }
    diameter += 32;
    let placed = 1, radius = diameter;
    while (placed < ids.length) {
      const clearance = diameter * 1.12;
      const capacity = Math.max(2, Math.floor(Math.PI / Math.asin(Math.max(0, Math.min(1, clearance / (2 * radius))))));
      const count = Math.min(capacity, ids.length - placed);
      for (let j = 0; j < count; j++) {
        const angle = -Math.PI / 2 + 2 * Math.PI * j / count;
        local.set(ids[placed + j], { x: Math.cos(angle) * radius, y: Math.sin(angle) * radius });
      }
      placed += count; radius += clearance;
    }
    const bounds = memoryGraphBounds(local, sizes);
    if (index % columns === 0 && index > 0) { x = 0; y += rowHeight + 56; rowHeight = 0; }
    for (const id of ids) { const point = graphEntry(local, id); positions.set(id, { x: point.x + x - bounds.left, y: point.y + y - bounds.top }); }
    x += bounds.width + 56; rowHeight = Math.max(rowHeight, bounds.height);
  }
  return { positions, sizes, bounds: memoryGraphBounds(positions, sizes) };
}

/** Computes actual node rectangles without allocating guessed cluster-cell sizes. */
export function memoryGraphBounds(positions: ReadonlyMap<string, Point>, sizes: ReadonlyMap<string, NodeSize>): GraphBounds {
  if (positions.size === 0) return { left: 0, top: 0, width: 1, height: 1 };
  let left = Infinity, top = Infinity, right = -Infinity, bottom = -Infinity;
  for (const [id, point] of positions) {
    const size = graphEntry(sizes, id);
    left = Math.min(left, point.x - size.width / 2); top = Math.min(top, point.y - size.height / 2);
    right = Math.max(right, point.x + size.width / 2); bottom = Math.max(bottom, point.y + size.height / 2);
  }
  return { left, top, width: right - left, height: bottom - top };
}

/** Fits actual graph bounds with the original forty-pixel viewport padding. */
export function fitMemoryGraph(bounds: GraphBounds, width: number, height: number): GraphCamera {
  const scale = Math.max(0.1, Math.min(1, Math.max(1, width - 80) / bounds.width, Math.max(1, height - 80) / bounds.height));
  return { scale, x: width / 2 - (bounds.left + bounds.width / 2) * scale, y: height / 2 - (bounds.top + bounds.height / 2) * scale };
}

/** Preserves the world point under the cursor at the original 0.1–5 zoom limits. */
export function zoomMemoryGraph(camera: GraphCamera, focal: Point, factor: number): GraphCamera {
  const anchor = { x: (focal.x - camera.x) / camera.scale, y: (focal.y - camera.y) / camera.scale };
  const scale = Math.max(0.1, Math.min(5, camera.scale * factor));
  return { scale, x: focal.x - anchor.x * scale, y: focal.y - anchor.y * scale };
}
