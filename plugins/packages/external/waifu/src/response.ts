import type { Settings } from "./api.js";
import { modeForChat } from "./mode.js";
import { SentenceSession } from "./sentences.js";
import { externalSettings, resolveEmotion } from "./service.js";

interface Response { settings: Settings; processor: SentenceSession; delivered: number }
const responses = new Map<string, Response>();

/** Projects ordinary native replies into sentence bubbles independently of external consumers. */
export async function responseHook(event: ToolPkg.ChatResponseHookEvent): Promise<ToolPkg.ChatResponseHookOutput> {
  const payload = event.eventPayload;
  const key = `${payload.chatId}:${payload.messageTimestamp}:${payload.variantIndex}`;
  switch (event.event) {
    case "started": {
      if (externalSettings(payload.chatId) !== null) return false;
      const mode = await modeForChat(payload.chatId);
      if (mode === null) return false;
      if (responses.size >= 64) throw new Error("Too many active Waifu chat replies");
      responses.set(key, { settings: mode.settings, processor: new SentenceSession(mode.settings), delivered: 0 });
      return true;
    }
    case "closed": responses.delete(key); return;
    case "snapshot":
    case "finished": {
      const response = responses.get(key);
      if (response === undefined) throw new Error("Waifu native response session does not exist");
      const snapshot = payload.parts.filter(part => part.kind === "markdown").map(part => part.content).join("\n");
      const segments = response.processor.feed(snapshot, event.event === "finished");
      const bubbles: ToolPkg.ResponseBubble[] = [];
      for (const raw of segments) {
        const segment = raw.kind === "emotion" ? await resolveEmotion(raw.category, response.settings) : raw;
        const content = segment.kind === "text" ? segment.text : `![${segment.emotion === null ? "图片" : segment.emotion}](<${segment.source}>)`;
        const delayMs = response.delivered > 0 && segment.kind === "text" ? Math.min(3000, Array.from(segment.text).length * response.settings.charDelayMs) : 0;
        bubbles.push({ parts: [{ partId: `waifu-${response.delivered++}`, kind: "markdown", sequence: 0, content, toolCallId: null, toolName: null, attributes: {} }], delayMs });
      }
      return { bubbles };
    }
  }
}
