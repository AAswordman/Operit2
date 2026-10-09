type Header = [string, string] | { key: string; value: string };

interface StreamRequest {
  method: string;
  url: string;
  headers?: Header[];
  formFields?: Header[];
  fileParts?: { fieldName: string; content: Uint8Array; contentType: string; fileName: string }[];
  body?: Uint8Array;
  followRedirects: boolean;
  connectTimeoutSeconds?: number;
  readTimeoutSeconds?: number;
}

interface ResponseHead {
  finalUrl: string;
  statusCode: number;
  statusMessage: string;
  headers: [string, string][];
}

type Closed = (error: string | null) => void;

/** Owns Fetch streams on the same worker as its Rust Host callbacks. */
export function createBrowserHttpStreamHost() {
  const streams = new Map<string, AbortController>();

  function open(
    id: string, request: StreamRequest, responseCallback: (head: ResponseHead) => void,
    chunkCallback: (chunk: Uint8Array) => void, closedCallback: Closed, requireSuccess: boolean,
  ): void {
    if (streams.has(id)) throw new Error(`HTTP byte stream is already open: ${id}`);
    const controller = new AbortController();
    streams.set(id, controller);
    void (async () => {
      let reader: ReadableStreamDefaultReader<Uint8Array> | undefined;
      let timer: ReturnType<typeof setTimeout> | undefined;
      let error: string | null = null;
      function timeout(seconds: number | undefined, phase: string): void {
        clearTimeout(timer);
        timer = undefined;
        if (seconds && seconds > 0) {
          timer = setTimeout(() => controller.abort(new Error(`HTTP ${phase} timed out`)), seconds * 1000);
        }
      }
      try {
        const headers = new Headers();
        for (const pair of request.headers || []) {
          headers.set(Array.isArray(pair) ? pair[0] : pair.key, Array.isArray(pair) ? pair[1] : pair.value);
        }
        let body: BodyInit | undefined;
        if (request.fileParts?.length || request.formFields?.length) {
          const form = new FormData();
          for (const pair of request.formFields || []) {
            form.append(Array.isArray(pair) ? pair[0] : pair.key, Array.isArray(pair) ? pair[1] : pair.value);
          }
          for (const part of request.fileParts || []) {
            form.append(part.fieldName, new Blob([new Uint8Array(part.content).buffer], {
              type: part.contentType,
            }), part.fileName);
          }
          body = form;
        } else if (request.body?.length) {
          body = new Uint8Array(request.body).buffer;
        }
        timeout(request.connectTimeoutSeconds, "response headers");
        const response = await fetch(request.url, {
          method: request.method, headers, body, signal: controller.signal,
          redirect: request.followRedirects ? "follow" : "manual",
        });
        clearTimeout(timer);
        timer = undefined;
        if (response.type === "opaque" || response.type === "opaqueredirect") {
          throw new Error("HTTP response metadata is unavailable; check CORS and redirect policy");
        }
        // Response streams deliver error statuses too. The consumer, not Fetch, interprets them.
        if (requireSuccess && !response.ok) throw new Error(`HTTP ${response.status}`);
        responseCallback({
          finalUrl: response.url || request.url, statusCode: response.status,
          statusMessage: response.statusText, headers: Array.from(response.headers.entries()),
        });
        if (controller.signal.aborted) throw controller.signal.reason;
        if (response.body) {
          reader = response.body.getReader();
          while (true) {
            timeout(request.readTimeoutSeconds, "response body");
            const result = await reader.read();
            if (controller.signal.aborted) throw controller.signal.reason;
            if (result.done) break;
            chunkCallback(Uint8Array.from(result.value));
          }
        }
      } catch (cause) {
        error = String(controller.signal.aborted ? controller.signal.reason : cause);
      } finally {
        if (error !== null && !controller.signal.aborted) controller.abort(new Error(error));
        clearTimeout(timer);
        reader?.releaseLock();
        // Remove before notifying Rust, allowing a close callback to reuse an id safely.
        streams.delete(id);
        closedCallback(error);
      }
    })();
  }

  return {
    openHttpByteStream(
      id: string, request: StreamRequest, opened: () => void,
      chunk: (bytes: Uint8Array) => void, closed: Closed,
    ): void {
      open(id, request, () => opened(), chunk, closed, true);
    },
    openHttpResponseStream(
      id: string, request: StreamRequest, response: (head: ResponseHead) => void,
      chunk: (bytes: Uint8Array) => void, closed: Closed,
    ): void {
      open(id, request, response, chunk, closed, false);
    },
    closeHttpByteStream(id: string): void {
      // Idempotent: a protocol result may race with EOF or a failed header request.
      streams.get(id)?.abort(new Error("HTTP stream cancelled"));
    },
  };
}
