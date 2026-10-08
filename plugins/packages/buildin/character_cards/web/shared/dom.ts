/** Requires a rendered element of the exact DOM class used by the editor. */
export function requireElement<T extends Element>(element: Element | null, constructor: { new(): T }): T {
  if (!(element instanceof constructor)) throw new Error("编辑器 DOM 结构无效");
  return element;
}

/** Requires action metadata instead of inventing an identity for malformed controls. */
export function dataValue(element: HTMLElement | SVGElement, key: string): string {
  const value = element.dataset[key];
  if (value === undefined) throw new Error(`界面操作缺少字段：${key}`);
  return value;
}

/** Narrows an exact schema key without inspecting fragments of the supplied text. */
export function isKey<K extends string>(value: string, keys: readonly K[]): value is K {
  return keys.some(
    /** Compares only complete declared field names. */
    key => value === key,
  );
}
