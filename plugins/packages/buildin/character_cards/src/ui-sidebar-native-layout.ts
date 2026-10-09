import type { ComposeDslContext, ComposeNode } from '../../../../types/compose-dsl';
import type { CharacterSidebarSection, ConversationSidebarGroup, CurrentSidebar, SidebarCatalog, SidebarChatSummary } from './ui-sidebar';

export interface SidebarLayoutState {
  data: SidebarCatalog; query: string; collapsed: string[]; expandedGroups: string[];
}
export interface SidebarLayoutActions {
  activate(chatId: string): { type: string; chatId: string };
  toggleSection(id: string): void;
  toggleGroup(id: string): void;
  menu(kind: "group" | "chat" | "ungrouped", id: string): ComposeNode;
  longPress(id: string): void;
  rename(id: string): void;
  deleteChat(id: string): void;
  drop(chatId: string, scopeId: string, groupId: string | null, targetId?: string): Promise<void>;
}

/** Exact dimensions from the pre-migration native drawer, not browser CSS approximations. */
export const legacySidebarMetrics = Object.freeze({
  categoryStart: 20, categoryEnd: 12, categoryAvatar: 22, categoryRadius: 18,
  groupStart: 46, groupEnd: 12, groupRadius: 12,
  chatStart: 56, chatEnd: 12, chatHeight: 34, chatRadius: 8, railWidth: 20,
  createHeight: 34, createRadius: 17, previewLimit: 4,
});

