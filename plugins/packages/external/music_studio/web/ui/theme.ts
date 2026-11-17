/** Display-only palette. Saved project colors are preserved for imports and AI tools. */
export const STUDIO_THEME = {
  paper: '#faf9f5', lane: '#fffefa', soft: '#f0eee6', line: '#e8e6dc',
  lineStrong: '#d2cec1', ink: '#141413', muted: '#716e64', accent: '#d97757',
  pianoBlackRow: '#f0eee6', pianoWhiteRow: '#faf9f5', blackKey: '#30302c',
} as const;

export function trackInk(color: string): string {
  const match = /^#([\da-f]{6})$/i.exec(color);
  if (!match) return STUDIO_THEME.accent;
  const value = Number.parseInt(match[1], 16);
  const r = ((value >> 16) & 255) / 255, g = ((value >> 8) & 255) / 255, b = (value & 255) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b), delta = max - min;
  if (delta < .05) return '#807b6e';
  let hue = (max === r ? (g - b) / delta : max === g ? (b - r) / delta + 2 : (r - g) / delta + 4) * 60;
  hue = (hue + 360) % 360;
  if (hue < 25 || hue >= 345) return '#b56548';
  if (hue < 65) return '#a5813e';
  if (hue < 155) return '#788c5d';
  if (hue < 200) return '#568b89';
  if (hue < 255) return '#6a9bcc';
  if (hue < 295) return '#88739e';
  return '#a86f8d';
}
