import type { CharacterRepository } from "./canonical";
import type { Card } from "./model";
import { assertStrings, assertTag, normalizeNames } from "./validation";

/** Composes a loaded character's prompt using only its explicitly attached and additional tags. */
export async function combineCharacterPrompt(
  card: Card,
  additionalTagIds: string[],
  promptFunctionType: "CHAT" | "VOICE",
  host: Pick<CharacterRepository, "getTag">,
): Promise<string> {

  assertStrings(additionalTagIds, "附加标签");
  if (promptFunctionType !== "CHAT" && promptFunctionType !== "VOICE") throw new Error("提示词功能类型无效");
  const parts: string[] = [];
  const setting = card.characterSetting.trim();
  if (setting !== "") parts.push(setting);
  const otherContent = (promptFunctionType === "CHAT" ? card.otherContentChat : card.otherContentVoice).trim();
  if (otherContent !== "") parts.push(otherContent);
  for (const id of normalizeNames([...card.attachedTagIds, ...additionalTagIds])) {
    const tag = await host.getTag(id);
    assertTag(tag);
    if (tag.id !== id) throw new Error("Prompt tag read returned a different identity: " + id);
    const content = tag.promptContent.trim();
    if (content !== "") parts.push(content);
  }
  const advanced = card.advancedCustomPrompt.trim();
  if (advanced !== "") parts.push(advanced);
  return parts.join("\n\n").trim();
}