/** Draws native role -> conversation group -> chat hierarchy with the legacy rounded surfaces. */
export function nativeSidebarRows(ctx: ComposeDslContext, current: CurrentSidebar, state: SidebarLayoutState,
  actions: SidebarLayoutActions, defaultAvatar: string | null): ComposeNode[] {
  const colors = ctx.MaterialTheme.colorScheme, m = legacySidebarMetrics;
  const query = state.query.trim().toLocaleLowerCase();
  const muted = colors.onSurfaceVariant.copy({ alpha: 0.80 });
  const rows: ComposeNode[] = [];
  const gap = (width: number): ComposeNode => ctx.UI.Spacer({ width });

  function more(kind: "chat" | "group", id: string, selected = false): ComposeNode {
    return ctx.UI.HoverOnly({ key: 'sidebar-' + kind + '-menu-' + id + '-visibility', alwaysVisible: selected },
      actions.menu(kind, id));
  }

  function chatRow(chat: SidebarChatSummary, section: CharacterSidebarSection, groupId: string | null): ComposeNode {
    const selected = current.chatSidebar.currentChatId === chat.id;
    const titleColor = selected ? colors.onSecondaryContainer : muted;
    const running = current.chatSidebar.activeStreamingChatIds.includes(chat.id);
    const status = ctx.UI.ActivityDots({ key: 'sidebar-status-' + chat.id, width: 10, height: 13.5,
      running, selected, restingColor: selected ? colors.onSecondaryContainer.copy({ alpha: 0.70 }) : colors.onSurfaceVariant.copy({ alpha: 0.30 }),
      activeColor: colors.primary });
    const content: ComposeNode[] = [ctx.UI.Draggable({ key: 'sidebar-chat-drag-' + chat.id, data: chat.id, dragType: 'character-sidebar.chat', enabled: true,
      feedback: ctx.UI.Row({ width: 220, height: 34, paddingHorizontal: 10, verticalAlignment: 'center', background: colors.surfaceContainerHighest, backgroundShape: { cornerRadius: 8 } },
        ctx.UI.Text({ text: chat.title, weight: 1, maxLines: 1, overflow: 'ellipsis', fontSize: 13, color: colors.onSurface })) }, status), gap(5), ctx.UI.Text({ key: 'sidebar-chat-title-' + chat.id, text: chat.title,
      weight: 1, fontSize: 13, fontWeight: selected ? '600' : '400', maxLines: 1, overflow: 'ellipsis', color: titleColor,
      ...{ letterSpacing: -0.1 } })];
    if (chat.pinned) content.push(gap(5), ctx.UI.Icon({ name: 'PushPin', size: 12, tint: titleColor.copy ? titleColor.copy({ alpha: 0.60 }) : titleColor }));
    if (chat.locked) content.push(gap(5), ctx.UI.Icon({ name: 'Lock', size: 12, tint: titleColor.copy ? titleColor.copy({ alpha: 0.60 }) : titleColor }));
    content.push(gap(4), more('chat', chat.id, selected));
    const surface = ctx.UI.Row({ key: 'sidebar-chat-hit-' + chat.id, weight: 1, height: 31,
      verticalAlignment: 'center', paddingHorizontal: 6,
      modifier: (selected ? ctx.Modifier.background(colors.secondaryContainer.copy({ alpha: 0.78 }), { cornerRadius: m.chatRadius })
        .border(1, colors.onSecondaryContainer.copy({ alpha: 0.08 }), { cornerRadius: m.chatRadius }) : ctx.Modifier).combinedClickable({ onClick: () => actions.activate(chat.id), onLongClick: () => actions.longPress(chat.id) }),
    }, content);
    const rail = ctx.UI.Box({ key: 'sidebar-chat-rail-' + chat.id, width: m.railWidth, height: m.chatHeight, contentAlignment: 'center' }, [
      ctx.UI.Box({ width: 1, height: m.chatHeight, background: colors.outlineVariant.copy({ alpha: 0.45 }) }),
      ...(selected ? [ctx.UI.Box({ width: 2, height: 16, background: colors.primary, backgroundShape: { cornerRadius: 1.5 } })] : []),
    ]);
    return ctx.UI.DragTarget({ key: 'sidebar-chat-drop-' + chat.id, acceptedTypes: ['character-sidebar.chat'],
      acceptedData: section.chats.filter(value => value.id !== chat.id).map(value => value.id),
      hoverBorderColor: colors.primary.copy({ alpha: 0.55 }), shape: { cornerRadius: 8 }, fillMaxWidth: true,
      onDrop: (id) => actions.drop(String(id), section.id, groupId, chat.id) },
    ctx.UI.Row({ key: 'sidebar-chat-' + chat.id, fillMaxWidth: true, height: m.chatHeight,
      paddingStart: m.chatStart, paddingEnd: m.chatEnd, verticalAlignment: 'center' }, [rail, gap(2),
      ctx.UI.HoverRegion({ weight: 1, hoverBackground: colors.onSurfaceVariant.copy({ alpha: 0.07 }), shape: { cornerRadius: m.chatRadius } },
        ctx.UI.SwipeActions({ key: 'sidebar-chat-swipe-' + chat.id, actionThreshold: 0.4,
          onStartAction: () => actions.rename(chat.id), onEndAction: () => actions.deleteChat(chat.id),
          startBackground: ctx.UI.Box({ fillMaxSize: true, paddingStart: 12, background: colors.primary, contentAlignment: 'start' },
            ctx.UI.Icon({ name: 'Edit', size: 18, tint: colors.onPrimary })),
          endBackground: ctx.UI.Box({ fillMaxSize: true, paddingEnd: 12, background: colors.error, contentAlignment: 'end' },
            ctx.UI.Icon({ name: 'Delete', size: 18, tint: colors.onError })),
        }, surface))]));
  }

  function chats(values: SidebarChatSummary[], id: string, section: CharacterSidebarSection, groupId: string | null, matchAll = false): ComposeNode[] {
    const matches = query === '' || matchAll ? values : values.filter(chat => chat.title.toLocaleLowerCase().includes(query));
    const expanded = state.expandedGroups.includes('preview:' + id);
    const preview = query !== '' || expanded ? matches : matches.filter((chat, index) => index < m.previewLimit || chat.pinned
      || current.chatSidebar.currentChatId === chat.id || current.chatSidebar.activeStreamingChatIds.includes(chat.id));
    const result = preview.map(chat => chatRow(chat, section, groupId));
    const hidden = matches.filter(chat => !preview.includes(chat)).length;
    if (query === '' && (hidden > 0 || expanded && matches.length > m.previewLimit)) result.push(ctx.UI.Row({
      key: 'sidebar-preview-' + id, fillMaxWidth: true, paddingStart: m.chatStart, paddingEnd: m.chatEnd, paddingTop: 2,
      height: 34, verticalAlignment: 'center', spacing: 8,
      onClick: () => actions.toggleGroup('preview:' + id),
    }, [ctx.UI.Icon({ name: expanded ? 'ExpandLess' : 'ExpandMore', size: 18, tint: colors.onSurfaceVariant.copy({ alpha: 0.72 }) }),
      ctx.UI.Text({ text: expanded ? '收起' : '展开更多 ' + hidden, style: 'labelMedium', fontWeight: '600', color: colors.onSurfaceVariant.copy({ alpha: 0.72 }) })]));
    return result;
  }

  function category(section: CharacterSidebarSection): ComposeNode {
    const collapsed = state.collapsed.includes(section.id);
    const avatar = section.kind === 'card' ? ctx.UI.Image({ key: 'sidebar-avatar-' + section.id,
      uri: section.avatarUri ?? defaultAvatar ?? '', width: m.categoryAvatar, height: m.categoryAvatar, contentScale: 'crop',
      contentDescription: section.title, modifier: ctx.Modifier.clip({ type: 'circle' }) })
      : ctx.UI.Box({ width: m.categoryAvatar, height: m.categoryAvatar, contentAlignment: 'center',
        background: colors.surfaceContainerLow.copy({ alpha: 0.72 }), backgroundShape: { type: 'circle' } },
      ctx.UI.Icon({ name: section.kind === 'group' ? 'Groups' : 'AccountTree', size: 14, tint: colors.onSurfaceVariant }));
    return ctx.UI.Column({ key: 'sidebar-section-' + section.id, fillMaxWidth: true,
      paddingStart: m.categoryStart, paddingEnd: m.categoryEnd, paddingTop: 10, paddingBottom: 5 },
      ctx.UI.DragTarget({ key: 'sidebar-category-drop-' + section.id, acceptedTypes: ['character-sidebar.chat'],
        acceptedData: section.chats.map(chat => chat.id), onDrop: (id) => actions.drop(String(id), section.id, null) },
      ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: 'center', height: 24, onClick: () => actions.toggleSection(section.id),
        modifier: ctx.Modifier.clip({ cornerRadius: m.categoryRadius }) }, [avatar, gap(8),
        ctx.UI.Text({ text: section.title, style: 'titleSmall', fontWeight: '700', maxLines: 1, overflow: 'ellipsis',
          modifier: ctx.Modifier.widthIn({ max: 170 }), color: colors.onSurface }), gap(8),
        ctx.UI.Text({ text: String(section.chats.length), style: 'labelSmall', fontWeight: '700', color: colors.onSurfaceVariant.copy({ alpha: 0.64 }) }),
        ctx.UI.GradientRule({ weight: 1, paddingHorizontal: 10, height: 2, startColor: colors.outlineVariant.copy({ alpha: 0.62 }), endColor: colors.outlineVariant.copy({ alpha: 0 }) }),
        ctx.UI.Icon({ name: collapsed ? 'KeyboardArrowDown' : 'KeyboardArrowUp', size: 23, tint: colors.onSurfaceVariant.copy({ alpha: 0.78 }) }),
      ])));
  }

  function groupRow(group: ConversationSidebarGroup, section: CharacterSidebarSection, ungrouped = false): ComposeNode {
    const expanded = !state.collapsed.includes('group:' + group.id);
    const groupBody = ctx.UI.Row({ fillMaxWidth: true, verticalAlignment: 'center', paddingStart: 12, paddingEnd: 8,
      paddingTop: 6, paddingBottom: 6, ...({background: colors.surfaceContainerLow.copy({ alpha: 0.72 }), backgroundShape: {cornerRadius: m.groupRadius}, modifier: ctx.Modifier.border(1, colors.outlineVariant.copy({alpha: 0.24}), {cornerRadius: m.groupRadius})}), onClick: () => actions.toggleSection('group:' + group.id) }, [
      ctx.UI.Icon({ name: 'FolderOutlined', size: 16, tint: colors.onSurfaceVariant.copy({ alpha: 0.76 }) }), gap(7),
      ctx.UI.Text({ text: group.name, weight: 1, style: 'labelLarge', fontWeight: '700', maxLines: 1, overflow: 'ellipsis', color: colors.onSurface.copy({ alpha: 1 }) }),
      ...(group.pinned ? [gap(4), ctx.UI.Icon({ name: 'PushPinRounded', size: 12, tint: colors.onSurfaceVariant.copy({ alpha: 0.65 }) })] : []),
      gap(2), ungrouped ? ctx.UI.HoverOnly({ width: 24, height: 24 }, actions.menu('ungrouped', section.id)) : more('group', group.id), gap(2),
      ctx.UI.Icon({ name: expanded ? 'KeyboardArrowUp' : 'KeyboardArrowDown', size: 20, tint: colors.onSurfaceVariant.copy({ alpha: 0.62 }) }),
    ]);
    return ctx.UI.Column({ key: 'sidebar-group-' + group.id, fillMaxWidth: true, paddingStart: m.groupStart,
      paddingEnd: m.groupEnd, paddingTop: 4, paddingBottom: expanded ? 2 : 0 },
      ctx.UI.DragTarget({ key: 'sidebar-group-drop-' + group.id, acceptedTypes: ['character-sidebar.chat'],
        acceptedData: section.chats.map(chat => chat.id), hoverBorderColor: colors.primary.copy({ alpha: 0.55 }), shape: { cornerRadius: m.groupRadius },
        onDrop: (id) => actions.drop(String(id), section.id, ungrouped ? null : group.id) },
      ctx.UI.HoverRegion({ shape: { cornerRadius: m.groupRadius } }, groupBody)));
  }

  const sections: CharacterSidebarSection[] = state.data.view === 'characters' ? state.data.sections : state.data.scopes.map(scope => ({
    id: scope.id, title: scope.title, kind: scope.kind ?? 'unbound', selection: scope.ownerSelection, avatarUri: scope.avatarUri ?? null,
    chats: [...scope.ungrouped, ...scope.groups.flatMap(group => group.chats)], conversationGroups: scope.groups, ungroupedChats: scope.ungrouped,
  }));
  for (const section of sections) {
    const groups = section.conversationGroups ?? [], ungrouped = section.ungroupedChats ?? section.chats;
    if (section.chats.length === 0 && groups.length === 0) continue;
    const categoryMatches = query === '' || section.title.toLocaleLowerCase().includes(query);
    const visibleGroups = groups.filter(group => categoryMatches || group.name.toLocaleLowerCase().includes(query) || group.chats.some(chat => chat.title.toLocaleLowerCase().includes(query)));
    if (!categoryMatches && visibleGroups.length === 0 && !ungrouped.some(chat => chat.title.toLocaleLowerCase().includes(query))) continue;
    rows.push(category(section));
    if (state.collapsed.includes(section.id)) continue;
    for (const group of visibleGroups) {
      rows.push(groupRow(group, section));
      if (!state.collapsed.includes('group:' + group.id)) rows.push(...chats(group.chats, group.id, section, group.id, categoryMatches || group.name.toLocaleLowerCase().includes(query)));
    }
    if (ungrouped.length > 0 && (categoryMatches || '未分组'.includes(query) || ungrouped.some(chat => chat.title.toLocaleLowerCase().includes(query)))) {
      const id = 'ungrouped:' + section.id;
      rows.push(groupRow({ id, name: '未分组', pinned: ungrouped.every(chat => chat.pinned), displayOrder: 0, chats: ungrouped }, section, true));
      if (!state.collapsed.includes('group:' + id)) rows.push(...chats(ungrouped, id, section, null, categoryMatches || '未分组'.includes(query)));
    }
  }
  if (rows.length === 0) rows.push(ctx.UI.Text({ text: query === '' ? '暂无会话' : '没有匹配的会话', color: muted, fontSize: 12, paddingHorizontal: 20, paddingVertical: 12 }));
  return rows;
}
