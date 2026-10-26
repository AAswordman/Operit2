export interface DualModePalette {
  light: Record<string, string>;
  dark: Record<string, string>;
}

export interface PalettePreset {
  id: string;
  label: string;
  seedHex: string;
}

export const PALETTE_SEED_PRESETS: ReadonlyArray<PalettePreset> = [
  { id: "ocean", label: "蓝色", seedHex: "#36618E" },
  { id: "emerald", label: "绿色", seedHex: "#1E6F50" },
  { id: "amber", label: "紫色", seedHex: "#8B5CF6" },
  { id: "sunset", label: "橙色", seedHex: "#C2593F" },
  { id: "sakura", label: "粉色", seedHex: "#B84A72" },
  { id: "graphite", label: "灰色", seedHex: "#475569" },
];

function clamp(v: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, v));
}

function hexToRgb(hex: string): [number, number, number] {
  const clean = hex.replace(/^#/, "").slice(0, 6);
  const num = parseInt(clean.padEnd(6, "0"), 16);
  if (Number.isNaN(num)) return [54, 97, 142];
  return [(num >> 16) & 255, (num >> 8) & 255, num & 255];
}

function rgbToHsl(r: number, g: number, b: number): [number, number, number] {
  const rn = r / 255;
  const gn = g / 255;
  const bn = b / 255;
  const max = Math.max(rn, gn, bn);
  const min = Math.min(rn, gn, bn);
  let h = 0;
  let s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    switch (max) {
      case rn:
        h = (gn - bn) / d + (gn < bn ? 6 : 0);
        break;
      case gn:
        h = (bn - rn) / d + 2;
        break;
      default:
        h = (rn - gn) / d + 4;
        break;
    }
    h *= 60;
  }
  return [h, s * 100, l * 100];
}

function hslToHex8(h: number, s: number, l: number, alpha = 255): string {
  const hn = ((h % 360) + 360) % 360;
  const sn = clamp(s, 0, 100) / 100;
  const ln = clamp(l, 0, 100) / 100;
  const c = (1 - Math.abs(2 * ln - 1)) * sn;
  const x = c * (1 - Math.abs(((hn / 60) % 2) - 1));
  const m = ln - c / 2;
  let r1 = 0;
  let g1 = 0;
  let b1 = 0;
  if (hn < 60) [r1, g1, b1] = [c, x, 0];
  else if (hn < 120) [r1, g1, b1] = [x, c, 0];
  else if (hn < 180) [r1, g1, b1] = [0, c, x];
  else if (hn < 240) [r1, g1, b1] = [0, x, c];
  else if (hn < 300) [r1, g1, b1] = [x, 0, c];
  else [r1, g1, b1] = [c, 0, x];

  const toHex = (n: number) =>
    clamp(Math.round((n + m) * 255), 0, 255)
      .toString(16)
      .toUpperCase()
      .padStart(2, "0");
  const aHex = clamp(Math.round(alpha), 0, 255)
    .toString(16)
    .toUpperCase()
    .padStart(2, "0");
  return `#${toHex(r1)}${toHex(g1)}${toHex(b1)}${aHex}`;
}

/**
 * Generates a complete Material 3 46-token light + 46-token dark palette
 * from a single seed color (#RRGGBB or #RRGGBBAA).
 */
