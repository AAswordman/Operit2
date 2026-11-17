/** View-only navigation math. No transport, project writes or note editing. */
export interface Point { x: number; y: number }
export interface PianoViewport { start: number; span: number; top: number; rows: number }
export interface TimeBounds { start: number; end: number }
export type ZoomAxis = "time" | "pitch" | "both";
export const clamp = (n: number, min: number, max: number): number => Math.max(min, Math.min(max, n));
export function clampPiano(view: PianoViewport, bounds: TimeBounds): PianoViewport {
  const span = clamp(view.span, Math.min(.25, bounds.end-bounds.start), bounds.end-bounds.start);
  const rows = clamp(view.rows, 4, 128);
  return { start: clamp(view.start, bounds.start, bounds.end-span), span, top: clamp(view.top, rows, 128), rows };
}
/** Normalized anchors may move as well as spread: pinch and translation are one transform. */
export function transformPiano(view: PianoViewport, bounds: TimeBounds, scale: number, before: Point, after: Point, axis: ZoomAxis = "both"): PianoViewport {
  const next = clampPiano({ ...view, span: axis === "pitch" ? view.span : view.span/scale, rows: axis === "time" ? view.rows : view.rows/scale }, bounds);
  next.start = view.start + view.span*before.x - next.span*after.x;
  next.top = view.top - view.rows*before.y + next.rows*after.y;
  return clampPiano(next, bounds);
}
export function panPiano(view: PianoViewport, bounds: TimeBounds, dx: number, dy: number, width: number, height: number): PianoViewport {
  return clampPiano({ ...view, start: view.start + dx/Math.max(1,width)*view.span, top: view.top - dy/Math.max(1,height)*view.rows }, bounds);
}
export function transformTimeline(width: number, left: number, min: number, max: number, scale: number, beforeX: number, afterX = beforeX): { width: number; left: number } {
  const next = clamp(width*scale, min, max);
  return { width: next, left: clamp((left+beforeX)/width*next-afterX, 0, Math.max(0,next-min)) };
}
export function wheelPixels(value: number, mode: number, page: number): number { return value*(mode === 1 ? 16 : mode === 2 ? page : 1); }
export interface NavigationSurface {
  element: HTMLElement; kind: "timeline" | "piano";
  pan(dx: number, dy: number): void;
  zoom(scale: number, before: Point, after: Point, axis: ZoomAxis): void;
}
interface Pointer { point: Point; target: Element }
interface Geometry { center: Point; distance: number }
function geometry(points: Pointer[]): Geometry {
  const a = points[0].point, b = points[1]?.point ?? a;
  return { center: { x: (a.x+b.x)/2, y: (a.y+b.y)/2 }, distance: Math.hypot(a.x-b.x,a.y-b.y) };
}
/** Delegated, non-passive handlers keep pointer captures alive while view transforms change. */
export class ViewNavigation {
  private pointers = new Map<number, Pointer>();
  private surface: NavigationSurface | null = null;
  private previous: Geometry | null = null;
  private origin: Point | null = null;
  private dragged = false;
  private multi = false;
  private suppressUntil = 0;
  constructor(private root: HTMLElement, private resolve: (target: EventTarget | null) => NavigationSurface | null) {
    root.addEventListener("wheel", this.wheel, { passive: false });
    root.addEventListener("pointerdown", this.down);
    window.addEventListener("pointermove", this.move, { passive: false });
    window.addEventListener("pointerup", this.up);
    window.addEventListener("pointercancel", this.cancelPointer);
    window.addEventListener("blur", () => this.cancel());
    root.addEventListener("contextmenu", e => { if (this.pointers.size > 0) e.preventDefault(); });
  }
  /** Call before rebuilding the UI. Detached captures must never become an editing click. */
  cancel(): void {
    if (this.pointers.size && (this.dragged || this.multi)) this.suppressUntil = performance.now()+600;
    for (const [id, p] of this.pointers) this.release(p.target,id);
    this.pointers.clear(); this.surface = null; this.previous = this.origin = null; this.dragged = this.multi = false;
  }
  consumeClick(event: MouseEvent): boolean { return event.detail > 0 && performance.now() < this.suppressUntil; }
  private release(target: Element, id: number): void { try { if (target.hasPointerCapture(id)) target.releasePointerCapture(id); } catch { /* Already detached/cancelled. */ } }
  private wheel = (event: WheelEvent): void => {
    const surface = this.resolve(event.target); if (!surface) return;
    const dx = wheelPixels(event.deltaX,event.deltaMode,surface.element.clientWidth), dy = wheelPixels(event.deltaY,event.deltaMode,surface.element.clientHeight);
    const point = { x: event.clientX, y: event.clientY };
    if (event.ctrlKey || event.metaKey || (event.altKey && surface.kind === "piano")) {
      event.preventDefault();
      surface.zoom(Math.exp(clamp(-dy*.002,-1.5,1.5)),point,point,event.altKey && !event.ctrlKey && !event.metaKey ? "pitch" : "time");
    } else if (event.shiftKey) { event.preventDefault(); surface.pan(dx+dy,0); }
    else if (surface.kind === "piano") { event.preventDefault(); surface.pan(dx,dy); }
    // Ordinary timeline wheel/trackpad scrolling remains native and accessible.
  };
  private down = (event: PointerEvent): void => {
    // A fresh deliberate press is not a ghost click from the previous gesture.
    if (event.pointerType !== "touch" && event.button === 0 && !this.pointers.size) this.suppressUntil = 0;
    if (event.pointerType !== "touch" && !(event.pointerType === "mouse" && event.button === 1)) return;
    const surface = this.resolve(event.target); if (!surface) return;
    if (this.surface && this.surface.element !== surface.element) return;
    if (!this.pointers.size) {
      this.suppressUntil = 0; this.surface = surface; this.dragged = this.multi = false;
      this.origin = { x: event.clientX, y: event.clientY };
    }
    if (event.pointerType === "mouse") { event.preventDefault(); this.dragged = true; }
    const target = surface.kind === "piano" ? surface.element : event.target as Element;
    this.pointers.set(event.pointerId,{ point: { x: event.clientX, y: event.clientY }, target });
    // Capture on the original target: a stationary one-finger tap still opens its clip/button.
    try { target.setPointerCapture(event.pointerId); } catch { /* A browser may cancel capture. */ }
    this.multi ||= this.pointers.size > 1;
    this.previous = geometry([...this.pointers.values()]);
  };
  private move = (event: PointerEvent): void => {
    const pointer = this.pointers.get(event.pointerId), surface = this.surface;
    if (!pointer || !surface || !this.previous) return;
    if (!surface.element.isConnected) { this.cancel(); return; }
    pointer.point = { x: event.clientX, y: event.clientY };
    const now = geometry([...this.pointers.values()].slice(0,2));
    if (this.pointers.size > 1) {
      event.preventDefault();
      const scale = this.previous.distance > 8 && now.distance > 8 ? now.distance/this.previous.distance : 1;
      surface.zoom(scale,this.previous.center,now.center,surface.kind === "piano" ? "both" : "time");
      this.dragged = true;
    } else {
      if (!this.dragged && this.origin && Math.hypot(now.center.x-this.origin.x,now.center.y-this.origin.y) < 6) return;
      event.preventDefault(); surface.pan(this.previous.center.x-now.center.x,this.previous.center.y-now.center.y); this.dragged = true;
    }
    this.previous = now;
  };
  private up = (event: PointerEvent): void => {
    const pointer = this.pointers.get(event.pointerId); if (!pointer) return;
    if (this.dragged || this.multi) this.suppressUntil = performance.now()+600;
    this.pointers.delete(event.pointerId); this.release(pointer.target,event.pointerId);
    if (this.pointers.size) this.previous = geometry([...this.pointers.values()]);
    else { this.surface = null; this.previous = this.origin = null; this.dragged = this.multi = false; }
  };
  private cancelPointer = (event: PointerEvent): void => {
    if (!this.pointers.has(event.pointerId)) return;
    this.suppressUntil = performance.now()+600; this.up(event);
  };
}
