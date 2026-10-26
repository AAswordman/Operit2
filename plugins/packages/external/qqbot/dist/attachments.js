"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.attachments = attachments;
const model_js_1 = require("./model.js");
/** Downloads declared QQ attachments through Host files and preserves their Chat input metadata. */
async function attachments(event) {
    if (event.rawPayload === undefined)
        throw new Error("QQ attachment processing requires the original dispatch payload");
    const body = (0, model_js_1.object)(event.rawPayload.d, "message.d");
    if (body.attachments === undefined)
        return [];
    if (!Array.isArray(body.attachments) || body.attachments.length > 10)
        throw new Error("QQ attachments must be an array of at most ten items");
    if (body.attachments.length === 0)
        return [];
    const directory = `${getPluginConfigDir()}/incoming/message-${encodeURIComponent(event.messageId)}`;
    const created = await Tools.Files.mkdir(directory, true);
    if (!created.successful)
        throw new Error(created.details);
    const result = [];
    for (let index = 0; index < body.attachments.length; index++) {
        const item = (0, model_js_1.object)(body.attachments[index], `attachments[${index}]`);
        const url = (0, model_js_1.nonblank)(item.url, "attachment.url");
        if (!/^https:\/\//i.test(url))
            throw new Error("QQ attachment URL must use HTTPS");
        const fileName = (0, model_js_1.nonblank)(item.filename, "attachment.filename");
        const mimeType = (0, model_js_1.nonblank)(item.content_type, "attachment.content_type");
        (0, model_js_1.integer)(item.size, "attachment.size", 0, 20 * 1024 * 1024);
        const filePath = `${directory}/${index}`;
        const downloaded = await Tools.Files.download(url, filePath);
        if (!downloaded.successful)
            throw new Error(downloaded.details);
        const info = await Tools.Files.info(filePath);
        result.push({ filePath, fileName, mimeType, fileSize: (0, model_js_1.integer)(info.size, "downloaded attachment size", 0, 20 * 1024 * 1024), content: filePath });
    }
    return result;
}
