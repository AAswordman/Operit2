/** The browser owns complete input; C stores only a UTF-8 display preview. */
export function draftPreview(text: string): string {
  let bytes = 0, end = 0;
  for (const char of text) {
    const size = new TextEncoder().encode(char).length;
    if (bytes + size > 511) break;
    bytes += size; end += char.length;
  }
  return text.slice(0, end);
}
export function checkedDraft(text: string): string {
  if (new TextEncoder().encode(text).length > 16384) throw new Error('消息超过 16384 字节，请拆分发送');
  return text;
}
