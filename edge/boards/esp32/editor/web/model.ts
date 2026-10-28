export const WIDTH = 320;
export const HEIGHT = 240;

export interface ViewportRect {
  left: number;
  top: number;
  width: number;
  height: number;
}

export type PixelMode = 'rgb565' | 'rgb332';

/** Converts a client coordinate into the firmware's logical 320 by 240 space. */
export function point(clientX: number, clientY: number, rect: ViewportRect): {x: number; y: number} {
  return {
    x: Math.max(0, Math.min(WIDTH - 1, Math.floor((clientX - rect.left) * WIDTH / rect.width))),
    y: Math.max(0, Math.min(HEIGHT - 1, Math.floor((clientY - rect.top) * HEIGHT / rect.height))),
  };
}

/** Converts an RGB color into the selected display color format. */
export function pixel(r: number, g: number, b: number, mode: PixelMode): [number, number, number] {
  if(mode==='rgb332'){const rr=r>>5,gg=g>>5,bb=b>>6;return [Math.round(rr*255/7),Math.round(gg*255/7),Math.round(bb*255/3)];}
  return [Math.round((r>>3)*255/31),Math.round((g>>2)*255/63),Math.round((b>>3)*255/31)];
}