export function generateDualModePaletteFromSeed(seedHex: string): DualModePalette {
  const [r, g, b] = hexToRgb(seedHex);
  const [h, rawS] = rgbToHsl(r, g, b);
  const pS = clamp(rawS, 28, 78);
  const secH = (h + 14) % 360;
  const secS = clamp(pS * 0.36, 14, 34);
  const terH = (h + 58) % 360;
  const terS = clamp(pS * 0.48, 20, 46);
  const neuH = h;
  const neuS = clamp(pS * 0.12, 4, 14);
  const nvS = clamp(pS * 0.2, 8, 22);

  const light: Record<string, string> = {
    primary: hslToHex8(h, pS, 38),
    onPrimary: "#FFFFFFFF",
    primaryContainer: hslToHex8(h, clamp(pS + 12, 30, 88), 90),
    onPrimaryContainer: hslToHex8(h, pS, 12),
    primaryFixed: hslToHex8(h, clamp(pS + 12, 30, 88), 90),
    primaryFixedDim: hslToHex8(h, clamp(pS + 8, 28, 82), 80),
    onPrimaryFixed: hslToHex8(h, pS, 12),
    onPrimaryFixedVariant: hslToHex8(h, pS, 28),

    secondary: hslToHex8(secH, secS, 39),
    onSecondary: "#FFFFFFFF",
    secondaryContainer: hslToHex8(secH, clamp(secS + 14, 20, 60), 90),
    onSecondaryContainer: hslToHex8(secH, secS, 13),
    secondaryFixed: hslToHex8(secH, clamp(secS + 14, 20, 60), 90),
    secondaryFixedDim: hslToHex8(secH, clamp(secS + 10, 18, 52), 80),
    onSecondaryFixed: hslToHex8(secH, secS, 13),
    onSecondaryFixedVariant: hslToHex8(secH, secS, 29),

    tertiary: hslToHex8(terH, terS, 40),
    onTertiary: "#FFFFFFFF",
    tertiaryContainer: hslToHex8(terH, clamp(terS + 16, 26, 72), 91),
    onTertiaryContainer: hslToHex8(terH, terS, 14),
    tertiaryFixed: hslToHex8(terH, clamp(terS + 16, 26, 72), 91),
    tertiaryFixedDim: hslToHex8(terH, clamp(terS + 10, 22, 62), 81),
    onTertiaryFixed: hslToHex8(terH, terS, 14),
    onTertiaryFixedVariant: hslToHex8(terH, terS, 30),

    error: "#BA1A1AFF",
    onError: "#FFFFFFFF",
    errorContainer: "#FFDAD6FF",
    onErrorContainer: "#410002FF",

    surface: hslToHex8(neuH, neuS, 98),
    onSurface: hslToHex8(neuH, neuS, 11),
    surfaceDim: hslToHex8(neuH, neuS, 87),
    surfaceBright: hslToHex8(neuH, neuS, 98),
    surfaceContainerLowest: "#FFFFFFFF",
    surfaceContainerLow: hslToHex8(neuH, neuS, 96),
    surfaceContainer: hslToHex8(neuH, neuS, 94),
    surfaceContainerHigh: hslToHex8(neuH, neuS, 92),
    surfaceContainerHighest: hslToHex8(neuH, neuS, 90),
    onSurfaceVariant: hslToHex8(neuH, nvS, 29),
    outline: hslToHex8(neuH, nvS, 47),
    outlineVariant: hslToHex8(neuH, nvS, 79),
    shadow: "#000000FF",
    scrim: "#000000FF",
    inverseSurface: hslToHex8(neuH, neuS, 19),
    onInverseSurface: hslToHex8(neuH, neuS, 95),
    inversePrimary: hslToHex8(h, clamp(pS + 8, 28, 82), 80),
    surfaceTint: hslToHex8(h, pS, 38),
  };

  const dark: Record<string, string> = {
    primary: hslToHex8(h, clamp(pS + 8, 28, 82), 80),
    onPrimary: hslToHex8(h, pS, 18),
    primaryContainer: hslToHex8(h, pS, 28),
    onPrimaryContainer: hslToHex8(h, clamp(pS + 12, 30, 88), 90),
    primaryFixed: hslToHex8(h, pS, 24),
    primaryFixedDim: hslToHex8(h, pS, 19),
    onPrimaryFixed: hslToHex8(h, clamp(pS + 12, 30, 88), 92),
    onPrimaryFixedVariant: hslToHex8(h, clamp(pS + 8, 24, 76), 78),

    secondary: hslToHex8(secH, clamp(secS + 10, 18, 52), 80),
    onSecondary: hslToHex8(secH, secS, 20),
    secondaryContainer: hslToHex8(secH, secS, 29),
    onSecondaryContainer: hslToHex8(secH, clamp(secS + 14, 20, 60), 90),
    secondaryFixed: hslToHex8(secH, secS, 22),
    secondaryFixedDim: hslToHex8(secH, secS, 18),
    onSecondaryFixed: hslToHex8(secH, clamp(secS + 14, 20, 60), 92),
    onSecondaryFixedVariant: hslToHex8(secH, clamp(secS + 10, 18, 52), 78),

    tertiary: hslToHex8(terH, clamp(terS + 10, 22, 62), 81),
    onTertiary: hslToHex8(terH, terS, 18),
    tertiaryContainer: hslToHex8(terH, terS, 30),
    onTertiaryContainer: hslToHex8(terH, clamp(terS + 16, 26, 72), 91),
    tertiaryFixed: hslToHex8(terH, terS, 24),
    tertiaryFixedDim: hslToHex8(terH, terS, 28),
    onTertiaryFixed: hslToHex8(terH, clamp(terS + 16, 26, 72), 92),
    onTertiaryFixedVariant: hslToHex8(terH, clamp(terS + 10, 22, 62), 78),

    error: "#FFB4ABFF",
    onError: "#690005FF",
    errorContainer: "#93000AFF",
    onErrorContainer: "#FFDAD6FF",

    surface: hslToHex8(neuH, neuS, 8),
    onSurface: hslToHex8(neuH, neuS, 90),
    surfaceDim: hslToHex8(neuH, neuS, 8),
    surfaceBright: hslToHex8(neuH, neuS, 23),
    surfaceContainerLowest: hslToHex8(neuH, neuS, 5),
    surfaceContainerLow: hslToHex8(neuH, neuS, 11),
    surfaceContainer: hslToHex8(neuH, neuS, 13),
    surfaceContainerHigh: hslToHex8(neuH, neuS, 17),
    surfaceContainerHighest: hslToHex8(neuH, neuS, 21),
    onSurfaceVariant: hslToHex8(neuH, nvS, 79),
    outline: hslToHex8(neuH, nvS, 58),
    outlineVariant: hslToHex8(neuH, nvS, 29),
    shadow: "#000000FF",
    scrim: "#000000FF",
    inverseSurface: hslToHex8(neuH, neuS, 90),
    onInverseSurface: hslToHex8(neuH, neuS, 19),
    inversePrimary: hslToHex8(h, pS, 38),
    surfaceTint: hslToHex8(h, clamp(pS + 8, 28, 82), 80),
  };

  return { light, dark };
}

