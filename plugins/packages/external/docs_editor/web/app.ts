import './style.css';
import { createIcons, MessageSquareQuote, Save } from 'lucide';
import { maxBytes, type Command, type DocumentInfo, type Reply } from '../src/protocol';
import { base64, type Bytes, type Editor } from './editor';
import { mountDocx } from './docx';
import { mountPptx } from './pptx';
import { mountPdf } from './pdf';

interface Host {
  bootstrap(): Promise<{ info: DocumentInfo; locale: string; length: number; chunkSize: number }>;
  readPart(offset: number): Promise<string>;
  editorReady(): Promise<boolean>;
  beginSave(length: number): Promise<boolean>;
  appendSave(chunk: string): Promise<boolean>;
  commitSave(): Promise<boolean>;
  quote(text: string): Promise<unknown>;
  dirty(dirty: boolean): Promise<unknown>;
}

declare global {
  interface Window {
    OfficeHost?: Host;
    OfficeEditor?: { submit(key: string, command: Command): boolean; result(key: string): string };
  }
}

const save = document.querySelector<HTMLButtonElement>('#save')!;
const quote = document.querySelector<HTMLButtonElement>('#quote')!;
const status = document.querySelector('#status')!;
const alert = document.querySelector<HTMLElement>('#error')!;
const mount = { docx: mountDocx, pptx: mountPptx, pdf: mountPdf };
let host: Host;
let editor: Editor;
let info: DocumentInfo;
let revision = 0;
let saved = 0;
let saving = false;
let running = false;
let lastDirty = false;
let quoteNoticeTimer: ReturnType<typeof setTimeout> | undefined;
let chinese = navigator.language.startsWith('zh');
let chunkSize = 0;
let dirtyDelivery: Promise<unknown> = Promise.resolve();
const results = new Map<string, string>();

const phrase = (en: string, zh: string) => chinese ? zh : en;

function showError(error: unknown) {
  alert.hidden = false;
  alert.textContent = error instanceof Error ? error.message : String(error);
}

function sync() {
  const dirty = revision !== saved;
  status.textContent = saving ? phrase('Saving…', '保存中…')
    : quoteNoticeTimer !== undefined ? phrase('No text selected', '未选中文本')
    : dirty ? phrase('Unsaved', '未保存') : phrase('Saved', '已保存');
  save.disabled = !editor || saving || running || !dirty;
  if (dirty !== lastDirty) {
    lastDirty = dirty;
    dirtyDelivery = dirtyDelivery.then(() => host.dirty(dirty)).catch(showError);
  }
}

function changed() {
  revision++;
  sync();
}

async function saveBytes(getBytes: () => Promise<Bytes>) {
  if (saving) throw new Error(phrase('A save is already running', '正在保存'));
  const version = revision;
  saving = true;
  sync();
  try {
    const bytes = await getBytes();
    if (!bytes.length || bytes.length > maxBytes) throw new Error(phrase('File exceeds the 24 MiB save limit', '文件超过 24 MiB 保存限制'));
    if (revision !== version) throw new Error(phrase('The file changed while preparing the save. Try again.', '生成保存内容时文档发生了变化，请重试。'));
    const encoded = base64(bytes);
    await host.beginSave(encoded.length);
    for (let offset = 0; offset < encoded.length; offset += chunkSize) await host.appendSave(encoded.slice(offset, offset + chunkSize));
    if (!await host.commitSave()) throw new Error(phrase('Could not save', '保存失败'));
    saved = version;
  } finally {
    saving = false;
    sync();
    await dirtyDelivery;
  }
}

