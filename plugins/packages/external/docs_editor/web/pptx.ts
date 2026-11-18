import { mountEditor } from '@office-kit/pptx-editor';
import { natural, record, text } from '../src/protocol';
import { pageText, type Editor, type EditorOptions } from './editor';

export async function mountPptx(options: EditorOptions): Promise<Editor> {
  const deck = mountEditor(options.root, { source: options.bytes, fileName: options.info.name, locale: 'en', compact: true, onSave: async bytes => options.saveBytes(new Uint8Array(bytes)) });
  await deck.ready;
  deck.on('change', options.changed);
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
