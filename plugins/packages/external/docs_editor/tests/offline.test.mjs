import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { readFile } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { join } from 'node:path';
import { test } from 'node:test';
import { runInNewContext } from 'node:vm';
import { unzipSync, strFromU8 } from 'fflate';

const dependencies = process.env.OFFICE_TEST_NODE_MODULES;
const require = dependencies ? createRequire(join(dependencies, 'office-test.cjs')) : null;
const root = new URL('../dist/web/', import.meta.url);
const files = new Map([['/', ['index.html', 'text/html']], ['/app.js', ['app.js', 'application/javascript']], ['/app.css', ['app.css', 'text/css']], ['/pdfium.wasm', ['pdfium.wasm', 'application/wasm']]]);
const paragraph = 'Replace this exact sentence.';

async function fixtures() {
  const { Document, Packer, Paragraph, Header, Footer, ImageRun, TextRun, HeadingLevel, PageBreak } = require('docx');
  const PptxGen = require('pptxgenjs');
  const { PDFDocument } = require('pdf-lib');
  const png = Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aM1sAAAAASUVORK5CYII=', 'base64');
  const word = new Document({ sections: [{
    headers: { default: new Header({ children: [new Paragraph('Test header')] }) },
    footers: { default: new Footer({ children: [new Paragraph('Test footer')] }) },
    children: [new Paragraph({ text: 'Test title', heading: HeadingLevel.HEADING_1 }),
      new Paragraph({ children: [new TextRun({ text: paragraph, bold: true })] }),
      new Paragraph({ children: [new ImageRun({ type: 'png', data: png, transformation: { width: 20, height: 20 } })] }),
      new Paragraph({ children: [new PageBreak(), new TextRun('Second page')] })],
  }] });
  const deck = new PptxGen();
  deck.addSlide().addText('Slide body', { x: 1, y: 1, w: 4, h: 1 });
  const pdf = await PDFDocument.create();
  const pdfPage = pdf.addPage();
  pdfPage.drawText('PDF body', { x: 60, y: 700 });
  pdf.getForm().createTextField('reviewer').addToPage(pdfPage, { x: 60, y: 650, width: 200, height: 26 });
  return { docx: await Packer.toBuffer(word), pptx: Buffer.from(await deck.write({ outputType: 'arraybuffer' })), pdf: Buffer.from(await pdf.save()) };
}

