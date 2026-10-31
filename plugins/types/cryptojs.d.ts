/**
 * Type definitions for the native-bridged CryptoJS object.
 * This file provides the type information for the global `CryptoJS` object,
 * which is implemented via a native bridge in the application.
 */

declare namespace CryptoJS {
    namespace binary {
        /** Generates 1..64 cryptographically secure bytes as hexadecimal. */
        function randomHex(length: number): string;
        /** Computes MD5 over base64-encoded bytes, not over their textual representation. */
        function md5(base64: string): string;
        /** Encrypts base64 bytes with a 32-character hexadecimal AES-128 key and PKCS#7 padding. */
        function encryptAesEcb(base64: string, keyHex: string): string;
        /** Decrypts AES-128-ECB base64 bytes and returns base64 plaintext. */
        function decryptAesEcb(base64: string, keyHex: string): string;
    }
    /**
     * A result object that mimics CryptoJS's WordArray, primarily providing a `toString` method.
     */
    interface WordArray {
        /**
         * Converts the WordArray to a string.
         * @param encoding The encoding to use (e.g., `CryptoJS.enc.Utf8`). In our bridge, this is a hint and the native side handles encoding.
         */
        toString(encoding?: any): string;
    }

    /**
     * Computes the MD5 hash of a message.
     * @param message The message to hash.
     * @returns A WordArray object representing the hash.
     */
    function MD5(message: string): WordArray;

    const AES: {
        /**
         * Decrypts a ciphertext.
         * @param ciphertext The data to decrypt.
         * @param key The key to use for decryption. In our specific JMComic bridge, this is the timestamp.
         * @param cfg The configuration object. In our specific JMComic bridge, this is the secret.
         * @returns A WordArray object representing the decrypted plaintext.
         */
        decrypt(ciphertext: string, key: any, cfg?: any): WordArray;
    };

    const enc: {
        Hex: {
            /**
             * Parses a hex string into a WordArray.
             * @param hexStr The hex string to parse.
             */
            parse(hexStr: string): WordArray;
        };
        Utf8: any;
    };

    const pad: {
        Pkcs7: any;
    };

    const mode: {
        ECB: any;
    };
}
