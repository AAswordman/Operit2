import type { GraphDialog } from "../../bridge/contracts";
import { requireElement } from "../../shared/dom";
import type { GraphCamera } from "./layout";

const cameraFrames = new WeakMap<GraphDialog, number>();

/** Stops navigation at the currently displayed camera so direct input can take over. */
export function cancelGraphCameraAnimation(dialog: GraphDialog): void {
  const frame = cameraFrames.get(dialog);
  if (frame === undefined) return;
  cancelAnimationFrame(frame);
  cameraFrames.delete(dialog);
}

/** Moves the camera smoothly while keeping graph selection and pointer input responsive. */
export function animateGraphCamera(dialog: GraphDialog, target: GraphCamera, root: HTMLElement, finish: () => void): void {
  cancelGraphCameraAnimation(dialog);
  const start = { ...dialog.camera }, started = performance.now(), duration = 320;
  /** Updates only the camera transform and resolves the exact target on the final frame. */
  function tick(now: number): void {
    const surface = root.querySelector<SVGSVGElement>("dialog[open] [data-graph-surface]");
    if (surface === null || surface.dataset.owner !== dialog.ownerKey) {
      cancelGraphCameraAnimation(dialog); return;
    }
    const progress = Math.min(1, Math.max(0, (now - started) / duration));
    const eased = progress * progress * (3 - 2 * progress);
    dialog.camera = progress === 1 ? { ...target } : {
      x: start.x + (target.x - start.x) * eased,
      y: start.y + (target.y - start.y) * eased,
      scale: start.scale * Math.pow(target.scale / start.scale, eased),
    };
    requireElement(surface.firstElementChild, SVGGElement).setAttribute("transform", `translate(${dialog.camera.x},${dialog.camera.y}) scale(${dialog.camera.scale})`);
    if (progress === 1) { cameraFrames.delete(dialog); finish(); }
    else cameraFrames.set(dialog, requestAnimationFrame(tick));
  }
  cameraFrames.set(dialog, requestAnimationFrame(tick));
}
