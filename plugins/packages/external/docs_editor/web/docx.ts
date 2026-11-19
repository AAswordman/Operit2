import { SuperDoc } from 'superdoc';
import 'superdoc/style.css';
import { strFromU8, unzipSync } from 'fflate';
import { natural, text } from '../src/protocol';
import { base64, pageText, type Editor, type EditorOptions } from './editor';

const imageEntry = /^word\/media\/[^/]+$/;
const headerEntry = /^word\/(header|footer)\d+\.xml$/;
const w = 'http://schemas.openxmlformats.org/wordprocessingml/2006/main';

/** Mounts the paginated Word editor and exposes view and editing controls. */
export async function mountDocx(options: EditorOptions): Promise<Editor> {
  options.toolbar.hidden = false;
  let live = false;
  let instance!: SuperDoc;
  await new Promise<void>((resolve, reject) => {
    instance = new SuperDoc({
      selector: options.root,
      documents: [{ id: options.info.id, type: 'docx', data: new File([options.bytes], options.info.name, { type: 'application/vnd.openxmlformats-officedocument.wordprocessingml.document' }) }],
      documentMode: 'editing', contained: true, telemetry: { enabled: false },
      zoom: { mode: 'fit-width', fitWidth: { min: 10, max: 100, padding: 26 } },
      user: { name: 'Operit', email: 'workspace@operit.local' },
      toolbar: '#toolbar',
      onReady: () => resolve(),
      onEditorUpdate: () => { if (live) options.changed(); },
      onException: event => { if (!live) reject(event.error ?? new Error('DOCX failed to open')); },
    });
  });
  live = true;
  const api = () => {
    const doc = instance.activeEditor?.doc;
    if (!doc) throw new Error('DOCX editing surface is unavailable');
    return doc;
  };
  const snapshot = async () => new Uint8Array(await (await instance.export({ triggerDownload: false })).arrayBuffer());
  const presentation = instance.activeEditor!.presentationEditor!;
  const selectionOverlay = presentation.overlayElement!;
  const viewport = options.root.querySelector<HTMLElement>('.presentation-editor__viewport')!;
  let anchor: { point: { x: number; y: number }; x: number; y: number } | undefined;

  /** Restores the document coordinate under the gesture at the current zoom. */
  function restoreAnchor() {
    if (!anchor) return;
    const bounds = viewport.getBoundingClientRect();
    options.root.scrollLeft += bounds.left + anchor.x * presentation.zoom - anchor.point.x;
    options.root.scrollTop += bounds.top + anchor.y * presentation.zoom - anchor.point.y;
  }

  const selectionPaint = new MutationObserver(restoreAnchor);

  /** Gives viewport control back to the next non-zoom input. */
  function clearAnchor() {
    anchor = undefined;
    selectionPaint.disconnect();
  }

  for (const event of ['pointerdown', 'keydown', 'wheel']) {
    document.addEventListener(event, clearAnchor, { capture: true, passive: true });
  }

  /** Keeps the point under the gesture stationary as the page scales. */
  function setZoom(percent: number, focus?: { x: number; y: number }) {
    if (percent === instance.getZoom()) return;
    if (focus) {
      const bounds = viewport.getBoundingClientRect();
      anchor = {
        point: focus,
        x: (focus.x - bounds.left) / presentation.zoom,
        y: (focus.y - bounds.top) / presentation.zoom,
      };
      selectionPaint.observe(selectionOverlay, { childList: true, subtree: true });
    } else {
      clearAnchor();
    }
    instance.setZoom(percent);
    queueMicrotask(restoreAnchor);
  }

  /** Restores automatic width fitting and releases the previous zoom anchor. */
  function fitWidth() {
    clearAnchor();
    instance.setZoomMode('fit-width');
  }
  async function selection() {
    return (await api().selection.current({ includeText: true })).text ?? '';
  }
  return {
    zoom: {
      get: () => instance.getZoom(),
      set: setZoom,
      fit: fitWidth,
    },
    selection, snapshot,
    async read(section, offset) {
      if (section === 'body') return pageText(await api().getText({}), offset);
      if (section === 'selection') return pageText(await selection(), offset);
      if (section === 'headers') {
        const parts = unzipSync(await snapshot(), { filter: file => headerEntry.test(file.name) && file.originalSize <= 2 * 1024 * 1024 });
        const value = Object.entries(parts).map(([entry, bytes]) => {
          const xml = new DOMParser().parseFromString(strFromU8(bytes), 'application/xml');
          const paragraphs = Array.from(xml.getElementsByTagNameNS(w, 'p'));
          return `${entry}\n${paragraphs.map(p => Array.from(p.getElementsByTagNameNS(w, 't'), node => node.textContent).join('')).join('\n')}`;
        }).join('\n\n');
        return pageText(value, offset);
      }
      if (section === 'images') {
        const images: { entry: string; bytes: number }[] = [];
        unzipSync(await snapshot(), { filter: file => {
          if (imageEntry.test(file.name)) images.push({ entry: file.name, bytes: file.originalSize });
          return false;
        } });
        return { images: images.slice(offset, offset + 50), nextOffset: offset + 50 < images.length ? offset + 50 : null };
      }
      if (section.startsWith('image:')) {
        const entry = section.slice(6);
        if (!imageEntry.test(entry)) throw new Error('Invalid embedded image entry');
        const image = unzipSync(await snapshot(), { filter: file => file.name === entry && file.originalSize <= 8 * 1024 * 1024 })[entry];
        if (!image) throw new Error('Image does not exist or exceeds 8 MiB');
        return { entry, encoding: 'base64', ...pageText(base64(new Uint8Array(image)), offset, 8000) };
      }
      throw new Error(`Unknown DOCX section: ${section}`);
    },
    async edit(change, assertRevision) {
      const doc = api();
      const expected = text(change.expected, 'expected text', 4000);
      if (!expected.trim()) throw new Error('Provide nonblank exact source text');
      const match = await doc.query.match({ select: { type: 'text', pattern: expected, caseSensitive: true }, limit: 2 });
      const index = change.occurrence === undefined ? undefined : natural(change.occurrence, 'occurrence');
      if (!match.total) throw new Error('Source text not found');
      if (index === undefined && match.total !== 1) throw new Error('Source text is ambiguous; specify a zero-based occurrence');
      const item = (index === undefined || index < 2) ? match.items[index ?? 0] :
        (await doc.query.match({ select: { type: 'text', pattern: expected, caseSensitive: true }, limit: 1, offset: index })).items[0];
      if (!item || item.matchKind !== 'text') throw new Error('Source text occurrence not found');
      assertRevision();
      let receipt: unknown;
      if (change.op === 'replace') {
        receipt = await doc.replace({ target: item.target, text: text(change.text, 'replacement') }, { expectedRevision: match.evaluatedRevision });
      } else if (change.op === 'format') {
        const { bold, italic, underline, align } = change;
        const inline: { bold?: boolean; italic?: boolean; underline?: boolean } = {};
        for (const [key, value] of [['bold', bold], ['italic', italic], ['underline', underline]] as const) {
          if (value === undefined) continue;
          if (typeof value !== 'boolean') throw new Error(`Invalid ${key} flag`);
          inline[key] = value;
        }
        if (align !== undefined) {
          if (Object.keys(inline).length) throw new Error('Apply alignment and inline formatting in separate edits');
          if (align !== 'left' && align !== 'center' && align !== 'right' && align !== 'justify') throw new Error('Invalid paragraph alignment');
          if (item.blocks.length !== 1) throw new Error('Alignment requires text in one paragraph');
          const { nodeType } = item.address;
          if (nodeType !== 'paragraph' && nodeType !== 'heading' && nodeType !== 'listItem') throw new Error('Not a paragraph');
          receipt = await doc.format.paragraph.setAlignment({ target: { ...item.address, nodeType }, alignment: align }, { expectedRevision: match.evaluatedRevision });
        } else {
          if (!Object.keys(inline).length) throw new Error('Specify bold, italic, underline or align');
          receipt = await doc.format.apply({ target: item.target, inline }, { expectedRevision: match.evaluatedRevision });
        }
      } else {
        throw new Error('Unsupported DOCX edit');
      }
      if (receipt && typeof receipt === 'object' && 'success' in receipt && receipt.success === false) throw new Error(`DOCX edit rejected: ${JSON.stringify(receipt)}`);
      return receipt;
    },
  };
}
