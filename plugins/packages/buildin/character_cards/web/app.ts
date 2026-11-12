import type { MemoryGraphDisplay } from "../src/ui-memory-graph";
import { fitMemoryGraph, graphEntry, layoutMemoryGraph, measureMemoryNode, memoryGraphBounds, zoomMemoryGraph, type GraphCamera, type GraphScene } from "./features/graph/layout";

interface GraphHost {
  load(): Promise<MemoryGraphDisplay>;
  select(kind: "memory" | "edge", id: string): Promise<void>;
}
declare global {
  interface Window {
    MemoryGraphHost: GraphHost;
    updateMemoryGraph(display: MemoryGraphDisplay): void;
    memoryGraphCamera(command: "in" | "out" | "fit"): void;
  }
}
const canvas = document.querySelector("svg");
if (!(canvas instanceof SVGSVGElement)) throw new Error("Missing graph canvas");
const surface = canvas;
let scene: GraphScene | null = null;
let camera: GraphCamera = { x: 0, y: 0, scale: 1 };
let signature = "";
const pointers = new Map<number, { x: number; y: number }>();
let moved = false;
let display: MemoryGraphDisplay | null = null;
let down = { x: 0, y: 0 };
let dragged: string | null = null;
let pressed: { kind: "memory" | "edge"; id: string } | null = null;
let viewport = { width: 0, height: 0 };

