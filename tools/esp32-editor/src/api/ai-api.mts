import {readFile, writeFile, rename} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import type {IncomingMessage, ServerResponse} from 'node:http';
import {applyOperations, validate, catalog} from '../layout/layout-model.mts';
import {routes} from '../layout/routes.mts';
import {isRecord, errorMessage} from '../layout/project-model.mts';
import type {LayoutDocument} from '../layout/project-model.mts';

const root = new URL('../../../../', import.meta.url);

export const editableSources = [
  'apps/esp32/ui_port/operit_mini_ui.c',
  'apps/esp32/ui_port/operit_ui.h',
  'apps/esp32/ui_port/operit_mini_ui.h',
  'apps/esp32/src/ui.rs',
  'apps/esp32/src/main.rs',
  'tools/esp32-editor/src/layout/routes.mts',
  'tools/esp32-editor/src/layout/layout-model.mts',
  'tools/esp32-editor/src/layout/project-model.mts',
  'tools/esp32-editor/web/editor.ts',
  'tools/esp32-editor/web/index.html',
  'tools/esp32-editor/web/style.css',
  'tools/esp32-editor/web/app.ts',
  'tools/esp32-editor/web/pages.ts',
  'apps/esp32/src/ui_capabilities.rs',
  'tools/esp32-editor/src/layout/package-layout.mts',
  'tools/esp32-editor/web/deploy.ts',
] as const;

export type EditableSource = (typeof editableSources)[number];

export interface CodeEdit {
  path: string;
  revision: string;
  find: string;
  replace: string;
}

export interface SourceFile {
  path: string;
  revision: string;
  content: string;
  updated?: string;
}

interface ChatMessage {
  role: string;
  content?: string | null;
  tool_calls?: ToolCall[];
  tool_call_id?: string;
}

interface ToolCall {
  id: string;
  type?: string;
  function: {
    name: string;
    arguments: string;
  };
}

interface ChatCompletion {
  choices?: Array<{message?: ChatMessage}>;
}

interface DevelopInput {
  endpoint: string;
  model: string;
  key?: string;
  context: {document: LayoutDocument};
  task: string;
}

interface Proposal {
  summary?: unknown;
  operations?: unknown[];
  codeEdits?: unknown[];
}

/** SHA-256 hex digest of file contents. */
function hash(text: string): string {
  return createHash('sha256').update(text).digest('hex');
}

/** Reads an allow-listed source file together with its revision. */
async function readSource(path: string): Promise<SourceFile> {
  if (!(editableSources as readonly string[]).includes(path)) throw new Error('文件不在可编辑源码范围');
  const content = await readFile(new URL(path, root), 'utf8');
  return {path, revision: hash(content), content};
}

/** Applies exact unique find/replace edits in memory and returns the resulting files. */
export async function previewCodeEdits(edits: unknown): Promise<SourceFile[]> {
  if (!Array.isArray(edits) || !edits.length || edits.length > 12) throw new Error('代码提案需要 1–12 项精确替换');
  const files = new Map<string, SourceFile>();
  for (const raw of edits) {
    if (!isRecord(raw)) throw new Error('代码替换必须包含 find / replace');
    if (typeof raw.path !== 'string') throw new Error('代码替换必须包含 find / replace');
    const original = files.get(raw.path) ?? {...await readSource(raw.path)};
    if (original.revision !== raw.revision) throw new Error('源码已变化：' + raw.path + '，请重新读取');
    if (typeof raw.find !== 'string' || !raw.find || typeof raw.replace !== 'string') {
      throw new Error('代码替换必须包含 find / replace');
    }
    const content = original.updated ?? original.content;
    const at = content.indexOf(raw.find);
    if (at < 0 || content.indexOf(raw.find, at + 1) >= 0) throw new Error('替换位置缺失或不唯一：' + raw.path);
    original.updated = content.slice(0, at) + raw.replace + content.slice(at + raw.find.length);
    files.set(raw.path, original);
  }
  return [...files.values()];
}

let writes: Promise<unknown> = Promise.resolve();

