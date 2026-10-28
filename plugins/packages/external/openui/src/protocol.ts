export const ORIGIN = "https://openui.operit.local/";

export interface ProgramInput {
  program: string;
  isStreaming: boolean;
  revision: number;
}

export interface OpenUiAction {
  type: string;
  params: Record<string, unknown>;
  humanFriendlyMessage: string;
  formState?: Record<string, unknown>;
  formName?: string;
}

/** Removes only the plugin's explicit XML envelope and its incomplete closing delimiter. */
export function parseOpenUi(xml: string): { program: string; isStreaming: boolean } {
  const opening = /^\s*<openui\s*>/i.exec(xml);
  if (opening === null) throw new Error("OpenUI requires an <openui> envelope");
  const body = xml.slice(opening[0].length);
  const closing = /<\/openui\s*>\s*$/i.exec(body);
  if (closing !== null) return { program: body.slice(0, closing.index), isStreaming: false };
  const delimiter = "</openui>";
  for (let length = delimiter.length - 1; length > 0; length--) {
    if (body.slice(-length).toLowerCase() === delimiter.slice(0, length)) {
      return { program: body.slice(0, -length), isStreaming: true };
    }
  }
  return { program: body, isStreaming: true };
}

/** Requires one JSON object at the browser-to-plugin boundary. */
export function requireObject(value: unknown): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new Error("OpenUI expected a JSON object");
  }
  return value as Record<string, unknown>;
}

/** Decodes the host's documented nested browser argument array without accepting another format. */
export function browserArguments(args: unknown[], count: number): unknown[] {
  if (args.length !== 1 || !Array.isArray(args[0]) || args[0].length !== count) {
    throw new Error("OpenUI bridge argument count mismatch");
  }
  return args[0];
}

/** Validates an upstream action before it can cross a host capability boundary. */
export function requireAction(value: unknown): OpenUiAction {
  const action = requireObject(value);
  if (typeof action.type !== "string" || typeof action.humanFriendlyMessage !== "string") {
    throw new Error("Invalid OpenUI action");
  }
  requireObject(action.params);
  return action as unknown as OpenUiAction;
}

/** Accepts only explicit HTTP or HTTPS link actions. */
export function requireWebUrl(value: unknown): string {
  if (typeof value !== "string" || !/^https?:\/\/[^\s]+$/i.test(value)) {
    throw new Error("OpenUI links require an HTTP or HTTPS URL");
  }
  return value;
}
