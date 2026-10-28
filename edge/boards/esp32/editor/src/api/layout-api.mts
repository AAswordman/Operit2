import {readFile, writeFile, rename} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import type {IncomingMessage, ServerResponse} from 'node:http';
import {componentSource} from '../source/component-source.mts';
import {projectRoutes, isRecord, errorMessage, errorStatus, withStatus} from '../layout/project-model.mts';
import type {LayoutDocument} from '../layout/project-model.mts';
import {validate, applyOperations, catalog, actions, routes, eventBindings} from '../layout/layout-model.mts';

export const documentPath = new URL('../../../ui/layout.json', import.meta.url);

export interface LoadedLayout {
  document: LayoutDocument;
  revision: string;
}

/** Reads the shared layout JSON and its content hash. */
export async function loadLayout(): Promise<LoadedLayout> {
  const data = await readFile(documentPath, 'utf8');
  const document = JSON.parse(data.replace(/^\uFEFF/, '')) as LayoutDocument;
  return {document, revision: createHash('sha256').update(data).digest('hex')};
}

let writes: Promise<unknown> = Promise.resolve();

/** Writes a full document or a patch, using optimistic revision checking. */
export function saveLayout(
  revision: string | undefined,
  document: unknown,
  operations: unknown,
): Promise<LoadedLayout> {
  const result = writes.then(async () => {
    const current = await loadLayout();
    if (!revision || revision !== current.revision) {
      throw withStatus(409, '布局已被其他编辑器或 Agent 修改，请重新读取');
    }
    const next = operations ? applyOperations(current.document, operations) : document;
    const errors = validate(next);
    if (errors.length) throw withStatus(422, errors.join('; '));
    if (JSON.stringify(next) === JSON.stringify(current.document)) return current;
    const tmp = new URL('../../../ui/layout.json.tmp', import.meta.url);
    await writeFile(tmp, JSON.stringify(next, null, 2) + '\n');
    await rename(tmp, documentPath);
    return loadLayout();
  });
  writes = result.catch(() => undefined);
  return result;
}

/** JSON response helper for layout HTTP routes. */
function respond(res: ServerResponse, status: number, value: unknown): void {
  res.writeHead(status, {'Content-Type': 'application/json', 'Cache-Control': 'no-store'});
  res.end(JSON.stringify(value));
}

/** Serves layout, component catalog, validation and source-reference HTTP routes. */
export async function layoutRoute(req: IncomingMessage, res: ServerResponse, url: URL): Promise<boolean> {
  if (!['/api/layout', '/api/components', '/api/layout/validate', '/api/component-source'].includes(url.pathname)) {
    return false;
  }
  try {
    if (url.pathname === '/api/component-source') {
      if (req.method !== 'GET') {
        respond(res, 405, {error: 'GET required'});
        return true;
      }
      respond(res, 200, await componentSource(url.searchParams.get('id'), url.searchParams.get('revision')));
      return true;
    }
    if (url.pathname === '/api/components') {
      if (req.method !== 'GET') {
        respond(res, 405, {error: 'GET required'});
        return true;
      }
      const dynamic = projectRoutes((await loadLayout()).document);
      respond(res, 200, {
        catalog,
        maxPages: 12,
        maxNodesPerPage: 24,
        maxComplexityPerPage: 40,
        actions: [...actions, ...dynamic.map((route) => route.id)],
        routes: [...routes, ...dynamic],
        eventBindings,
      });
      return true;
    }
    if (req.method === 'GET' && url.pathname === '/api/layout') {
      respond(res, 200, await loadLayout());
      return true;
    }
    if (!req.method || !['PUT', 'PATCH', 'POST'].includes(req.method)) {
      respond(res, 405, {error: 'Unsupported method'});
      return true;
    }
    let body = '';
    for await (const chunk of req) {
      body += typeof chunk === 'string' ? chunk : chunk.toString();
      if (Buffer.byteLength(body) > 65536) {
        respond(res, 413, {error: 'Layout body exceeds 64 KiB'});
        return true;
      }
    }
    const input: unknown = JSON.parse(body);
    if (url.pathname === '/api/layout/validate') {
      const document = isRecord(input) && input.document !== undefined ? input.document : input;
      respond(res, 200, {errors: validate(document)});
      return true;
    }
    if (!['PUT', 'PATCH'].includes(req.method)) {
      respond(res, 405, {error: 'Use PUT or PATCH'});
      return true;
    }
    if (!isRecord(input)) throw new Error('请求必须是对象');
    const saved = await saveLayout(
      typeof input.revision === 'string' ? input.revision : undefined,
      input.document as LayoutDocument | undefined,
      req.method === 'PATCH' ? input.operations : null,
    );
    respond(res, 200, saved);
  } catch (error) {
    respond(res, errorStatus(error) ?? 400, {error: errorMessage(error)});
  }
  return true;
}
