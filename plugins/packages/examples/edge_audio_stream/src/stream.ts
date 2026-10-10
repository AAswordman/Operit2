import type { EdgeAudioFormat } from "../../../../types/results";
import type { EdgeAudioInputOptions } from "../../../../types/edge";

/** Implement this adapter using your local recognizer or cloud STT session. */
export interface StreamingStt<T> {
  start(format: EdgeAudioFormat): Promise<void>;
  write(pcm: Uint8Array): Promise<void>;
  finish(): Promise<T>;
  abort(error: unknown): Promise<void>;
}

/** QuickJS does not require browser atob or Node Buffer for this adapter. */
export function decodePcm(base64: string, byteLength: number): Uint8Array {
  const alphabet = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
  const output = new Uint8Array(byteLength);
  let bits = 0, accumulator = 0, offset = 0;
  for (const character of base64) {
    if (character === "=") break;
    const value = alphabet.indexOf(character);
    if (value < 0) throw new Error("Invalid PCM Base64");
    accumulator = (accumulator << 6) | value;
    bits += 6;
    if (bits >= 8) {
      bits -= 8;
      if (offset >= output.length) throw new Error("PCM length mismatch");
      output[offset++] = (accumulator >>> bits) & 255;
    }
  }
  if (offset !== byteLength) throw new Error("PCM length mismatch");
  return output;
}

/** Runs within a normal plugin invocation, with recording duration shorter
 * than that invocation's timeout. Do not run a recording loop in on_edge_event.
 */
export async function transcribeEdge<T>(
  nodeId: string,
  options: EdgeAudioInputOptions,
  stt: StreamingStt<T>,
): Promise<T> {
  const stream = await Tools.Edge.openAudioInput(nodeId, options);
  let sequence = 0, sampleOffset = 0;
  let failed = false;
  try {
    await stt.start(stream.format);
    while (true) {
      const block = await Tools.Edge.readAudioInput(stream.streamId);
      if (block.pending) continue;
      if (block.sequence !== sequence || block.sampleOffset !== sampleOffset) {
        throw new Error("Audio stream order mismatch");
      }
      if (block.done) {
        if (block.error) throw new Error(block.error);
        return await stt.finish();
      }
      // Await STT backpressure; this library does not accumulate the recording.
      await stt.write(decodePcm(block.dataBase64, block.byteLength));
      sequence++;
      sampleOffset += block.byteLength / (2 * stream.format.channels);
    }
  } catch (error) {
    failed = true;
    try { await stt.abort(error); } catch { /* Preserve the audio/STT failure. */ }
    throw error;
  } finally {
    try { await Tools.Edge.closeAudioInput(stream.streamId); }
    catch (closeError) { if (!failed) throw closeError; }
  }
}
