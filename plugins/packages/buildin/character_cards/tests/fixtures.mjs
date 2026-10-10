/** Provides deterministic plugin-domain records exclusively for isolated browser and codec tests. */
export function fixture() {
  const base = { description: "", characterSetting: "你是一位可靠、温和的助手。", openingStatement: "你好，有什么可以帮你的？", otherContentChat: "", otherContentVoice: "", avatarUri: null, attachedTagIds: ["tag-persona"], advancedCustomPrompt: "", marks: "", chatModelBindingMode: "FOLLOW_GLOBAL", chatModelId: null, ttsConfigId: null, themeConfigId: null, memoryBindingMode: "CHARACTER", sharedMemoryId: null, sharedMemoryMounts: [], toolAccessConfig: { enabled: false, allowedBuiltinTools: [], allowedPackages: [], allowedSkills: [], allowedMcpServers: [] }, createdAt: 1, updatedAt: 1 };
  const snapshot = {
    cards: [
      { ...base, id: "operit", name: "Operit", description: "默认角色卡", isDefault: true },
      { ...base, id: "travel", name: "旅行助理", description: "一起规划行程，记录沿途的故事。", isDefault: false, memoryBindingMode: "SHARED", sharedMemoryId: "shared-main", attachedTagIds: ["tag-persona", "tag-travel"] },
    ],
    groups: [{ id: "group-one", name: "日常讨论", description: "多个角色一起交流", themeConfigId: null, members: [{ characterCardId: "operit", orderIndex: 0 }, { characterCardId: "travel", orderIndex: 1 }], createdAt: 2, updatedAt: 3 }],
    stores: [{ id: "shared-main", name: "共享记忆", createdAt: 2, updatedAt: 3 }],
    tags: [{ id: "tag-persona", name: "友好", description: "温和的表达", promptContent: "使用友好的语言。", tagType: "TONE", createdAt: 2, updatedAt: 3 }, { id: "tag-travel", name: "旅行", description: "旅行助手", promptContent: "帮助规划旅行。", tagType: "CUSTOM", createdAt: 2, updatedAt: 3 }],
    models: [{ providerId: "dashscope", providerName: "DashScope", providerTypeId: "openai", endpoint: "https://fixture.invalid/v1", modelId: "qwen-plus", capabilities: { directImage: true, directAudio: false, directVideo: false, toolCall: true }, pricing: { billingMode: "TOKEN", inputPricePerMillion: 2, cachedInputPricePerMillion: 1, cacheWritePricePerMillion: null, outputPricePerMillion: 8, pricePerRequest: 0, currency: "CNY" } }],
    ttsConfigs: [{ id: "tts-one", name: "测试语音", providerType: "OPENAI", endpoint: "https://fixture.invalid/tts", apiKey: "fixture-key", model: "fixture-model", voice: "fixture-voice", responseFormat: "mp3", speed: 1, httpMethod: "POST", requestBody: "{}", contentType: "application/json", headers: [{ name: "X-Fixture", value: "complete" }], responsePipeline: [], createdAt: 2, updatedAt: 3 }],
    toolCatalog: { builtinTools: [{ name: "fixture-builtin", displayName: "内置工具", description: "真实形状的测试目录" }], packages: [{ name: "fixture-package", displayName: "测试包", description: "测试目录" }], skills: [{ name: "fixture-skill", displayName: "测试技能", description: "测试目录" }], mcpServers: [{ name: "fixture-mcp", displayName: "测试 MCP", description: "测试目录" }] },
    active: { CharacterCard: { id: "operit" } },
  };
  const items = [
    { id: "1", uuid: "memory-one", title: "喜欢安静的旅行", content: "偏好步行探索城市，避免拥挤的景点。", contentType: "text/plain", source: "manual", folderPath: "个人偏好", tags: [{ id: "1", name: "旅行" }], credibility: 0.9, importance: 0.8, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: 1, updatedAt: 2, lastAccessedAt: 3, properties: [{ id: "1", key: "fixture", value: "preserved" }] },
    { id: "2", uuid: "memory-two", title: "饮食偏好", content: "喜欢清淡的食物。", contentType: "text/plain", source: "manual", folderPath: "个人偏好", tags: [], credibility: 0.8, importance: 0.6, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: 1, updatedAt: 2, lastAccessedAt: 3, properties: [{ id: "1", key: "fixture", value: "preserved" }] },
    { id: "3", uuid: "memory-three", title: "下次行程", content: "计划寻找适合徒步的路线。", contentType: "text/plain", source: "manual", folderPath: "计划", tags: [], credibility: 0.8, importance: 0.7, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: 1, updatedAt: 2, lastAccessedAt: 3, properties: [{ id: "1", key: "fixture", value: "preserved" }] },
  ];
  const graph = { items, graph: { nodes: items.map(
    /** Converts fixture memories into the declared plugin graph node shape, not a production data source. */
    item => ({ id: item.uuid, label: item.title, color: 0, metadata: {} }),
  ), edges: [{ id: "1", sourceId: "memory-one", targetId: "memory-three", label: "影响", weight: 0.8, metadata: { description: "必须保留的原始关系描述" }, isCrossFolderLink: true }] } };
  const theme = { brightness: "light", colors: { primary: "#6750a4ff", onPrimary: "#ffffffff", primaryContainer: "#eaddffff", onPrimaryContainer: "#21005dff", onSurface: "#1d1b20ff", onSurfaceVariant: "#49454fff", surface: "#fffaffff", surfaceContainer: "#f3edf7ff", surfaceContainerHigh: "#ece6f0ff", surfaceContainerHighest: "#e6e0e9ff", outline: "#79747eff", outlineVariant: "#cac4d0ff", error: "#b3261eff" } };
  const links = [{ id: "1", sourceMemoryId: "1", targetMemoryId: "3", type_: "影响", weight: 0.8, description: "必须保留的原始关系描述" }];
  const settings = { autoSaveIntervalMinutes: 15, nextAutoSaveRunAtMs: 12345, memoryExtractionCustomRules: "测试提取规则", profileAutoUpdateEnabled: true, profileAutoUpdateLocked: false, cloudEmbeddingEnabled: true, cloudEmbeddingEndpoint: "https://fixture.invalid/embeddings", cloudEmbeddingApiKey: "fixture-embedding-key", cloudEmbeddingModel: "fixture-embedding" };
  const searchConfig = { scoreMode: "BALANCED", keywordWeight: 0.4, tagWeight: 0.2, vectorWeight: 0.3, edgeWeight: 0.1 };
  const importedCard = { ...snapshot.cards[0], id: "fixture-imported-card", name: "隔离 UI 导入返回角色", isDefault: false, createdAt: 10, updatedAt: 10 };
  const importCharacterSnapshot = { ...snapshot, cards: [...snapshot.cards, importedCard] };
  const themeChoices = [{ id: "independent-theme-one", label: "测试独立主题" }, { id: "independent-theme-two", label: "测试夜间主题" }];
  return { snapshot, graph, theme, themeChoices, links, settings, searchConfig, importCharacterSnapshot };
}

