import { mountEditor } from '@office-kit/pptx-editor';
import { natural, record, text } from '../src/protocol';
import { pageText, type Editor, type EditorOptions } from './editor';

/** Mounts the presentation editor with host actions in its existing toolbar. */
export async function mountPptx(options: EditorOptions): Promise<Editor> {
  const deck = mountEditor(options.root, { source: options.bytes, fileName: options.info.name, locale: 'en', compact: true, status: options.actions, onSave: async bytes => options.saveBytes(new Uint8Array(bytes)) });
  await deck.ready;
  deck.on('change', options.changed);
  // The pinned editor exposes zoom through its accessible status-bar controls.
  // Driving those controls uses its own layout and pointer-coordinate pipeline.
  const surface = options.root.firstElementChild!.shadowRoot!;
  const style = document.createElement('style');
  style.textContent = '.topbar.compact .filename { display: none; } slot[name="office-kit-status"] { display: flex; flex: none; position: sticky; right: 0; margin-inline-start: auto; background: var(--ok-ribbon); color: var(--ok-text); }';
  surface.append(style);
  const zoomSlider = () => surface.querySelector<HTMLInputElement>('input[type="range"][aria-label="Zoom percentage"]');
  let lastZoom = 100;
  const zoomValue = () => {
    const percent = Number.parseFloat(zoomSlider()?.getAttribute('aria-valuetext') ?? '');
    if (Number.isFinite(percent)) lastZoom = percent;
    return lastZoom;
  };
  const inspect = new Set(['listSlides', 'listShapes', 'listLayouts', 'listComments']);
  type ShapeSummary = { shape: { shapeId: number }; text?: string; cells?: string[][]; chart?: { title?: string } };
  async function selection() {
    const selected = deck.selection();
    const shapesBySlide = new Map<string, Map<number, ShapeSummary>>();
    for (const slide of new Set(selected.map(shape => shape.slide))) {
      const shapes = await deck.run('listShapes', { slide }) as ShapeSummary[];
      shapesBySlide.set(slide, new Map(shapes.map(shape => [shape.shape.shapeId, shape])));
    }
    return selected.map(ref => {
      const shape = shapesBySlide.get(ref.slide)?.get(ref.shapeId);
      const content = shape?.text || shape?.cells?.map(row => row.join('\t')).join('\n') || shape?.chart?.title;
      return [`${ref.slideIndex + 1}: ${ref.name} (${ref.shapeId})`, content].filter(Boolean).join('\n');
    }).join('\n\n');
  }
  return {
    zoom: {
      get: zoomValue,
      set: percent => {
        const slider = zoomSlider();
        if (!slider) return;
        const sorting = surface.querySelector('button[aria-label="Slide Sorter"][aria-pressed="true"]');
        const min = sorting ? 20 : 10;
        const max = sorting ? 200 : 400;
        percent = Math.max(min, Math.min(max, percent));
        // Each half of the slider spans the active view's limit to/from 100%.
        slider.value = String(percent <= 100 ? (percent - min) / (100 - min) * 1000 : 1000 + (percent - 100) / (max - 100) * 1000);
        slider.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
      },
      fit: () => surface.querySelector<HTMLButtonElement>('button[aria-label="Fit slide to current window"]')?.click(),
    },
    selection,
    snapshot: async () => new Uint8Array(await deck.snapshot()),
    async read(section, offset) {
      if (section === 'selection') return pageText(await selection(), offset);
      if (section === 'tools') {
        const tools = (await deck.tools()).filter(tool => !inspect.has(tool.name));
        return { tools: tools.slice(offset, offset + 8), nextOffset: offset + 8 < tools.length ? offset + 8 : null };
      }
      const tool = section === 'body' ? 'listSlides' : section.startsWith('shapes:') ? 'listShapes' : null;
      if (!tool) throw new Error('Unknown PPTX section');
      let input: Record<string, unknown> = {};
      if (tool === 'listShapes') {
        const slides = await deck.run('listSlides', {}) as { slide: string }[];
        const selected = slides[natural(Number(section.slice(7)), 'slide')];
        if (!selected) throw new Error('Slide not found');
        input = { slide: selected.slide };
      }
      return pageText(JSON.stringify(await deck.run(tool, input)), offset);
    },
    async edit(change, assertRevision) {
      if (change.op !== 'pptx') throw new Error('Unsupported PPTX edit');
      const name = text(change.name, 'presentation command', 100);
      if (inspect.has(name) || !(await deck.tools()).some(tool => tool.name === name)) throw new Error('Unknown PPTX edit command');
      assertRevision();
      return deck.run(name, record(change.input));
    },
  };
}