/** Writes previewed code edits to disk, restoring originals if any write fails. */
async function applyCode(edits: unknown): Promise<{files: Array<{path: string; revision: string}>}> {
  const task = writes.then(async () => {
    const files = await previewCodeEdits(edits);
    const applied: SourceFile[] = [];
    try {
      for (const file of files) {
        if (file.updated === undefined) throw new Error('替换位置缺失或不唯一：' + file.path);
        const url = new URL(file.path, root);
        const tmp = new URL(file.path + '.ai-tmp', root);
        await writeFile(tmp, file.updated);
        await rename(tmp, url);
        applied.push(file);
      }
    } catch (error) {
      for (const file of applied) await writeFile(new URL(file.path, root), file.content);
      throw error;
    }
    return {
      files: files.map((file) => {
        if (file.updated === undefined) throw new Error('替换位置缺失或不唯一：' + file.path);
        return {path: file.path, revision: hash(file.updated)};
      }),
    };
  });
  writes = task.catch(() => undefined);
  return task;
}

/** Reads a fetch JSON body with a hard size cap. */
async function limitedJSON(response: Response, max = 512 * 1024): Promise<unknown> {
  if (!response.body) throw new Error('AI 服务未返回消息');
  let text = '';
  let size = 0;
  const decoder = new TextDecoder();
  const reader = response.body.getReader();
  while (true) {
    const part = await reader.read();
    if (part.done) break;
    const chunk = part.value;
    size += chunk.byteLength;
    if (size > max) throw new Error('AI 响应过大');
    text += decoder.decode(chunk, {stream: true});
  }
  return JSON.parse(text + decoder.decode()) as unknown;
}

/** Runs the develop tool loop against an OpenAI-compatible chat endpoint. */
async function develop(input: DevelopInput, signal: AbortSignal): Promise<{
  summary: string;
  operations: unknown[];
  codeEdits: unknown[];
  notice?: string;
}> {
  const {endpoint, model, key, context, task} = input;
  const url = new URL(endpoint.replace(/\/$/, '') + '/chat/completions');
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) {
    throw new Error('API 地址无效');
  }
  if (typeof model !== 'string' || !model.trim() || typeof task !== 'string' || !task.trim()) {
    throw new Error('请填写模型和任务');
  }
  if (validate(context?.document).length) throw new Error('当前布局无效');
  const system =
    `You implement Operit ESP32 UI projects. Reply in Chinese. Context contains the actual project, active page/component and source references. Use read_source to inspect existing source before proposing code changes. No shell execution. Return a JSON object {summary,operations:[],codeEdits:[]} without markdown. Layout operations: add (pageId,node), update (id,changes), remove(id), document(changes), addPage(page:{id,name,background,nodes,swipeLeft?,swipeRight?}), updatePage(id,changes), removePage(id). Root nodes are home; pages are additional pages. version2 enabled:true entryPage controls boot. All IDs unique globally, page IDs <=18 ASCII chars. 320x240, max12 pages,24 nodes/page,complexity40/page. Parent must earlier panel in same page. Text ASCII. Optional label binding clock/connection/expression/theme/pairing/space/chat and fontSize14/48. Actions go:PAGE_ID or registered routes. Composite buttons use panel + child buttons, not opaque buttonmatrix. Code edits use {path,revision,find,replace}, exact unique find from read_source. Propose only necessary changes. The user reviews before applying. Need implement new routes in shared C and catalogue; do not claim saved/built. Available sources: ${JSON.stringify(editableSources)}. Components: ${JSON.stringify(catalog)}. Routes: ${JSON.stringify(routes)}.`;
  const messages: ChatMessage[] = [
    {role: 'system', content: system},
    {role: 'user', content: JSON.stringify({task, context})},
  ];
  const tools = [
    {
      type: 'function',
      function: {
        name: 'read_source',
        description: 'Read an allowed project source file and its revision',
        parameters: {
          type: 'object',
          properties: {path: {type: 'string', enum: [...editableSources]}},
          required: ['path'],
          additionalProperties: false,
        },
      },
    },
  ];
  for (let round = 0; round < 7; round++) {
    if (Buffer.byteLength(JSON.stringify(messages)) > 768 * 1024) throw new Error('任务和源码上下文超过 768 KiB，请缩小任务范围');
    const headers: Record<string, string> = {'Content-Type': 'application/json'};
    if (key) headers.Authorization = 'Bearer ' + key;
    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify({model, messages, tools, tool_choice: 'auto'}),
      signal,
    });
    if (!response.ok) throw new Error('AI 服务请求失败，HTTP ' + response.status);
    const result = (await limitedJSON(response)) as ChatCompletion;
    const message = result.choices?.[0]?.message;
    if (!message) throw new Error('AI 服务未返回消息');
    if (message.tool_calls?.length) {
      if (message.tool_calls.length > 4) throw new Error('单轮读取源码过多');
      messages.push(message);
      for (const call of message.tool_calls) {
        let output: unknown;
        try {
          if (call.function.name !== 'read_source') throw new Error('不支持的工具');
          const args: unknown = JSON.parse(call.function.arguments);
          if (!isRecord(args) || typeof args.path !== 'string') throw new Error('不支持的工具');
          output = await readSource(args.path);
        } catch (error) {
          output = {error: errorMessage(error)};
        }
        messages.push({role: 'tool', tool_call_id: call.id, content: JSON.stringify(output)});
      }
      continue;
    }
    const content = message.content || '';
    const json = content.replace(/^\s*```(?:json)?\s*/, '').replace(/\s*```\s*$/, '');
    let proposal: Proposal;
    try {
      proposal = JSON.parse(json) as Proposal;
    } catch {
      return {summary: content, operations: [], codeEdits: [], notice: 'AI 未给出结构化修改，可继续描述任务。'};
    }
    if (proposal.operations?.length) applyOperations(context.document, proposal.operations);
    if (proposal.codeEdits?.length) await previewCodeEdits(proposal.codeEdits);
    return {
      summary: String(proposal.summary || 'AI 修改提案'),
      operations: proposal.operations || [],
      codeEdits: proposal.codeEdits || [],
    };
  }
  throw new Error('源码读取轮数达到上限，请缩小任务范围');
}

