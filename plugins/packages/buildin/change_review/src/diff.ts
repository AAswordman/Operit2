/** Semantic rows remain plugin-owned; the host only lays out the supplied content. */
export type LineKind = 'context' | 'add' | 'remove';
export interface Segment { start: number; end: number }
export interface DiffLine {
  lineNumber: number;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  text: string;
  kind: LineKind;
  segments: Segment[];
}
export interface Hunk {
  header: string;
  oldStart: number;
  oldCount: number;
  newStart: number;
  newCount: number;
  rows: DiffLine[];
}
export interface DiffDocument { hunks: Hunk[]; additions: number; deletions: number; maxLineLength: number }
export type DiffItem =
  | { type: 'gap'; count: number; oldStartLine: number; newStartLine: number; expandable: boolean }
  | { type: 'hunk'; header: string }
  | { type: 'line'; row: DiffLine }
  | { type: 'pair'; left: DiffLine | null; right: DiffLine | null };

/** Finds the same changed middle range after trimming shared UTF-16 prefix/suffix. */
export function changedRanges(oldText: string, newText: string): [Segment[], Segment[]] {
  const shared = Math.min(oldText.length, newText.length);
  let prefix = 0;
  while (prefix < shared && oldText[prefix] === newText[prefix]) prefix++;
  let suffix = 0;
  while (suffix < shared - prefix && oldText[oldText.length - 1 - suffix] === newText[newText.length - 1 - suffix]) suffix++;
  const oldEnd = oldText.length - suffix, newEnd = newText.length - suffix;
  return [oldEnd > prefix ? [{ start: prefix, end: oldEnd }] : [], newEnd > prefix ? [{ start: prefix, end: newEnd }] : []];
}

/** Pairs adjacent removal/addition blocks without highlighting unchanged context. */
function emphasize(rows: DiffLine[]): DiffLine[] {
  let index = 0;
  while (index < rows.length) {
    if (rows[index].kind !== 'remove') { index++; continue; }
    const removed = index;
    while (index < rows.length && rows[index].kind === 'remove') index++;
    const added = index;
    while (index < rows.length && rows[index].kind === 'add') index++;
    for (let pair = 0; pair < Math.min(added - removed, index - added); pair++) {
      const [left, right] = changedRanges(rows[removed + pair].text, rows[added + pair].text);
      rows[removed + pair].segments = left;
      rows[added + pair].segments = right;
    }
  }
  return rows;
}

/** Mirrors the original horizontal extent estimate, including tabs and wide glyphs. */
export function visualWidth(text: string): number {
  let width = 0;
  for (const character of text) width += character === '\t' ? 4 : character.codePointAt(0)! > 0x7F ? 2 : 1;
  return width;
}