test('bundled Office tool metadata and registered resources', async () => {
  const archive = unzipSync(await readFile(new URL('../dist/docs_editor.toolpkg', import.meta.url)));
  const manifest = JSON.parse(strFromU8(archive['manifest.json']));
  for (const resource of manifest.resources) assert.ok(archive[resource.path]?.length, resource.key);
  const metadata = strFromU8(archive['dist/tools.js']).match(/\/\* METADATA\s*([\s\S]*?)\*\//);
  const tools = JSON.parse(metadata[1]);
  assert.equal(tools.enabled_by_default, true);
  assert.deepEqual(tools.tools.map(t => t.name), ['list', 'read', 'edit', 'save']);
  const load = (file, globals = {}) => {
    const module = { exports: {} };
    runInNewContext(strFromU8(archive[file]), { module, ...globals });
    return module.exports;
  };
  const screen = load('dist/screen.ui.js');
  const navigation = [];
  const main = load('dist/main.js', {
    require: path => {
      assert.equal(path, './screen.ui.js');
      return screen;
    },
    ToolPkg: {
      ipc: { on() {} },
      registerUiRoute: route => assert.equal(route.screen, screen.default),
      registerNavigationEntry: entry => navigation.push(entry),
    },
  });
  assert.equal(typeof screen.default, 'function');
  assert.equal(main.registerToolPkg(), true);
  assert.equal(navigation[0].surface, 'workspace_file');
  assert.deepEqual([...navigation[0].params.extensions], ['docx', 'pptx', 'pdf']);
});

test('Office bridge ignores empty quotes and returns the exact selected text', async () => {
  const archive = unzipSync(await readFile(new URL('../dist/docs_editor.toolpkg', import.meta.url)));
  const module = { exports: {} };
  runInNewContext(strFromU8(archive['dist/screen.ui.js']), {
    module,
    getLang: () => 'en',
    ToolPkg: { ipc: { on() {} }, readResource: async () => '/asset' },
  });
  let quote;
  const state = { input: { path: '/workspace/test.docx', name: 'test.docx' } };
  const initialize = module.exports.default({
    useState: (key, fallback) => [state[key] ?? fallback, () => {}],
    useRef: (_, current) => ({ current }),
    useEffect() {},
    createWebViewController: () => ({
      addJavascriptInterface(name, methods) {
        assert.equal(name, 'OfficeHost');
        quote = methods.quote;
      },
    }),
    UI: { Box: props => props.onLoad, Text() {} },
  });
  await initialize();
  assert.equal(quote(), null);
  assert.equal(quote([' \r\n\t']), null);
  const text = '  Selected <text> & punctuation.\nNext line  ';
  assert.deepEqual({ ...quote([text]) }, { workspaceFileAction: 'quote', text });
  assert.deepEqual({ ...quote([text, 'quote-1']) }, { workspaceFileAction: 'quote', text });
  assert.equal(quote([text, 'quote-1']), null, 'a retried request cannot create a second attachment');
  assert.deepEqual({ ...quote([text, 'quote-2']) }, { workspaceFileAction: 'quote', text }, 'an explicit new quote is allowed');
});

test('offline DOCX, PPTX and PDF editor/tool round trips', { skip: !require && 'Set OFFICE_TEST_NODE_MODULES to the bundled workspace dependencies', timeout: 100000 }, async () => {
  const data = await fixtures();
  const { chromium, webkit } = require('playwright');
  const browser = await (process.env.OFFICE_WEBKIT ? webkit : chromium).launch({ executablePath: process.env.OFFICE_CHROME });
  const server = createServer(async (request, response) => {
    const file = files.get(request.url);
    if (!file) return response.writeHead(404).end();
    response.writeHead(200, { 'Content-Type': file[1] }).end(await readFile(new URL(file[0], root)));
  });
  try {
    await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
    const origin = `http://127.0.0.1:${server.address().port}`;
    for (const [format, file] of Object.entries(data)) {
      const page = await browser.newPage({ viewport: { width: 900, height: 760 } });
      const external = [];
      const errors = [];
      let stored = null;
      await page.route('**/*', route => {
        const url = route.request().url();
        if (url.startsWith(`${origin}/`)) return route.continue();
        external.push(url);
        return route.abort();
      });
      page.on('console', message => { if (message.type() === 'error') errors.push(message.text()); });
      page.on('pageerror', error => errors.push(String(error)));
      await page.exposeFunction('writeOfficeTestFile', encoded => { stored = Buffer.from(encoded, 'base64'); });
      await page.addInitScript(({ format, data }) => {
        let buffer = '';
        window.OfficeHost = {
          bootstrap: async () => ({ info: { format, id: 'test', path: `/test.${format}`, name: `test.${format}` }, locale: 'en', length: data.length, chunkSize: 1024 }),
          readPart: async offset => data.slice(offset, offset + 1024),
          editorReady: async () => true,
          dirty: async () => true,
          quote: async (quote, requestId) => {
            window.officeQuoteCount = (window.officeQuoteCount ?? 0) + 1;
            window.lastOfficeQuote = quote;
            if (typeof requestId !== 'string') throw new Error('Quote request must have an identity');
          },
          beginSave: async () => { buffer = ''; return true; },
          appendSave: async chunk => { buffer += chunk; return true; },
          commitSave: async () => { await window.writeOfficeTestFile(buffer); return true; },
        };
      }, { format, data: file.toString('base64') });
      await page.goto(origin);
      try {
        await page.waitForFunction(() => window.OfficeEditor || !document.getElementById('error').hidden, null, { timeout: 25000 });
        assert.equal(await page.locator('#error').textContent(), '', `${format}: ${errors.join('\n')}`);
      } catch (error) {
        throw new Error(`${format} did not start: ${errors.join('\n')}`, { cause: error });
      }
      let sequence = 0;
      const command = async (request, expectedFailure = false) => {
        const key = String(++sequence);
        assert.equal(await page.evaluate(({ key, request }) => window.OfficeEditor.submit(key, request), { key, request }), true);
        const handle = await page.waitForFunction(key => window.OfficeEditor.result(key) || false, key, { timeout: 15000 });
        const reply = JSON.parse(await handle.jsonValue());
        assert.equal(reply.ok, !expectedFailure, `${format}: ${JSON.stringify(reply)}\n${errors.join('\n')}`);
        return reply.ok ? reply.value : reply.error;
      };
      const first = await command({ kind: 'read' });
      await testZoom(page, format);
      assert.equal((await command({ kind: 'read' })).revision, first.revision, 'zoom does not edit the document');
      const quote = page.getByRole('button', { name: 'Quote selection in chat' });
      await testEmptyQuote(page, quote, format);
      let edit;
      if (format === 'docx') {
        assert.match(first.data.text, /Replace this exact sentence/);
        assert.match((await command({ kind: 'read', section: 'headers' })).data.text, /Test header[\s\S]*Test footer/);
        const image = (await command({ kind: 'read', section: 'images' })).data.images[0];
        assert.ok(image.bytes > 0);
        assert.ok((await command({ kind: 'read', section: `image:${image.entry}` })).data.text.length > 0);
        edit = { op: 'replace', expected: paragraph, text: 'Updated live by the Office tool.' };
      } else if (format === 'pptx') {
        const shapes = JSON.parse((await command({ kind: 'read', section: 'shapes:0' })).data.text);
        const ref = shapes[0].shape;
        assert.ok(ref?.slide && Number.isInteger(ref.shapeId));
        edit = { op: 'pptx', name: 'setShapeText', input: { shape: ref, value: 'Updated slide' } };
      } else {
        assert.equal(first.data.pages, 1);
        assert.match((await command({ kind: 'read', section: 'page:1' })).data.text, /PDF body/);
        edit = { op: 'note', text: 'Office test note', page: 1 };
      }
      const changed = await command({ kind: 'edit', change: edit, revision: first.revision });
      assert.ok(changed.revision > first.revision);
      assert.match(await command({ kind: 'edit', change: edit, revision: first.revision }, true), /Stale/);
      let saveRevision = changed.revision;
      if (format === 'docx') {
        assert.match((await command({ kind: 'read' })).data.text, /Updated live by the Office tool/);
        const visible = page.getByText('Updated live by the Office tool.', { exact: true }).first();
        await visible.waitFor();
        assert.ok(await page.locator('.superdoc-page').count() >= 2, 'DOCX preserves pagination');
        await visible.click();
        await page.keyboard.press('Home');
        await page.keyboard.press('Shift+End');
        await page.evaluate(() => {
          const quote = document.querySelector('#quote');
          quote.click();
          quote.dispatchEvent(new MouseEvent('click', { bubbles: true }));
        });
        await page.waitForFunction(() => window.lastOfficeQuote === 'Updated live by the Office tool.');
        assert.equal(await page.evaluate(() => window.officeQuoteCount), 1, 'one click attaches one selection');
        assert.equal(await page.getByText('No text selected', { exact: true }).isVisible(), false);
        const formatted = await command({ kind: 'edit', revision: changed.revision, change: {
          op: 'format', expected: 'Updated live by the Office tool.', italic: true,
        } });
        assert.ok(formatted.revision > changed.revision);
        await visible.click();
        await page.keyboard.press('End');
        await page.keyboard.insertText(' Manually edited.');
        await page.waitForFunction(() => document.querySelector('#editor').innerText.includes('Manually edited.'));
        const manual = await command({ kind: 'read' });
        assert.match(manual.data.text, /Manually edited\./);
        saveRevision = manual.revision;
      }
      if (format === 'pdf') {
        const field = page.getByRole('textbox', { name: 'reviewer', exact: true });
        await field.click({ timeout: 5000 });
        await field.pressSequentially('Offline reviewer');
        await page.locator('#filename').click();
        const filled = await command({ kind: 'read' });
        assert.ok(filled.revision > changed.revision, 'PDF form input changes the working copy');
        saveRevision = filled.revision;
      }
      const saved = await command({ kind: 'save', revision: saveRevision });
      assert.equal(saved.dirty, false);
      assert.ok(stored?.length > 0);
      if (format === 'docx') {
        const xml = strFromU8(unzipSync(stored)['word/document.xml']);
        assert.match(xml, /Updated live by the Office tool/);
        assert.match(xml, /Manually edited/);
        assert.match(xml, /<w:i(?:\s|\/|>)/);
      }
      if (format === 'pptx') assert.match(strFromU8(unzipSync(stored)['ppt/slides/slide1.xml']), /Updated slide/);
      if (format === 'pdf') {
        const { PDFArray, PDFDict, PDFDocument, PDFName } = require('pdf-lib');
        const pdf = await PDFDocument.load(stored);
        assert.equal(pdf.getForm().getTextField('reviewer').getText(), 'Offline reviewer');
        const notes = pdf.getPage(0).node.lookup(PDFName.of('Annots'), PDFArray);
        const note = Array.from({ length: notes.size() }, (_, index) => notes.lookup(index, PDFDict))
          .find(annotation => annotation.get(PDFName.of('Subtype')) === PDFName.of('Text'));
        assert.equal(note?.get(PDFName.of('Contents')).decodeText(), 'Office test note');
      }
      assert.deepEqual(external, [], `${format} made an external request`);
      await page.close();
    }
  } finally {
    await browser.close();
    server.close();
  }
});

test('Office opens documents over 24 MiB and reports read failures separately from size limits', async () => {
  const archive = unzipSync(await readFile(new URL('../dist/docs_editor.toolpkg', import.meta.url)));
  const module = { exports: {} };
  let file;
  let host;
  runInNewContext(strFromU8(archive['dist/screen.ui.js']), {
    module,
    getLang: () => 'en',
    Tools: { Files: { readBinary: async () => file } },
    ToolPkg: { ipc: { on() {} }, readResource: async () => '/asset' },
  });
  const state = { input: { path: '/workspace/large.docx', name: 'large.docx' } };
  const initialize = module.exports.default({
    useState: (key, fallback) => [state[key] ?? fallback, () => {}],
    useRef: (_, current) => ({ current }),
    useEffect() {},
    createWebViewController: () => ({ addJavascriptInterface: (_, methods) => { host = methods; } }),
    UI: { Box: props => props.onLoad, Text() {} },
  });
  await initialize();
  const size = 25 * 1024 * 1024;
  file = { size, contentBase64: Buffer.alloc(size).toString('base64') };
  const info = await host.bootstrap();
  assert.equal(info.length, file.contentBase64.length);
  assert.equal(host.readPart([0]).length, info.chunkSize);
  file = { size: 129 * 1024 * 1024, contentBase64: 'AA==' };
  await assert.rejects(host.bootstrap(), /129\.0 MiB; the limit is 128 MiB/);
  file = { successful: false, details: 'File not found' };
  await assert.rejects(host.bootstrap(), /Could not read large.docx: File not found/);
  file = { size: 0, contentBase64: '' };
  await assert.rejects(host.bootstrap(), /document is empty/);
});

async function testZoom(page, format) {
  // Read each editor's visible native zoom control, including shadow DOM controls.
  const control = format === 'docx' ? page.locator('[aria-label="Zoom"] .sd-button-label')
    : format === 'pptx' ? page.getByRole('slider', { name: 'Zoom percentage' })
    : page.getByRole('textbox', { name: 'Set zoom' });
  await control.waitFor();
  assert.equal(await page.locator('#zoom').count(), 0, 'no duplicate plugin zoom bar');
  assert.equal(await control.count(), 1, `${format} has one native zoom control`);
  const percent = () => control.evaluate(node => Number.parseFloat(node.getAttribute('aria-valuetext') ?? node.value ?? node.textContent));
  async function until(predicate) {
    const deadline = Date.now() + 5000;
    while (Date.now() < deadline) {
      if (predicate(await percent())) return;
      await page.waitForTimeout(25);
    }
    assert.ok(predicate(await percent()), `${format}: unexpected native zoom ${await percent()}`);
  }
  await until(value => value > 0);
  if (format === 'docx') {
    await page.setViewportSize({ width: 520, height: 760 });
    await page.waitForFunction(() => {
      const viewport = document.querySelector('#editor').getBoundingClientRect();
      const paper = document.querySelector('.superdoc-page').getBoundingClientRect();
      return paper.width <= viewport.width;
    });
    const viewport = await page.locator('#editor').boundingBox();
    const paper = await page.locator('.superdoc-page').first().boundingBox();
    assert.ok(paper.x >= viewport.x - 1 && paper.x + paper.width <= viewport.x + viewport.width + 1, 'fit width keeps the whole Word page visible');
    await page.setViewportSize({ width: 900, height: 760 });
    await control.waitFor();
  }
  const editor = await page.locator('#editor').boundingBox();
  await page.mouse.move(editor.x + editor.width / 2, editor.y + editor.height / 2);
  const initial = await percent();
  await page.keyboard.down('Control');
  await page.mouse.wheel(0, -60);
  await page.keyboard.up('Control');
  await until(value => value > initial);
  const enlarged = await percent();
  await page.mouse.wheel(0, 60);
  await page.evaluate(() => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  assert.equal(await percent(), enlarged, 'ordinary scrolling does not zoom');
  await page.evaluate(() => {
    const root = document.querySelector('#editor');
    for (let i = 0; i < 60; i++) root.dispatchEvent(new WheelEvent('wheel', {
      deltaY: -0.1, ctrlKey: true, bubbles: true, cancelable: true,
    }));
  });
  await until(value => value > enlarged);
  const beforePinch = await percent();
  await page.evaluate(() => {
    const root = document.querySelector('#editor');
    for (const [type, scale] of [['gesturestart', 1], ['gesturechange', 0.75], ['gestureend', 0.75]]) {
      const event = new Event(type, { bubbles: true, cancelable: true });
      Object.defineProperty(event, 'scale', { value: scale });
      root.dispatchEvent(event);
    }
  });
  await until(value => value < beforePinch);
  await page.locator('#editor').evaluate(root => root.dispatchEvent(new KeyboardEvent('keydown', {
    key: '0', ctrlKey: true, bubbles: true, cancelable: true,
  })));
  await until(value => value > 0);
  await page.setViewportSize({ width: 900, height: 760 });
}

async function testEmptyQuote(page, quote, format) {
  await quote.click();
  try {
    await page.getByText('No text selected', { exact: true }).waitFor({ timeout: 2000 });
  } catch (error) {
    const state = await page.evaluate(() => ({
      status: document.querySelector('#status').textContent,
      error: document.querySelector('#error').textContent,
      quote: window.lastOfficeQuote,
    }));
    throw new Error(`${format}: missing empty-selection notice (${JSON.stringify(state)})`, { cause: error });
  }
  assert.equal(await page.evaluate(() => window.lastOfficeQuote), undefined, format);
  assert.equal(await page.locator('#error').isVisible(), false, format);
}
