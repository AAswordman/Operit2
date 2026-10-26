import type { Segment, Settings } from "./api.js";

const ADDRESS = /^(?:https?:\/\/|www\.)[^\s<>。！？；，、]+|^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}|^(?:[A-Za-z0-9-]+\.)+[A-Za-z]{2,}(?:\/[^\s<>。！？；，、]*)?/;

export type RawSegment = Segment | { kind: "emotion"; category: string };

/** Removes presentation markers while preserving literal URLs, link targets and inline code. */
function clean(value: string, removePunctuation: boolean): string {
  const entities: string[] = [];
  let protectedText = "", index = 0;
  /** Stores literal inline content outside presentation-marker processing. */
  function protect(content: string): string { const key = `\uE000${entities.length}\uE001`; entities.push(content); return key; }
  while (index < value.length) {
    const rest = value.slice(index);
    const code = /^(`+)([\s\S]*?)\1(?!`)/.exec(rest);
    if (code !== null) { protectedText += protect(code[2]); index += code[0].length; continue; }
    if (value[index] === "[") {
      const end = markdownEnd(value, index);
      if (end !== null) {
        const link = /^\[([\s\S]*)\]\(([\s\S]*)\)$/.exec(value.slice(index, end))!;
        protectedText += protect(`${link[1]} (${link[2]})`); index = end; continue;
      }
    }
    const address = ADDRESS.exec(rest);
    if (address !== null) { protectedText += protect(address[0]); index += address[0].length; continue; }
    protectedText += value[index++];
  }
  let result = protectedText.replace(/^\s*(?:#{1,6}\s+|>\s*|[-+*]\s+|\d+\.\s+)/gm, "")
    .replace(/(\*\*\*|___|\*\*|__|~~|\*|_)([^\n]+?)\1/g, "$2")
    .replace(/\s+/g, " ").trim();
  if (removePunctuation) result = result.replace(/(?<!\.)[。！？!?；;.]([”’"'」』）)\]]*)$/u, "$1");
  return result.replace(/\uE000(\d+)\uE001/g, (_match, key: string) => entities[Number(key)]);
}

/** Finds a balanced Markdown link or image, including nested URL parentheses. */
function markdownEnd(source: string, start: number): number | null {
  const labelStart = source[start] === "!" ? start + 1 : start;
  let depth = 1, index = labelStart + 1;
  for (; index < source.length && depth > 0; index++) {
    if (source[index] === "\\") { index++; continue; }
    if (source[index] === "[") depth++;
    if (source[index] === "]") depth--;
  }
  if (depth !== 0 || source[index] !== "(") return null;
  depth = 1;
  for (index++; index < source.length; index++) {
    if (source[index] === "\\") { index++; continue; }
    if (source[index] === "(") depth++;
    if (source[index] === ")" && --depth === 0) return index + 1;
  }
  return null;
}

/** Parses stable sentence boundaries and atomic media without splitting protected Markdown entities. */
export function splitSnapshot(source: string, final: boolean, settings: Settings): RawSegment[] {
  const result: RawSegment[] = [];
  let start = 0, index = 0;
  const inline: string[] = [];

  /** Emits only nonempty readable text from a completed source range. */
  function flush(end: number, protectedBlock = false): void {
    const raw = source.slice(start, end);
    const text = protectedBlock ? raw.trim() : clean(raw, settings.removePunctuation);
    if (text !== "") result.push({ kind: "text", text });
    start = end;
  }

  while (index < source.length) {
    const rest = source.slice(index);
    const lineStart = index === 0 || source[index - 1] === "\n";
    const fence = lineStart ? /^(?:`{3,}|~{3,})[^\n]*\n/.exec(rest) : null;
    if (fence !== null) {
      const marker = /^(`{3,}|~{3,})/.exec(fence[0])![1];
      const closing = new RegExp(`^${marker}[${marker[0]}]*[ \\t]*(?:\\r?\\n|$)`, "m");
      const end = closing.exec(rest.slice(fence[0].length));
      flush(index);
      if (end === null) { if (final) throw new Error("Unclosed fenced code block in waifu output"); return result; }
      index += fence[0].length + end.index + end[0].length;
      flush(index, true); continue;
    }
    if (rest.startsWith("$$")) {
      const end = source.indexOf("$$", index + 2);
      flush(index);
      if (end === -1) { if (final) throw new Error("Unclosed block formula in waifu output"); return result; }
      index = end + 2; flush(index, true); continue;
    }
    if (lineStart && rest.startsWith("|")) {
      const block = /^(?:\|[^\n]*(?:\n|$))+/.exec(rest)![0];
      flush(index);
      if (!final && index + block.length === source.length) return result;
      index += block.length; flush(index, true); continue;
    }
    if (source[index] === "<") {
      const emotion = /^<emotion>\s*([^<>]+?)\s*<\/emotion>/.exec(rest);
      if (emotion !== null) {
        flush(index);
        if (settings.enableEmoticons) result.push({ kind: "emotion", category: emotion[1] });
        index += emotion[0].length; start = index; continue;
      }
      if (/^<(?:think|thinking|tool|tool_result|status)(?:\s|>)/.test(rest)) {
        const opening = /^<([a-z_]+)(?:\s[^>]*)?>/.exec(rest);
        flush(index);
        if (opening === null) { if (final) throw new Error("Malformed internal tag in waifu output"); return result; }
        const closing = `</${opening[1]}>`, end = source.indexOf(closing, index + opening[0].length);
        if (end === -1) { if (final) throw new Error("Unclosed internal tag in waifu output"); return result; }
        index = end + closing.length; start = index; continue;
      }
      if (rest.startsWith("<emotion>")) {
        if (final) throw new Error("Unclosed emotion or markup tag in waifu output");
        return result;
      }
      const tag = /^<\/?[A-Za-z_][^>]*>/.exec(rest);
      if (tag !== null) throw new Error(`Unsupported markup in waifu output: ${tag[0]}`);
      if (/^<\/?[A-Za-z_]*$/.test(rest) || rest.startsWith("<emotion>")) {
        if (final) throw new Error("Unclosed emotion or markup tag in waifu output");
        return result;
      }
    }
    if (rest.startsWith("![") || rest.startsWith("[")) {
      const end = markdownEnd(source, index);
      if (end === null) { if (!final) return result; throw new Error("Unclosed Markdown link or image in waifu output"); }
      if (rest.startsWith("![")) {
        flush(index);
        const image = /^!\[([^\]]*)\]\(([\s\S]+)\)$/.exec(source.slice(index, end))!;
        result.push({ kind: "image", source: image[2].trim(), emotion: null }); start = end;
      }
      index = end; continue;
    }
    const address = ADDRESS.exec(rest);
    if (address !== null) {
      const value = address[0].replace(/[。！？!?；;，,、”’」』）)\]]+$/u, "");
      if (!final && index + value.length === source.length) return result;
      index += value.length; continue;
    }
    if (source[index] === "\\") { index += 2; continue; }
    const delimiter = /^(?:\*\*\*|___|\*\*|__|~~|`+|\*|_)/.exec(rest);
    if (delimiter !== null && (delimiter[0] !== "_" || !/[\p{L}\p{N}]/u.test(source[index - 1] || ""))) {
      const value = delimiter[0];
      if (inline[inline.length - 1] !== value && /\s/.test(source[index + value.length] || " ")) { index += value.length; continue; }
      if (inline[inline.length - 1] === value) inline.pop(); else inline.push(value);
      index += value.length; continue;
    }
    const char = source[index];
    const sentenceEnd = /[。！？!?；;]/u.test(char) || char === "." && (index + 1 === source.length || /\s/.test(source[index + 1]));
    const lineEnd = char === "\n" && /^(?:\s*(?:#{1,6}\s|>\s|[-+*]\s|\d+\.\s))/.test(source.slice(start, index));
    if ((sentenceEnd || lineEnd) && inline.length === 0) {
      let end = index + 1;
      while (end < source.length && /[。！？!?；;.…”’"'」』）)\]]/u.test(source[end])) end++;
      if (!final && end === source.length) return result;
      flush(end); index = end; continue;
    }
    index++;
  }
  if (final) {
    if (inline.length !== 0) throw new Error("Unclosed inline Markdown in waifu output");
    flush(source.length);
  }
  return result;
}

/** Rejects revisions of delivered content and resolves each emotion exactly once. */
export class SentenceSession {
  private emitted: RawSegment[] = [];

  /** Captures the immutable configuration for a single response stream. */
  constructor(private readonly settings: Settings) {}

  /** Returns newly stable segments and explicitly rejects changed irreversible prefixes. */
  feed(snapshot: string, final: boolean): RawSegment[] {
    if (typeof snapshot !== "string" || snapshot.length > 2 * 1024 * 1024) throw new Error("Waifu snapshot must be a string within 2 MiB");
    const segments = splitSnapshot(snapshot, final, this.settings);
    if (segments.length < this.emitted.length || this.emitted.some((item, index) => JSON.stringify(item) !== JSON.stringify(segments[index]))) throw new Error("Chat revised an already delivered waifu segment");
    const fresh = segments.slice(this.emitted.length);
    this.emitted = final ? [] : segments;
    return fresh;
  }
}
