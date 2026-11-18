import type { DocumentZoom } from './editor';

// Use the editor's own zoom so selection, caret and export keep their coordinates.
export function mountZoom(root: HTMLElement, zoom: DocumentZoom) {
  const set = (percent: number) => {
    if (Number.isFinite(percent)) zoom.set(Math.min(300, Math.max(10, percent)));
  };

  let gestureStart = 100;
  let gesturing = false;
  let wheelZoom = zoom.get();
  let lastWheel = -Infinity;
  root.addEventListener('wheel', event => {
    if (!(event.ctrlKey || event.metaKey)) return;
    event.preventDefault();
    event.stopPropagation();
    if (gesturing) return;
    const unit = event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? root.clientHeight : 1;
    const now = performance.now();
    if (now - lastWheel > 200) wheelZoom = zoom.get();
    lastWheel = now;
    // Exponential scaling keeps both high-resolution trackpads and wheel mice smooth.
    // Accumulate sub-percent deltas even if the embedded editor rounds its zoom.
    wheelZoom = Math.min(300, Math.max(10, wheelZoom * Math.exp(-Math.max(-100, Math.min(100, event.deltaY * unit)) * 0.005)));
    set(wheelZoom);
  }, { passive: false, capture: true });
  // WebKit emits GestureEvents for a Mac trackpad; Chromium emits ctrl+wheel.
  root.addEventListener('gesturestart', event => {
    event.preventDefault();
    gestureStart = zoom.get();
    gesturing = true;
  }, { passive: false });
  root.addEventListener('gesturechange', event => {
    event.preventDefault();
    const scale = (event as Event & { scale: number }).scale;
    if (gesturing && scale > 0) set(gestureStart * scale);
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
