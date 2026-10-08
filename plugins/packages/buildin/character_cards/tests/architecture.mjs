import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import path from "node:path";
import { mountedBusinessViolations } from "./mounted-paths.mjs";
import { uiProductionViolations } from "./ui-consumers.mjs";

const repository = fileURLToPath(new URL("../../../../../", import.meta.url));
const packageRoot = "plugins/packages/buildin/character_cards";
const coreScopes = [
  "core/crates/plugin/sdk/src", "core/crates/plugin/sdk/build.rs",
  "core/crates/plugin/javascript-bridge/src",
  "plugins/types/chat.d.ts", "plugins/types/index.d.ts", "plugins/types/toolpkg.d.ts",
];
const dedicatedNames = /\bCharacterCards(?:Host|Bridge|Runtime|Dto|DTO|Import|Character|Group|Tag|ActivePrompt|Memory|Model|Tts|Tool|Chat|Graph)[A-Za-z0-9_]*\b|\bCharacterCards[A-Za-z0-9_]*(?:Host|Bridge|Runtime|Dto|DTO|Import)[A-Za-z0-9_]*\b|\bcharacter_cards_(?:host|bridge|dto|runtime|import)[A-Za-z0-9_]*\b/;
const dedicatedModule = /(?:^|\/)(?:CharacterCards(?:Host|Bridge|Runtime|Dto|DTO|Import)[A-Za-z0-9_]*|character_cards_(?:host|bridge|dto|runtime|import)[A-Za-z0-9_]*)\.(?:rs|ts|js|mjs)$/;
const retiredFramework = /\b(?:HostProvider|PluginDataStore|SurfaceContribution)\b|\bToolPkg\.(?:data|sources)\b/;
const pluginSourceImport = /["'][^"'\r\n]*plugins[\\/]+packages[\\/]+buildin[\\/]+character_cards[\\/]+src[\\/]+[^"'\r\n]*["']/;

/** Enumerates actual source files deterministically without replacing absent scopes. */
function sourceFiles(directory) {
  const result = [];
  for (const entry of readdirSync(path.join(repository, directory), { withFileTypes: true })) {
    const relative = `${directory}/${entry.name}`;
    if (entry.isDirectory()) result.push(...sourceFiles(relative));
    else if (entry.isFile() && /\.(?:rs|ts|js|mjs)$/.test(entry.name)) result.push(relative);
  }
  return result.sort();
}

/** Reads the specified real source without treating missing files as successful evidence. */
function source(relative) { return readFileSync(path.join(repository, relative), "utf8"); }

/** Rejects dedicated contracts/imports in the exact SDK, bridge and declaration scopes. */
export function coreArchitectureViolations() {
  const violations = [];
  for (const scope of coreScopes) {
    const files = /\.(?:rs|ts)$/.test(scope) ? [scope] : sourceFiles(scope);
    for (const file of files) {
      const text = source(file), match = dedicatedNames.exec(text);
      if (match !== null) violations.push(`${file}: forbidden character-plugin-specific host/DTO/runtime contract ${match[0]}`);
      if (dedicatedModule.test(file)) violations.push(`${file}: dedicated Core module must be removed`);
      if (retiredFramework.test(text)) violations.push(`${file}: withdrawn provider/storage framework remains in production source`);
      if (pluginSourceImport.test(text)) violations.push(`${file}: directly imports character-plugin business source`);
    }
  }
  return violations;
}

/** Rejects withdrawn production globals, dedicated SDK DTO imports, and command-caller bridges. */
export function pluginArchitectureViolations() {
  const violations = [];
  for (const file of sourceFiles(`${packageRoot}/src`)) {
    const text = source(file);
    if (/\bTools\.SoftwareSettings\.exec\b/.test(text)) violations.push(`${file}: provider invokes the old CLI bridge`);
    if (retiredFramework.test(text)) violations.push(`${file}: uses a withdrawn provider/storage framework`);
    if (/import\s+(?:type\s+)?[^;]*?from\s*["'][^"']*(?:\/tests\/|\/fixtures(?:\.[a-z]+)?["'])/.test(text)) violations.push(`${file}: imports production test fixtures`);
    if (/\bTools\.Memory\b/.test(text)) violations.push(`${file}: provider depends on lossy tool-summary transport`);
    if (/\bCharacterCards\b/.test(text)) violations.push(`${file}: depends on a withdrawn plugin-specific host global`);
    if (/import\s+type\s*\{[^}]*\bCharacterCards[A-Z][A-Za-z0-9_]*[^}]*\}\s*from\s*["'][^"']*types\/chat["']/.test(text)) violations.push(`${file}: imports withdrawn dedicated SDK DTOs`);
  }
  return violations;
}

/** Returns every real architecture violation without introducing a replacement production dependency. */
export function architectureViolations() {
  return [...coreArchitectureViolations(), ...pluginArchitectureViolations(), ...mountedBusinessViolations(), ...uiProductionViolations()];
}
