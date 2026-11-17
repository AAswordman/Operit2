"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.attachments = attachments;
const model_js_1 = require("./model.js");
const messages_js_1 = require("./messages.js");
/** Downloads declared QQ attachments through Host files and preserves their Chat input metadata. */
async function attachments(event) {
    const declared = (0, messages_js_1.messageAttachments)(event);
    if (declared.length === 0)
        return [];
    const directory = `${getPluginConfigDir()}/incoming/message-${encodeURIComponent(event.messageId)}`;
    const created = await Tools.Files.mkdir(directory, true);
    if (!created.successful)
        throw new Error(created.details);
    const result = [];
    for (let index = 0; index < declared.length; index++) {
        const item = declared[index];
        const url = (0, model_js_1.nonblank)(item.voiceWavUrl ?? item.url, "attachment.url");
        if (!/^https:\/\//i.test(url))
            throw new Error("QQ attachment URL must use HTTPS");
        const fileName = item.voiceWavUrl ? item.fileName.replace(/\.[^.]*$/, "") + ".wav" : (0, model_js_1.nonblank)(item.fileName, "attachment.filename");
        const mimeType = item.voiceWavUrl ? "audio/wav" : item.mimeType === "voice" ? "audio/silk" : item.mimeType === "file" ? "application/octet-stream" : item.mimeType;
        if (item.size !== undefined)
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
