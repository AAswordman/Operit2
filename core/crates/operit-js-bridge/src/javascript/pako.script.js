var pako = (function () {
    return {
        /**
         * Decompresses DEFLATE data.
         * This is a native-bridged implementation.
         * @param {string} data The compressed data, as a Base64 string or a binary handle.
         * @param {object} options Must include `{ to: 'string' }`.
         * @returns {string} The decompressed string.
         */
        inflate: function (data, options) {
            if (!options || options.to !== 'string') {
                throw new Error('This pako bridge only supports inflate(data, { to: "string" })');
            }

            if (typeof data !== 'string') {
                throw new Error('Input data must be a string (Base64 or binary handle).');
            }

            // Call the synchronous native interface
            const result = NativeInterface.decompress(data, 'deflate');

            // Check for a structured error message from native
            if (result && typeof result === 'string' && result.startsWith('{"nativeError"')) {
                try {
                    const errorInfo = JSON.parse(result);
                    throw new Error('Native decompression failed: ' + errorInfo.nativeError);
                } catch (e) {
                    throw new Error('Native decompression failed and could not parse error message: ' + result);
                }
            }

            return result; // The decompressed string
        }
    };
})(); 
__operitExpose('pako', pako);
