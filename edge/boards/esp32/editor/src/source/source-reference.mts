import {findNode, nodePointer} from '../layout/project-model.mts';
import type {LayoutDocument, LayoutNode} from '../layout/project-model.mts';

export interface SourceField {
  line: number;
  jsonPointer: string;
}

export interface SourceReference {
  path: string;
  componentId: string;
  jsonPointer: string;
  revision: string;
  startLine: number;
  endLine: number;
  fields: Record<string, SourceField>;
  excerpt: string;
  component: LayoutNode;
}

interface SourceRange {
  start: number;
  end: number;
}

/** Parse offsets from the actual file, including minified JSON and escaped strings. Line numbers are hints at a particular revision; IDs remain the stable identity. */
export function layoutSourceReference(text: string, componentId: string, revision: string): SourceReference | null {
  const document = JSON.parse(text.replace(/^\uFEFF/, '')) as LayoutDocument;
  const found = findNode(document, componentId);
  if (!found) return null;
  const ranges = new Map<string, SourceRange>();
  let at = 0;
  const whitespace = (): void => {
    while (/\s/.test(text[at] || '') && at < text.length) at++;
  };
  /** Parses a JSON string token and returns its decoded value. */
  function string(): string {
    const start = at++;
    while (at < text.length) {
      const c = text[at++];
      if (c === '\\') at++;
      else if (c === '"') break;
    }
    return JSON.parse(text.slice(start, at)) as string;
  }
  /** Walks a JSON value and records byte ranges keyed by JSON pointer. */
  function value(pointer: string): void {
    whitespace();
    const start = at;
    if (text[at] === '{') {
      at++;
      whitespace();
      while (text[at] !== '}') {
        const key = string();
        whitespace();
        at++;
        value(pointer + '/' + key.replace(/~/g, '~0').replace(/\//g, '~1'));
        whitespace();
        if (text[at] !== ',') break;
        at++;
        whitespace();
      }
      at++;
    } else if (text[at] === '[') {
      at++;
      whitespace();
      let child = 0;
      while (text[at] !== ']') {
        value(pointer + '/' + child++);
        whitespace();
        if (text[at] !== ',') break;
        at++;
      }
      at++;
    } else if (text[at] === '"') {
      string();
    } else {
      while (at < text.length && !/[\s,}\]]/.test(text[at])) at++;
    }
    ranges.set(pointer, {start, end: at});
  }
  value('');
  const line = (offset: number): number => text.slice(0, offset).split('\n').length;
  const jsonPointer = nodePointer(document, componentId);
  if (!jsonPointer) return null;
  const range = ranges.get(jsonPointer);
  if (!range) return null;
  const fields: Record<string, SourceField> = {};
  for (const key of ['id', 'action', 'longAction']) {
    const field = ranges.get(jsonPointer + '/' + key);
    if (field) fields[key] = {line: line(field.start), jsonPointer: jsonPointer + '/' + key};
  }
  return {
    path: 'edge/boards/esp32/ui/layout.json',
    componentId,
    jsonPointer,
    revision,
    startLine: line(range.start),
    endLine: line(range.end - 1),
    fields,
    excerpt: text.slice(range.start, range.end),
    component: found.node,
  };
}
