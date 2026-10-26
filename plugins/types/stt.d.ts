// Generated from operit-plugin-sdk Rust declarations.

import type { SttTranscribeResultData } from "./results";

export namespace Stt {
  export interface TranscribeRequest {
    /**
     * Encoded audio file bytes, such as WAV; raw PCM must first be framed.
     */
    audio: number[];
    fileName: string;
    contentType: string;
    language?: string;
    /**
     * Uses the selected host configuration when omitted.
     */
    configId?: string;
  }

  function transcribe(request: TranscribeRequest): Promise<SttTranscribeResultData>;
}