async function execute(command: Command) {
  const expected = command.revision;
  const assertRevision = () => {
    if (revision !== expected) throw new Error(`Stale document revision (current: ${revision}). Read the document again.`);
  };
  const version = revision;
  switch (command.kind) {
    case 'read': {
      const data = await editor.read(command.section ?? 'body', command.offset ?? 0);
      if (version !== revision) throw new Error('Document changed while reading; retry');
      return { id: info.id, format: info.format, revision, dirty: revision !== saved, data };
    }
    case 'edit': {
      if (!command.change) throw new Error('Missing edit');
      assertRevision();
      const data = await editor.edit(command.change, assertRevision);
      if (revision === version) changed();
      return { id: info.id, revision, dirty: revision !== saved, data };
    }
    case 'save':
      assertRevision();
      if (revision !== saved) await saveBytes(() => editor.snapshot());
      return { id: info.id, revision, dirty: revision !== saved };
  }
}

async function bootstrap() {
  // The native bridge may arrive after document-start injection on some WebViews.
  for (let i = 0; !window.OfficeHost && i < 200; i++) await new Promise(resolve => setTimeout(resolve, 50));
  if (!window.OfficeHost) throw new Error('Office workspace bridge unavailable');
  host = window.OfficeHost;
  const init = await host.bootstrap();
  info = init.info;
  chunkSize = init.chunkSize;
  chinese = init.locale.startsWith('zh');
  document.documentElement.lang = chinese ? 'zh' : 'en';
  document.body.dataset.format = info.format;
  document.querySelector('#format')!.textContent = info.format.toUpperCase();
  const filename = document.querySelector<HTMLElement>('#filename')!;
  filename.textContent = filename.title = info.name;
  for (const [button, label] of [[save, phrase('Save', '保存')], [quote, phrase('Quote selection in chat', '引用选区到聊天')]] as const) {
    button.title = label;
    button.setAttribute('aria-label', label);
  }
  createIcons({ icons: { Save, MessageSquareQuote } });
  status.textContent = phrase('Opening…', '正在打开…');
  const chunks: Bytes[] = [];
  let length = 0;
  for (let offset = 0; offset < init.length; offset += chunkSize) {
    const part = Uint8Array.from(atob(await host.readPart(offset)), c => c.charCodeAt(0));
    chunks.push(part);
    length += part.length;
  }
  const bytes = new Uint8Array(length);
  let index = 0;
  for (const part of chunks) {
    bytes.set(part, index);
    index += part.length;
  }
  editor = await mount[info.format]({ bytes, info, chinese, root: document.querySelector('#editor')!, toolbar: document.querySelector('#toolbar')!, changed,
    saveBytes: data => saveBytes(() => Promise.resolve(data)).catch(error => { showError(error); throw error; }) });
  window.OfficeEditor = {
    submit(key, command) {
      if (running || saving) return false;
      running = true;
      sync();
      void execute(command).then<Reply, Reply>(value => ({ ok: true, value }), error => {
        console.error(error?.stack ?? error);
        return { ok: false, error: error instanceof Error ? error.message : String(error) };
      }).then(reply => {
        results.clear();
        results.set(key, JSON.stringify(reply));
      }).finally(() => { running = false; sync(); });
      return true;
    },
    result(key) {
      const value = results.get(key) || '';
      results.delete(key);
      return value;
    },
  };
  await host.editorReady();
  quote.disabled = false;
  sync();
}

// Preserve the editor's selection while interacting with the outer toolbar.
quote.addEventListener('pointerdown', event => event.preventDefault());
quote.addEventListener('click', () => {
  void editor.selection().then(value => {
    const snippet = value.slice(0, 12000);
    clearTimeout(quoteNoticeTimer);
    quoteNoticeTimer = undefined;
    if (!snippet.trim()) {
      quoteNoticeTimer = setTimeout(() => { quoteNoticeTimer = undefined; sync(); }, 3000);
      sync();
      return;
    }
    sync();
    return host.quote(snippet);
  }).catch(showError);
});
save.addEventListener('click', () => {
  alert.hidden = true;
  void saveBytes(() => editor.snapshot()).catch(showError);
});
document.addEventListener('keydown', event => {
  if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's' && info?.format !== 'pptx') {
    event.preventDefault();
    if (!save.disabled) save.click();
  }
});
void bootstrap().catch(showError);
