import type { DocumentInfo } from '../src/protocol';

export type Bytes = Uint8Array<ArrayBuffer>;
export interface Editor {
  zoom?: DocumentZoom;
  selection(): Promise<string>;
  read(section: string, offset: number): Promise<unknown>;
  edit(change: Record<string, unknown>, assertRevision: () => void): Promise<unknown>;
  snapshot(): Promise<Bytes>;
}
export interface DocumentZoom {
  get(): number;
  set(percent: number, focus?: { x: number; y: number }): void;
  fit(): void;
}
export type EditorOptions = {
  root: HTMLElement;
  toolbar: HTMLElement;
  bytes: Bytes;
  info: DocumentInfo;
  chinese: boolean;
  changed: () => void;
  saveBytes: (bytes: Bytes) => Promise<void>;
};

export function pageText(value: string, offset: number, size = 9000) {
  const end = Math.min(value.length, offset + size);
  return { text: value.slice(offset, end), offset, nextOffset: end < value.length ? end : null };
}

export function base64(bytes: Bytes): string {
  let raw = '';
  for (let i = 0; i < bytes.length; i += 16384) raw += String.fromCharCode(...bytes.subarray(i, i + 16384));
  return btoa(raw);
}
