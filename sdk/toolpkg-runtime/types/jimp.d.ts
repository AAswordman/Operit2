// Type definitions for the native-bridged Jimp implementation

declare namespace Jimp {
    class JimpWrapper {
        readonly id: string;
        crop(x: number, y: number, w: number, h: number): Promise<JimpWrapper>;
        composite(src: JimpWrapper, x: number, y: number): Promise<this>;
        getWidth(): Promise<number>;
        getHeight(): Promise<number>;
        getBase64(mime: string): Promise<string>;
        release(): Promise<void>;
    }

    function read(base64: string): Promise<JimpWrapper>;
    function create(w: number, h: number): Promise<JimpWrapper>;

    const MIME_JPEG: 'image/jpeg';
    const MIME_PNG: 'image/png';
} 