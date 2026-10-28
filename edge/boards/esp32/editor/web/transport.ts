import type {RequestOptions} from './types.js';
import {errorMessage} from './types.js';

/** Sends a JSON request through the embedded host bridge or the local editor server. */
export async function request<T>(path: string, options: RequestOptions = {}): Promise<T> {
  const {method = 'GET', body, signal} = options;
  if (window.operitHost?.request) {
    return window.operitHost.request<T>({path, method, body, signal});
  }

  const response = await fetch('.' + path, {
    method,
    headers: body === undefined ? {} : {'Content-Type': 'application/json'},
    body: body === undefined ? undefined : JSON.stringify(body),
    cache: 'no-store',
    signal,
  });
  const result: unknown = await response.json();
  if (!response.ok) {
    const message =
      typeof result === 'object' &&
      result !== null &&
      !Array.isArray(result) &&
      'error' in result &&
      typeof result.error === 'string'
        ? result.error
        : `请求失败 (${response.status})`;
    throw new Error(message);
  }
  return result as T;
}

/** Converts an unknown request failure into an Error for UI handlers. */
export function requestError(error: unknown): Error {
  return new Error(errorMessage(error));
}
