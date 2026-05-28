/**
 * Type definitions for the native-bridged pako object.
 * This file provides the type information for the global `pako` object,
 * which is implemented via a native bridge in the application for ZLIB decompression.
 */
declare namespace pako {
    /**
     * Options for the inflate function.
     */
    interface InflateOptions {
        /**
         * Specifies the output type. 'string' will decompress to a UTF-8 string.
         */
        to?: 'string';
    }

    /**
     * Decompresses ZLIB compressed data.
     * @param data The compressed data, typically a Base64 encoded string from a binary response.
     * @param options Decompression options.
     * @returns The decompressed data, as a string if options.to is 'string'.
     */
    function inflate(data: string | Uint8Array, options?: InflateOptions): string;
} 