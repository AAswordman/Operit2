"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.responseHook = responseHook;
const mode_js_1 = require("./mode.js");
const sentences_js_1 = require("./sentences.js");
const service_js_1 = require("./service.js");
const responses = new Map();
/** Projects ordinary native replies into sentence bubbles independently of external consumers. */
async function responseHook(event) {
    const payload = event.eventPayload;
    const key = `${payload.chatId}:${payload.messageTimestamp}:${payload.variantIndex}`;
    switch (event.event) {
        case "started": {
            if ((0, service_js_1.externalSettings)(payload.chatId) !== null)
                return false;
            const mode = await (0, mode_js_1.modeForChat)(payload.chatId);
            if (mode === null)
                return false;
            if (responses.size >= 64)
                throw new Error("Too many active Waifu chat replies");
            responses.set(key, { settings: mode.settings, processor: new sentences_js_1.SentenceSession(mode.settings), delivered: 0 });
            return true;
        }
        case "closed":
            responses.delete(key);
            return;
        case "snapshot":
        case "finished": {
            const response = responses.get(key);
            if (response === undefined)
                throw new Error("Waifu native response session does not exist");
            const snapshot = payload.parts.filter(part => part.kind === "markdown").map(part => part.content).join("\n");
            const segments = response.processor.feed(snapshot, event.event === "finished");
            const bubbles = [];
            for (const raw of segments) {
                const segment = raw.kind === "emotion" ? await (0, service_js_1.resolveEmotion)(raw.category, response.settings) : raw;
                const content = segment.kind === "text" ? segment.text : `![${segment.emotion === null ? "图片" : segment.emotion}](<${segment.source}>)`;
                const delayMs = response.delivered > 0 && segment.kind === "text" ? Math.min(3000, Array.from(segment.text).length * response.settings.charDelayMs) : 0;
                bubbles.push({ parts: [{ partId: `waifu-${response.delivered++}`, kind: "markdown", sequence: 0, content, toolCallId: null, toolName: null, attributes: {} }], delayMs });
            }
            return { bubbles };
        }
    }
}
