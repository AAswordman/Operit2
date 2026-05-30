/**
 * Native Bridge for CryptoJS
 * 
 * This script creates a global `CryptoJS` object that mimics the original library's API
 * but delegates the actual cryptographic operations to the high-performance native
 * implementation provided by the app's `JsEngine`.
 * 
 * This allows scripts to use standard `CryptoJS` syntax like:
 * - CryptoJS.MD5("message").toString()
 * - CryptoJS.AES.decrypt(data, key, { mode, padding }).toString(CryptoJS.enc.Utf8)
 * 
 * ...while benefiting from the speed and reliability of native Java/Kotlin crypto libraries.
 */
var CryptoJS = (function () {

    // --- Private Helper ---
    // The core bridge function to call native crypto operations
    function _nativeCrypto(algorithm, operation, args) {
        const resultJson = NativeInterface.crypto(algorithm, operation, JSON.stringify(args));
        // Only parse if it's our specific error object format.
        if (resultJson && typeof resultJson === 'string' && resultJson.startsWith('{"nativeError"')) {
            try {
                return JSON.parse(resultJson);
            } catch (e) {
                // Should not happen if the check is correct, but for safety:
                return { nativeError: "Failed to parse native error JSON." };
            }
        }
        // Otherwise, it's the successfully decrypted string.
        return resultJson;
    }

    // Creates a mock WordArray object that the jmcomic script can use.
    function createWordArrayResult(data) {
        return {
            data: data, // Store the raw result
            toString: function (encoding) {
                // The actual result is already a string from the native side.
                // We ignore the encoding hint (like Utf8) because the native
                // side has already handled correct string encoding.
                return this.data;
            }
        };
    }

    // MD5 Algorithm
    const MD5 = function (message) {
        const hash = _nativeCrypto('md5', 'hash', [message]);
        return createWordArrayResult(hash);
    };

    // AES Algorithm
    const AES = {
        decrypt: function (ciphertext, key, cfg) {
            // Reverted to standard signature. The key is now a WordArray-like object.
            // We extract the raw hex string from it to pass to the native side.
            const keyString = (key && typeof key === 'object' && key.data) ? key.data : String(key);

            // We now only need ciphertext and the key string.
            const decrypted = _nativeCrypto('aes', 'decrypt', [ciphertext, keyString]);

            // Check if native side returned an error object
            if (decrypted && decrypted.nativeError) {
                // To maintain compatibility with CryptoJS error handling, we can't throw here.
                // We return a result that will hopefully fail in a meaningful way later.
                // Returning an empty string is a common pattern.
                console.error("Native decryption failed:", decrypted.nativeError);
                return createWordArrayResult("");
            }

            return createWordArrayResult(decrypted);
        }
    };

    // Encodings (placeholders to satisfy the API)
    const enc = {
        Hex: {
            parse: function (hexStr) {
                // This is used by the JM script to parse the key.
                // We don't need to do anything but return a WordArray-like object
                // containing the raw hex string, as our native bridge will handle it.
                return createWordArrayResult(hexStr);
            }
        },
        Utf8: {
            // This is just a marker object used in `toString(CryptoJS.enc.Utf8)`.
            // No implementation needed.
        }
    };

    // Padding Schemes (placeholders to satisfy the API)
    const pad = {
        Pkcs7: {} // Marker object, no implementation needed.
    };

    // Modes (placeholders to satisfy the API)
    const mode = {
        ECB: {} // Marker object, no implementation needed.
    };

    // --- Export Public API ---
    return {
        MD5: MD5,
        AES: AES,
        enc: enc,
        pad: pad,
        mode: mode
    };
})(); 
__operitExpose('CryptoJS', CryptoJS);
