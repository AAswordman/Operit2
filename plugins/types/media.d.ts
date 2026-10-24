// Generated from operit-plugin-sdk Rust declarations.

import type { MediaRegistrationResultData } from "./results";

/**
 * Registers image, audio, and video bytes in the host media pools.
 */
export namespace Media {
  /**
   * Selects the pool and link type for one registration.
   */
  export type Kind = "image" | "audio" | "video";

  /**
   * Registers media that the caller already holds as base64.
   */
  export interface RegisterOptions {
    /**
     * Selects image, audio, or video.
     */
    type: Kind;
    /**
     * Contains the raw or data-URL base64 payload.
     */
    base64: string;
    /**
     * Supplies a MIME type when the payload does not identify itself.
     */
    mimeType?: string;
  }

  /**
   * Registers media stored at a virtual file-system path.
   */
  export interface RegisterFromPathOptions {
    /**
     * Selects image, audio, or video.
     */
    type: Kind;
    /**
     * Identifies the virtual file-system path to read.
     */
    path: string;
  }

  /**
   * Registers base64 media and returns its canonical link.
   */
  function register(options: RegisterOptions): Promise<MediaRegistrationResultData>;
  /**
   * Reads a virtual file and registers it as media.
   */
  function registerFromPath(options: RegisterFromPathOptions): Promise<MediaRegistrationResultData>;
}