/** Parses a develop request body. */
function parseDevelopInput(input: unknown): DevelopInput {
  if (!isRecord(input)) throw new Error('请填写模型和任务');
  if (typeof input.endpoint !== 'string' || typeof input.model !== 'string' || typeof input.task !== 'string') {
    throw new Error('请填写模型和任务');
  }
  if (!isRecord(input.context) || !isRecord(input.context.document)) throw new Error('当前布局无效');
  return {
    endpoint: input.endpoint,
    model: input.model,
    key: typeof input.key === 'string' ? input.key : undefined,
    context: {document: input.context.document as LayoutDocument},
    task: input.task,
  };
}

/** Serves AI develop and apply-code HTTP routes. */
export async function aiRoute(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (!['/api/ai/develop', '/api/ai/apply-code'].includes(url.pathname)) return false;
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), 120000);
  res.on('close', () => {
    if (!res.writableEnded) controller.abort();
  });
  try {
    if (req.method !== 'POST') {
      res.writeHead(405);
      res.end();
      return true;
    }
    let body = '';
    for await (const chunk of req) {
      body += typeof chunk === 'string' ? chunk : chunk.toString();
      if (Buffer.byteLength(body) > 256 * 1024) throw new Error('请求超过 256 KiB');
    }
    const input: unknown = JSON.parse(body);
    const result = url.pathname.endsWith('apply-code')
      ? await applyCode(isRecord(input) ? input.codeEdits : undefined)
      : await develop(parseDevelopInput(input), controller.signal);
    res.writeHead(200, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
    res.end(JSON.stringify(result));
  } catch (error) {
    if (!res.destroyed) {
      res.writeHead(400, {'Content-Type': 'application/json'});
      res.end(JSON.stringify({
        error: controller.signal.aborted ? 'AI 请求已取消或超时' : errorMessage(error),
      }));
    }
  } finally {
    clearTimeout(timer);
  }
  return true;
}
