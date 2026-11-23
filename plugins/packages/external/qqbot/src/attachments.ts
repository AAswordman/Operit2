import type { Chat } from "../../../../types/chat";
import { integer, nonblank, type InboundEvent } from "./model.js";
import { messageAttachments } from "./messages.js";
import type { ChannelAttachment } from "./channel-api.js";

export const CONSOLE_LIMIT = 8 * 1024 * 1024;
export const CHAT_ATTACHMENT_LIMIT = 20 * 1024 * 1024;
const qqMediaUrl = /^https:\/\/(?:[a-z0-9-]+\.)*(?:qq\.com(?:\.cn)?|qpic\.cn)\//i;

export function consoleAttachment(item: ChannelAttachment) {
  const url = item.voiceWavUrl ?? item.url;
  if (!qqMediaUrl.test(url) || url.length > 4096 || (item.size ?? 0) > CONSOLE_LIMIT) return null;
  const fileName = item.voiceWavUrl ? item.fileName.replace(/\.[^.]*$/, "") + ".wav" : item.fileName;
  return { url, fileName: fileName.slice(0,150), mimeType: (item.voiceWavUrl ? "audio/wav" : item.mimeType).slice(0,100) };
}

export async function downloadConsoleAttachment(item: NonNullable<ReturnType<typeof consoleAttachment>>) {
  if (!qqMediaUrl.test(item.url)) throw Error("QQ 附件地址不受信任");
  const result = await Tools.Net.http({url:item.url, method:"GET", responseType:"arraybuffer", follow_redirects:false, connect_timeout:15, read_timeout:45});
  if (result.statusCode !== 200) throw Error(`QQ 附件下载 HTTP ${result.statusCode}`);
  const data = nonblank(result.contentBase64,"attachment data");
  if (data.length * 3 / 4 > CONSOLE_LIMIT + 2) throw Error("QQ 附件超过 8 MiB");
  return {fileName:item.fileName, mimeType:item.mimeType, contentBase64:data};
}

/** Downloads eligible QQ attachments and reports files above the Chat limit. */
export async function attachments(event: InboundEvent): Promise<{
  files: Chat.SendAttachment[];
  oversized: { fileName: string; sizeBytes: number }[];
}> {
  const declared = messageAttachments(event);
  const files: Chat.SendAttachment[] = [];
  const oversized: { fileName: string; sizeBytes: number }[] = [];
  let directory: string | undefined;
  for (const [index, item] of declared.entries()) {
    const fileName = item.voiceWavUrl ? item.fileName.replace(/\.[^.]*$/, "") + ".wav" : nonblank(item.fileName, "attachment.filename");
    if (item.size !== undefined && item.size > CHAT_ATTACHMENT_LIMIT) {
      oversized.push({ fileName: fileName.slice(0, 150), sizeBytes: item.size });
      continue;
    }
    const url = nonblank(item.voiceWavUrl ?? item.url, "attachment.url");
    if (!/^https:\/\//i.test(url)) throw new Error("QQ attachment URL must use HTTPS");
    if (directory === undefined) {
      directory = `${getPluginConfigDir()}/incoming/message-${encodeURIComponent(event.messageId)}`;
      const created = await Tools.Files.mkdir(directory, true);
      if (!created.successful) throw new Error(created.details);
    }
    const filePath = `${directory}/${index}`;
    const downloaded = await Tools.Files.download(url, filePath);
    if (!downloaded.successful) throw new Error(downloaded.details);
    const info = await Tools.Files.info(filePath);
    const fileSize = integer(info.size, "downloaded attachment size", 0, Number.MAX_SAFE_INTEGER);
    if (fileSize > CHAT_ATTACHMENT_LIMIT) {
      oversized.push({ fileName: fileName.slice(0, 150), sizeBytes: fileSize });
      try {
        const deleted = await Tools.Files.deleteFile(filePath, false);
        if (!deleted.successful) console.error("[qqbot] Could not remove oversized attachment");
      } catch { console.error("[qqbot] Could not remove oversized attachment"); }
      continue;
    }
    const mimeType = item.voiceWavUrl ? "audio/wav" : item.mimeType === "voice" ? "audio/silk" : item.mimeType === "file" ? "application/octet-stream" : item.mimeType;
    files.push({ filePath, fileName, mimeType, fileSize, content: filePath });
  }
  return { files, oversized };
}
