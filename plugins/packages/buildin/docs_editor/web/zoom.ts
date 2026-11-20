import type { DocumentZoom } from './editor';

/** Routes document gestures to the editor's zoom without scaling the toolbar. */
export function mountZoom(root: HTMLElement, zoom: DocumentZoom, captureTouch: boolean) {
  const set = (percent: number, focus?: { x: number; y: number }) => {
    if (Number.isFinite(percent)) zoom.set(Math.min(300, Math.max(10, percent)), focus);
  };

  let gestureStart = 100;
  let gesturing = false;
  let touching = false;
  let wheelZoom = zoom.get();
  let lastWheel = -Infinity;
  if (captureTouch) {
    let startDistance = 0;
    let startZoom = 100;
    let frame: number | undefined;
    let pending: { percent: number; focus: { x: number; y: number } } | undefined;
    const touchPointers = new Map<number, { target: EventTarget; primary: boolean }>();

    /** Applies only the newest touch sample for each paint. */
    function applyPinch() {
      frame = undefined;
      if (pending) set(pending.percent, pending.focus);
      pending = undefined;
    }

    /** Clears the gesture, committing its last sample only on release. */
    function finishPinch(commit: boolean) {
      if (frame !== undefined) cancelAnimationFrame(frame);
      if (commit) applyPinch();
      frame = undefined;
      pending = undefined;
      touching = false;
    }

    /** Measures touch separation in screen coordinates regardless of document scroll. */
    function distance(touches: TouchList) {
      const [first, second] = Array.from(touches);
      return Math.hypot(second.clientX - first.clientX, second.clientY - first.clientY);
    }

    const capture = { passive: false, capture: true };
    root.addEventListener('pointerdown', event => {
      if (event.pointerType === 'touch') {
        touchPointers.set(event.pointerId, { target: event.composedPath()[0], primary: event.isPrimary });
      }
    }, true);
    for (const type of ['pointerup', 'pointercancel'] as const) {
      root.addEventListener(type, event => touchPointers.delete(event.pointerId), true);
    }
    root.addEventListener('touchstart', event => {
      if (touching) {
        event.preventDefault();
        event.stopPropagation();
        if (event.touches.length > 2) finishPinch(false);
        return;
      }
      if (event.touches.length !== 2 || !Array.from(event.touches).every(touch =>
        touch.target instanceof Node && root.contains(touch.target))) return;
      startDistance = distance(event.touches);
      if (startDistance === 0) return;
      startZoom = zoom.get();
      touching = true;
      lastWheel = -Infinity;
      event.preventDefault();
      event.stopPropagation();
      for (const [pointerId, pointer] of touchPointers) {
        pointer.target.dispatchEvent(new PointerEvent('pointercancel', {
          bubbles: true, composed: true, pointerId, pointerType: 'touch', isPrimary: pointer.primary,
        }));
      }
    }, capture);
    root.addEventListener('touchmove', event => {
      if (!touching) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.touches.length !== 2) return;
      const [first, second] = Array.from(event.touches);
      pending = {
        percent: startZoom * distance(event.touches) / startDistance,
        focus: { x: (first.clientX + second.clientX) / 2, y: (first.clientY + second.clientY) / 2 },
      };
      frame ??= requestAnimationFrame(applyPinch);
    }, capture);
    root.addEventListener('touchend', event => {
      if (!touching) return;
      event.preventDefault();
      event.stopPropagation();
      if (event.touches.length < 2) finishPinch(true);
    }, capture);
    root.addEventListener('touchcancel', () => finishPinch(false), capture);
  }

  root.addEventListener('wheel', event => {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    event.stopPropagation();
    if (gesturing || touching) return;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? root.clientHeight : 1;
    const now = performance.now();
    if (now - lastWheel > 200) wheelZoom = zoom.get();
    lastWheel = now;
    // Exponential scaling keeps both high-resolution trackpads and wheel mice smooth.
    // Accumulate sub-percent deltas even if the embedded editor rounds its zoom.
    wheelZoom = Math.min(300, Math.max(10, wheelZoom * Math.exp(-Math.max(-100, Math.min(100, event.deltaY * unit)) * 0.005)));
    set(wheelZoom, { x: event.clientX, y: event.clientY });
  }, { passive: false, capture: true });
  // WebKit emits GestureEvents for a Mac trackpad; Chromium emits ctrl+wheel.
  root.addEventListener('gesturestart', event => {
    event.preventDefault();
    if (touching) return;
    gestureStart = zoom.get();
    lastWheel = -Infinity;
    gesturing = true;
  }, { passive: false });
  root.addEventListener('gesturechange', event => {
    event.preventDefault();
    const scale = (event as Event & { scale: number }).scale;
    if (gesturing && !touching && scale > 0) set(gestureStart * scale);
  }, { passive: false });
  root.addEventListener('gestureend', event => {
    event.preventDefault();
    gesturing = false;
  }, { passive: false });
  root.addEventListener('keydown', event => {
    if (!(event.ctrlKey || event.metaKey) || !['+', '=', '-', '0'].includes(event.key)) return;
    event.preventDefault();
    event.stopPropagation();
    if (event.key === '0') zoom.fit();
    else set(zoom.get() + (event.key === '-' ? -10 : 10));
  }, true);
}