/** Installs an isolated browser interface without supplying a production service or persistence implementation. */
export function installBrowserFixture(data) {
  /** Copies only the isolated fixture records. */
  const copy = value => JSON.parse(JSON.stringify(value));
  window.testCalls = [];
  window.testFailure = null;
  window.testRecords = copy(data.snapshot);
  window.testGraph = copy(data.graph);
  window.testProfile = "# 用户资料\n\n喜欢安静的旅行。";
  window.CharacterMemoryHost = {
    /** Supplies the explicit browser-test theme without proving host integration. */
    async currentTheme() { return copy(data.theme); },
    /** Implements only the finite test operations asserted by the browser suite. */
    async request(operation) {
      window.testCalls.push(copy(operation));
      if (window.testFailure === operation.action) throw new Error("TEST_HOST_REJECTED");
      switch (operation.action) {
        case "snapshot": return copy(window.testRecords);
        // These are explicitly labelled presentation rows, not real SDK Theme configs or host integration evidence.
        case "listThemeChoices": return copy(data.themeChoices);
        case "readUser": return { ownerKey: operation.ownerKey, content: window.testProfile };
        case "writeUser": window.testProfile = operation.content; return { saved: true };
        case "graph": return copy(window.testGraph);
        case "searchMemory": return [copy(window.testGraph.items[0])];
        case "saveCharacter": {
          const index = window.testRecords.cards.findIndex(
            /** Finds the exact test record being saved. */
            item => item.id === operation.card.id,
          );
          if (operation.create) window.testRecords.cards.push({ ...copy(operation.card), id: "created-card" });
          else window.testRecords.cards[index] = copy(operation.card);
          return copy(window.testRecords);
        }
        case "saveGroup": {
          // The test must declare its full expected response; no group CRUD or implicit host result is implemented here.
          window.testRecords = copy(data.groupSaveSnapshot); return copy(window.testRecords);
        }
        case "importCharacter": {
          // This predetermined response verifies the UI contract only; no import or storage business runs here.
          window.testRecords = copy(data.importCharacterSnapshot);
          return copy(window.testRecords);
        }
        case "activate": window.testRecords.active = operation.type === "card" ? { CharacterCard: { id: operation.id } } : { CharacterGroup: { id: operation.id } }; return copy(window.testRecords);
        case "createLink": {
          const source = window.testGraph.items.find(
            /** Resolves the relationship's exact source title in fixture data. */
            item => item.title === operation.sourceTitle,
          );
          const target = window.testGraph.items.find(
            /** Resolves the relationship's exact target title in fixture data. */
            item => item.title === operation.targetTitle,
          );
          window.testGraph.graph.edges.push({ id: "2", sourceId: source.uuid, targetId: target.uuid, label: operation.linkType, weight: operation.weight, metadata: { description: operation.description }, isCrossFolderLink: source.folderPath !== target.folderPath });
          return copy(window.testGraph);
        }
        case "saveMemory": {
          if (operation.originalTitle !== null) {
            const item = window.testGraph.items.find(
              /** Resolves the exact existing title selected in the editor test. */
              item => item.title === operation.originalTitle,
            );
            Object.assign(item, { title: operation.title, content: operation.content, contentType: operation.contentType, source: operation.source, credibility: operation.credibility, importance: operation.importance, folderPath: operation.folderPath });
            const node = window.testGraph.graph.nodes.find(
              /** Keeps the node's title synchronized with the updated fixture memory. */
              node => node.id === item.uuid,
            );
            node.label = operation.title; return copy(window.testGraph);
          }
          const item = { id: "4", uuid: "created-memory", title: operation.title, content: operation.content, contentType: operation.contentType, source: operation.source, credibility: operation.credibility, importance: operation.importance, folderPath: operation.folderPath, documentPath: null, isDocumentNode: false, chunkIndexFilePath: null, createdAt: 4, updatedAt: 4, lastAccessedAt: 4, properties: [], tags: [] };
          window.testGraph.items.push(item); window.testGraph.graph.nodes.push({ id: item.uuid, label: item.title, color: 0, metadata: {} });
          return copy(window.testGraph);
        }
        default: throw new Error(`Unexpected fixture operation: ${operation.action}`);
      }
    },
    /** Records explicit exports without writing outside the browser fixture. */
    // Labelled browser resource/picker adapters, not filesystem or domain implementations.
    async avatarImage() { return "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+a3ioAAAAASUVORK5CYII="; },
    async chooseAvatar() { return null; },
    async exportFile(path, content) { window.testCalls.push({ action: "exportFile", path, content }); return true; },
  };
}