/**
 * Extracts a vibrant representative seed color (#RRGGBB) from an image URL (Blob URL or Data URL).
 */
export async function extractSeedColorFromImageUrl(imageUrl: string): Promise<string> {
  return new Promise<string>((resolve, reject) => {
    const img = new Image();
    img.crossOrigin = "anonymous";
    img.onload = () => {
      try {
        const canvas = document.createElement("canvas");
        const size = 40;
        canvas.width = size;
        canvas.height = size;
        const ctx = canvas.getContext("2d");
        if (!ctx) {
          resolve("#36618E");
          return;
        }
        ctx.drawImage(img, 0, 0, size, size);
        const data = ctx.getImageData(0, 0, size, size).data;
        let weightedR = 0;
        let weightedG = 0;
        let weightedB = 0;
        let totalWeight = 0;

        for (let i = 0; i < data.length; i += 4) {
          const r = data[i];
          const g = data[i + 1];
          const b = data[i + 2];
          const a = data[i + 3];
          if (a < 128) continue;
          const [, s, l] = rgbToHsl(r, g, b);
          // Favor mid-lightness, saturated pixels so muddy black/white doesn't dominate
          const lightnessFactor = 1 - Math.abs(l - 50) / 55;
          const weight = Math.max(0.15, (s / 100) * Math.max(0.2, lightnessFactor));
          weightedR += r * weight;
          weightedG += g * weight;
          weightedB += b * weight;
          totalWeight += weight;
        }

        if (totalWeight <= 0) {
          resolve("#36618E");
          return;
        }
        const finalR = clamp(Math.round(weightedR / totalWeight), 0, 255);
        const finalG = clamp(Math.round(weightedG / totalWeight), 0, 255);
        const finalB = clamp(Math.round(weightedB / totalWeight), 0, 255);
        const hex =
          "#" +
          [finalR, finalG, finalB]
            .map(v => v.toString(16).toUpperCase().padStart(2, "0"))
            .join("");
        resolve(hex);
      } catch (err) {
        reject(err);
      }
    };
    img.onerror = () => reject(new Error("无法加载背景图以提取主题色"));
    img.src = imageUrl;
  });
}
