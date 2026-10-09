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
 * ...while using the shared Rust runtime crypto operations through the host value bridge.
 */
var CryptoJS = (function () {

    /** Executes crypto through the synchronous structured host binding. */
    function _nativeCrypto(algorithm, operation, args) {
        return __operitNativeCrypto(algorithm, operation, args);
    }

    /** Wraps the direct host string using the public CryptoJS WordArray shape. */
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

    /** Computes a digest through the structured host crypto binding. */
    const MD5 = function (message) {
        const hash = _nativeCrypto('md5', 'hash', [String(message)]);
        return createWordArrayResult(hash);
    };

    // AES Algorithm
    const AES = {
        /** Decrypts through the required host binding and propagates the actual crypto error. */
        decrypt: function (ciphertext, key, cfg) {
            // Reverted to standard signature. The key is now a WordArray-like object.
            // We extract the raw hex string from it to pass to the native side.
            const keyString = (key && typeof key === 'object' && key.data) ? key.data : String(key);

            // We now only need ciphertext and the key string.
            const decrypted = _nativeCrypto('aes', 'decrypt', [String(ciphertext), String(keyString)]);

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
