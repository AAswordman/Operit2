import EmbedPDF, { PdfAnnotationIcon, PdfAnnotationSubtype, ZoomMode, type ZoomPlugin, type AnnotationPlugin, type DocumentManagerPlugin, type ExportPlugin, type FormPlugin, type SelectionPlugin } from '@embedpdf/snippet';
import { natural, text } from '../src/protocol';
import { pageText, type Editor, type EditorOptions } from './editor';

export async function mountPdf(options: EditorOptions): Promise<Editor> {
  const viewer = EmbedPDF.init({ type: 'container', target: options.root, wasmUrl: new URL('pdfium.wasm', location.href).href, worker: false,
    fontFallback: null, fonts: { ui: null, signature: null }, tabBar: 'never',
    disabledCategories: ['document-open', 'document-close'],
  });
  if (!viewer) throw new Error('Could not start PDF viewer');
  const registry = await viewer.registry;
  const manager = registry.getPlugin<DocumentManagerPlugin>('document-manager')?.provides();
  const selectionApi = registry.getPlugin<SelectionPlugin>('selection')?.provides();
  const annotations = registry.getPlugin<AnnotationPlugin>('annotation')?.provides();
  const exporter = registry.getPlugin<ExportPlugin>('export')?.provides();
  const form = registry.getPlugin<FormPlugin>('form')?.provides();
  const engine = registry.getEngine();
  if (!manager || !selectionApi || !annotations || !exporter || !engine) throw new Error('PDF editing modules are unavailable');
  const loaded = await manager.openDocumentBuffer({ name: options.info.name, buffer: options.bytes.buffer, documentId: options.info.id }).toPromise();
  const document = await loaded.task.toPromise();
  const textSelection = selectionApi.forDocument(document.id);
  const annotation = annotations.forDocument(document.id);
  const save = exporter.forDocument(document.id);
  const zoom = registry.getPlugin<ZoomPlugin>('zoom')?.provides().forDocument(document.id);
  annotation.onAnnotationEvent(event => { if (event.type !== 'loaded' && !event.committed) options.changed(); });
  form?.forDocument(document.id).onFieldValueChange(options.changed);
  async function selection() {
    if (!textSelection.getState().selection) return '';
    return (await textSelection.getSelectedText().toPromise()).join('\n');
  }
  return {
    zoom: zoom && {
      get: () => zoom.getState().currentZoomLevel * 100,
      set: percent => zoom.requestZoom(percent / 100),
      fit: () => zoom.requestZoom(ZoomMode.FitWidth),
    },
    selection,
    async snapshot() {
      await annotation.commit().toPromise();
      return new Uint8Array(await save.saveAsCopy().toPromise());
    },
    async read(section, offset) {
      if (section === 'body') return { pages: document.pages.length };
      if (section === 'selection') return pageText(await selection(), offset);
      if (section.startsWith('page:')) {
        const index = natural(Number(section.slice(5)), 'page') - 1;
        if (!document.pages[index]) throw new Error('Page not found');
        return pageText(await engine.extractText(document, [index]).toPromise(), offset);
      }
      throw new Error('Unknown PDF section');
    },
    async edit(change, assertRevision) {
      if (change.op !== 'note') throw new Error('PDF tool edits add annotations; they do not alter original page text');
      const pageIndex = natural(change.page, 'page') - 1;
      const page = document.pages[pageIndex];
      if (!page) throw new Error('Page not found');
      const contents = text(change.text, 'note', 3000).trim();
      if (!contents) throw new Error('Note cannot be empty');
      const x = natural(change.x ?? 30, 'x');
      const y = natural(change.y ?? 30, 'y');
      if (x + 20 > page.size.width || y + 20 > page.size.height) throw new Error('Note outside page');
      assertRevision();
      const id = crypto.randomUUID();
      annotation.createAnnotation(pageIndex, { id, pageIndex, type: PdfAnnotationSubtype.TEXT, contents, flags: ['print'], icon: PdfAnnotationIcon.Note,
        rect: { origin: { x, y }, size: { width: 20, height: 20 } }, author: 'Operit', strokeColor: '#EAA92D' });
      await annotation.commit().toPromise();
      return { id, page: pageIndex + 1 };
    },
  };
}