/** Parses unified hunks and explicitly enabled headerless chat fragments. */
export function parseDiff(diff: string, allowImplicitHunks = false): DiffDocument {
  const hunks: Hunk[] = [];
  let hunk: Hunk | null = null;
  let oldLine = 1, newLine = 1;
  /** Commits one nonempty hunk and its paired intraline emphasis. */
  function flush(): void {
    if (hunk !== null && hunk.rows.length > 0) {
      hunk.rows = emphasize(hunk.rows);
      hunks.push(hunk);
    }
  }
  for (const raw of diff.split('\n')) {
    const line = raw.endsWith('\r') ? raw.slice(0, -1) : raw;
    const header = /^@@ -(\d+)(?:,(\d+))? \+(\d+)(?:,(\d+))? @@.*/.exec(line);
    if (header !== null) {
      flush();
      oldLine = Number(header[1]); newLine = Number(header[3]);
      hunk = { header: line, oldStart: oldLine, oldCount: header[2] === undefined ? 1 : Number(header[2]),
        newStart: newLine, newCount: header[4] === undefined ? 1 : Number(header[4]), rows: [] };
      continue;
    }
    if (allowImplicitHunks && hunk === null && !line.startsWith('+++') && !line.startsWith('---') && (line.startsWith('+') || line.startsWith('-'))) {
      hunk = { header: '@@', oldStart: 1, oldCount: 0, newStart: 1, newCount: 0, rows: [] };
      oldLine = 1; newLine = 1;
    }
    if (hunk === null || line.startsWith('\\')) continue;
    if (line.startsWith('+')) {
      hunk.rows.push({ lineNumber: newLine, newLineNumber: newLine++, oldLineNumber: null, text: line.slice(1), kind: 'add', segments: [] });
    } else if (line.startsWith('-')) {
      hunk.rows.push({ lineNumber: oldLine, oldLineNumber: oldLine++, newLineNumber: null, text: line.slice(1), kind: 'remove', segments: [] });
    } else if (line.startsWith(' ')) {
      hunk.rows.push({ lineNumber: newLine, oldLineNumber: oldLine++, newLineNumber: newLine++, text: line.slice(1), kind: 'context', segments: [] });
    }
  }
  flush();
  const rows = hunks.flatMap(item => item.rows);
  return { hunks, additions: rows.filter(row => row.kind === 'add').length,
    deletions: rows.filter(row => row.kind === 'remove').length,
    maxLineLength: rows.reduce((maximum, row) => Math.max(maximum, visualWidth(row.text)), 0) };
}

/** Resolves a complete unchanged region only when the supplied current file covers it. */
function gapRows(lines: string[], oldStart: number, newStart: number, count: number): DiffLine[] | null {
  if (newStart < 1 || newStart + count - 1 > lines.length) return null;
  return Array.from({ length: count }, (_, offset) => ({ lineNumber: newStart + offset,
    oldLineNumber: oldStart + offset, newLineNumber: newStart + offset,
    text: lines[newStart + offset - 1], kind: 'context', segments: [] }));
}

/** Organizes both native-equivalent layouts, including independently expandable gaps. */
export function diffItems(document: DiffDocument, split: boolean, fileLines: string[] | null, expandedGaps: Set<number> | null): DiffItem[] {
  const items: DiffItem[] = [];
  let oldEnd = 1, newEnd = 1, first = true;
  for (const hunk of document.hunks) {
    const count = first ? hunk.oldStart - 1 : hunk.oldStart - oldEnd;
    const oldStartLine = first ? 1 : oldEnd, newStartLine = first ? 1 : newEnd;
    if (count > 0) {
      const resolved = fileLines !== null && (expandedGaps === null || expandedGaps.has(newStartLine))
        ? gapRows(fileLines, oldStartLine, newStartLine, count) : null;
      if (resolved !== null && resolved.length > 0) {
        for (const row of resolved) items.push(split ? { type: 'pair', left: row, right: row } : { type: 'line', row });
      } else {
        items.push({ type: 'gap', count, oldStartLine, newStartLine, expandable: fileLines !== null });
      }
    }
    first = false; oldEnd = hunk.oldStart + hunk.oldCount; newEnd = hunk.newStart + hunk.newCount;
    if (!split) {
      if (hunk.header !== '@@') items.push({ type: 'hunk', header: hunk.header });
      for (const row of hunk.rows) items.push({ type: 'line', row });
      continue;
    }
    let index = 0;
    while (index < hunk.rows.length) {
      const row = hunk.rows[index];
      if (row.kind === 'context') { items.push({ type: 'pair', left: row, right: row }); index++; continue; }
      const removed = index;
      while (index < hunk.rows.length && hunk.rows[index].kind === 'remove') index++;
      const added = index;
      while (index < hunk.rows.length && hunk.rows[index].kind === 'add') index++;
      for (let pair = 0; pair < Math.max(added - removed, index - added); pair++) {
        items.push({ type: 'pair', left: pair < added - removed ? hunk.rows[removed + pair] : null,
          right: pair < index - added ? hunk.rows[added + pair] : null });
      }
    }
  }
  return items;
}
