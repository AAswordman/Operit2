import type { Card, Group } from "./model";
import { groupAvatarLayout, groupPortrait, type GroupPortrait } from "./group-avatar";

const portraits = new Map<string, Promise<string>>();

/** Reads the explicitly selected file, data URI or HTTP image through shared host services. */
async function imageBytes(uri: string): Promise<string> {
  const inline = /^data:image\/[^;,]+;base64,(.+)$/.exec(uri);
  if (inline !== null) return inline[1];
  if (/^https?:\/\//i.test(uri)) {
    const response = await Tools.Net.http({ url: uri, method: "GET", responseType: "arraybuffer", validateStatus: true });
    if (typeof response.contentBase64 !== "string") throw new Error("头像请求未返回图片内容：" + uri);
    return response.contentBase64;
  }
  return (await Tools.Files.readBinary(uri)).contentBase64;
}

/** Composites the native image handles and releases every allocation even when decoding or drawing fails. */
async function composePortrait(portrait: GroupPortrait): Promise<string> {
  const size = 300, layout = groupAvatarLayout(portrait.members.length, size);
  const canvas = await Jimp.create(size, size);
  try {
    const defaultAvatar = portrait.members.some(member => member.avatarUri === null)
      ? await ToolPkg.readResource("character_default_avatar", "operit-avatar.png") : null;
    let index = 0;
    for (const row of layout.rows) for (let column = 0; column < row.count; column++) {
      const member = portrait.members[index++], uri = member.avatarUri ?? defaultAvatar!;
      const source = await Jimp.read(await imageBytes(uri));
      try {
        const tile = await source.cover(Math.round(layout.tile), Math.round(layout.tile));
        try { await canvas.composite(tile, Math.round(row.x + column * (layout.tile + layout.gap)), Math.round(row.y)); }
        finally { await tile.release(); }
      } finally { await source.release(); }
    }
    return "data:image/png;base64," + await canvas.getBase64(Jimp.MIME_PNG);
  } finally { await canvas.release(); }
}

/** Publishes the same derived portrait to generic host identity APIs without persisting generated files. */
export async function groupAvatarUri(group: Group, cards: readonly Card[]): Promise<string | null> {
  const portrait = groupPortrait(group, cards);
  if (portrait.uri !== null) return portrait.uri;
  if (portrait.members.length === 0) return null;
  const key = JSON.stringify(portrait.members);
  let pending = portraits.get(key);
  if (pending === undefined) {
    pending = composePortrait(portrait);
    portraits.set(key, pending);
    if (portraits.size > 64) portraits.delete(portraits.keys().next().value!);
    void pending.catch(() => { if (portraits.get(key) === pending) portraits.delete(key); });
  }
  return pending;
}
