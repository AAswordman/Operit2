/** Simulator-only presentation adapter. No ToolPkg/JS is executed on firmware. */
import type { RuntimeModule } from './types.ts';
interface Layer {
    asset: string;
    x: number;
    y: number;
    scale: number;
    anchor: string;
    frame: number;
    play: boolean;
    loop: boolean;
    target?: string;
}
interface Asset {
    w: number;
    h: number;
    frames: number;
    ms: number;
    palette: Array<[
        number,
        number
    ]>;
    indices: Uint8Array;
}
export interface SceneView {
    active: boolean;
    revision: number;
    rect: {
        x: number;
        y: number;
        w: number;
        h: number;
    };
    background: number;
    elapsed: number;
    layers: Layer[];
    packs: Array<{
        id: string;
        data: string;
    }>;
}
export function decodePacks(packs: SceneView['packs']): Map<string, Asset> {
    const result = new Map<string, Asset>();
    for (const pack of packs) {
        const bytes = Uint8Array.from(atob(pack.data), c => c.charCodeAt(0));
        const data = new DataView(bytes.buffer);
        let at = 5;
        for (let i = 0; i < bytes[4]; i++) {
            const n = bytes[at++];
            const id = new TextDecoder().decode(bytes.subarray(at, at + n));
            at += n;
            const size = data.getUint16(at, true);
            at += 2;
            const end = at + size;
            const [w, h, frames, count] = bytes.subarray(at + 4, at + 8);
            const ms = data.getUint16(at + 8, true);
            at += 10;
            const palette: Asset['palette'] = [];
            for (let p = 0; p < count; p++, at += 3)
                palette.push([data.getUint16(at, true), bytes[at + 2]]);
            result.set(`${pack.id}:${id}`, { w, h, frames, ms, palette, indices: bytes.subarray(at, end) });
            at = end;
        }
    }
    return result;
}
function blend(bg: number, fg: number, alpha: number): number {
    const mix = (b: number, f: number) => Math.floor((f * alpha + b * (255 - alpha) + 127) / 255);
    return (mix(bg >> 11, fg >> 11) << 11) | (mix((bg >> 5) & 63, (fg >> 5) & 63) << 5) | mix(bg & 31, fg & 31);
}
export function paintScene(view: SceneView, assets: Map<string, Asset>, strip: Uint8Array, y: number, rows: number, elapsed: number): void {
    if (!view.active)
        return;
    const r = view.rect;
    const bytes = new DataView(strip.buffer, strip.byteOffset, strip.byteLength);
    for (let row = 0; row < rows; row++) {
        const sy = y + row;
        if (sy < r.y || sy >= r.y + r.h)
            continue;
        for (let sx = r.x; sx < r.x + r.w; sx++) {
            let color = view.background;
            for (const l of view.layers) {
                const a = assets.get(l.asset);
                if (!a)
                    continue;
                const w = a.w * l.scale, h = a.h * l.scale;
                const left = l.x - (l.anchor === 'tl' ? 0 : Math.floor(w / 2));
                const top = l.y - (l.anchor === 'center' ? Math.floor(h / 2) : l.anchor === 'bc' ? h : 0);
                const x = sx - r.x - left, yy = sy - r.y - top;
                if (x < 0 || yy < 0 || x >= w || yy >= h)
                    continue;
                let frame = l.frame;
                if (l.play) {
                    frame += Math.floor(elapsed / a.ms);
                    frame = l.loop ? frame % a.frames : Math.min(frame, a.frames - 1);
                }
                const [fg, alpha] = a.palette[a.indices[frame * a.w * a.h + Math.floor(yy / l.scale) * a.w + Math.floor(x / l.scale)]];
                color = blend(color, fg, alpha);
            }
            bytes.setUint16((row * 320 + sx) * 2, color, true);
        }
    }
}
export class ScenePreview {
    private runtime?: RuntimeModule;
    private pointer?: number;
    private view?: SceneView;
    private assets = new Map<string, Asset>();
    private loadedAt = 0;
    private request = 0;
    private wanted = '';
    update(runtime: RuntimeModule, summary?: {
        active: boolean;
        revision: number;
    }): void {
        if (this.runtime !== runtime) {
            if (this.runtime && this.pointer !== undefined)
                this.runtime.removeFunction(this.pointer);
            this.runtime = runtime;
            this.pointer = runtime.addFunction((p, length, y, rows, tick) => {
                if (this.view)
                    paintScene(this.view, this.assets, runtime.HEAPU8.subarray(p, p + length), y, rows, tick);
            }, 'viiiiii');
            runtime.ccall('operit_ui_scene_painter', null, ['number'], [this.pointer]);
            this.wanted = '';
            this.view = undefined;
        }
        const key = `${summary?.active ?? false}:${summary?.revision ?? 0}`;
        if (key === this.wanted)
            return;
        this.wanted = key;
        const request = ++this.request;
        if (!summary?.active) {
            this.view = undefined;
            runtime.ccall('operit_ui_set_scene', null, ['number', 'number', 'number'], [0, summary?.revision ?? 0, 0]);
            return;
        }
        void fetch('/api/simulator/scene-view').then(r => { if (!r.ok)
            throw new Error('Scene preview unavailable'); return r.json(); }).then((view: SceneView | null) => {
            if (request !== this.request || this.runtime !== runtime || !view)
                return;
            this.assets = decodePacks(view.packs);
            this.view = view;
            this.loadedAt = performance.now();
            this.tick();
        }).catch(() => { if (request === this.request)
            this.wanted = ''; });
    }
    tick(): void {
        if (this.runtime && this.view)
            this.runtime.ccall('operit_ui_set_scene', null, ['number', 'number', 'number'], [this.view.active ? 1 : 0, this.view.revision, Math.floor(this.view.elapsed + performance.now() - this.loadedAt) >>> 0]);
    }
}
