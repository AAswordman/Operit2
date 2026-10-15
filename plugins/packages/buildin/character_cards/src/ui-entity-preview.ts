import type { ComposeDslContext, ComposeNode } from "../../../../types/compose-dsl";
import type { Card, Group, Snapshot, TagEdit } from "./model";
import type { ThemeChoice } from "./selection-application";

interface PreviewContent {
  card: Card | null;
  group: Group | null;
  snapshot: Snapshot;
  tags: readonly TagEdit[];
  avatar: string | null;
  themes: readonly ThemeChoice[] | null;
  themeError: string;
  configurationExpanded: boolean;
  onToggleConfiguration(): void;
}

/** Builds a reading view from the same saved records using only generic DSL nodes. */
export function renderEntityPreview(ctx: ComposeDslContext, input: PreviewContent): ComposeNode[] {
  const colors = ctx.MaterialTheme.colorScheme;
  const entity = input.card ?? input.group;
  if (entity === null) return [];
  const card = input.card;
  const selectable = (key: string, value: string, subdued = false): ComposeNode =>
    ctx.UI.SelectionContainer({ key }, ctx.UI.Text({ text: value, fillMaxWidth: true, style: "bodyMedium", color: subdued ? colors.onSurfaceVariant : colors.onSurface }));
  const pill = (key: string, label: string): ComposeNode => ctx.UI.Box({ key, paddingHorizontal: 10, paddingVertical: 5,
    backgroundColor: colors.primaryContainer.copy({ alpha: 0.38 }), backgroundShape: { cornerRadius: 8 },
  }, ctx.UI.Text({ text: label, style: "labelMedium", color: colors.onPrimaryContainer }));
  const attachedTags = card === null ? [] : input.tags.filter(tag => card.attachedTagIds.includes(tag.id));
  const identity = ctx.UI.Column({ key: "entity-preview-identity", fillMaxWidth: true, spacing: 14, paddingBottom: 22 }, [
    ctx.UI.Row({ fillMaxWidth: true, spacing: 16, verticalAlignment: "center" }, [
      card === null
        ? ctx.UI.Box({ width: 72, height: 72, contentAlignment: "center", backgroundColor: colors.primaryContainer.copy({ alpha: 0.48 }), backgroundShape: { cornerRadius: 20 } }, ctx.UI.Icon({ name: "GroupsOutlined", size: 34, tint: colors.primary }))
        : ctx.UI.Image({ key: "entity-avatar", ...(card.avatarUri === null ? { path: input.avatar ?? undefined } : { uri: card.avatarUri }), width: 72, height: 72,
          contentScale: "crop", contentDescription: card.name, modifier: ctx.Modifier.clip({ cornerRadius: 20 }) }),
      ctx.UI.Column({ weight: 1, spacing: 6 }, [
        ctx.UI.Text({ key: "entity-preview-name", text: entity.name, style: "headlineSmall", fontSize: 24, fontWeight: "700", fillMaxWidth: true }),
        ...(card === null || card.isDefault ? [ctx.UI.Text({ text: card === null ? input.group!.members.length + " 位角色" : "默认角色卡", style: "labelMedium", color: colors.onSurfaceVariant })] : []),
      ]),
    ]),
    ...(entity.description.trim() === "" ? [] : [selectable("entity-preview-description", entity.description, true)]),
    ...(attachedTags.length === 0 ? [] : [ctx.UI.FlowRow({ key: "entity-preview-tags", spacing: 6, runSpacing: 6 }, attachedTags.map(tag => pill("entity-preview-tag-" + tag.id, tag.name)))]),
  ]);
  const contentSection = (key: string, title: string, value: string, quote = false): ComposeNode[] => value.trim() === "" ? [] : [
    ctx.UI.Box({ key: "entity-preview-" + key, fillMaxWidth: true, paddingBottom: 22 },
      ctx.UI.Column({ fillMaxWidth: true, spacing: 10,
        ...(quote ? { padding: 18, backgroundColor: colors.primaryContainer.copy({ alpha: 0.22 }), backgroundShape: { cornerRadius: 16 } } : {}),
      }, [
        ctx.UI.Row({ fillMaxWidth: true, spacing: 8, verticalAlignment: "center" }, [
          ...(quote ? [ctx.UI.Icon({ name: "FormatQuote", size: 20, tint: colors.primary })] : []),
          ctx.UI.Text({ text: title, style: "titleSmall", fontWeight: "600", color: quote ? colors.primary : colors.onSurfaceVariant, weight: 1 }),
        ]),
        selectable("entity-preview-" + key + "-text", value),
      ])),
  ];
  const themeLabel = entity.themeConfigId === null ? "跟随当前主题" : input.themeError !== "" ? "主题名称暂时无法读取" : input.themes === null ? "正在读取主题名称…" : input.themes.find(theme => theme.id === entity.themeConfigId)?.label ?? "主题配置已不可用";
  const rows: { key: string; label: string; value: string }[] = [];
  const contents: ComposeNode[] = [identity];
  if (card !== null) {
    contents.push(...contentSection("opening", "开场白", card.openingStatement, true), ...contentSection("setting", "角色设定", card.characterSetting),
      ...contentSection("chat", "聊天附加内容", card.otherContentChat), ...contentSection("voice", "语音附加内容", card.otherContentVoice),
      ...contentSection("prompt", "高级自定义 Prompt", card.advancedCustomPrompt), ...contentSection("notes", "备注", card.marks));
    const model = input.snapshot.models.find(model => model.modelId === card.chatModelId);
    rows.push({ key: "model", label: "聊天模型", value: card.chatModelBindingMode === "FIXED_MODEL" ? model === undefined ? "固定模型已不可用" : model.providerName + " · " + model.modelId : "跟随全局模型" },
      { key: "theme", label: "主题", value: themeLabel },
      { key: "tts", label: "语音", value: card.ttsConfigId === null ? "跟随全局语音配置" : input.snapshot.ttsConfigs.find(config => config.id === card.ttsConfigId)?.name ?? "语音配置已不可用" },
      { key: "memory", label: "记忆库", value: card.memoryBindingMode === "SHARED" ? input.snapshot.stores.find(store => store.id === card.sharedMemoryId)?.name ?? "共享记忆库已不可用" : "角色独立记忆库" });
    if (card.sharedMemoryMounts.length !== 0) rows.push({ key: "mounts", label: "共享记忆挂载", value: card.sharedMemoryMounts.map(mount => {
      const name = input.snapshot.stores.find(store => store.id === mount.sharedMemoryId)?.name ?? "记忆库已不可用";
      const permissions = [mount.readable ? "可读" : "", mount.writable ? "可写" : ""].filter(Boolean).join("、") || "无访问权限";
      return name + "（" + permissions + "）";
    }).join("\n") });
    const policy = card.toolAccessConfig;
    const toolNames = [
      { ids: policy.allowedBuiltinTools, catalog: input.snapshot.toolCatalog.builtinTools },
      { ids: policy.allowedPackages, catalog: input.snapshot.toolCatalog.packages },
      { ids: policy.allowedSkills, catalog: input.snapshot.toolCatalog.skills },
      { ids: policy.allowedMcpServers, catalog: input.snapshot.toolCatalog.mcpServers },
    ].flatMap(source => source.ids.map(id => source.catalog.find(item => item.name === id)?.displayName ?? "来源已不可用"));
    rows.push({ key: "tools", label: "工具访问", value: !policy.enabled ? "跟随全局工具配置" : toolNames.length === 0 ? "未允许任何工具来源" : toolNames.join("、") });
  } else {
    const members = [...input.group!.members].sort((a, b) => a.orderIndex - b.orderIndex);
    contents.push(ctx.UI.Column({ key: "entity-preview-members", fillMaxWidth: true, spacing: 10, paddingBottom: 22 }, [
      ctx.UI.Text({ text: "组内角色", style: "titleSmall", fontWeight: "600", color: colors.onSurfaceVariant }),
      ...(members.length === 0 ? [ctx.UI.Text({ text: "暂未添加角色", style: "bodyMedium", color: colors.onSurfaceVariant })] : members.map((member, index) => {
        const record = input.snapshot.cards.find(card => card.id === member.characterCardId);
        return ctx.UI.Row({ key: "entity-preview-member-" + member.characterCardId, fillMaxWidth: true, paddingVertical: 10, spacing: 12, verticalAlignment: "center" }, [
          ctx.UI.Text({ text: String(index + 1).padStart(2, "0"), style: "labelMedium", color: colors.onSurfaceVariant }),
          ctx.UI.Image({ width: 40, height: 40, ...(record?.avatarUri == null ? { path: input.avatar ?? undefined } : { uri: record.avatarUri }), contentScale: "crop", contentDescription: record?.name ?? "角色已移除", modifier: ctx.Modifier.clip({ cornerRadius: 12 }) }),
          ctx.UI.Column({ weight: 1, spacing: 4 }, [ctx.UI.Text({ text: record?.name ?? "角色已移除", style: "titleSmall", fontWeight: "600", fillMaxWidth: true }),
            ...(record?.description.trim() ? [ctx.UI.Text({ text: record.description, style: "bodySmall", color: colors.onSurfaceVariant, fillMaxWidth: true })] : [])]),
        ]);
      })),
    ]));
    rows.push({ key: "theme", label: "主题", value: themeLabel });
  }
  contents.push(ctx.UI.Column({ key: "entity-preview-config", fillMaxWidth: true, backgroundColor: colors.surfaceContainerLow, backgroundShape: { cornerRadius: 16 } }, [
    ctx.UI.Surface({ key: "entity-preview-config-toggle", fillMaxWidth: true, color: "transparent", shape: { cornerRadius: 16 }, onClick: input.onToggleConfiguration },
      ctx.UI.Row({ fillMaxWidth: true, padding: 16, spacing: 10, verticalAlignment: "center" }, [
        ctx.UI.Icon({ name: "Tune", size: 20, tint: colors.onSurfaceVariant }),
        ctx.UI.Column({ weight: 1, spacing: 3 }, [ctx.UI.Text({ text: "配置摘要", style: "titleSmall", fontWeight: "600" }),
          ctx.UI.Text({ text: card === null ? "主题配置" : "模型、语音与记忆", style: "bodySmall", color: colors.onSurfaceVariant })]),
        ctx.UI.Icon({ name: input.configurationExpanded ? "ExpandLess" : "ExpandMore", size: 20, tint: colors.onSurfaceVariant }),
      ])),
    ...(input.configurationExpanded ? [ctx.UI.Column({ key: "entity-preview-config-details", fillMaxWidth: true, paddingStart: 16, paddingEnd: 16, paddingBottom: 18, spacing: 14 },
      rows.map(row => ctx.UI.Column({ key: "entity-preview-config-" + row.key, fillMaxWidth: true, spacing: 4 }, [
        ctx.UI.Text({ text: row.label, style: "labelMedium", color: colors.onSurfaceVariant }), selectable("entity-preview-config-" + row.key + "-value", row.value),
      ])))] : []),
  ]));
  return contents;
}
