"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.ReplyTextStream = void 0;
exports.attachmentKind = attachmentKind;
exports.messageDetails = messageDetails;
exports.messageAttachments = messageAttachments;
exports.externalText = externalText;
exports.chatText = chatText;
const model_js_1 = require("./model.js");
/** Normalizes platform MIME aliases without asking Chat to understand QQ's wire format. */
function attachmentKind(mime) {
    if (mime.startsWith("image/"))
        return "image";
    if (mime.startsWith("video/"))
        return "video";
    if (mime === "voice" || mime.startsWith("audio/"))
        return "audio";
    return "file";
}
/** Preserves only declared attachment metadata, keeping download URLs out of prompt text. */
function decodeAttachments(value) {
    if (value === undefined)
        return [];
    if (!Array.isArray(value) || value.length > 10)
        throw new Error("QQ attachments must be an array of at most ten items");
    return value.map((value, index) => {
        const item = (0, model_js_1.object)(value, `attachments[${index}]`);
        const mimeType = (0, model_js_1.nonblank)(item.content_type, "attachment.content_type");
        const result = { kind: attachmentKind(mimeType), url: (0, model_js_1.nonblank)(item.url, "attachment.url"),
            fileName: item.filename === undefined || item.filename === "" ? `attachment-${index + 1}` : (0, model_js_1.nonblank)(item.filename, "attachment.filename"), mimeType };
        for (const field of ["size", "width", "height"]) {
            if (item[field] !== undefined)
                result[field] = (0, model_js_1.integer)(item[field], `attachment.${field}`, 0, Number.MAX_SAFE_INTEGER);
        }
        if (item.voice_wav_url !== undefined && item.voice_wav_url !== "")
            result.voiceWavUrl = (0, model_js_1.nonblank)(item.voice_wav_url, "attachment.voice_wav_url");
        if (item.asr_refer_text !== undefined)
            result.transcript = (0, model_js_1.text)(item.asr_refer_text, "attachment.asr_refer_text");
        return result;
    });
}
/** Bounds recursive quote/forward payloads before they reach storage, downloads or prompts. */
function decodeElements(value, depth, budget) {
    if (value === undefined)
        return [];
    if (!Array.isArray(value))
        throw new Error("QQ msg_elements must be an array");
    if (value.length === 0)
        return [];
    if (depth > 5 || budget.count + value.length > 50)
        throw new Error("QQ nested message limit exceeded");
    budget.count += value.length;
    return value.map(value => {
        const item = (0, model_js_1.object)(value, "msg_element");
        const result = { text: item.content === undefined ? "" : (0, model_js_1.text)(item.content, "msg_element.content"),
            attachments: decodeAttachments(item.attachments), elements: decodeElements(item.msg_elements, depth + 1, budget) };
        if (item.message_type !== undefined)
            result.messageType = (0, model_js_1.integer)(item.message_type, "msg_element.message_type", 0, Number.MAX_SAFE_INTEGER);
        if (item.msg_idx !== undefined)
            result.referenceIndex = (0, model_js_1.nonblank)(item.msg_idx, "msg_element.msg_idx");
        if (item.ark_data !== undefined)
            result.card = (0, model_js_1.object)(item.ark_data, "msg_element.ark_data");
        return result;
    });
}
/** Keeps reference indices in typed routing fields rather than interpolating them into visible text. */
function messageDetails(body, authorId) {
    const author = (0, model_js_1.object)(body.author, "message.author");
    const result = { author: { id: authorId, name: typeof author.username === "string" ? author.username : "",
            avatar: typeof author.avatar === "string" ? author.avatar : "", bot: author.bot === true },
        attachments: decodeAttachments(body.attachments), elements: decodeElements(body.msg_elements, 0, { count: 0 }) };
    if (body.message_type !== undefined)
        result.messageType = (0, model_js_1.integer)(body.message_type, "message.message_type", 0, Number.MAX_SAFE_INTEGER);
    if (body.ark_data !== undefined)
        result.card = (0, model_js_1.object)(body.ark_data, "message.ark_data");
    if (body.message_scene !== undefined) {
        const scene = (0, model_js_1.object)(body.message_scene, "message.message_scene");
        if (scene.ext !== undefined) {
            if (!Array.isArray(scene.ext))
                throw new Error("message_scene.ext must be an array");
            for (const value of scene.ext) {
                const entry = (0, model_js_1.text)(value, "message_scene.ext entry"), split = entry.indexOf("=");
                const key = entry.slice(0, split), index = entry.slice(split + 1);
                if (split > 0 && index !== "") {
                    if (key === "msg_idx")
                        result.referenceIndex = index;
                    if (key === "ref_msg_idx")
                        result.referencedMessageIndex = index;
                }
            }
        }
    }
    return result;
}
/** Flattens actual media from nested messages with one aggregate download limit. */
function messageAttachments(event) {
    const details = event.attachments === undefined && event.rawPayload !== undefined
        ? messageDetails((0, model_js_1.object)(event.rawPayload.d, "message.d"), event.userOpenId) : event;
    const result = [...(details.attachments ?? [])];
    const visit = (elements) => { for (const element of elements) {
        result.push(...element.attachments);
        visit(element.elements);
    } };
    visit(details.elements ?? []);
    if (result.length > 10)
        throw new Error("QQ message contains more than ten attachments including quoted messages");
    return result;
}
/** Removes known internal reply envelopes. Native references are set separately from trusted routing metadata. */
function externalText(value) {
    return value
        .replace(/\[\[(?:reply_?to(?::[^\]\r\n]*)?|reply_?to_?current)\]\]/gi, "")
        .replace(/\[(?:CQ:reply,[^\]\r\n]*|reply(?:_?id|_?to)?\s*[:=][^\]\r\n]*)\]/gi, "")
        .replace(/<reply(?:_?to)?\b[^>]*\/>/gi, "")
        .replace(/<\/?reply(?:_?to)?\b[^>]*>/gi, "")
        .replace(/\[QQ (?:c2c|group); sender=[^\]\r\n]*\]/g, "")
        .replace(/(?:\[\[reply_?to\b|\[CQ:reply,|\[reply(?:_?id|_?to)?\s*[:=]|<\/?reply(?:_?to)?\b)[^\r\n]*$/gi, "")
        .trim();
}
/** Emits readable quote/card/voice context without serializing opaque platform metadata. */
function chatText(event) {
    const parts = [];
    const cardText = (card) => {
        if (card === undefined)
            return;
        const fields = typeof card.fields === "object" && card.fields !== null && !Array.isArray(card.fields) ? card.fields : {};
        const values = [card.ark_name, card.prompt, fields.title, fields.desc].filter((value) => typeof value === "string" && value !== "");
        if (values.length > 0)
            parts.push(`[卡片] ${values.map(externalText).join(" · ")}`);
    };
    if (event.content.trim() !== "")
        parts.push(externalText(event.content));
    cardText(event.card);
    const visit = (elements, depth) => {
        for (const element of elements) {
            const readable = externalText(element.text);
            if (readable !== "")
                parts.push(`${">".repeat(depth)} ${readable.replace(/\n/g, `\n${">".repeat(depth)} `)}`);
            cardText(element.card);
            visit(element.elements, depth + 1);
        }
    };
    visit(event.elements ?? [], 1);
    for (const attachment of messageAttachments(event))
        if (attachment.transcript)
            parts.push(`[语音转写] ${externalText(attachment.transcript)}`);
    return parts.join("\n\n");
}
/** Holds reply markers that Waifu splits across sentences so fragments cannot leak independently. */
class ReplyTextStream {
    constructor() {
        this.pending = "";
    }
    push(fragment, final = false) {
        const text = this.pending + fragment;
        this.pending = "";
        const starts = ["[[reply_to", "[[replyto", "[CQ:reply", "[reply:", "[reply_id", "[replyid", "[reply_to", "[replyto", "<reply", "</reply", "[QQ "];
        let boundary = text.length;
        for (let index = text.length - 1; index >= 0; index--) {
            if (text[index] !== "[" && text[index] !== "<")
                continue;
            const suffix = text.slice(index), lower = suffix.toLowerCase();
            const start = starts.find(start => start.toLowerCase().startsWith(lower) || lower.startsWith(start.toLowerCase()));
            if (start === undefined)
                continue;
            const end = start.startsWith("[[") ? "]]" : start.startsWith("<") ? ">" : "]";
            if (!suffix.includes(end))
                boundary = index;
        }
        if (boundary < text.length) {
            if (!final) {
                this.pending = text.slice(boundary);
                if (this.pending.length > 8192)
                    throw new Error("QQ reply marker exceeds the rendering limit");
            }
            return externalText(text.slice(0, boundary));
        }
        return externalText(text);
    }
}
exports.ReplyTextStream = ReplyTextStream;
