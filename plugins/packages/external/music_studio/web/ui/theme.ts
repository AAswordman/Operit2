/** Display-only palette, resolved from the same CSS tokens as the DOM. */
const canvasTokens = {
  paper: '--bg', lane: '--panel', soft: '--soft', line: '--line', lineStrong: '--line-strong',
  ink: '--fg', muted: '--muted', accent: '--accent', green: '--green',
  pianoBlackRow: '--raised', pianoWhiteRow: '--bg', blackKey: '--key-black', blackKeyInk: '--key-black-ink',
} as const;
export const STUDIO_THEME = {} as Record<keyof typeof canvasTokens, string>;
type Brightness = 'light' | 'dark';
declare global {
  interface Window { MusicStudioTheme?: { apply(snapshot: { brightness: string }): void } }
}

/** The host owns brightness in-app; a standalone preview follows system appearance. */
export function followStudioTheme(onChange: () => void): () => void {
  const media = window.matchMedia('(prefers-color-scheme: dark)');
  let hostRevision = 0, disposed = false, retry: ReturnType<typeof setTimeout> | undefined;
  function update(brightness: Brightness): void {
    document.documentElement.dataset.theme = brightness;
    const style = getComputedStyle(document.documentElement);
    for (const key of Object.keys(canvasTokens) as (keyof typeof canvasTokens)[]) STUDIO_THEME[key] = style.getPropertyValue(canvasTokens[key]).trim();
    onChange();
  }
  function apply(snapshot: { brightness: string }): void {
    if (disposed || !snapshot || (snapshot.brightness !== 'light' && snapshot.brightness !== 'dark')) return;
    hostRevision++;
    update(snapshot.brightness);
  }
  const endpoint = { apply };
  window.MusicStudioTheme = endpoint;
  function systemChanged(): void { if (!hostRevision) update(media.matches ? 'dark' : 'light'); }
  media.addEventListener('change', systemChanged);
  // Also refresh Canvas tokens if the user changes contrast accessibility settings.
  const contrast = window.matchMedia('(prefers-contrast: more)');
  function contrastChanged(): void { update(document.documentElement.dataset.theme as Brightness); }
  contrast.addEventListener('change', contrastChanged);
  systemChanged();
  async function bootstrap(attempt = 0): Promise<void> {
    const revision = hostRevision;
    if (disposed) return;
    try {
      if (window.MusicHost?.getTheme) {
        const theme = await window.MusicHost.getTheme();
        // A live host update can arrive while this bridge response is in flight.
        if (!disposed && hostRevision === revision) apply(theme);
        return;
      }
      if (/^(localhost|127\.0\.0\.1|\[::1\])$/.test(location.hostname) && !window.MusicHost) return;
    } catch { /* The native bridge may still be attaching during the first page load. */ }
    if (!disposed && attempt < 8) retry = setTimeout(() => void bootstrap(attempt + 1), 400);
  }
  void bootstrap();
  return () => {
    disposed = true;
    if (retry) clearTimeout(retry);
    media.removeEventListener('change', systemChanged);
    contrast.removeEventListener('change', contrastChanged);
    if (window.MusicStudioTheme === endpoint) delete window.MusicStudioTheme;
  };
}

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
