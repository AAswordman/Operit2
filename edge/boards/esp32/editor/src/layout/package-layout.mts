import {validate, catalog} from './layout-model.mts';
import {pagesOf, isRecord} from './project-model.mts';
import type {LayoutDocument} from './project-model.mts';

export const packageLimit = 28672;

/** Computes CRC-32 of a byte sequence. Compact, versioned data interpreted by the prebuilt self-drawn UI runtime. No compiler. */
export function crc32(bytes: Uint8Array): number {
  let crc = 0xffffffff;
  for (const b of bytes) {
    crc ^= b;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

/** Packs a validated v2 project into the OUI2 binary the firmware store accepts. */
export function packLayout(doc: unknown): Uint8Array {
  const errors = validate(doc);
  if (errors.length) throw new Error(errors.join('; '));
  if (!isRecord(doc) || doc.version !== 2 || !doc.enabled) throw new Error('部署需要启用 v2 项目页面');
  const project = doc as LayoutDocument;
  const pages = pagesOf(project);
  const data: number[] = [];
  const u8 = (v: number): void => {
    data.push(v & 255);
  };
  const u16 = (v: number): void => {
    u8(v);
    u8(v >>> 8);
  };
  const u32 = (v: number): void => {
    u16(v);
    u16(v >>> 16);
  };
  const str = (s: string | undefined): void => {
    const bytes = new TextEncoder().encode(s || '');
    u8(bytes.length);
    data.push(...bytes);
  };
  const color = (s: string): void => {
    u32(parseInt(s.slice(1), 16));
  };
  u8(pages.length);
  u8(pages.findIndex((p) => p.id === project.entryPage));
  u16(320);
  u16(240);
  for (const page of pages) {
    str(page.id);
    str(page.swipeLeft);
    str(page.swipeRight);
    color(page.background);
    u8(page.nodes.length);
    for (const n of page.nodes) {
      const item = catalog.find((c) => c.type === n.type);
      if (!item) throw new Error('未知组件 ' + n.type);
      u8(item.code);
      u8(n.parent ? page.nodes.findIndex((p) => p.id === n.parent) : 255);
      for (const v of [n.x, n.y, n.w, n.h]) u16(v);
      color(n.color);
      u8(n.radius);
      u8(n.value);
      u8(n.fontSize || 14);
      for (const s of [n.text, n.action, n.longAction, n.binding]) str(s);
    }
  }
  const payload = Uint8Array.from(data);
  const out = new Uint8Array(16 + payload.length);
  const view = new DataView(out.buffer);
  out.set([79, 85, 73, 50]);
  view.setUint32(4, payload.length, true);
  view.setUint32(8, crc32(payload), true);
  view.setUint32(12, 1, true);
  out.set(payload, 16);
  if (out.length > packageLimit) throw new Error(`布局包 ${out.length} 字节超过 ${packageLimit} 字节的设备预算`);
  return out;
}
