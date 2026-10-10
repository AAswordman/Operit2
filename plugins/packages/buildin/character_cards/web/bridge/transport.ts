import type { CharacterMemoryBridge } from "./contracts";

/** Requires the sole registered bridge instead of selecting another transport. */
export function host(): CharacterMemoryBridge {
  const bridge = window.CharacterMemoryHost;
  if (bridge === undefined) throw new Error("CharacterMemoryHost 尚未注册");
  const methods: readonly (keyof CharacterMemoryBridge)[] = ["currentTheme", "currentScreen", "completeScreen", "cancelScreen", "request", "exportFile", "avatarImage", "chooseAvatar"];
  for (const method of methods) if (typeof bridge[method] !== "function") throw new Error(`CharacterMemoryHost.${method} 尚未注册`);
  return bridge;
}

/** Waits for the existing WebView interface registration, then requires its complete declared contract. */
export async function waitForHost(): Promise<void> {
  const deadline = performance.now() + 5000;
  while (window.CharacterMemoryHost === undefined) {
    if (performance.now() >= deadline) throw new Error("CharacterMemoryHost 尚未注册，请从应用中的角色卡插件打开此页面。");
    /** Waits one registration frame without selecting an alternate transport. */
    await new Promise<void>(resolve => setTimeout(resolve, 20));
  }
  host();
}
