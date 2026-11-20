import type { CharacterStorage } from "./preferences";
import { recordPrefix } from "./preferences";
import { assertObject } from "../validation";

/** Copies legacy configuration avatars into shared data before publishing their new references. */
export async function migrateCharacterAvatars(storage: CharacterStorage): Promise<void> {
  const sourceDirectory = ToolPkg.getConfigDir().replace(/\/$/, "") + "/avatars/";
  const targetDirectory = ToolPkg.getSpaceDataDir().replace(/\/$/, "") + "/avatars";
  if (sourceDirectory === targetDirectory + "/") return;
  const cards = storage.configuration.cards;
  const updates: { key: string; source: string; target: string }[] = [];
  const copied = new Map<string, string>();
  for (const [key, value] of (await cards.data()).entries()) {
    if (!key.startsWith(recordPrefix)) continue;
    assertObject(value, "saved character");
    const source = value.avatarUri;
    if (typeof source !== "string" || !source.startsWith(sourceDirectory)) continue;
    const name = source.slice(sourceDirectory.length);
    if (!/^[^/\\\0]+$/.test(name) || name === "." || name === "..") throw new Error("Invalid legacy avatar path: " + source);
    let target = copied.get(source);
    if (target === undefined) {
      const original = await Tools.Files.exists(source);
      if (!original.exists) {
        console.warn("[character_cards] 旧头像文件缺失，保留原引用；请在保有图片的原设备升级或重新选择头像：" + source);
        continue;
      }
      if (original.isDirectory) throw new Error("Legacy avatar is a directory: " + source);
      target = targetDirectory + "/" + name;
      const existing = await Tools.Files.exists(target);
      if (existing.exists) {
        const [before, after] = await Promise.all([Tools.Files.readBinary(source), Tools.Files.readBinary(target)]);
        if (before.contentBase64 !== after.contentBase64) throw new Error("Shared avatar conflicts with its legacy source: " + target);
      } else {
        const created = await Tools.Files.mkdir(targetDirectory, true);
        if (!created.successful) throw new Error(created.details);
        const result = await Tools.Files.copy(source, target);
        if (!result.successful) throw new Error(result.details);
      }
      copied.set(source, target);
    }
    updates.push({ key, source, target });
  }
  if (updates.length === 0) return;
  await cards.edit(preferences => {
    for (const update of updates) {
      const current = preferences.get(update.key);
      assertObject(current, "saved character");
      if (current.avatarUri !== update.source) throw new Error("Character avatar changed during migration: " + update.key);
      preferences.set(update.key, { ...current, avatarUri: update.target });
    }
  });
}
