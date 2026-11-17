import type { Project } from "../../src/shared/model";
export function encodeWav(channels: Float32Array[], rate: number): Uint8Array {
  const count = channels.length, frames = channels[0].length;
  const bytes = new Uint8Array(44 + frames * count * 2), v = new DataView(bytes.buffer);
  const string = (offset: number, value: string): void => { for (let i = 0; i < value.length; i++) bytes[offset + i] = value.charCodeAt(i); };
  string(0, "RIFF"); v.setUint32(4, bytes.length - 8, true); string(8, "WAVE"); string(12, "fmt ");
  v.setUint32(16, 16, true); v.setUint16(20, 1, true); v.setUint16(22, count, true); v.setUint32(24, rate, true);
  v.setUint32(28, rate * count * 2, true); v.setUint16(32, count * 2, true); v.setUint16(34, 16, true);
  string(36, "data"); v.setUint32(40, bytes.length - 44, true);
  for (let i = 0; i < frames; i++) for (let c = 0; c < count; c++) {
    const x = channels[c][i], s = Number.isFinite(x) ? Math.max(-1, Math.min(1, x)) : 0;
    v.setInt16(44 + (i * count + c) * 2, Math.round(s * (s < 0 ? 32768 : 32767)), true);
  }
  return bytes;
}

export interface AudioAnalysis { rms: [number, number]; correlation: number; sideRms: number; midRms: number; maxVoices: number; sections: { name: string; rms: number; peak: number }[] }
export function analyzeAudio(buffer: {getChannelData(c: number): Float32Array; sampleRate: number}, project: Project, maxVoices: number): AudioAnalysis {
  const left = buffer.getChannelData(0), right = buffer.getChannelData(1); let ll = 0, rr = 0, lr = 0, side = 0, mid = 0;
  for (let i = 0; i < left.length; i++) { const l = left[i], r = right[i]; ll += l * l; rr += r * r; lr += l * r; side += ((l - r) / 2) ** 2; mid += ((l + r) / 2) ** 2; }
  const sections = project.sections.map(section => {
    const start = Math.floor(section.start * 60 / project.bpm * buffer.sampleRate), end = Math.min(left.length, Math.floor((section.start + section.length) * 60 / project.bpm * buffer.sampleRate));
    let power = 0, peak = 0; for (let i = start; i < end; i++) { power += (left[i] ** 2 + right[i] ** 2) / 2; peak = Math.max(peak, Math.abs(left[i]), Math.abs(right[i])); }
    return { name: section.name, rms: Math.sqrt(power / Math.max(1, end - start)), peak };
  });
  return { rms: [Math.sqrt(ll / left.length), Math.sqrt(rr / left.length)], correlation: lr / Math.max(1e-20, Math.sqrt(ll * rr)), sideRms: Math.sqrt(side / left.length), midRms: Math.sqrt(mid / left.length), maxVoices, sections };
}