/** Creates SVG content without interpreting user labels as HTML. */
function svg<K extends keyof SVGElementTagNameMap>(tag: K, attributes: Record<string, string>, text?: string): SVGElementTagNameMap[K] {
  const element = document.createElementNS("http://www.w3.org/2000/svg", tag);
  for (const [name, value] of Object.entries(attributes)) element.setAttribute(name, value);
  if (text !== undefined) element.textContent = text;
  return element;
}
/** Applies the camera transform to the existing geometry. */
function paintCamera(): void { surface.querySelector("g")?.setAttribute("transform", `translate(${camera.x} ${camera.y}) scale(${camera.scale})`); surface.querySelectorAll<SVGTextElement>(".edge-label").forEach(label => { label.style.display = camera.scale >= 0.55 ? "" : "none"; }); }
/** Fits the current graph using the actual canvas size. */
function fit(): void { if (scene === null) return; camera = fitMemoryGraph(memoryGraphBounds(scene.positions, scene.sizes), surface.clientWidth, surface.clientHeight); paintCamera(); }
/** Reports display bridge failures without exposing mutation APIs. */
function failure(error: unknown): void { const status = document.getElementById("status"); if (status !== null) status.textContent = String(error); }
/** Resolves the canvas point from real client coordinates. */
function point(event: { clientX: number; clientY: number }): { x: number; y: number } { const bounds = surface.getBoundingClientRect(); return { x: event.clientX - bounds.left, y: event.clientY - bounds.top }; }
/** Replaces only display geometry and retains the camera when topology is unchanged. */
window.updateMemoryGraph = function updateMemoryGraph(value: MemoryGraphDisplay): void {

  display = value;
  const palette = value.theme.colors;
  document.documentElement.style.colorScheme = value.theme.brightness;
  for (const role of ["surface", "onSurface", "primary", "onPrimary", "outline", "primaryContainer", "onPrimaryContainer", "surfaceContainerHigh", "onSurfaceVariant", "error"]) document.documentElement.style.setProperty("--" + role, palette[role]);
  const nextSignature = JSON.stringify(value.data.graph);
  const changed = nextSignature !== signature;
  if (changed) { signature = nextSignature; scene = layoutMemoryGraph(value.data.graph.nodes, value.data.graph.edges, measureMemoryNode); }
  if (scene === null) throw new Error("Graph layout was not initialized");
  const group = svg("g", {});
  for (const edge of value.data.graph.edges) {
    const source = graphEntry(scene.positions, edge.sourceId), target = graphEntry(scene.positions, edge.targetId);
    const line = svg("line", { x1: String(source.x), y1: String(source.y), x2: String(target.x), y2: String(target.y), class: "edge" + (value.selectedEdge === edge.id ? " selected" : ""), "stroke-width": String(Math.max(0.7, Math.min(4, edge.weight * (edge.isCrossFolderLink ? 1.6 : 2.4)))), ...(edge.isCrossFolderLink ? { "stroke-dasharray": "8 6" } : {}) });
    const hit = svg("line", { x1: String(source.x), y1: String(source.y), x2: String(target.x), y2: String(target.y), class: "edge-hit", "data-edge": edge.id, tabindex: "0", role: "button", "aria-label": "关系 " + (edge.label ?? "") });
    group.append(line, hit);
    const label = svg("text", { x: String((source.x + target.x) / 2), y: String((source.y + target.y) / 2 - 8), class: "edge-label" }, edge.label ?? ""); group.append(label);
  }
  for (const node of value.data.graph.nodes) {
    const location = graphEntry(scene.positions, node.id), size = graphEntry(scene.sizes, node.id);
    const element = svg("g", { transform: `translate(${location.x} ${location.y})`, class: "node" + (value.selected === node.id ? " selected" : ""), "data-memory": node.id, tabindex: "0", role: "button", "aria-label": node.label });
    element.append(svg("rect", { x: String(-size.width / 2), y: String(-size.height / 2), width: String(size.width), height: String(size.height), rx: String(Math.min(14, size.height / 2)), style: "fill:color-mix(in srgb,#" + (node.color >>> 0).toString(16).padStart(8, "0").slice(2) + " 18%,var(--surfaceContainerHigh))" }));
    size.lines.forEach((line, index) => element.append(svg("text", { x: "0", y: String((index - (size.lines.length - 1) / 2) * 14.4 + 4) }, line)));
    group.append(element);
  }
  surface.replaceChildren(group);
  const status = document.getElementById("status"); if (status !== null) status.textContent = value.data.items.length === 0 ? "暂无记忆" : "";
  if (changed) fit(); else paintCamera();
};
/** Accepts only the three camera operations emitted by native toolbar controls. */
window.memoryGraphCamera = function memoryGraphCamera(command): void {
  if (command === "fit") { fit(); return; }
  camera = zoomMemoryGraph(camera, { x: surface.clientWidth / 2, y: surface.clientHeight / 2 }, command === "in" ? 1.25 : 1 / 1.25); paintCamera();
};
/** Resolves only canvas node and relationship targets. */
function selection(element: EventTarget | null): { kind: "memory" | "edge"; id: string } | null {
  const target = element instanceof Element ? element.closest<SVGElement>("[data-memory], [data-edge]") : null;
  if (target?.dataset.memory !== undefined) return { kind: "memory", id: target.dataset.memory };
  if (target?.dataset.edge !== undefined) return { kind: "edge", id: target.dataset.edge };
  return null;
}
surface.addEventListener("pointerdown", event => {
  const location = point(event);
  if (pointers.size === 0) { moved = false; down = location; pressed = selection(event.target); dragged = pressed?.kind === "memory" ? pressed.id : null; }
  else { moved = true; dragged = null; }
  pointers.set(event.pointerId, location); surface.setPointerCapture(event.pointerId);
});
surface.addEventListener("pointermove", event => {
  const previous = pointers.get(event.pointerId); if (previous === undefined) return;
  const current = point(event), other = [...pointers.entries()].find(([id]) => id !== event.pointerId)?.[1];
  if (Math.hypot(current.x - down.x, current.y - down.y) > 4) moved = true;
  if (other !== undefined) {
    const before = Math.hypot(previous.x - other.x, previous.y - other.y), after = Math.hypot(current.x - other.x, current.y - other.y);
    if (before > 0) camera = zoomMemoryGraph(camera, { x: (previous.x + other.x) / 2, y: (previous.y + other.y) / 2 }, after / before);
    camera = { ...camera, x: camera.x + (current.x - previous.x) / 2, y: camera.y + (current.y - previous.y) / 2 };
  } else if (dragged !== null && scene !== null && moved) {
    const position = graphEntry(scene.positions, dragged);
    scene.positions.set(dragged, { x: position.x + (current.x - previous.x) / camera.scale, y: position.y + (current.y - previous.y) / camera.scale });
    if (display !== null) window.updateMemoryGraph(display);
  } else if (dragged === null) camera = { ...camera, x: camera.x + current.x - previous.x, y: camera.y + current.y - previous.y };
  pointers.set(event.pointerId, current); paintCamera();
});
surface.addEventListener("pointerup", event => {
  if (!moved && pointers.size === 1 && pressed !== null) void window.MemoryGraphHost.select(pressed.kind, pressed.id).catch(failure);
  pointers.delete(event.pointerId); dragged = null; pressed = null;
  if (surface.hasPointerCapture(event.pointerId)) surface.releasePointerCapture(event.pointerId);
});
surface.addEventListener("pointercancel", event => { pointers.delete(event.pointerId); moved = true; dragged = null; pressed = null; });
surface.addEventListener("wheel", event => { event.preventDefault(); const delta = event.deltaY * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? surface.clientHeight : 1); camera = zoomMemoryGraph(camera, point(event), Math.exp(-delta * 0.001)); paintCamera(); }, { passive: false });
surface.addEventListener("keydown", event => { if (event.key !== "Enter" && event.key !== " ") return; const target = selection(event.target); if (target !== null) { event.preventDefault(); void window.MemoryGraphHost.select(target.kind, target.id).catch(failure); } });
new ResizeObserver(() => { const next = { width: surface.clientWidth, height: surface.clientHeight }; if (viewport.width > 0 && viewport.height > 0) { camera.x += (next.width - viewport.width) / 2; camera.y += (next.height - viewport.height) / 2; paintCamera(); } else fit(); viewport = next; }).observe(surface);
void window.MemoryGraphHost.load().then(window.updateMemoryGraph).catch(failure);
