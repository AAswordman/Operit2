import type { Chat } from "../../../../types/chat";
import { integer, nonblank, type InboundEvent } from "./model.js";
import { messageAttachments } from "./messages.js";

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
