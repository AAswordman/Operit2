import type { Models } from "../../../../types/models";
import type { Tts } from "../../../../types/tts";
import type { Catalog } from "../../../../types/catalog";
import type { CharacterDirectories } from "../src/model";

/** Derives controlled directory payloads from the generated tool result fields. */
type DirectoryApis = { models: typeof Models.listSummaries; tts: typeof Tts.listConfigs; tools: typeof Catalog.readToolSources };

/** Supplies explicit unit-test dependencies, not configured native-host directories or production defaults. */
export interface HostDirectoriesTestInput {
  models: Awaited<ReturnType<DirectoryApis["models"]>>["models"];
  ttsConfigs: Awaited<ReturnType<DirectoryApis["tts"]>>["configs"];
  toolCatalog: Awaited<ReturnType<DirectoryApis["tools"]>>;
}

type Equal<A, B> = [A] extends [B] ? [B] extends [A] ? true : false : false;
type Assert<T extends true> = T;

/** Checks the plugin's connected directory results against actual generated SDK declarations. */
export type HostDirectoryContracts = [
  Assert<Equal<Awaited<ReturnType<CharacterDirectories["listModels"]>>, HostDirectoriesTestInput["models"]>>,
  Assert<Equal<Awaited<ReturnType<CharacterDirectories["listTtsConfigs"]>>, HostDirectoriesTestInput["ttsConfigs"]>>,
  Assert<Equal<Awaited<ReturnType<CharacterDirectories["readToolCatalog"]>>, HostDirectoriesTestInput["toolCatalog"]>>,
];

/** Preserves complete nonempty model, speech and all four tool-source inputs at the test-only host boundary. */
export const hostDirectoriesTestInput: HostDirectoriesTestInput = {
  models: [
    {
      providerId: "fixture-provider-token",
      providerName: "Controlled token provider",
      providerTypeId: "fixture-provider-type",
      endpoint: "https://model-directory.example.invalid/v1",
      modelId: "fixture-model-token",
      capabilities: { directImage: true, directAudio: false, directVideo: true, toolCall: true },
      pricing: {
        billingMode: "TOKEN", inputPricePerMillion: 1.25, cachedInputPricePerMillion: null,
        cacheWritePricePerMillion: 0.4, outputPricePerMillion: 3.5, pricePerRequest: 0, currency: "USD",
      },
    },
    {
      providerId: "fixture-provider-unpriced",
      providerName: "Controlled unpriced provider",
      providerTypeId: "fixture-unpriced-type",
      endpoint: "https://unpriced-directory.example.invalid/api",
      modelId: "fixture-model-unpriced",
      capabilities: { directImage: false, directAudio: true, directVideo: false, toolCall: false },
      pricing: null,
    },
  ],
  ttsConfigs: [
    {
      id: "fixture-tts-config", name: "Controlled complete speech config", providerType: "fixture-tts-type",
      endpoint: "https://speech-directory.example.invalid/synthesize", apiKey: "fixture-nonsecret-key",
      model: "fixture-speech-model", voice: "fixture-voice", responseFormat: "wav", speed: 1.2,
      httpMethod: "POST", requestBody: '{"text":"{{text}}","voice":"{{voice}}"}', contentType: "application/json",
      headers: [{ name: "X-Fixture-Request", value: "complete-request-header" }],
      responsePipeline: [
        { stepType: "json", path: "audio.content", headers: [{ name: "X-Fixture-Response", value: "complete-response-header" }] },
      ],
      createdAt: 1720000000000, updatedAt: 1720000001000,
    },
  ],
  toolCatalog: {
    builtinTools: [{ name: "fixture.builtin.read", displayName: "Controlled builtin", description: "Complete builtin description" }],
    packages: [{ name: "fixture.package.tools", displayName: "Controlled package", description: "Complete package description" }],
    skills: [{ name: "fixture.skill.instructions", displayName: "Controlled skill", description: "Complete skill description" }],
    mcpServers: [{ name: "fixture.mcp.server", displayName: "Controlled MCP", description: "Complete MCP description" }],
  },
};
