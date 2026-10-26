/**
 * Registers one v1 image and returns only its canonical link.
 * Path registration uses the documented Android file mapping.
 * @param {'base64' | 'path'} source
 * @param {string} payload
 * @param {string | null | undefined} mimeType
 */
function __operitRegisterV1Image(source, payload, mimeType) {
    var request = { type: 'image', source: source };
    if (mimeType != null) request.mimeType = String(mimeType);
    if (source === 'path') request.path = __operitV1FileAddress(payload, 'android');
    else request.base64 = payload;
    var raw = __operitNativeRegisterMedia(JSON.stringify(request));
    var result = JSON.parse(raw);
    if (!result || typeof result.link !== 'string' || result.link === '') {
        throw new Error('Image registration did not return a link');
    }
    return result.link;
}
