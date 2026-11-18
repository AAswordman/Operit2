import type { Chat } from "../../../../types/chat";
import { integer, nonblank, type InboundEvent } from "./model.js";
import { messageAttachments } from "./messages.js";
import type { ChannelAttachment } from "./channel-api.js";

export const CONSOLE_LIMIT = 8 * 1024 * 1024;
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

/** Downloads declared QQ attachments through Host files and preserves their Chat input metadata. */
export async function attachments(event: InboundEvent): Promise<Chat.SendAttachment[]> {
  const declared = messageAttachments(event);
  if (declared.length === 0) return [];
  const directory = `${getPluginConfigDir()}/incoming/message-${encodeURIComponent(event.messageId)}`;
  const created = await Tools.Files.mkdir(directory, true);
  if (!created.successful) throw new Error(created.details);
  const result: Chat.SendAttachment[] = [];
  for (let index = 0; index < declared.length; index++) {
    const item = declared[index];
    const url = nonblank(item.voiceWavUrl ?? item.url, "attachment.url");
    if (!/^https:\/\//i.test(url)) throw new Error("QQ attachment URL must use HTTPS");
    const fileName = item.voiceWavUrl ? item.fileName.replace(/\.[^.]*$/, "") + ".wav" : nonblank(item.fileName, "attachment.filename");
    const mimeType = item.voiceWavUrl ? "audio/wav" : item.mimeType === "voice" ? "audio/silk" : item.mimeType === "file" ? "application/octet-stream" : item.mimeType;
    if (item.size !== undefined) integer(item.size, "attachment.size", 0, 20 * 1024 * 1024);
    const filePath = `${directory}/${index}`;
    const downloaded = await Tools.Files.download(url, filePath);
    if (!downloaded.successful) throw new Error(downloaded.details);
    const info = await Tools.Files.info(filePath);
    result.push({ filePath, fileName, mimeType, fileSize: integer(info.size, "downloaded attachment size", 0, 20 * 1024 * 1024), content: filePath });
  }
  return result;
}
