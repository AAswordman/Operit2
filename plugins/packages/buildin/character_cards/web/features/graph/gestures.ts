import type { EditorContext } from "../../bridge/context";
import type { UIState } from "../../bridge/contracts";
import { dataValue, requireElement } from "../../shared/dom";
import { graphEntry, zoomMemoryGraph } from "./layout";
/** Owns pointer, wheel and keyboard gestures without replacing the pressed SVG target. */
export function createGraphGestures(context: EditorContext) {
  const { state, topDialog, updateGraphSurface, dialogsRoot, onClick } = context;
  /** Requires the exact active pointer rather than synthesizing a gesture. */
  function pointerById(id: number) {
    const pointer = state.pointers.get(id);
    if (pointer === undefined) throw new Error("图谱指针状态不一致");
    return pointer;
  }
  /** Tracks graph presses without capturing or replacing the native click target. */
  function onPointerDown(event: PointerEvent): void {
    if (!(event.target instanceof Element) || state.dialogs.length === 0 || state.busy) return;
    const surface = event.target.closest<SVGSVGElement>("[data-graph-surface]");
    if (surface === null || event.button !== 0) return;
    const action = event.target.closest<HTMLElement | SVGElement>("[data-action]");
    if (action !== null && action.dataset.action !== "select-memory" && action.dataset.action !== "select-edge") return;
    const dialog = topDialog("graph"), nodeId = action !== null && action.dataset.action === "select-memory" ? dataValue(action, "id") : null;
    const nodePosition = nodeId === null ? null : graphEntry(dialog.positions, nodeId);
    if (nodeId !== null && nodePosition === undefined) throw new Error("无法定位拖动的记忆节点");
    if (state.pointers.size === 0) state.gestureMoved = false;
    state.pointers.set(event.pointerId, {
      surface, node: nodeId === null || nodePosition === null ? null : { id: nodeId, start: { x: nodePosition.x, y: nodePosition.y } },
      startX: event.clientX, startY: event.clientY, x: event.clientX, y: event.clientY,
    });
  }

  /** Starts capture only for actual drag or pinch gestures and preserves click-only presses. */
  function onPointerMove(event: PointerEvent): void {
    if (!state.pointers.has(event.pointerId)) return;
    const pointer = pointerById(event.pointerId), previous = [...state.pointers.values()];
    const before = previous.map(
      /** Captures previous screen coordinates before updating the current pointer. */
      point => ({ x: point.x, y: point.y }),
    );
    pointer.x = event.clientX; pointer.y = event.clientY;
    const after = [...state.pointers.values()], dialog = topDialog("graph"), wasMoved = state.gestureMoved;
    if (after.length === 1 && !wasMoved && Math.hypot(pointer.x - pointer.startX, pointer.y - pointer.startY) <= 6) return;
    state.gestureMoved = true;
    for (const [pointerId, active] of state.pointers) {
      if (!active.surface.hasPointerCapture(pointerId)) active.surface.setPointerCapture(pointerId);
    }
    event.preventDefault();
    if (after.length === 1) {
      if (pointer.node !== null) {
        dialog.positions.set(pointer.node.id, {
          x: pointer.node.start.x + (pointer.x - pointer.startX) / dialog.camera.scale,
          y: pointer.node.start.y + (pointer.y - pointer.startY) / dialog.camera.scale,
        });
      } else {
        dialog.camera.x += pointer.x - (wasMoved ? before[0].x : pointer.startX);
        dialog.camera.y += pointer.y - (wasMoved ? before[0].y : pointer.startY);
      }
    } else {
      const start = { x: (before[0].x + before[1].x) / 2, y: (before[0].y + before[1].y) / 2 };
      const end = { x: (after[0].x + after[1].x) / 2, y: (after[0].y + after[1].y) / 2 };
      const distance = Math.hypot(before[0].x - before[1].x, before[0].y - before[1].y);
      const currentDistance = Math.hypot(after[0].x - after[1].x, after[0].y - after[1].y);
      dialog.camera.x += end.x - start.x; dialog.camera.y += end.y - start.y;
      if (distance > 0 && currentDistance > 0) {
        const bounds = pointer.surface.getBoundingClientRect();
        dialog.camera = zoomMemoryGraph(dialog.camera, { x: end.x - bounds.left, y: end.y - bounds.top }, currentDistance / distance);
      }
    }
    updateGraphSurface(dialog);
  }

  /** Releases only the pointer capture owned by the ending native-style gesture. */
  function onPointerUp(event: PointerEvent): void {
    if (!state.pointers.has(event.pointerId)) return;
    const pointer = pointerById(event.pointerId); state.pointers.delete(event.pointerId);
    if (pointer.surface.hasPointerCapture(event.pointerId)) pointer.surface.releasePointerCapture(event.pointerId);
  }

  /** Applies bounded graph zoom without intercepting ordinary editor scrolling. */
  function onWheel(event: WheelEvent): void {
    if (!(event.target instanceof Element) || event.target.closest<SVGSVGElement>("[data-graph-surface]") === null) return;
    event.preventDefault();
    const dialog = topDialog("graph"), bounds = requireElement(event.target.closest<SVGSVGElement>("[data-graph-surface]"), SVGSVGElement).getBoundingClientRect(); dialog.camera = zoomMemoryGraph(dialog.camera, { x: event.clientX - bounds.left, y: event.clientY - bounds.top }, Math.exp(-event.deltaY * 0.0015));
    updateGraphSurface(dialog);
  }

  /** Activates keyboard-accessible graph nodes with the same finite action handler. */
  function onKeyDown(event: KeyboardEvent): void {
    if (event.key === "Enter" && event.target instanceof Element && event.target.matches("[data-graph-search]")) {
      event.preventDefault(); requireElement(dialogsRoot.querySelector("dialog:last-child [data-action=search-graph]"), HTMLButtonElement).click(); return;
    }
    if ((event.key === "Enter" || event.key === " ") && event.target instanceof Element && event.target.matches(".graph-node,.graph-edge-action")) { event.preventDefault(); onClick(event); }
  }
  return { onPointerDown, onPointerMove, onPointerUp, onWheel, onKeyDown };
}

/** Releases graph-owned pointer captures when their modal surface is closed. */
export function discardGraphPointers(state: UIState): void {
  for (const [pointerId, pointer] of state.pointers) {
    if (pointer.surface.hasPointerCapture(pointerId)) pointer.surface.releasePointerCapture(pointerId);
  }
  state.pointers.clear();
  state.gestureMoved = false;
}
