// A native-bridged implementation mimicking the Jimp API for image processing.
const Jimp = (function () {
    'use strict';

    /** Executes image operations through an owning-call host Promise without callback globals. */
    function _nativeImage(operation, args) {
        return __operitInvokeHostAsync(__operitNativeImageProcessing, [operation, args]);
    }

    class JimpWrapper {
        /** Wraps one explicitly owned runtime image handle. */
        constructor(id) {
            if (!id) throw new Error("Cannot create Jimp object without a native image ID.");
            this.id = id;
        }
        /** Returns a new image through the scoped host image Promise. */
        async crop(x, y, w, h) {
            const newId = await _nativeImage('crop', [this.id, x, y, w, h]);
            return new JimpWrapper(newId);
        }
        /** Composites another owned image through the structured host binding. */
        async composite(srcWrapper, x, y) {
            if (!(srcWrapper instanceof JimpWrapper)) throw new Error("Source image must be a Jimp object.");
            await _nativeImage('composite', [this.id, srcWrapper.id, x, y]);
            return this;
        }
        /** Reads the owned image width through the host Promise. */
        async getWidth() { return await _nativeImage('getWidth', [this.id]); }
        /** Reads the owned image height through the host Promise. */
        async getHeight() { return await _nativeImage('getHeight', [this.id]); }
        /** Encodes the owned image in the selected public output format. */
        async getBase64(mime) { return await _nativeImage('getBase64', [this.id, mime || Jimp.MIME_JPEG]); }
        /** Releases the exact owned image handle through the host Promise. */
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
