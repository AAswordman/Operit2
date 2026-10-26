import type { Chat } from "../../../../types/chat";
import { integer, nonblank, object, type InboundEvent } from "./model.js";

/** Downloads declared QQ attachments through Host files and preserves their Chat input metadata. */
export async function attachments(event: InboundEvent): Promise<Chat.SendAttachment[]> {
  if (event.rawPayload === undefined) throw new Error("QQ attachment processing requires the original dispatch payload");
  const body = object(event.rawPayload.d, "message.d");
  if (body.attachments === undefined) return [];
  if (!Array.isArray(body.attachments) || body.attachments.length > 10) throw new Error("QQ attachments must be an array of at most ten items");
  if (body.attachments.length === 0) return [];
  const directory = `${getPluginConfigDir()}/incoming/message-${encodeURIComponent(event.messageId)}`;
  const created = await Tools.Files.mkdir(directory, true);
  if (!created.successful) throw new Error(created.details);
  const result: Chat.SendAttachment[] = [];
  for (let index = 0; index < body.attachments.length; index++) {
    const item = object(body.attachments[index], `attachments[${index}]`);
    const url = nonblank(item.url, "attachment.url");
    if (!/^https:\/\//i.test(url)) throw new Error("QQ attachment URL must use HTTPS");
    const fileName = nonblank(item.filename, "attachment.filename");
    const mimeType = nonblank(item.content_type, "attachment.content_type");
    integer(item.size, "attachment.size", 0, 20 * 1024 * 1024);
    const filePath = `${directory}/${index}`;
    const downloaded = await Tools.Files.download(url, filePath);
    if (!downloaded.successful) throw new Error(downloaded.details);
    const info = await Tools.Files.info(filePath);
    result.push({ filePath, fileName, mimeType, fileSize: integer(info.size, "downloaded attachment size", 0, 20 * 1024 * 1024), content: filePath });
  }
  return result;
}
