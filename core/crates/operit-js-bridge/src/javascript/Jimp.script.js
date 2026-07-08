// A native-bridged implementation mimicking the Jimp API for image processing.
const Jimp = (function () {
    'use strict';

    // This helper calls the native interface and wraps it in a promise
    function _nativeImage(operation, args) {
        return new Promise((resolve, reject) => {
            const callbackId = '_jimp_' + Date.now() + '_' + Math.random().toString(36).substr(2, 9);
            window[callbackId] = (result, isError) => {
                delete window[callbackId];
                if (isError) {
                    reject(new Error(result));
                } else {
                    resolve(result);
                }
            };
            try {
                NativeInterface.image_processing(callbackId, operation, JSON.stringify(args || []));
            } catch (e) {
                reject(e);
            }
        });
    }

    class JimpWrapper {
        constructor(id) {
            if (!id) throw new Error("Cannot create Jimp object without a native image ID.");
            this.id = id;
        }
        async crop(x, y, w, h) {
            const newId = await _nativeImage('crop', [this.id, x, y, w, h]);
            return new JimpWrapper(newId);
        }
        async composite(srcWrapper, x, y) {
            if (!(srcWrapper instanceof JimpWrapper)) throw new Error("Source image must be a Jimp object.");
            await _nativeImage('composite', [this.id, srcWrapper.id, x, y]);
            return this;
        }
        async getWidth() { return await _nativeImage('getWidth', [this.id]); }
        async getHeight() { return await _nativeImage('getHeight', [this.id]); }
        async getBase64(mime) { return await _nativeImage('getBase64', [this.id, mime || Jimp.MIME_JPEG]); }
        async release() {
            if (this.id) {
                await _nativeImage('release', [this.id]);
                this.id = undefined;
            }
        }
    }

    return {
        read: async (base64) => new JimpWrapper(await _nativeImage('read', [base64])),
        create: async (w, h) => new JimpWrapper(await _nativeImage('create', [w, h])),
        MIME_JPEG: 'image/jpeg',
        MIME_PNG: 'image/png'
    };
})(); 
__operitExpose('Jimp', Jimp);
