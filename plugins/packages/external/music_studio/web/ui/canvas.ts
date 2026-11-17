import { STUDIO_THEME as theme, trackInk } from "./theme";
import { type Project, type Track, noteName } from "../../src/shared/model";
import type { PianoViewport } from "./navigation";
import type { WasmAudioEngine } from "../audio/wasm-engine";
export function context(canvas: HTMLCanvasElement): { ctx: CanvasRenderingContext2D; w: number; h: number } {
  const rect = canvas.getBoundingClientRect(); const ratio = Math.min(2, devicePixelRatio || 1); const width = Math.max(1, Math.round(rect.width * ratio)); const height = Math.max(1, Math.round(rect.height * ratio));
  if (canvas.width !== width || canvas.height !== height) { canvas.width = width; canvas.height = height; }
  const ctx = canvas.getContext("2d")!; ctx.setTransform(ratio, 0, 0, ratio, 0, 0); ctx.clearRect(0, 0, rect.width, rect.height); return { ctx, w: rect.width, h: rect.height };
}
export function drawTrack(canvas: HTMLCanvasElement, track: Track, project: Project, beat: number, start = 0, end = project.bars*project.beatsPerBar): void {
  const { ctx, w, h } = context(canvas); const total = end-start;
  ctx.fillStyle = theme.lane; ctx.fillRect(0, 0, w, h);
  for (let b = Math.ceil(start/project.beatsPerBar); b <= Math.floor(end/project.beatsPerBar); b++) { ctx.strokeStyle = b % 2 ? theme.line : theme.lineStrong; ctx.beginPath(); ctx.moveTo((b*project.beatsPerBar-start)/total*w, 0); ctx.lineTo((b*project.beatsPerBar-start)/total*w, h); ctx.stroke(); }
  if (track.notes.length) {
    let lo = 127, hi = 0; for (const n of track.notes) { lo = Math.min(lo, n.pitch); hi = Math.max(hi, n.pitch); }
    ctx.fillStyle = trackInk(track.color); ctx.globalAlpha = track.mute ? 0.2 : 0.8;
    for (const n of track.notes) { if (n.start+n.duration <= start || n.start >= end) continue; const y = 33 + (hi - n.pitch) / Math.max(12, hi - lo) * (h - 47); ctx.fillRect((Math.max(start,n.start)-start) / total * w, y, Math.max(2, (Math.min(end,n.start+n.duration)-Math.max(start,n.start)) / total * w - 1), track.synth.engine === "drums" ? 5 : 4); }
    ctx.globalAlpha = 1;
  }
  ctx.fillStyle = theme.accent; if (beat >= start && beat < end) ctx.fillRect((beat-start) / total * w, 0, 1.5, h);
}
export function pitchBounds(track: Track): [number, number] { let low = track.synth.engine === "drums" ? 35 : 48; let high = track.synth.engine === "drums" ? 57 : 72; for (const n of track.notes) { low = Math.min(low, n.pitch - 1); high = Math.max(high, n.pitch + 1); } return [Math.max(0, low), Math.min(127, high)]; }
export function drawPiano(canvas: HTMLCanvasElement, track: Track, project: Project, beat: number, start: number, end: number, view?: PianoViewport): void {
  const { ctx, w, h } = context(canvas); const [low, high] = pitchBounds(track); const top = view?.top ?? high+1; const rows = view?.rows ?? high-low+1; const rowH = h / rows; const gutter = 42; const total = end - start;
  for (let pitch = Math.max(0,Math.floor(top-rows)); pitch <= Math.min(127,Math.ceil(top)-1); pitch++) {
    const y = (top - 1 - pitch) * rowH; const black = [1, 3, 6, 8, 10].includes(pitch % 12);
    ctx.fillStyle = black ? theme.pianoBlackRow : theme.pianoWhiteRow; ctx.fillRect(gutter, y, w - gutter, rowH);
    ctx.fillStyle = black ? theme.blackKey : theme.lane; ctx.fillRect(0, y, gutter - 2, rowH - 1);
    if (pitch % 12 === 0 || track.synth.engine === "drums") { ctx.fillStyle = black ? theme.paper : theme.muted; ctx.font = "9px monospace"; ctx.fillText(noteName(pitch), 5, y + Math.min(rowH - 1, 10)); }
  }
  const tickStep = Math.max(1,Math.ceil(total*4/Math.max(1,(w-gutter)/6)));
  for (let tick = Math.ceil(start*4/tickStep)*tickStep; tick <= Math.floor(end*4); tick += tickStep) { ctx.strokeStyle = tick % (project.beatsPerBar*4) === 0 ? theme.lineStrong : tick % 4 === 0 ? theme.line : theme.soft; const x = gutter + (tick/4-start) / total * (w-gutter); ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, h); ctx.stroke(); }
  ctx.save(); ctx.beginPath(); ctx.rect(gutter,0,w-gutter,h); ctx.clip();
  for (const note of track.notes) {
    if (note.start + note.duration <= start || note.start >= start + total) continue;
    const x = gutter + Math.max(0, note.start - start) / total * (w - gutter); const width = Math.min(note.start + note.duration, start + total) - Math.max(start, note.start);
    ctx.fillStyle = trackInk(track.color); ctx.globalAlpha = 0.4 + note.velocity * 0.6; ctx.fillRect(x + 1, (top - 1 - note.pitch) * rowH + 1, Math.max(2, width / total * (w - gutter) - 2), Math.max(2, rowH - 2));
  }
  ctx.restore(); ctx.globalAlpha = 1;
  if (beat >= start && beat < start + total) { ctx.fillStyle = theme.accent; ctx.fillRect(gutter + (beat - start) / total * (w - gutter), 0, 1.5, h); }
}
export function drawScope(canvas: HTMLCanvasElement, engine: WasmAudioEngine, mode: "spectrum" | "waveform"): void {
  const { ctx, w, h } = context(canvas);
  ctx.strokeStyle = theme.line; for (let i = 1; i < 4; i++) { ctx.beginPath(); ctx.moveTo(0, h * i / 4); ctx.lineTo(w, h * i / 4); ctx.stroke(); }
  if (mode === "spectrum") {
    const data = engine.spectrum(); const count = 40; const step = w / count;
    for (let i = 0; i < count; i++) { const at = Math.floor(Math.pow(i / count, 2) * (data.length - 1)); const height = Math.max(2, data[at] / 255 * (h - 10)); ctx.fillStyle = i > 29 ? "#788c5d" : theme.accent; ctx.globalAlpha = 0.3 + height / h * 0.7; ctx.fillRect(i * step + 1, h - height, Math.max(1, step - 3), height); } ctx.globalAlpha = 1;
  } else { const data = engine.waveform(); ctx.strokeStyle = theme.accent; ctx.lineWidth = 1.5; ctx.beginPath(); for (let i = 0; i < data.length; i += 2) { const x = i / data.length * w; const y = h / 2 - data[i] * h * 0.46; if (i === 0) ctx.moveTo(x, y); else ctx.lineTo(x, y); } ctx.stroke(); }
}
